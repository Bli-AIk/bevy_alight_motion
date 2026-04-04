//! Keeps RTT cameras and render textures synchronized with their embed scenes.
//! It updates camera placement, projection, dynamic-resolution sizing, and the
//! matching sprite or mesh representation when an embed scene moves or scales.
//!
//! 负责让 RTT 相机和渲染纹理持续与嵌套场景同步。它会在 embed scene 发生
//! 位移或缩放时，更新相机位置、投影、动态分辨率尺寸，以及与之配套的 sprite 或 mesh
//! 表现。

use bevy::prelude::*;
use bevy::render::render_resource::Extent3d;
use bevy::sprite::Anchor;

use super::{
    EMBED_RTT_CAMERA_Z, EmbedSceneRtt, EmbedSceneRttCamera, EmbedSceneRttCaptureRoot,
    compute_embed_visible_rect, resize_render_texture, scene_local_rect,
    sync_dynamic_resolution_mesh, sync_dynamic_resolution_sprite, transformed_rect_edge_lengths,
};

fn parented_camera_uses_local_projection() -> bool {
    std::env::var_os("AM_PARENT_RTT_LOCAL_PROJECTION").is_some()
}

fn unparented_camera_uses_full_scale() -> bool {
    std::env::var_os("AM_RTT_CAMERA_FULL_SCALE").is_some()
}

fn flatten_parented_rtt_to_world_enabled() -> bool {
    std::env::var_os("AM_FLATTEN_PARENTED_RTT_TO_WORLD").is_some()
}

fn keep_full_resolution_for_group_fill(group_fill: Option<&crate::effects::AmGroupFill>) -> bool {
    let keep_gradient_only =
        std::env::var_os("AM_GROUP_FILL_GRADIENT_ONLY_FULL_RESOLUTION").is_some();

    group_fill.is_some_and(|fill| {
        if keep_gradient_only {
            matches!(
                fill.fill_type,
                crate::effects::GroupFillType::Gradient { .. }
            )
        } else {
            fill.fill_type != crate::effects::GroupFillType::None
        }
    })
}

fn sign_axis(value: f32) -> f32 {
    if value.is_sign_negative() { -1.0 } else { 1.0 }
}

/// After RTT sync overwrites the Mesh2d geometry, re-synchronize the material's
/// `original_size` so the shader sees dimensions that match the actual mesh.
/// Without this, the unified effect system's scale-baked values would persist
/// and cause sampling artefacts in the GPU shader.
fn sync_embed_material_original_size(
    unified_materials: &mut Assets<crate::masked_sprite::UnifiedEffectMaterial>,
    mat_handle: Option<&MeshMaterial2d<crate::masked_sprite::UnifiedEffectMaterial>>,
    rtt: &EmbedSceneRtt,
    mesh_rect: super::EmbedVisibleRect,
) {
    let Some(mat_handle) = mat_handle else { return };
    let mesh_w = mesh_rect.width();
    let mesh_h = mesh_rect.height();
    let new_size = Vec4::new(rtt.scene_width, rtt.scene_height, mesh_w, mesh_h);
    // Check via immutable access first to avoid a spurious AssetEvent::Modified.
    let needs_update = unified_materials
        .get(&mat_handle.0)
        .is_some_and(|m| m.uniform_data.original_size != new_size);
    if !needs_update {
        return;
    }
    if let Some(material) = unified_materials.get_mut(&mat_handle.0) {
        material.uniform_data.original_size = new_size;
    }
}

fn projection_size_for_rtt_camera(
    parent_camera_to_embed: bool,
    visible_size: Vec2,
    effective_size: Vec2,
) -> Vec2 {
    let use_visible_projection = (parent_camera_to_embed
        && parented_camera_uses_local_projection())
        || (!parent_camera_to_embed && unparented_camera_uses_full_scale());
    if use_visible_projection {
        visible_size
    } else {
        effective_size
    }
}

pub fn sync_rtt_capture_root_system(
    mut commands: Commands,
    embed_query: Query<
        (
            Entity,
            &EmbedSceneRtt,
            &Transform,
            &GlobalTransform,
            Option<&Children>,
        ),
        Without<EmbedSceneRttCaptureRoot>,
    >,
    mut capture_root_query: Query<
        &mut Transform,
        (With<EmbedSceneRttCaptureRoot>, Without<EmbedSceneRtt>),
    >,
    camera_query: Query<(), With<EmbedSceneRttCamera>>,
    content_marker_query: Query<&crate::scene::AmEmbedContentMarker>,
) {
    let flatten_parented_rtt_to_world = flatten_parented_rtt_to_world_enabled();

    for (embed_entity, rtt, embed_transform, embed_global, children) in embed_query.iter() {
        let Some(capture_root) = rtt.capture_root else {
            continue;
        };

        if let Ok(mut capture_root_transform) = capture_root_query.get_mut(capture_root) {
            if flatten_parented_rtt_to_world {
                let (global_scale, global_rotation, global_translation) =
                    embed_global.to_scale_rotation_translation();
                capture_root_transform.translation = global_translation;
                capture_root_transform.rotation = global_rotation;
                capture_root_transform.scale = Vec3::new(global_scale.x, global_scale.y, 1.0);
            } else {
                capture_root_transform.translation = Vec3::ZERO;
                capture_root_transform.rotation = Quat::IDENTITY;
                capture_root_transform.scale = Vec3::new(
                    sign_axis(embed_transform.scale.x),
                    sign_axis(embed_transform.scale.y),
                    1.0,
                );
            }
        }

        let Some(children) = children else {
            continue;
        };

        for child in children.iter() {
            if child == capture_root || camera_query.get(child).is_ok() {
                continue;
            }

            let Ok(marker) = content_marker_query.get(child) else {
                continue;
            };

            if marker.embed_entity == embed_entity {
                commands.entity(capture_root).add_child(child);
            }
        }
    }
}

pub fn sync_rtt_camera_position_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut unified_materials: ResMut<Assets<crate::masked_sprite::UnifiedEffectMaterial>>,
    mut embed_query: Query<(
        Entity,
        &EmbedSceneRtt,
        &GlobalTransform,
        Option<&Name>,
        &crate::animation::AmAnimated,
        Option<&crate::effects::AmGroupFill>,
        Option<&mut Sprite>,
        Option<&mut Anchor>,
        Option<&Mesh2d>,
        Option<&MeshMaterial2d<crate::masked_sprite::UnifiedEffectMaterial>>,
    )>,
    mut camera_query: Query<(
        &mut EmbedSceneRttCamera,
        &mut Transform,
        &mut Projection,
        Option<&ChildOf>,
    )>,
) {
    let disable_resize = std::env::var_os("AM_DISABLE_RTT_RESIZE").is_some();
    let parent_cameras_to_embed = std::env::var_os("AM_PARENT_RTT_CAMERA_TO_EMBED").is_some();

    for (mut camera_marker, mut camera_transform, mut projection, camera_parent) in
        camera_query.iter_mut()
    {
        if let Ok((
            embed_entity,
            rtt,
            embed_global,
            name,
            animated,
            group_fill,
            sprite,
            anchor,
            mesh2d,
            unified_material_handle,
        )) = embed_query.get_mut(camera_marker.embed_entity)
        {
            // Skip the entire sync when the embed's GlobalTransform is unchanged.
            let current_affine = embed_global.to_matrix();
            if camera_marker.last_affine == Some(current_affine) {
                continue;
            }
            camera_marker.last_affine = Some(current_affine);
            let full_rect = scene_local_rect(rtt.scene_width, rtt.scene_height);
            let visible_rect = compute_embed_visible_rect(rtt, embed_global, animated);
            let visible_size = Vec2::new(
                visible_rect.width().max(1.0),
                visible_rect.height().max(1.0),
            );
            let local_center = visible_rect.center();

            let effective_size = transformed_rect_edge_lengths(full_rect, embed_global.affine());

            if std::env::var_os("AM_RTT_RECT_TRACE").is_some() {
                let label = name
                    .map(|n| n.as_str().to_owned())
                    .unwrap_or_else(|| format!("{embed_entity:?}"));
                let (global_scale, _, global_translation) =
                    embed_global.to_scale_rotation_translation();
                bevy::log::warn!(
                    "[RTT-RECT] {} dyn={} scene=({:.1},{:.1}) visible=({:.1},{:.1}) center=({:.1},{:.1}) effective=({:.1},{:.1}) scale=({:.3},{:.3}) translation=({:.1},{:.1})",
                    label,
                    rtt.dynamic_resolution,
                    rtt.scene_width,
                    rtt.scene_height,
                    visible_size.x,
                    visible_size.y,
                    local_center.x,
                    local_center.y,
                    effective_size.x,
                    effective_size.y,
                    global_scale.x,
                    global_scale.y,
                    global_translation.x,
                    global_translation.y,
                );
            }

            let (global_scale, embed_rotation, embed_translation) =
                embed_global.to_scale_rotation_translation();
            let parent_camera_to_embed = parent_cameras_to_embed && camera_parent.is_some();

            if parent_camera_to_embed {
                camera_transform.translation = Vec3::new(0.0, 0.0, EMBED_RTT_CAMERA_Z);
                camera_transform.rotation = Quat::IDENTITY;
                camera_transform.scale = Vec3::ONE;
            } else if unparented_camera_uses_full_scale() {
                camera_transform.translation =
                    Vec3::new(embed_translation.x, embed_translation.y, EMBED_RTT_CAMERA_Z);
                camera_transform.rotation = embed_rotation;
                camera_transform.scale = Vec3::new(global_scale.x, global_scale.y, 1.0);
            } else {
                camera_transform.translation =
                    Vec3::new(embed_translation.x, embed_translation.y, EMBED_RTT_CAMERA_Z);
                camera_transform.rotation = embed_rotation;
                camera_transform.scale =
                    Vec3::new(global_scale.x.signum(), global_scale.y.signum(), 1.0);
            }

            if let Projection::Orthographic(ref mut ortho) = *projection {
                let projection_size = projection_size_for_rtt_camera(
                    parent_camera_to_embed,
                    visible_size,
                    effective_size,
                );
                ortho.scaling_mode = bevy::camera::ScalingMode::Fixed {
                    width: projection_size.x.max(1.0),
                    height: projection_size.y.max(1.0),
                };
            }

            let new_extent = Extent3d {
                width: effective_size.x.ceil().max(1.0) as u32,
                height: effective_size.y.ceil().max(1.0) as u32,
                depth_or_array_layers: 1,
            };
            let keep_full_resolution_for_group_fill =
                keep_full_resolution_for_group_fill(group_fill);

            // Stretch-enabled embeds: the unified effect system (Update) already
            // expanded the mesh and set original_size for the stretch shader.
            // The sync must NOT overwrite those values, nor downscale the render
            // texture — the shader needs the full-resolution RTT content.
            let stretch_active = unified_material_handle
                .and_then(|h| unified_materials.get(&h.0))
                .is_some_and(|m| m.is_stretch_enabled());

            if !disable_resize && !keep_full_resolution_for_group_fill && !stretch_active {
                resize_render_texture(&mut images, &rtt.render_texture, new_extent);
            }
            let texture_size = images
                .get(&rtt.render_texture)
                .map(|image| {
                    Vec2::new(
                        image.texture_descriptor.size.width as f32,
                        image.texture_descriptor.size.height as f32,
                    )
                })
                .unwrap_or_else(|| Vec2::new(new_extent.width as f32, new_extent.height as f32));

            if let Some(mut sprite) = sprite {
                sync_dynamic_resolution_sprite(
                    &mut commands,
                    embed_entity,
                    &mut sprite,
                    anchor,
                    rtt,
                    visible_rect,
                    full_rect,
                    visible_size,
                    local_center,
                    texture_size,
                );
            } else if let Some(mesh2d) = mesh2d.filter(|_| !stretch_active) {
                // Group-fill embeds keep the texture at full resolution, so the mesh
                // must also stay at full extent. Cropping it to visible_rect would
                // shift the mesh center away from (0,0) without any Anchor to
                // compensate (unlike the Sprite path), causing position errors.
                let mesh_rect =
                    [visible_rect, full_rect][keep_full_resolution_for_group_fill as usize];
                let mesh_sz = Vec2::new(mesh_rect.width(), mesh_rect.height());
                let tex_match = (texture_size.x - mesh_sz.x).abs() <= 0.5
                    && (texture_size.y - mesh_sz.y).abs() <= 0.5;
                sync_dynamic_resolution_mesh(
                    &mut meshes,
                    mesh2d,
                    rtt,
                    mesh_rect,
                    full_rect,
                    tex_match,
                );
                sync_embed_material_original_size(
                    &mut unified_materials,
                    unified_material_handle,
                    rtt,
                    mesh_rect,
                );
            }
        }
    }
}

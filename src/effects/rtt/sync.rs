//! This file keeps RTT cameras and render textures synchronized with their embed scenes.
//! It updates camera placement, projection, dynamic-resolution sizing, and the
//! matching sprite or mesh representation when an embed scene moves or scales.
//!
//! 这个文件负责让 RTT 相机和渲染纹理持续与嵌套场景同步。它会在 embed scene 发生
//! 位移或缩放时，更新相机位置、投影、动态分辨率尺寸，以及与之配套的 sprite 或 mesh
//! 表现。

use bevy::prelude::*;
use bevy::render::render_resource::Extent3d;
use bevy::sprite::Anchor;

use super::{
    EMBED_RTT_CAMERA_Z, EmbedSceneRtt, EmbedSceneRttCamera, compute_embed_visible_rect,
    resize_render_texture, scene_local_rect, sync_dynamic_resolution_mesh,
    sync_dynamic_resolution_sprite, transformed_rect_edge_lengths,
};

pub fn sync_rtt_camera_position_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut embed_query: Query<(
        Entity,
        &EmbedSceneRtt,
        &GlobalTransform,
        Option<&Name>,
        &crate::animation::AmAnimated,
        Option<&mut Sprite>,
        Option<&mut Anchor>,
        Option<&Mesh2d>,
    )>,
    mut camera_query: Query<(
        &EmbedSceneRttCamera,
        &mut Transform,
        &mut Projection,
        Option<&ChildOf>,
    )>,
) {
    let disable_resize = std::env::var_os("AM_DISABLE_RTT_RESIZE").is_some();
    let parent_cameras_to_embed = std::env::var_os("AM_PARENT_RTT_CAMERA_TO_EMBED").is_some();

    for (camera_marker, mut camera_transform, mut projection, camera_parent) in
        camera_query.iter_mut()
    {
        if let Ok((embed_entity, rtt, embed_global, name, animated, sprite, anchor, mesh2d)) =
            embed_query.get_mut(camera_marker.embed_entity)
        {
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
            if parent_cameras_to_embed && camera_parent.is_some() {
                camera_transform.translation = Vec3::new(0.0, 0.0, EMBED_RTT_CAMERA_Z);
                camera_transform.rotation = Quat::IDENTITY;
                camera_transform.scale = Vec3::ONE;
            } else {
                camera_transform.translation =
                    Vec3::new(embed_translation.x, embed_translation.y, EMBED_RTT_CAMERA_Z);
                camera_transform.rotation = embed_rotation;
                camera_transform.scale =
                    Vec3::new(global_scale.x.signum(), global_scale.y.signum(), 1.0);
            }

            if let Projection::Orthographic(ref mut ortho) = *projection {
                ortho.scaling_mode = bevy::camera::ScalingMode::Fixed {
                    width: effective_size.x,
                    height: effective_size.y,
                };
            }

            let new_extent = Extent3d {
                width: effective_size.x.ceil().max(1.0) as u32,
                height: effective_size.y.ceil().max(1.0) as u32,
                depth_or_array_layers: 1,
            };
            if !disable_resize {
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
            } else if let Some(mesh2d) = mesh2d {
                sync_dynamic_resolution_mesh(&mut meshes, mesh2d, rtt, visible_rect, full_rect);
            }
        }
    }
}

use bevy::camera::visibility::RenderLayers;
use bevy::camera::{RenderTarget, ScalingMode};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;

use super::{
    AmEmbedMask, AmGroupFill, EMBED_RTT_CAMERA_FAR, EMBED_RTT_CAMERA_NEAR, EMBED_RTT_CAMERA_Z,
    EmbedSceneRenderLayerPool, EmbedSceneRtt, EmbedSceneRttCamera, GroupFillType,
    NeedsEmbedSceneRtt,
};

#[derive(Component)]
pub(crate) struct PendingGroupFillTextureRefresh(u8);

fn insert_group_fill_debug_sprite(
    commands: &mut Commands,
    entity: Entity,
    render_texture_handle: Handle<Image>,
    scene_width: f32,
    scene_height: f32,
) {
    commands.entity(entity).insert(Sprite {
        image: render_texture_handle,
        custom_size: Some(Vec2::new(scene_width, scene_height)),
        ..default()
    });
}

fn insert_group_fill_mesh(
    commands: &mut Commands,
    fill: &AmGroupFill,
    entity: Entity,
    render_texture_handle: Handle<Image>,
    scene_width: f32,
    scene_height: f32,
    fill_materials: &mut Assets<crate::group_fill::GroupFillMaterial>,
    meshes: &mut Assets<Mesh>,
) {
    use crate::group_fill::{GroupFillMaterial, GroupFillUniform};

    let uniform = match &fill.fill_type {
        GroupFillType::Color => GroupFillUniform {
            fill_color: fill.fill_color,
            gradient_config: Vec4::ZERO,
            ..default()
        },
        GroupFillType::Gradient {
            gradient_type,
            start_color,
            end_color,
            points,
        } => GroupFillUniform {
            fill_color: Vec4::ONE,
            gradient_config: Vec4::new(*gradient_type as f32, 0.0, 0.0, 0.0),
            gradient_start_color: *start_color,
            gradient_end_color: *end_color,
            gradient_points: *points,
        },
        GroupFillType::None => unreachable!(),
    };
    let material = fill_materials.add(GroupFillMaterial {
        uniform_data: uniform,
        texture: Some(render_texture_handle),
    });
    let mesh = meshes.add(Rectangle::new(scene_width, scene_height));
    commands.entity(entity).insert((
        Mesh2d(mesh),
        MeshMaterial2d(material),
        PendingGroupFillTextureRefresh(8),
    ));
}

pub fn setup_embed_scene_rtt_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut layer_pool: ResMut<EmbedSceneRenderLayerPool>,
    mut fill_materials: ResMut<Assets<crate::group_fill::GroupFillMaterial>>,
    mut unified_materials: ResMut<Assets<crate::masked_sprite::UnifiedEffectMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    _color_materials: ResMut<Assets<ColorMaterial>>,
    query: Query<
        (
            Entity,
            &NeedsEmbedSceneRtt,
            &Transform,
            &GlobalTransform,
            Option<&AmGroupFill>,
            &crate::animation::AmAnimated,
            Option<&AmEmbedMask>,
        ),
        Without<EmbedSceneRtt>,
    >,
    parent_query: Query<&ChildOf>,
    embed_rtt_query: Query<&EmbedSceneRtt>,
) {
    let debug_show_fill_rtt = std::env::var_os("AM_GROUP_FILL_DEBUG_SHOW_RTT").is_some();
    let trace_renderlayers = std::env::var_os("AM_RENDERLAYER_TRACE").is_some();
    let parent_cameras_to_embed = std::env::var_os("AM_PARENT_RTT_CAMERA_TO_EMBED").is_some();

    for (
        entity,
        needs_rtt,
        _embed_transform,
        embed_global,
        group_fill,
        animated,
        embed_mask,
    ) in query.iter()
    {
        let Some(render_layer) = layer_pool.allocate() else {
            bevy::log::warn!(
                "No available render layers for embedScene {:?}. Max 31 concurrent embedScenes supported.",
                entity
            );
            continue;
        };

        let render_texture = Image::new_target_texture(
            needs_rtt.scene_width.max(1.0) as u32,
            needs_rtt.scene_height.max(1.0) as u32,
            TextureFormat::Rgba8UnormSrgb,
            None,
        );
        let render_texture_handle = images.add(render_texture);
        let render_layer_usize = render_layer as usize;

        let global_scale = embed_global.to_scale_rotation_translation().0;
        let effective_width = needs_rtt.scene_width * global_scale.x.abs();
        let effective_height = needs_rtt.scene_height * global_scale.y.abs();
        let parent_camera_to_embed = parent_cameras_to_embed
            && parent_query
                .get(entity)
                .ok()
                .and_then(|child_of| embed_rtt_query.get(child_of.parent()).ok())
                .is_some();

        let initial_camera_transform = if parent_camera_to_embed {
            Transform::from_translation(Vec3::new(0.0, 0.0, EMBED_RTT_CAMERA_Z))
        } else {
            let (_, embed_rotation, embed_translation) =
                embed_global.to_scale_rotation_translation();
            Transform {
                translation: Vec3::new(
                    embed_translation.x,
                    embed_translation.y,
                    EMBED_RTT_CAMERA_Z,
                ),
                rotation: embed_rotation,
                scale: Vec3::new(global_scale.x.signum(), global_scale.y.signum(), 1.0),
            }
        };

        let camera_entity = commands
            .spawn((
                Name::new(format!("EmbedSceneRttCamera[layer={}]", render_layer)),
                EmbedSceneRttCamera {
                    embed_entity: entity,
                    render_layer,
                },
                Camera2d,
                Camera {
                    clear_color: ClearColorConfig::Custom(Color::NONE),
                    order: -(render_layer as isize),
                    ..default()
                },
                RenderTarget::Image(render_texture_handle.clone().into()),
                Projection::Orthographic(OrthographicProjection {
                    scaling_mode: ScalingMode::Fixed {
                        width: effective_width,
                        height: effective_height,
                    },
                    near: EMBED_RTT_CAMERA_NEAR,
                    far: EMBED_RTT_CAMERA_FAR,
                    ..OrthographicProjection::default_2d()
                }),
                RenderLayers::layer(render_layer_usize),
                initial_camera_transform,
            ))
            .id();

        if parent_camera_to_embed {
            commands.entity(entity).add_child(camera_entity);
        }

        let sprite_render_layer = if let Ok(child_of) = parent_query.get(entity) {
            let parent = child_of.parent();
            if let Ok(parent_rtt) = embed_rtt_query.get(parent) {
                RenderLayers::layer(parent_rtt.render_layer as usize)
            } else {
                RenderLayers::layer(0)
            }
        } else {
            RenderLayers::layer(0)
        };

        if trace_renderlayers {
            bevy::log::warn!(
                "[RTT-SetupLayer] embed={:?} render_layer={} sprite_layer={:?}",
                entity,
                render_layer,
                sprite_render_layer,
            );
        }

        commands
            .entity(entity)
            .remove::<NeedsEmbedSceneRtt>()
            .insert((
                EmbedSceneRtt {
                    render_texture: render_texture_handle.clone(),
                    camera_entity,
                    render_layer,
                    scene_width: needs_rtt.scene_width,
                    scene_height: needs_rtt.scene_height,
                    dynamic_resolution: needs_rtt.dynamic_resolution,
                },
                sprite_render_layer,
            ));

        if embed_mask.is_some() {
            bevy::log::debug!(
                "[RTT] Mask embed {:?}: RTT setup without sprite (layer={})",
                entity,
                render_layer
            );
        } else if let Some(fill) = group_fill.filter(|fill| fill.fill_type != GroupFillType::None) {
            if debug_show_fill_rtt {
                insert_group_fill_debug_sprite(
                    &mut commands,
                    entity,
                    render_texture_handle,
                    needs_rtt.scene_width,
                    needs_rtt.scene_height,
                );
            } else {
                insert_group_fill_mesh(
                    &mut commands,
                    fill,
                    entity,
                    render_texture_handle,
                    needs_rtt.scene_width,
                    needs_rtt.scene_height,
                    &mut fill_materials,
                    &mut meshes,
                );
            }
        } else {
            let has_wipe = animated.wipe_end.value != Some(1.0)
                || !animated.wipe_end.keyframes.is_empty()
                || animated.wipe_start.value.is_some()
                || !animated.wipe_start.keyframes.is_empty();
            let has_stretch = animated.stretch_amount.value.is_some()
                || !animated.stretch_amount.keyframes.is_empty()
                || animated.stretch_angle.value.is_some()
                || !animated.stretch_angle.keyframes.is_empty()
                || animated.stretch_offset.value.is_some()
                || !animated.stretch_offset.keyframes.is_empty()
                || animated.stretch_smooth.value.is_some()
                || !animated.stretch_smooth.keyframes.is_empty()
                || animated.stretch_seg2_amount.value.is_some()
                || !animated.stretch_seg2_amount.keyframes.is_empty()
                || animated.stretch_seg2_angle.value.is_some()
                || !animated.stretch_seg2_angle.keyframes.is_empty()
                || animated.stretch_seg2_offset.value.is_some()
                || !animated.stretch_seg2_offset.keyframes.is_empty()
                || animated.stretch_seg2_smooth.value.is_some()
                || !animated.stretch_seg2_smooth.keyframes.is_empty();
            let has_blur = animated.blur_strength.value.is_some()
                || !animated.blur_strength.keyframes.is_empty();
            let has_stretch2 = animated.stretch2_scale.value.is_some()
                || !animated.stretch2_scale.keyframes.is_empty();
            let has_pixelate = animated.pixelate_size.value.is_some()
                || !animated.pixelate_size.keyframes.is_empty();
            let has_threshold = animated.threshold_value.value.is_some()
                || !animated.threshold_value.keyframes.is_empty();
            let has_grid = animated.grid_spacing.value.is_some()
                || !animated.grid_spacing.keyframes.is_empty();
            let has_solidcolor = animated.solid_color_alpha.value.is_some()
                || !animated.solid_color_alpha.keyframes.is_empty();
            let has_replace_color = animated.replace_old_color.w > 0.0
                || animated.replace_new_color.value.is_some()
                || !animated.replace_new_color.keyframes.is_empty();
            let needs_unified = has_wipe
                || has_stretch
                || has_blur
                || has_stretch2
                || has_pixelate
                || has_threshold
                || has_grid
                || has_solidcolor
                || has_replace_color
                || animated.exposure_has_effect
                || animated.wavewarp2_has_effect
                || animated.mirror_has_effect
                || animated.lift_has_effect
                || animated.rays_has_effect
                || animated.rgb_split_enabled
                || animated.chromakey_enabled;

            if needs_unified {
                let width = needs_rtt.scene_width;
                let height = needs_rtt.scene_height;
                let material = unified_materials.add(crate::masked_sprite::UnifiedEffectMaterial {
                    uniform_data: crate::masked_sprite::UnifiedEffectUniform {
                        color: Vec4::new(1.0, 1.0, 1.0, 1.0),
                        original_size: Vec4::new(width, height, width, height),
                        ..default()
                    },
                    texture: Some(render_texture_handle),
                    lift_comp_texture: None,
                    mask_texture: None,
                });
                let mesh = meshes.add(Rectangle::new(width, height));
                commands.entity(entity).insert((
                    Mesh2d(mesh),
                    MeshMaterial2d(material),
                    crate::masked_sprite::UnifiedEffectMarker,
                ));
            } else {
                commands.entity(entity).insert(Sprite {
                    image: render_texture_handle,
                    custom_size: Some(Vec2::new(needs_rtt.scene_width, needs_rtt.scene_height)),
                    ..default()
                });
            }
        }

        bevy::log::trace!(
            "[RTT] Set up RTT for embedScene {:?}: layer={}, size={}x{}",
            entity,
            render_layer,
            needs_rtt.scene_width,
            needs_rtt.scene_height
        );
    }
}

pub(crate) fn refresh_group_fill_material_texture_system(
    mut commands: Commands,
    query: Query<(
        Entity,
        &EmbedSceneRtt,
        &MeshMaterial2d<crate::group_fill::GroupFillMaterial>,
        &PendingGroupFillTextureRefresh,
    )>,
    mut materials: ResMut<Assets<crate::group_fill::GroupFillMaterial>>,
) {
    for (entity, rtt, material_handle, pending) in query.iter() {
        let Some(material) = materials.get_mut(&material_handle.0) else {
            continue;
        };

        material.texture = Some(rtt.render_texture.clone());

        if pending.0 <= 1 {
            commands
                .entity(entity)
                .remove::<PendingGroupFillTextureRefresh>();
        } else {
            commands
                .entity(entity)
                .insert(PendingGroupFillTextureRefresh(pending.0 - 1));
        }
    }
}

pub fn fix_nested_embed_render_layers_system(
    mut commands: Commands,
    embed_query: Query<(Entity, &EmbedSceneRtt, &RenderLayers)>,
    parent_query: Query<&ChildOf>,
    embed_rtt_query: Query<&EmbedSceneRtt>,
) {
    let trace_renderlayers = std::env::var_os("AM_RENDERLAYER_TRACE").is_some();

    for (entity, _rtt, current_layers) in embed_query.iter() {
        let Ok(child_of) = parent_query.get(entity) else {
            continue;
        };
        let parent = child_of.parent();
        let Ok(parent_rtt) = embed_rtt_query.get(parent) else {
            continue;
        };
        let expected_layer = RenderLayers::layer(parent_rtt.render_layer as usize);
        if *current_layers != expected_layer {
            if trace_renderlayers {
                bevy::log::warn!(
                    "[RTT-FixLayer] embed={:?} was={:?} now={:?}",
                    entity,
                    current_layers,
                    expected_layer,
                );
            } else {
                bevy::log::trace!(
                    "[RTT] Fixing nested embed {:?} RenderLayers: was {:?}, now layer {}",
                    entity,
                    current_layers,
                    parent_rtt.render_layer
                );
            }
            commands.entity(entity).insert(expected_layer);
        }
    }
}

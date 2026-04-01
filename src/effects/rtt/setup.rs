//! Sets up RTT infrastructure for composite embed scenes.
//! It creates render textures, cameras, render-layer assignments, and any
//! matching sprite or mesh bindings needed so an embed scene can render off-screen
//! and then be composited back into its parent layer.
//!
//! 负责为 composite 路径的嵌套场景建立 RTT 基础设施。它会创建渲染纹理、
//! 相机、render layer 分配，以及与之对应的 sprite 或 mesh 绑定，让 embed scene
//! 可以先离屏渲染，再被合成回父图层里。

use bevy::camera::visibility::RenderLayers;
use bevy::camera::{RenderTarget, ScalingMode};
use bevy::prelude::*;

use super::setup_helpers::{
    PendingGroupFillTextureRefresh, composite_camera_order, dynamic_render_layer,
    flatten_parented_rtt_to_world_enabled, insert_group_fill_debug_sprite, insert_group_fill_mesh,
    mirrored_capture_root_enabled, parented_camera_uses_local_projection,
    plain_rtt_uses_straight_alpha, selected_embed_rtt_format, sign_axis, trace_group_fill_mode,
    trace_rtt_setup_enabled, unparented_camera_uses_full_scale,
};
use super::{
    AmEmbedMask, AmGroupFill, EMBED_RTT_CAMERA_FAR, EMBED_RTT_CAMERA_NEAR, EMBED_RTT_CAMERA_Z,
    EmbedSceneRenderLayerPool, EmbedSceneRtt, EmbedSceneRttCamera, EmbedSceneRttCaptureRoot,
    GroupFillType, NeedsEmbedSceneRtt,
};

/// Count how many pending-RTT ancestors an entity has, used to sort embeds
/// so parents are processed before children within a single frame.
///
/// 计算实体有多少个待处理 RTT 的祖先，用于排序使父级先于子级处理。
fn embed_pending_depth(
    entity: Entity,
    parent_query: &Query<&ChildOf>,
    pending_query: &Query<(), With<NeedsEmbedSceneRtt>>,
) -> u32 {
    let mut depth = 0;
    let mut current = entity;
    while let Ok(child_of) = parent_query.get(current) {
        current = child_of.parent();
        if pending_query.get(current).is_ok() {
            depth += 1;
        }
    }
    depth
}

/// Walk up the Bevy hierarchy to find the nearest ancestor embed's render
/// layer. Checks both committed `EmbedSceneRtt` and the local
/// `processed_layers` map for parents processed earlier in the same frame.
///
/// 沿 Bevy 层级向上查找最近的 embed 祖先 render layer，
/// 同时检查本帧已处理但尚未 commit 的父级。
fn ancestor_embed_render_layer(
    entity: Entity,
    parent_query: &Query<&ChildOf>,
    embed_rtt_query: &Query<&EmbedSceneRtt>,
    processed_layers: &std::collections::HashMap<Entity, usize>,
) -> RenderLayers {
    let mut current = entity;
    while let Ok(child_of) = parent_query.get(current) {
        let ancestor = child_of.parent();
        if let Ok(ancestor_rtt) = embed_rtt_query.get(ancestor) {
            return dynamic_render_layer(ancestor_rtt.render_layer);
        }
        if let Some(&ancestor_layer) = processed_layers.get(&ancestor) {
            return dynamic_render_layer(ancestor_layer);
        }
        current = ancestor;
    }
    RenderLayers::layer(0)
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
            Option<&Children>,
        ),
        Without<EmbedSceneRtt>,
    >,
    parent_query: Query<&ChildOf>,
    embed_rtt_query: Query<&EmbedSceneRtt>,
    pending_embed_rtt_query: Query<(), With<NeedsEmbedSceneRtt>>,
    layer_spec_query: Query<&crate::scene::AmLayerSpec>,
) {
    let debug_show_fill_rtt = std::env::var_os("AM_GROUP_FILL_DEBUG_SHOW_RTT").is_some();
    let trace_renderlayers = std::env::var_os("AM_RENDERLAYER_TRACE").is_some();
    let parent_cameras_to_embed = std::env::var_os("AM_PARENT_RTT_CAMERA_TO_EMBED").is_some();
    let render_texture_format = selected_embed_rtt_format();

    // Sort entities by nesting depth (parent before child) so all levels can be
    // set up in a single pass within one frame instead of deferring child embeds.
    // 按嵌套深度排序（父先子后），在单帧内一次遍历完成所有层级的 RTT 设置。
    let mut entities_with_depth: Vec<(Entity, u32)> = query
        .iter()
        .map(|(e, ..)| {
            let depth = embed_pending_depth(e, &parent_query, &pending_embed_rtt_query);
            (e, depth)
        })
        .collect();
    entities_with_depth.sort_by_key(|&(_, depth)| depth);
    let mut processed_layers = std::collections::HashMap::<Entity, usize>::new();

    for (entity, _depth) in entities_with_depth {
        // Skip if parent is still pending AND wasn't processed in this pass.
        if let Ok(child_of) = parent_query.get(entity) {
            let parent = child_of.parent();
            let parent_pending = pending_embed_rtt_query.get(parent).is_ok();
            let parent_processed = processed_layers.contains_key(&parent);
            if parent_pending && !parent_processed {
                continue;
            }
        }

        let Ok((
            _,
            needs_rtt,
            embed_transform,
            embed_global,
            group_fill,
            animated,
            embed_mask,
            children,
        )) = query.get(entity)
        else {
            continue;
        };

        let Some(render_layer) = layer_pool.allocate() else {
            bevy::log::warn!("No available render layer for embedScene {:?}.", entity);
            continue;
        };

        let render_texture = Image::new_target_texture(
            needs_rtt.scene_width.max(1.0) as u32,
            needs_rtt.scene_height.max(1.0) as u32,
            render_texture_format,
            None,
        );
        let render_texture_handle = images.add(render_texture);
        let (global_scale, embed_rotation, embed_translation) =
            embed_global.to_scale_rotation_translation();
        let effective_width = needs_rtt.scene_width * global_scale.x.abs();
        let effective_height = needs_rtt.scene_height * global_scale.y.abs();
        // Opt-in camera parenting avoids decomposing mirrored/rotated embed globals into a
        // fresh TRS every frame. Letting the camera inherit the exact embed hierarchy is a more
        // faithful path for parented embeds, especially when negative scales participate.
        let parent_camera_to_embed = parent_cameras_to_embed && parent_query.get(entity).is_ok();
        let mirrored_embed = embed_transform.scale.x < 0.0 || embed_transform.scale.y < 0.0;
        let flatten_parented_rtt_to_world =
            parent_camera_to_embed && flatten_parented_rtt_to_world_enabled();
        let capture_root = if flatten_parented_rtt_to_world
            || (parent_camera_to_embed && mirrored_embed && mirrored_capture_root_enabled())
        {
            let capture_root_transform = if flatten_parented_rtt_to_world {
                Transform {
                    translation: embed_translation,
                    rotation: embed_rotation,
                    scale: Vec3::new(global_scale.x, global_scale.y, 1.0),
                }
            } else {
                Transform::from_scale(Vec3::new(
                    sign_axis(embed_transform.scale.x),
                    sign_axis(embed_transform.scale.y),
                    1.0,
                ))
            };
            let capture_root_entity = commands
                .spawn((
                    Name::new(format!("EmbedSceneRttCaptureRoot[layer={}]", render_layer)),
                    EmbedSceneRttCaptureRoot {
                        embed_entity: entity,
                    },
                    capture_root_transform,
                    GlobalTransform::default(),
                    Visibility::Inherited,
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    dynamic_render_layer(render_layer),
                ))
                .id();
            if !flatten_parented_rtt_to_world {
                commands.entity(entity).add_child(capture_root_entity);
            }

            if let Some(children) = children {
                commands
                    .entity(capture_root_entity)
                    .add_children(children.as_ref());
            }

            Some(capture_root_entity)
        } else {
            None
        };
        // Keep the existing world-space projection by default. An embed-local projection path is
        // still available behind an env gate for debugging camera hierarchy issues.
        let use_local_projection = (parent_camera_to_embed
            && parented_camera_uses_local_projection())
            || (!parent_camera_to_embed && unparented_camera_uses_full_scale());
        let projection_width = if use_local_projection {
            needs_rtt.scene_width.max(1.0)
        } else {
            effective_width.max(1.0)
        };
        let projection_height = if use_local_projection {
            needs_rtt.scene_height.max(1.0)
        } else {
            effective_height.max(1.0)
        };

        let initial_camera_transform = if parent_camera_to_embed {
            Transform::from_translation(Vec3::new(0.0, 0.0, EMBED_RTT_CAMERA_Z))
        } else if unparented_camera_uses_full_scale() {
            Transform {
                translation: Vec3::new(
                    embed_translation.x,
                    embed_translation.y,
                    EMBED_RTT_CAMERA_Z,
                ),
                rotation: embed_rotation,
                scale: Vec3::new(global_scale.x, global_scale.y, 1.0),
            }
        } else {
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
        let camera_order =
            composite_camera_order(entity, render_layer, &parent_query, &layer_spec_query);

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
                    order: camera_order,
                    ..default()
                },
                RenderTarget::Image(render_texture_handle.clone().into()),
                Projection::Orthographic(OrthographicProjection {
                    scaling_mode: ScalingMode::Fixed {
                        width: projection_width,
                        height: projection_height,
                    },
                    near: EMBED_RTT_CAMERA_NEAR,
                    far: EMBED_RTT_CAMERA_FAR,
                    ..OrthographicProjection::default_2d()
                }),
                dynamic_render_layer(render_layer),
                initial_camera_transform,
            ))
            .id();

        if let Some(capture_root) = capture_root {
            commands.entity(capture_root).add_child(camera_entity);
        } else if parent_camera_to_embed {
            commands.entity(entity).add_child(camera_entity);
        }

        let sprite_render_layer =
            ancestor_embed_render_layer(entity, &parent_query, &embed_rtt_query, &processed_layers);

        if trace_renderlayers {
            bevy::log::warn!(
                "[RTT-SetupLayer] embed={:?} render_layer={} sprite_layer={:?} camera_order={} parent_camera_to_embed={} dynamic_resolution={}",
                entity,
                render_layer,
                sprite_render_layer,
                camera_order,
                parent_camera_to_embed,
                needs_rtt.render_plan.dynamic_resolution,
            );
            bevy::log::warn!(
                "[RTT-SetupTexture] embed={:?} format={:?}",
                entity,
                render_texture_format,
            );
        }

        commands
            .entity(entity)
            .remove::<NeedsEmbedSceneRtt>()
            .insert((
                EmbedSceneRtt {
                    render_texture: render_texture_handle.clone(),
                    camera_entity,
                    capture_root,
                    render_layer,
                    scene_width: needs_rtt.scene_width,
                    scene_height: needs_rtt.scene_height,
                    dynamic_resolution: needs_rtt.render_plan.dynamic_resolution,
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
                trace_group_fill_mode(
                    trace_renderlayers,
                    entity,
                    render_layer,
                    "debug-sprite",
                    &fill.fill_type,
                );
                insert_group_fill_debug_sprite(
                    &mut commands,
                    entity,
                    render_texture_handle,
                    needs_rtt.scene_width,
                    needs_rtt.scene_height,
                );
            } else {
                trace_group_fill_mode(
                    trace_renderlayers,
                    entity,
                    render_layer,
                    "mesh",
                    &fill.fill_type,
                );
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

            let trace_replace_setup =
                std::env::var_os("AM_TRACE_RTT_SETUP_REPLACE").is_some() && has_replace_color;
            if trace_rtt_setup_enabled(animated.layer_id) || trace_replace_setup {
                bevy::log::warn!(
                    "[RTT-SetupTrace] layer={} render_layer={} replace_old={:?} replace_new_static={:?} has_replace={} needs_unified={} has_pixelate={} has_threshold={} has_solidcolor={}",
                    animated.layer_id,
                    render_layer,
                    animated.replace_old_color,
                    animated.replace_new_color.value,
                    has_replace_color,
                    needs_unified,
                    has_pixelate,
                    has_threshold,
                    has_solidcolor,
                );
            }

            let width = needs_rtt.scene_width;
            let height = needs_rtt.scene_height;
            // Composite RTTs carry explicit offscreen/premultiplied contracts.
            // Keep even plain embeds on the unified mesh path so dynamic-resolution UV sync and
            // premultiplied-alpha sampling stay consistent across GPUs.
            let texture_source_contract = if needs_unified || !plain_rtt_uses_straight_alpha() {
                needs_rtt.render_plan.composite_source_contract
            } else {
                crate::effects::TextureSourceContract::layer_texture()
            };
            let mut source_flags = texture_source_contract.to_uniform_flags();
            source_flags.w = 1.0;
            let material = unified_materials.add(crate::masked_sprite::UnifiedEffectMaterial {
                uniform_data: crate::masked_sprite::UnifiedEffectUniform {
                    color: Vec4::new(1.0, 1.0, 1.0, 1.0),
                    original_size: Vec4::new(width, height, width, height),
                    source_flags,
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
                crate::animation::AmUnifiedMeshState::default(),
            ));
        }

        bevy::log::trace!(
            "[RTT] Set up RTT for embedScene {:?}: layer={}, size={}x{}",
            entity,
            render_layer,
            needs_rtt.scene_width,
            needs_rtt.scene_height
        );

        processed_layers.insert(entity, render_layer);
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
        let expected_layer = dynamic_render_layer(parent_rtt.render_layer);
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

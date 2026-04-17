//! # lift_composite.rs
//!
//! # 复制背景效果 - RTT合成管线
//!
//! Lift (Copy Background) effect - RTT composite pipeline.
//! Renders all layers below a lift/blend layer to a dedicated texture that the
//! layer's shader samples as its background.
//!
//! 将某个 lift/blend 图层之下的所有图层渲染到该图层专属的纹理，供 shader 作为背景采样。

use bevy::camera::RenderTarget;
use bevy::camera::ScalingMode;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;

use crate::animation::AmAnimated;
use crate::effects::EmbedSceneRenderLayerPool;
use crate::masked_sprite::UnifiedEffectMaterial;
use crate::scene::AmVisualSpawned;
use crate::sdf_material::SdfMaterial;

#[derive(Component, Debug, Clone, Copy)]
pub struct LiftCompositeCameraMarker {
    pub owner_entity: Entity,
    pub render_layer: usize,
}

#[derive(Component, Debug, Clone)]
pub(crate) struct LiftCompositeBinding {
    pub texture_handle: Handle<Image>,
    pub camera_entity: Entity,
    pub render_layer: usize,
    pub cutoff_z: f32,
}

fn needs_composite(animated: &AmAnimated) -> bool {
    animated.lift_has_effect || animated.blend_mode.is_blend()
}

fn dynamic_render_layer(layer: usize) -> RenderLayers {
    RenderLayers::from_layers(&[layer])
}

fn merged_render_layers(
    current_layers: Option<&RenderLayers>,
    render_layer: usize,
) -> RenderLayers {
    current_layers
        .cloned()
        .unwrap_or_default()
        .with(render_layer)
}

fn lift_camera_order(cutoff_z: f32) -> isize {
    -2_000 + (cutoff_z * 1_000.0).round() as isize
}

type UnifiedCompositeLayersQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut RenderLayers,
    (
        With<MeshMaterial2d<UnifiedEffectMaterial>>,
        Without<LiftCompositeCameraMarker>,
    ),
>;

type SdfCompositeLayersQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut RenderLayers,
    (
        With<MeshMaterial2d<SdfMaterial>>,
        Without<LiftCompositeCameraMarker>,
    ),
>;

type SpriteCompositeLayersQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut RenderLayers,
    (
        With<Sprite>,
        With<AmVisualSpawned>,
        Without<LiftCompositeCameraMarker>,
    ),
>;

fn assign_layer_to_background_entity(
    commands: &mut Commands,
    entity: Entity,
    current_layers: Option<&RenderLayers>,
    render_layer: usize,
) {
    let composite_layer = dynamic_render_layer(render_layer);
    let already_has = current_layers.is_some_and(|layers| layers.intersects(&composite_layer));
    if already_has {
        return;
    }

    commands
        .entity(entity)
        .insert(merged_render_layers(current_layers, render_layer));
}

pub(crate) fn setup_lift_composite_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut layer_pool: ResMut<EmbedSceneRenderLayerPool>,
    window_query: Query<&Window>,
    clear_color: Res<ClearColor>,
    new_layers_query: Query<
        (Entity, &AmAnimated, &Transform),
        (Added<AmAnimated>, Without<LiftCompositeBinding>),
    >,
    unified_layers_query: Query<
        (Entity, &Transform, Option<&RenderLayers>),
        (
            With<MeshMaterial2d<UnifiedEffectMaterial>>,
            Without<LiftCompositeCameraMarker>,
        ),
    >,
    sdf_layers_query: Query<
        (Entity, &Transform, Option<&RenderLayers>),
        (
            With<MeshMaterial2d<SdfMaterial>>,
            Without<LiftCompositeCameraMarker>,
        ),
    >,
    sprite_layers_query: Query<
        (Entity, &Transform, Option<&RenderLayers>),
        (
            With<Sprite>,
            With<AmVisualSpawned>,
            Without<LiftCompositeCameraMarker>,
        ),
    >,
) {
    for (owner_entity, animated, transform) in new_layers_query.iter() {
        if !needs_composite(animated) {
            continue;
        }

        let Some(render_layer) = layer_pool.allocate() else {
            bevy::log::warn!(
                "[LiftComposite] No available render layer for owner {:?}",
                owner_entity
            );
            continue;
        };

        let cutoff_z = transform.translation.z;
        let canvas_w = animated.canvas_width.max(1.0);
        let canvas_h = animated.canvas_height.max(1.0);
        let (tex_w, tex_h) = if let Some(window) = window_query.iter().next() {
            (window.width() as u32, window.height() as u32)
        } else {
            (canvas_w as u32, canvas_h as u32)
        };

        let render_texture = crate::effects::create_rtt_image(
            tex_w.max(1),
            tex_h.max(1),
            // Keep blend backgrounds in linear high precision. Difference/exclusion/divide
            // amplify tiny encode/decode and 8-bit rounding drift across drivers.
            TextureFormat::Rgba16Float,
            Some("lift_composite_rtt"),
        );
        let texture_handle = images.add(render_texture);
        let camera_order = lift_camera_order(cutoff_z);

        let camera_entity = commands
            .spawn((
                Name::new(format!(
                    "LiftCompositeCamera[layer={},owner={:?}]",
                    render_layer, owner_entity
                )),
                LiftCompositeCameraMarker {
                    owner_entity,
                    render_layer,
                },
                Camera2d,
                Camera {
                    clear_color: ClearColorConfig::Custom(clear_color.0),
                    order: camera_order,
                    is_active: false,
                    ..default()
                },
                RenderTarget::Image(texture_handle.clone().into()),
                Projection::Orthographic(OrthographicProjection {
                    scaling_mode: ScalingMode::Fixed {
                        width: canvas_w,
                        height: canvas_h,
                    },
                    near: -1000.0,
                    far: 1000.0,
                    ..OrthographicProjection::default_2d()
                }),
                dynamic_render_layer(render_layer),
                Transform::from_xyz(0.0, 0.0, 1000.0),
                crate::effects::PendingCameraActivation,
            ))
            .id();

        for (layer_entity, layer_transform, current_layers) in unified_layers_query
            .iter()
            .chain(sdf_layers_query.iter())
            .chain(sprite_layers_query.iter())
        {
            if layer_transform.translation.z < cutoff_z {
                assign_layer_to_background_entity(
                    &mut commands,
                    layer_entity,
                    current_layers,
                    render_layer,
                );
            }
        }

        commands.entity(owner_entity).insert(LiftCompositeBinding {
            texture_handle,
            camera_entity,
            render_layer,
            cutoff_z,
        });

        bevy::log::trace!(
            "[LiftComposite] owner={:?} cutoff_z={:.6} render_layer={} camera={:?} texture={}x{}",
            owner_entity,
            cutoff_z,
            render_layer,
            camera_entity,
            tex_w,
            tex_h,
        );
    }
}

pub(crate) fn propagate_lift_render_layers_system(
    mut commands: Commands,
    bindings_query: Query<(Entity, &LiftCompositeBinding)>,
    new_unified_layers: Query<
        (Entity, &Transform, Option<&RenderLayers>),
        (
            Added<MeshMaterial2d<UnifiedEffectMaterial>>,
            Without<LiftCompositeCameraMarker>,
        ),
    >,
    new_sdf_layers: Query<
        (Entity, &Transform, Option<&RenderLayers>),
        (
            Added<MeshMaterial2d<SdfMaterial>>,
            Without<LiftCompositeCameraMarker>,
        ),
    >,
    new_sprite_layers: Query<
        (Entity, &Transform, Option<&RenderLayers>),
        (
            Added<Sprite>,
            With<AmVisualSpawned>,
            Without<LiftCompositeCameraMarker>,
        ),
    >,
) {
    for (entity, transform, current_layers) in new_unified_layers
        .iter()
        .chain(new_sdf_layers.iter())
        .chain(new_sprite_layers.iter())
    {
        for (owner_entity, binding) in bindings_query.iter() {
            if entity == owner_entity || transform.translation.z >= binding.cutoff_z {
                continue;
            }

            assign_layer_to_background_entity(
                &mut commands,
                entity,
                current_layers,
                binding.render_layer,
            );
        }
    }
}

pub(crate) fn update_lift_comp_material_system(
    bindings_query: Query<(Entity, &AmAnimated, &LiftCompositeBinding)>,
    material_handles: Query<&MeshMaterial2d<UnifiedEffectMaterial>>,
    mut materials: ResMut<Assets<UnifiedEffectMaterial>>,
) {
    for (entity, animated, binding) in bindings_query.iter() {
        if !needs_composite(animated) {
            continue;
        }

        let Ok(mat_handle) = material_handles.get(entity) else {
            continue;
        };
        let Some(material) = materials.get_mut(&mat_handle.0) else {
            continue;
        };

        let needs_update = material.lift_comp_texture.as_ref() != Some(&binding.texture_handle);
        if needs_update {
            material.lift_comp_texture = Some(binding.texture_handle.clone());
            bevy::log::trace!(
                "[LiftComposite] Assigned composite texture to owner {:?} (layer={}, camera={:?})",
                entity,
                binding.render_layer,
                binding.camera_entity,
            );
        }
    }
}

pub(crate) fn cleanup_lift_composite_system(
    mut commands: Commands,
    mut layer_pool: ResMut<EmbedSceneRenderLayerPool>,
    owner_query: Query<(), With<LiftCompositeBinding>>,
    camera_query: Query<(Entity, &LiftCompositeCameraMarker)>,
    mut renderable_layers: ParamSet<(
        UnifiedCompositeLayersQuery,
        SdfCompositeLayersQuery,
        SpriteCompositeLayersQuery,
    )>,
    mut released_layers: Local<Vec<usize>>,
) {
    released_layers.clear();

    for (camera_entity, marker) in camera_query.iter() {
        if owner_query.get(marker.owner_entity).is_ok() {
            continue;
        }

        released_layers.push(marker.render_layer);
        layer_pool.release(marker.render_layer);
        commands.entity(camera_entity).despawn();
        bevy::log::trace!(
            "[LiftComposite] Cleaned orphaned camera {:?} for owner {:?} on layer {}",
            camera_entity,
            marker.owner_entity,
            marker.render_layer,
        );
    }

    if released_layers.is_empty() {
        return;
    }

    for render_layer in released_layers.iter().copied() {
        for mut layers in &mut renderable_layers.p0() {
            *layers = layers.clone().without(render_layer);
        }
        for mut layers in &mut renderable_layers.p1() {
            *layers = layers.clone().without(render_layer);
        }
        for mut layers in &mut renderable_layers.p2() {
            *layers = layers.clone().without(render_layer);
        }
    }
}

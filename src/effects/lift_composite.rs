//! # lift_composite.rs
//!
//! # 复制背景效果 - RTT合成管线
//!
//! Lift (Copy Background) effect - RTT composite pipeline.
//! Renders all layers below a lift layer to a texture, which the lift shader samples from.
//!
//! 将lift图层下方的所有图层渲染到纹理，供lift着色器采样。

use bevy::camera::RenderTarget;
use bevy::camera::ScalingMode;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

use crate::animation::AmAnimated;
use crate::masked_sprite::UnifiedEffectMaterial;
use crate::scene::AmVisualSpawned;
use crate::sdf_material::SdfMaterial;

/// Dedicated RenderLayer for lift/blend composite rendering.
/// Layer 31 is reserved for this purpose (layers 1-30 used by embed RTT pool).
const LIFT_COMPOSITE_RENDER_LAYER: usize = 31;

/// Marker component for the lift composite RTT camera.
#[derive(Component)]
pub struct LiftCompositeCameraMarker;

/// Resource tracking lift/blend composite state for the current scene.
/// Shared by both lift (copy background) and blend mode effects.
#[derive(Resource, Default)]
pub struct LiftCompositeState {
    /// Handle to the RTT texture containing the background composite.
    pub texture_handle: Option<Handle<Image>>,
    /// Entity of the composite camera.
    pub camera_entity: Option<Entity>,
    /// Z value cutoff (layers below this z are included in the composite).
    pub lift_layer_z: f32,
    /// Canvas dimensions for the composite texture.
    pub canvas_width: f32,
    pub canvas_height: f32,
    /// Whether the composite has been initialized.
    pub initialized: bool,
}

/// System to detect lift/blend-mode layers and set up RTT composite rendering.
///
/// When a layer with `lift_has_effect` or a non-Normal blend mode is found:
/// 1. Creates an RTT texture at canvas resolution
/// 2. Creates a camera on RenderLayer 31 that renders to the texture
/// 3. Adds RenderLayer 31 to all layers with lower z values
/// 4. Stores the texture handle for material assignment
pub fn setup_lift_composite_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<LiftCompositeState>,
    window_query: Query<&Window>,
    clear_color: Res<ClearColor>,
    // Newly spawned layers with lift effect or blend mode
    new_layers_query: Query<(Entity, &AmAnimated, &Transform), Added<AmAnimated>>,
    // All renderable layers (for finding layers below lift)
    // Must cover all visual layer types: UnifiedEffect, SDF, and plain Sprite
    unified_layers_query: Query<
        (Entity, &Transform),
        (
            With<MeshMaterial2d<UnifiedEffectMaterial>>,
            Without<LiftCompositeCameraMarker>,
        ),
    >,
    sdf_layers_query: Query<
        (Entity, &Transform),
        (
            With<MeshMaterial2d<SdfMaterial>>,
            Without<LiftCompositeCameraMarker>,
        ),
    >,
    sprite_layers_query: Query<
        (Entity, &Transform),
        (
            With<Sprite>,
            With<AmVisualSpawned>,
            Without<LiftCompositeCameraMarker>,
        ),
    >,
) {
    if state.initialized {
        return;
    }

    // Find the lowest z value among all lift/blend layers
    let mut min_z = f32::MAX;
    let mut canvas_w = 0.0_f32;
    let mut canvas_h = 0.0_f32;
    let mut found = false;

    for (_entity, animated, transform) in new_layers_query.iter() {
        let needs_composite = animated.lift_has_effect || animated.blend_mode.is_blend();
        if !needs_composite {
            continue;
        }
        let z = transform.translation.z;
        if z < min_z {
            min_z = z;
            canvas_w = animated.canvas_width;
            canvas_h = animated.canvas_height;
        }
        found = true;
    }

    if !found {
        return;
    }

    bevy::log::trace!(
        "[LiftComposite] Detected lift/blend layer at z={:.6}, canvas={}x{}",
        min_z,
        canvas_w,
        canvas_h
    );

    // Use window dimensions for RTT texture (smaller = faster for software rendering)
    let (tex_w, tex_h) = if let Some(window) = window_query.iter().next() {
        (window.width() as u32, window.height() as u32)
    } else {
        (canvas_w as u32, canvas_h as u32)
    };

    // Create RTT texture at window resolution
    let size = Extent3d {
        width: tex_w.max(1),
        height: tex_h.max(1),
        depth_or_array_layers: 1,
    };

    let mut render_texture = Image {
        texture_descriptor: TextureDescriptor {
            label: Some("lift_composite_rtt"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };
    render_texture.resize(size);
    let texture_handle = images.add(render_texture);

    // Create composite camera:
    // - Fixed projection matching canvas dimensions
    // - Renders on RenderLayer 31 (dedicated for lift/blend composite)
    // - Order = -50 (renders before main camera but after embed RTT cameras)
    // - Centered at origin (matching the shader's UV formula)
    let camera_entity = commands
        .spawn((
            Name::new("LiftCompositeCamera"),
            LiftCompositeCameraMarker,
            Camera2d,
            Camera {
                clear_color: ClearColorConfig::Custom(clear_color.0),
                order: -50,
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
            RenderLayers::layer(LIFT_COMPOSITE_RENDER_LAYER),
            Transform::from_xyz(0.0, 0.0, 1000.0),
        ))
        .id();

    // Add RenderLayer 31 to all existing layers below the cutoff z
    // Must handle both UnifiedEffectMaterial and SdfMaterial entities
    let mut added_count = 0u32;
    let unified_count = unified_layers_query.iter().count();
    let sdf_count = sdf_layers_query.iter().count();
    let sprite_count = sprite_layers_query.iter().count();
    bevy::log::trace!(
        "[LiftComposite] Searching bg layers: {} unified, {} sdf, {} sprite, cutoff z={:.6}",
        unified_count,
        sdf_count,
        sprite_count,
        min_z
    );
    for (layer_entity, layer_transform) in unified_layers_query
        .iter()
        .chain(sdf_layers_query.iter())
        .chain(sprite_layers_query.iter())
    {
        bevy::log::trace!(
            "[LiftComposite] Checking entity {:?} at z={:.6} (below cutoff {}? {})",
            layer_entity,
            layer_transform.translation.z,
            min_z,
            layer_transform.translation.z < min_z
        );
        if layer_transform.translation.z < min_z {
            added_count += 1;
            commands
                .entity(layer_entity)
                .insert(RenderLayers::from_layers(&[0, LIFT_COMPOSITE_RENDER_LAYER]));
            bevy::log::trace!(
                "[LiftComposite] Added layer 31 to entity {:?} at z={:.6}",
                layer_entity,
                layer_transform.translation.z
            );
        }
    }

    bevy::log::trace!(
        "[LiftComposite] Added RenderLayer 31 to {} background layers",
        added_count
    );

    state.texture_handle = Some(texture_handle);
    state.camera_entity = Some(camera_entity);
    state.lift_layer_z = min_z;
    state.canvas_width = canvas_w;
    state.canvas_height = canvas_h;
    state.initialized = true;

    bevy::log::trace!(
        "[LiftComposite] RTT setup complete: texture={}x{} (canvas={}x{}), camera={:?}",
        tex_w,
        tex_h,
        canvas_w as u32,
        canvas_h as u32,
        camera_entity
    );
}

/// System to assign RenderLayer 31 to newly spawned layers that are below the lift layer.
///
/// This handles layers that are spawned AFTER the lift composite is set up
/// (e.g., layers that appear later in the animation lifecycle).
pub fn propagate_lift_render_layers_system(
    mut commands: Commands,
    state: Res<LiftCompositeState>,
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
    if !state.initialized {
        return;
    }

    for (entity, transform, current_layers) in new_unified_layers
        .iter()
        .chain(new_sdf_layers.iter())
        .chain(new_sprite_layers.iter())
    {
        let already_has = current_layers
            .is_some_and(|l| l.intersects(&RenderLayers::layer(LIFT_COMPOSITE_RENDER_LAYER)));
        bevy::log::trace!(
            "[LiftComposite] Propagate check: entity {:?} z={:.6} cutoff={:.6} already_has={} below={}",
            entity,
            transform.translation.z,
            state.lift_layer_z,
            already_has,
            transform.translation.z < state.lift_layer_z
        );
        if transform.translation.z < state.lift_layer_z && !already_has {
            commands
                .entity(entity)
                .insert(RenderLayers::from_layers(&[0, LIFT_COMPOSITE_RENDER_LAYER]));
            bevy::log::trace!(
                "[LiftComposite] Late-added layer 31 to entity {:?} at z={:.6}",
                entity,
                transform.translation.z
            );
        }
    }
}

/// System to assign the composite texture to lift/blend layers' materials.
///
/// Runs every frame to catch materials that were created after the RTT texture.
pub fn update_lift_comp_material_system(
    state: Res<LiftCompositeState>,
    animated_query: Query<(Entity, &AmAnimated), With<MeshMaterial2d<UnifiedEffectMaterial>>>,
    material_handles: Query<&MeshMaterial2d<UnifiedEffectMaterial>>,
    mut materials: ResMut<Assets<UnifiedEffectMaterial>>,
) {
    let Some(ref texture_handle) = state.texture_handle else {
        return;
    };

    for (entity, animated) in animated_query.iter() {
        if !animated.lift_has_effect && !animated.blend_mode.is_blend() {
            continue;
        }

        let Ok(mat_handle) = material_handles.get(entity) else {
            continue;
        };

        let Some(material) = materials.get_mut(&mat_handle.0) else {
            continue;
        };

        // Only update if not already set
        if material.lift_comp_texture.is_none() {
            material.lift_comp_texture = Some(texture_handle.clone());
            bevy::log::trace!(
                "[LiftComposite] Assigned composite texture to lift layer {:?}",
                entity
            );
        }
    }
}

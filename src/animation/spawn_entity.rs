//! # spawn_entity.rs
//!
//! # 图层实体生成
//!
//! Spawning a complete entity from a PendingLayer with all components.
//! 从 PendingLayer 生成完整实体及所有组件。

use bevy::asset::Assets;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use std::collections::HashMap;

mod transform;
mod visual;

use crate::scene::{
    AmBlendingMode, AmElement, AmElementType, AmEntitySpawned, AmLayerMarker, AmLayerName,
    PendingLayer,
};
use crate::sdf_material::SdfMaterial;

use self::transform::build_spawn_setup;
use self::visual::spawn_visuals_for_layer;

/// Check if a layer is a descendant of another layer (direct or nested).
/// Spawn a complete entity from a PendingLayer.
///
/// For spatial decoupling of embed content:
/// - If `containing_embed_id != 0`, the entity is made a child of embed_contents_container
/// - But its coordinates remain in world space (relative to RTT camera at origin)
/// - The container has identity Transform so GlobalTransform equals Transform
/// - This provides organization while maintaining correct rendering
pub(super) fn spawn_layer_entity(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    unified_materials: &mut Assets<crate::masked_sprite::UnifiedEffectMaterial>,
    color_materials: &mut Assets<ColorMaterial>,
    sdf_materials: &mut Assets<SdfMaterial>,
    layer: &PendingLayer,
    images: &HashMap<String, Handle<Image>>,
    fonts: &HashMap<String, Handle<Font>>,
    white_pixel: Option<&Handle<Image>>,
    parent_entity: Entity,
    perspective_parent: Option<crate::scene::AmPerspectiveParent>,
    _embed_contents_container: Option<Entity>,
    inv_fit_scale: f32,
    embed_owner_id: u64,
    has_child_layers: bool,
    spawned_entities: &HashMap<u64, Entity>,
    global_time: f32,
    prealloc_texture: Option<Handle<Image>>,
) -> Entity {
    let entity_name = format!("Layer[{}]: {}", layer.id, layer.label);
    let spawn_setup = build_spawn_setup(layer, global_time, inv_fit_scale, embed_owner_id);
    let layer_time = spawn_setup.layer_time;
    let transform_to_use = spawn_setup.transform;
    let animated = spawn_setup.animated;

    // **Hybrid Rendering Pipeline**:
    // Non-hidden content starts visible and renders to Layer 0 (main camera).
    // For Composite strategy embeds, content will later be reassigned to RTT layers.
    // This ensures content is always visible and eliminates the first-frame hidden issue.
    //
    // Note: embed content that WAS using containing_embed_id for spatial decoupling
    // now uses Bevy parent-child hierarchy for RenderLayers propagation.
    // In Alight Motion, hiding a layer hides only that layer—its children remain visible.
    // Bevy's Visibility::Hidden cascades to all descendants. For null layers (no visual
    // content), cascade is harmful: a hidden perspective-null parent would make all its
    // children invisible to the RTT camera. We only set Hidden on layers that carry
    // visual content (shapes, images, text) where cascading is harmless.
    let initial_visibility =
        if layer.hidden && !matches!(layer.spec, crate::scene::AmLayerSpec::Null) {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };

    // Determine element type based on layer spec
    // 根据图层规格确定元素类型
    let element_type = match &layer.spec {
        crate::scene::AmLayerSpec::SpriteShape { .. } => AmElementType::Shape,
        crate::scene::AmLayerSpec::SdfShape { .. } => AmElementType::Shape,
        crate::scene::AmLayerSpec::Text { .. } => AmElementType::Text,
        crate::scene::AmLayerSpec::Image { .. } => AmElementType::Image,
        crate::scene::AmLayerSpec::Null => AmElementType::Null,
        crate::scene::AmLayerSpec::EmbedScene => AmElementType::EmbedScene,
        crate::scene::AmLayerSpec::Camera { .. } => AmElementType::Null,
    };

    // Create base entity with common components
    // Include RenderLayers::layer(0) by default - Direct strategy content stays on Layer 0
    // 创建带有通用组件的基础实体
    let entity = commands
        .spawn((
            Name::new(entity_name),
            AmLayerMarker {
                id: layer.id,
                label: layer.label.clone(),
            },
            // 2.3 标识与查询标准化 (Identification & Query Standardization)
            AmLayerName::new(layer.label.clone()),
            AmElement, // Marker for all AM-generated entities
            animated,
            layer.spec.clone(),
            transform_to_use,
            GlobalTransform::default(),
            initial_visibility,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            RenderLayers::layer(0), // Default to Layer 0 (main camera)
        ))
        .id();

    // 2.2 扩展钩子系统 - 触发 AmEntitySpawned 事件
    // (Hook System - trigger AmEntitySpawned event)
    commands.trigger(AmEntitySpawned {
        entity,
        layer_name: layer.label.clone(),
        layer_id: layer.id,
        element_type,
    });

    // If layer is hidden in AM, force it to stay hidden (no rendering of fill, stroke, or effects)
    if layer.hidden {
        commands.entity(entity).insert(crate::scene::AmForceHidden);
    }

    if layer.is_perspective_null {
        commands
            .entity(entity)
            .insert(crate::scene::AmPerspectiveNull);
    }
    if let Some(perspective_parent) = perspective_parent {
        commands.entity(entity).insert(perspective_parent);
    }

    // Add mask info component if this layer is affected by a mask
    if let Some(mask_info) = &layer.mask_info {
        commands.entity(entity).insert(mask_info.clone());
        bevy::log::debug!(
            "[Lifecycle] Layer '{}' has {} mask(s)",
            layer.label,
            mask_info.masks.len()
        );
    }

    // Add camera layer component if this is a camera layer
    if let crate::scene::AmLayerSpec::Camera { ref fov, base_z } = layer.spec {
        commands
            .entity(entity)
            .insert(crate::animation::AmCameraLayer {
                fov: fov.clone(),
                base_z,
                scene_width: layer.animated.canvas_width,
                scene_height: layer.animated.canvas_height,
            });
        bevy::log::info!(
            "[Lifecycle] Camera layer '{}' spawned (base_z={:.1})",
            layer.label,
            base_z
        );
    }

    // Add EmbedScene strategy evaluation marker
    if matches!(layer.spec, crate::scene::AmLayerSpec::EmbedScene) {
        let (scene_w, scene_h) = layer.embed_scene_size.unwrap_or((1280.0, 960.0));
        let render_plan = layer.embed_render_plan.unwrap_or_default();
        commands
            .entity(entity)
            .insert(crate::effects::NeedsStrategyEvaluation {
                scene_width: scene_w,
                scene_height: scene_h,
                has_scale_animation: !layer.animated.scale.keyframes.is_empty(),
                render_plan,
                prealloc_texture,
            });
        // Mark as mask embed if blending is mask/exclude
        if layer.blending_mode == AmBlendingMode::Mask
            || layer.blending_mode == AmBlendingMode::Exclude
        {
            commands.entity(entity).insert(crate::effects::AmEmbedMask);
            bevy::log::warn!(
                "[Lifecycle] Embed '{}' (id={}) marked as mask embed",
                layer.label,
                layer.id
            );
        }
    }

    // Add visual components based on spec (skip for mask and camera layers,
    // but allow EmbedScene masks - they need RTT for texture-based masking)
    let is_mask = layer.blending_mode == AmBlendingMode::Mask
        || layer.blending_mode == AmBlendingMode::Exclude;
    let is_embed_mask = is_mask && matches!(layer.spec, crate::scene::AmLayerSpec::EmbedScene);
    bevy::log::debug!(
        "[spawn_layer_entity] '{}' blending_mode={:?}, is_embed_mask={}, checking visual spawn",
        layer.label,
        layer.blending_mode,
        is_embed_mask
    );
    if (!is_mask || is_embed_mask)
        && !matches!(layer.spec, crate::scene::AmLayerSpec::Camera { .. })
    {
        spawn_visuals_for_layer(
            commands,
            meshes,
            unified_materials,
            color_materials,
            sdf_materials,
            layer,
            entity,
            images,
            fonts,
            white_pixel,
            inv_fit_scale,
            has_child_layers,
            layer_time,
            transform_to_use.rotation,
            global_time,
        );
    } else {
        bevy::log::trace!(
            "[Lifecycle] Skipping visual for mask layer '{}' (id={})",
            layer.label,
            layer.id
        );
    }

    // **Hybrid Rendering Pipeline**:
    // In Direct strategy, ALL content inherits transforms from their embed ancestors.
    // We make content a Bevy child of its parent (from pending.layers), NOT of embed_contents_container.
    // This allows proper Transform propagation through the hierarchy.
    //
    // Note: We still add AmEmbedContentMarker for lifecycle management,
    // but the content is parented to its actual parent (not the container).
    if embed_owner_id != 0 {
        // This is embed content - make it a child of its parent entity
        // This ensures proper Transform inheritance through the Bevy hierarchy
        commands.entity(parent_entity).add_child(entity);

        // Look up the embed entity and add marker for lifecycle management
        if let Some(&embed_entity) = spawned_entities.get(&embed_owner_id) {
            commands
                .entity(entity)
                .insert(crate::scene::AmEmbedContentMarker {
                    embed_entity,
                    embed_id: embed_owner_id,
                });
            bevy::log::debug!(
                "[Lifecycle] Embed content '{}' parented to {:?}, belongs to embed {} ({:?})",
                layer.label,
                parent_entity,
                embed_owner_id,
                embed_entity
            );
        } else {
            // Even if embed lookup fails, still parent correctly for Transform inheritance
            bevy::log::debug!(
                "[Lifecycle] Embed content '{}' parented to {:?} (embed {} not in spawned_entities)",
                layer.label,
                parent_entity,
                embed_owner_id
            );
        }
    } else {
        // Regular layer - add as child of parent
        commands.entity(parent_entity).add_child(entity);
    }

    // Insert echo runtime component if present
    if let Some(ref echo_runtime) = layer.echo_runtime {
        commands.entity(entity).insert(echo_runtime.clone());
    }

    entity
}

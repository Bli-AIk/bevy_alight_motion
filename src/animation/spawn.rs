//! # spawn.rs
//!
//! # 实体生成模块
//!
//! Entity spawning functions for AM layers including process_pending_layers,
//! spawn_layer_entity, and related helper functions.
//!
//! AM 图层的实体生成函数，包括 process_pending_layers、spawn_layer_entity 及相关辅助函数。

use bevy::asset::Assets;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use std::collections::HashMap;

use crate::scene::{
    AmBlendingMode,
    // 新组件导入 (New component imports)
    AmElement,
    AmElementType,
    AmEntitySpawned,
    AmLayerMarker,
    AmLayerName,
    AmPendingLayers,
    PendingLayer,
};
use crate::sdf_material::SdfMaterial;

use super::components::DEBUG_NEGATIVE_HEIGHT_SCALE;
use super::helpers::{get_initial_scale_from_animated, is_descendant_of};
use super::interpolation::{
    interpolate_float, interpolate_vec2, interpolate_vec3_with_extrapolation,
};
use super::visual::add_visual_components;

/// Count total layers including nested ones.
///
/// 计算图层总数（包括嵌套图层）。
pub fn count_total_layers(layers: &[PendingLayer]) -> usize {
    layers
        .iter()
        .map(|l| 1 + count_total_layers(&l.children))
        .sum()
}

/// Process pending layers recursively.
#[allow(clippy::too_many_arguments)]
pub(crate) fn process_pending_layers(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    unified_materials: &mut Assets<crate::masked_sprite::UnifiedEffectMaterial>,
    sdf_materials: &mut Assets<SdfMaterial>,
    pending: &mut AmPendingLayers,
    images: &HashMap<String, Handle<Image>>,
    fonts: &HashMap<String, Handle<Font>>,
    white_pixel: Option<&Handle<Image>>,
    global_time: f32,
    parent_entity: Entity,
    time_offset: i32,
    filter: &crate::scene::LayerFilter,
) {
    // We need to collect actions to avoid borrowing issues
    let mut to_spawn: Vec<usize> = Vec::new(); // indices of layers to spawn
    let mut to_despawn: Vec<u64> = Vec::new(); // layer_id

    // Helper function to check if an ancestor is active (with cycle detection)
    fn is_ancestor_active(
        layer_id: u64,
        layers: &[PendingLayer],
        global_time: f32,
        _time_offset: i32,
    ) -> bool {
        is_ancestor_active_impl(layer_id, layers, global_time, _time_offset, &mut Vec::new())
    }

    fn is_ancestor_active_impl(
        layer_id: u64,
        layers: &[PendingLayer],
        global_time: f32,
        _time_offset: i32,
        visited: &mut Vec<u64>,
    ) -> bool {
        // Cycle detection: if we've already visited this layer, assume active to break cycle
        if visited.contains(&layer_id) {
            bevy::log::warn!(
                "[Lifecycle] Cycle detected at layer_id={}, breaking recursion",
                layer_id
            );
            return true;
        }
        visited.push(layer_id);

        let layer = match layers.iter().find(|l| l.id == layer_id) {
            Some(l) => l,
            None => return true, // If not found, assume active (root)
        };

        if layer.parent == 0 {
            return true; // No parent, always considered active from parent perspective
        }

        // Self-reference check
        if layer.parent == layer.id {
            bevy::log::warn!(
                "[Lifecycle] Self-referencing parent at layer '{}' (id={}), treating as root",
                layer.label,
                layer.id
            );
            return true;
        }

        // Check parent's active status
        let parent = match layers.iter().find(|l| l.id == layer.parent) {
            Some(p) => p,
            None => return true, // Parent not in our list, assume active
        };

        // Use local_time for visibility (affected by speed)
        // local_time = (global_time - time_offset) * speed_multiplier
        let parent_local_time = parent.animated.calc_local_time(global_time);
        let parent_active = parent_local_time >= parent.start_time as f32
            && parent_local_time < parent.end_time as f32;

        if !parent_active {
            return false; // Parent is not active
        }

        // Recursively check grandparent
        is_ancestor_active_impl(layer.parent, layers, global_time, _time_offset, visited)
    }

    for (idx, layer) in pending.layers.iter().enumerate() {
        // Use local_time for visibility (affected by speed)
        // local_time = (global_time - time_offset) * speed_multiplier
        let local_time = layer.animated.calc_local_time(global_time);

        // Check if layer should be active (considering both own time range and parent's time range)
        // Note: AM uses half-open interval [start, end) for layer visibility
        let own_time_active =
            local_time >= layer.start_time as f32 && local_time < layer.end_time as f32;

        // Check if all ancestors are active
        let ancestors_active =
            is_ancestor_active(layer.id, &pending.layers, global_time, time_offset);

        let should_be_active = own_time_active && ancestors_active;

        let is_spawned = pending.spawned_entities.contains_key(&layer.id);

        // Debug: log layer status (only first 5 frames and first 10 layers)
        static mut DEBUG_FRAME: u32 = 0;
        unsafe {
            if DEBUG_FRAME < 5 && idx < 10 {
                bevy::log::debug!(
                    "[Lifecycle] Layer '{}' (id={}, parent={}): time={:.1}ms, local_time={:.1}, range={}..{}, own_active={}, ancestors_active={}, spawned={}",
                    layer.label,
                    layer.id,
                    layer.parent,
                    global_time,
                    local_time,
                    layer.start_time,
                    layer.end_time,
                    own_time_active,
                    ancestors_active,
                    is_spawned
                );
            }
            if idx == 0 {
                DEBUG_FRAME += 1;
            }
        }

        // 应用过滤器检查 (Apply filter check)
        let should_spawn_filtered = should_be_active && filter.should_spawn(&layer.label);

        if should_spawn_filtered && !is_spawned {
            to_spawn.push(idx);
        } else if !should_be_active && is_spawned {
            to_despawn.push(layer.id);
        }
    }

    // Despawn entities that are no longer active
    for layer_id in to_despawn {
        if let Some(entity) = pending.spawned_entities.remove(&layer_id) {
            // Find layer info for logging
            if let Some(layer) = pending.layers.iter().find(|l| l.id == layer_id) {
                bevy::log::trace!(
                    "  [Lifecycle] Despawning '{}' (id={})",
                    layer.label,
                    layer_id
                );
            }

            // Find all children of this layer (direct and nested) and despawn them first
            let children_to_remove: Vec<u64> = pending
                .layers
                .iter()
                .filter(|l| is_descendant_of(l.id, layer_id, &pending.layers))
                .map(|l| l.id)
                .collect();

            // Despawn children (deepest first would be ideal, but order doesn't matter much
            // since we're despawning them all)
            for child_id in children_to_remove {
                if let Some(child_entity) = pending.spawned_entities.remove(&child_id) {
                    if let Some(child) = pending.layers.iter().find(|l| l.id == child_id) {
                        bevy::log::trace!(
                            "    [Lifecycle] (cascade) Despawning child '{}' (id={})",
                            child.label,
                            child_id
                        );
                    }
                    commands.entity(child_entity).despawn();
                }
            }

            // Despawn the entity itself
            commands.entity(entity).despawn();
        }
    }

    // Sort layers to spawn by dependency (parents before children) using topological sort
    // Build a set of layer IDs being spawned this frame
    let spawning_ids: std::collections::HashSet<u64> =
        to_spawn.iter().map(|&idx| pending.layers[idx].id).collect();

    // Helper function to count dependency depth (how many ancestors are also being spawned)
    // For embed content, we also need to consider containing_embed_id as a dependency
    fn count_spawn_depth(
        layer_id: u64,
        layers: &[PendingLayer],
        spawning_ids: &std::collections::HashSet<u64>,
        visited: &mut std::collections::HashSet<u64>,
    ) -> usize {
        if visited.contains(&layer_id) {
            return 0; // Prevent infinite loop
        }
        visited.insert(layer_id);

        let layer = match layers.iter().find(|l| l.id == layer_id) {
            Some(l) => l,
            None => return 0,
        };

        // Calculate depth from parent chain
        let parent_depth = if layer.parent == 0 || !spawning_ids.contains(&layer.parent) {
            0
        } else {
            1 + count_spawn_depth(layer.parent, layers, spawning_ids, visited)
        };

        // For embed content, containing_embed_id must also be spawned first
        let embed_depth = if layer.containing_embed_id == 0
            || !spawning_ids.contains(&layer.containing_embed_id)
        {
            0
        } else {
            1 + count_spawn_depth(layer.containing_embed_id, layers, spawning_ids, visited)
        };

        // Return the maximum depth to ensure all dependencies are spawned first
        parent_depth.max(embed_depth)
    }

    // Sort by depth (lower depth = spawn first)
    to_spawn.sort_by_key(|&idx| {
        let layer_id = pending.layers[idx].id;
        let mut visited = std::collections::HashSet::new();
        count_spawn_depth(layer_id, &pending.layers, &spawning_ids, &mut visited)
    });

    // Spawn new entities in dependency order
    for idx in to_spawn {
        let layer = &pending.layers[idx];

        // Determine parent for this entity
        let actual_parent = if layer.parent != 0 {
            match pending.spawned_entities.get(&layer.parent) {
                Some(&e) => e,
                None => {
                    bevy::log::warn!(
                        "[Lifecycle] WARNING: Parent {} not found for '{}' (id={}), using root",
                        layer.parent,
                        layer.label,
                        layer.id
                    );
                    parent_entity
                }
            }
        } else {
            parent_entity
        };

        let entity = spawn_layer_entity(
            commands,
            meshes,
            unified_materials,
            sdf_materials,
            layer,
            images,
            fonts,
            white_pixel,
            actual_parent,
            pending.embed_contents_container,
            pending.inv_fit_scale,
            &pending.spawned_entities,
            global_time,
        );

        bevy::log::debug!(
            "[Lifecycle] Spawned '{}' (id={}, parent={}, embed={}, z={:.6}, time={}..{}ms) -> Entity {:?} parented to {:?}",
            layer.label,
            layer.id,
            layer.parent,
            layer.containing_embed_id,
            layer.transform.translation.z,
            layer.start_time,
            layer.end_time,
            entity,
            actual_parent
        );

        pending.spawned_entities.insert(layer.id, entity);
    }
}

/// Check if a layer is a descendant of another layer (direct or nested).
/// Spawn a complete entity from a PendingLayer.
///
/// For spatial decoupling of embed content:
/// - If `containing_embed_id != 0`, the entity is made a child of embed_contents_container
/// - But its coordinates remain in world space (relative to RTT camera at origin)
/// - The container has identity Transform so GlobalTransform equals Transform
/// - This provides organization while maintaining correct rendering
#[allow(clippy::too_many_arguments)]
fn spawn_layer_entity(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    unified_materials: &mut Assets<crate::masked_sprite::UnifiedEffectMaterial>,
    sdf_materials: &mut Assets<SdfMaterial>,
    layer: &PendingLayer,
    images: &HashMap<String, Handle<Image>>,
    fonts: &HashMap<String, Handle<Font>>,
    white_pixel: Option<&Handle<Image>>,
    parent_entity: Entity,
    _embed_contents_container: Option<Entity>,
    inv_fit_scale: f32,
    spawned_entities: &HashMap<u64, Entity>,
    global_time: f32,
) -> Entity {
    let entity_name = format!("Layer[{}]: {}", layer.id, layer.label);

    // Check if layer has any effects that need scale baking
    let has_wipe = layer.animated.wipe_end.value != Some(1.0)
        || !layer.animated.wipe_end.keyframes.is_empty()
        || layer.animated.wipe_start.value.is_some()
        || !layer.animated.wipe_start.keyframes.is_empty();

    let has_stretch = layer.animated.stretch_amount.value.is_some()
        || !layer.animated.stretch_amount.keyframes.is_empty()
        || layer.animated.stretch_angle.value.is_some()
        || !layer.animated.stretch_angle.keyframes.is_empty()
        || layer.animated.stretch_offset.value.is_some()
        || !layer.animated.stretch_offset.keyframes.is_empty()
        || layer.animated.stretch_smooth.value.is_some()
        || !layer.animated.stretch_smooth.keyframes.is_empty();

    let has_blur = layer.animated.blur_strength.value.is_some()
        || !layer.animated.blur_strength.keyframes.is_empty();

    let has_mask = layer.mask_info.is_some();
    let needs_effect = has_wipe || has_stretch || has_mask || has_blur;

    // Calculate correct initial position at spawn time (to prevent frame jump)
    // Use the same logic as animate_transform_system
    let animated = &layer.animated;

    // Calculate local time for animation interpolation
    let mut local_time = animated.calc_local_time(global_time);

    bevy::log::trace!(
        "[SpawnTime] '{}' global_time={:.1}, local_time={:.1}, start_time={}, end_time={}, time_offset={:.1}, speed={:.2}",
        layer.label,
        global_time,
        local_time,
        layer.start_time,
        layer.end_time,
        animated.time_offset,
        animated.speed_multiplier
    );

    // For embed content, add 0.5 frame offset to match AM's internal timing
    if layer.containing_embed_id != 0 && animated.speed_multiplier != 0.0 {
        let frame_duration_ms = 1000.0 / 30.0;
        local_time += frame_duration_ms * 0.5;
    }

    // Calculate normalized time within layer duration
    let layer_time = animated.calc_layer_time(local_time);

    // Get current scale for pivot compensation
    // For effect layers and SDF shapes, magnitude is baked into mesh, but we need the sign for flipping
    let actual_scale = interpolate_vec2(&animated.scale, layer_time).unwrap_or([1.0, 1.0]);
    let current_scale =
        if matches!(layer.spec, crate::scene::AmLayerSpec::SdfShape { .. }) || needs_effect {
            [1.0_f32, 1.0_f32]
        } else {
            actual_scale
        };

    // Calculate initial position using animation interpolation
    // Use extrapolation for location to improve accuracy before first keyframe
    let initial_position = if let Some(loc) =
        interpolate_vec3_with_extrapolation(&animated.location, layer_time)
    {
        let (mut bx, mut by) = if animated.has_parent {
            // For layers with parents, use local coordinates
            (loc[0], -loc[1])
        } else {
            // For root layers, convert from canvas coordinates
            (
                loc[0] - animated.canvas_width / 2.0,
                animated.canvas_height / 2.0 - loc[1],
            )
        };

        // Apply pivot compensation (simplified - full logic is in animate_transform_system)
        if let Some(pivot) = interpolate_vec2(&animated.pivot, layer_time) {
            let pivot_x = pivot[0];
            let pivot_y = pivot[1];

            if matches!(layer.spec, crate::scene::AmLayerSpec::SdfShape { .. }) {
                // SDF shapes: translation is at transform center
                bx += pivot_x;
                by -= pivot_y;
            } else if matches!(layer.spec, crate::scene::AmLayerSpec::EmbedScene) {
                // Embed scenes: need rotation-aware pivot compensation
                let rotation_deg = interpolate_float(&animated.rotation, layer_time).unwrap_or(0.0);
                let rotation_rad = (-rotation_deg).to_radians();
                let pivot_bevy_y = -pivot_y;
                let scaled_offset_x = -pivot_x * current_scale[0];
                let scaled_offset_y = -pivot_bevy_y * current_scale[1];
                let rotated_offset_x =
                    scaled_offset_x * rotation_rad.cos() - scaled_offset_y * rotation_rad.sin();
                let rotated_offset_y =
                    scaled_offset_x * rotation_rad.sin() + scaled_offset_y * rotation_rad.cos();
                bx += pivot_x + rotated_offset_x;
                by += pivot_bevy_y + rotated_offset_y;
            } else {
                // Standard shapes: pivot offset is already applied in collect_types.rs
                // via pivot_to_anchor_and_offset and transform.translation
                // No additional compensation needed here
            }
        }

        // Apply effect position offsets (transform2 effect)
        if let Some(effect_x) = interpolate_float(&animated.effect_pos_x, layer_time) {
            bx += effect_x;
        }
        if let Some(effect_y) = interpolate_float(&animated.effect_pos_y, layer_time) {
            by -= effect_y; // Y is inverted
        }

        // Apply font Y offset for text layers (to compensate for different font metrics)
        if !animated.has_parent {
            by -= animated.font_y_offset;
        }

        // Apply anchor offset compensation for SpriteShape with non-center pivot
        // NOTE: Skip for SDF shapes - their pivot is already handled above via `by -= pivot_y`
        if !matches!(layer.spec, crate::scene::AmLayerSpec::SdfShape { .. }) {
            bx += animated.anchor_offset.x;
            by += animated.anchor_offset.y;
        }

        Vec3::new(bx, by, layer.transform.translation.z)
    } else {
        layer.transform.translation
    };

    // Calculate initial rotation
    let initial_rotation = if let Some(rot_deg) = interpolate_float(&animated.rotation, layer_time)
    {
        Quat::from_rotation_z((-rot_deg).to_radians())
    } else {
        layer.transform.rotation
    };

    // Calculate initial scale
    let initial_scale =
        if needs_effect || matches!(layer.spec, crate::scene::AmLayerSpec::SdfShape { .. }) {
            // For effect layers and SDF shapes, keep only the sign of scale for flipping
            // The magnitude is baked into the mesh
            Vec3::new(actual_scale[0].signum(), actual_scale[1].signum(), 1.0)
        } else {
            Vec3::new(current_scale[0], current_scale[1], 1.0)
        };

    bevy::log::debug!(
        "[SpawnInit] '{}' layer_time={:.4}, pos=({:.1},{:.1},{:.4}), rot={:.2}°, scale=({:.3},{:.3})",
        layer.label,
        layer_time,
        initial_position.x,
        initial_position.y,
        initial_position.z,
        initial_rotation
            .to_euler(bevy::math::EulerRot::ZYX)
            .0
            .to_degrees(),
        initial_scale.x,
        initial_scale.y
    );

    // Create transform with calculated initial values
    let transform_to_use = Transform {
        translation: initial_position,
        rotation: initial_rotation,
        scale: initial_scale,
    };

    // Clone animated component and set inv_fit_scale for embed children
    // Use containing_embed_id to detect embed content, not embed_offset
    // (embed_offset can be ZERO when embed is at canvas center)
    let mut animated = layer.animated.clone();
    if animated.scale_assist_axis != 0 {
        bevy::log::info!(
            "[SPAWN] Layer '{}' has scale_assist_axis={}, keyframes={}",
            layer.label,
            animated.scale_assist_axis,
            animated.scale_assist.keyframes.len()
        );
    }
    if layer.containing_embed_id != 0 {
        animated.inv_fit_scale = inv_fit_scale;
    }

    // **Hybrid Rendering Pipeline**:
    // All content starts visible and renders to Layer 0 (main camera).
    // For Composite strategy embeds, content will later be reassigned to RTT layers.
    // This ensures content is always visible and eliminates the first-frame hidden issue.
    //
    // Note: embed content that WAS using containing_embed_id for spatial decoupling
    // now uses Bevy parent-child hierarchy for RenderLayers propagation.
    let initial_visibility = Visibility::Inherited;

    // Determine element type based on layer spec
    // 根据图层规格确定元素类型
    let element_type = match &layer.spec {
        crate::scene::AmLayerSpec::SpriteShape { .. } => AmElementType::Shape,
        crate::scene::AmLayerSpec::SdfShape { .. } => AmElementType::Shape,
        crate::scene::AmLayerSpec::Text { .. } => AmElementType::Text,
        crate::scene::AmLayerSpec::Image { .. } => AmElementType::Image,
        crate::scene::AmLayerSpec::Null => AmElementType::Null,
        crate::scene::AmLayerSpec::EmbedScene => AmElementType::EmbedScene,
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

    // Add mask info component if this layer is affected by a mask
    if let Some(mask_info) = &layer.mask_info {
        commands.entity(entity).insert(mask_info.clone());
        bevy::log::debug!(
            "[Lifecycle] Layer '{}' has {} mask(s)",
            layer.label,
            mask_info.masks.len()
        );
    }

    // Add visual components based on spec (skip for mask layers)
    bevy::log::debug!(
        "[spawn_layer_entity] '{}' blending_mode={:?}, checking visual spawn",
        layer.label,
        layer.blending_mode
    );
    if layer.blending_mode != AmBlendingMode::Mask && layer.blending_mode != AmBlendingMode::Exclude
    {
        // Extract initial scale from animated data for SDF shapes
        // (transform.scale is set to 1.0 for SDF shapes, actual scale is in animated)
        let initial_scale = get_initial_scale_from_animated(&layer.animated.scale);

        // Check if layer has wipe effect
        let has_wipe = layer.animated.wipe_end.value != Some(1.0)
            || !layer.animated.wipe_end.keyframes.is_empty()
            || layer.animated.wipe_start.value.is_some()
            || !layer.animated.wipe_start.keyframes.is_empty();

        // Check if layer has stretch segment effect
        let has_stretch = layer.animated.stretch_amount.value.is_some()
            || !layer.animated.stretch_amount.keyframes.is_empty()
            || layer.animated.stretch_angle.value.is_some()
            || !layer.animated.stretch_angle.keyframes.is_empty()
            || layer.animated.stretch_offset.value.is_some()
            || !layer.animated.stretch_offset.keyframes.is_empty()
            || layer.animated.stretch_smooth.value.is_some()
            || !layer.animated.stretch_smooth.keyframes.is_empty();

        // Check if layer has blur effect
        let has_blur = layer.animated.blur_strength.value.is_some()
            || !layer.animated.blur_strength.keyframes.is_empty();

        // Get initial wipe params
        let initial_wipe = if has_wipe {
            let wipe_start = layer.animated.wipe_start.value.unwrap_or(0.0);
            let wipe_end = layer.animated.wipe_end.value.unwrap_or(1.0);
            let wipe_angle = layer.animated.wipe_angle.value.unwrap_or(0.0);
            let wipe_feather = layer.animated.wipe_feather.value.unwrap_or(0.0);
            Some(Vec4::new(wipe_start, wipe_end, wipe_angle, wipe_feather))
        } else {
            None
        };

        // Get initial stretch segment params
        let initial_stretch = if has_stretch {
            let angle_deg = layer.animated.stretch_angle.value.unwrap_or(0.0);
            let angle_rad = angle_deg.to_radians();
            let stretch_px = layer.animated.stretch_amount.value.unwrap_or(0.0);
            let stretch_uv = stretch_px / 500.0;
            let offset_px = layer.animated.stretch_offset.value.unwrap_or(0.0);
            let offset_uv = offset_px / 500.0;
            let smooth = layer.animated.stretch_smooth.value.unwrap_or(0.0);
            let smooth_width = smooth * 0.3;
            Some(Vec4::new(angle_rad, stretch_uv, offset_uv, smooth_width))
        } else {
            None
        };

        // Get initial blur params and calculate max blur for mesh expansion
        let initial_blur = if has_blur {
            let blur_strength = layer.animated.blur_strength.value.unwrap_or(0.0);
            // AM strength 2.0 produces very strong blur
            // Use strength * 80 to match animate_unified_effect_system
            let blur_radius = blur_strength * 80.0;
            Some(Vec4::new(blur_radius, 0.0, 0.0, 0.0))
        } else {
            None
        };

        // Calculate maximum blur strength from keyframes for mesh expansion
        let max_blur_radius = if has_blur {
            let mut max_strength = layer.animated.blur_strength.value.unwrap_or(0.0);
            for kf in &layer.animated.blur_strength.keyframes {
                if let Ok(v) = kf.value.parse::<f32>() {
                    max_strength = max_strength.max(v);
                }
            }
            // Same multiplier as used in animation system
            max_strength * 80.0
        } else {
            0.0
        };

        // For embed content rendered to RTT, use original size (no scaling)
        // The final display size will be affected by embed's inherited fit_scale
        let size_scale = 1.0;

        // Calculate initial stretch mesh bounds and mesh_offset to prevent first frame jump
        // This replicates the logic from animate_unified_effect_system
        let (initial_mesh_offset, initial_stretch_mesh_bounds) = if has_stretch {
            // Use interpolation at layer_time to match animate_unified_effect_system
            let sprite_size =
                interpolate_vec2(&layer.animated.size, layer_time).unwrap_or([100.0, 100.0]);
            let scale = interpolate_vec2(&layer.animated.scale, layer_time).unwrap_or([1.0, 1.0]);
            let orig_width = (sprite_size[0] * scale[0]).abs().max(1.0);
            let orig_height = (sprite_size[1] * scale[1]).abs().max(1.0);

            // Get stretch parameters using interpolation
            let angle_deg =
                interpolate_float(&layer.animated.stretch_angle, layer_time).unwrap_or(0.0);
            let transform_rotation_rad = initial_rotation.to_euler(bevy::math::EulerRot::XYZ).2;
            let angle_rad = angle_deg.to_radians() + transform_rotation_rad;
            let stretch_px =
                interpolate_float(&layer.animated.stretch_amount, layer_time).unwrap_or(0.0);
            let offset_px =
                interpolate_float(&layer.animated.stretch_offset, layer_time).unwrap_or(0.0);

            // Calculate base_size (same logic as animate_unified_effect_system)
            let has_negative_size_y = sprite_size[1] < 0.0;
            let base_size = if has_negative_size_y {
                (orig_width * orig_width + orig_height * orig_height).sqrt()
                    * DEBUG_NEGATIVE_HEIGHT_SCALE
            } else if orig_width >= orig_height {
                orig_width
            } else {
                let rot_cos = transform_rotation_rad.cos().abs();
                let rot_sin = transform_rotation_rad.sin().abs();
                let world_w = orig_width * rot_cos + orig_height * rot_sin;
                0.8 * world_w + 0.2 * orig_width
            };
            let base_divisor = base_size / 4.27; // Best match for reference
            let stretch_factor = 1.0 + stretch_px / base_divisor;

            let mut actual_stretch_px = orig_width * stretch_factor - orig_width;

            // Apply embed RTT compensation if this is embed content
            if layer.containing_embed_id != 0 {
                let ratio = layer.animated.canvas_height / 960.0;
                actual_stretch_px *= ratio;
            }

            let angle_factor = 1.0 - 0.1 * angle_rad.sin().abs();
            let half_gap = actual_stretch_px * 0.5 * angle_factor;

            let rotate = |x: f32, y: f32, angle: f32| -> (f32, f32) {
                let c = angle.cos();
                let s = angle.sin();
                (x * c - y * s, x * s + y * c)
            };

            let transform_vertex = |vx: f32, vy: f32| -> (f32, f32) {
                let (rx, ry) = rotate(vx, vy, angle_rad);
                let shifted_x = rx + offset_px;
                let pushed_x = rx + shifted_x.signum() * half_gap;
                rotate(pushed_x, ry, -angle_rad)
            };

            let hw = orig_width / 2.0;
            let hh = orig_height / 2.0;
            let corners = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)];

            let mut min_x = f32::MAX;
            let mut max_x = f32::MIN;
            let mut min_y = f32::MAX;
            let mut max_y = f32::MIN;

            for (cx, cy) in corners {
                let (tx, ty) = transform_vertex(cx, cy);
                min_x = min_x.min(tx);
                max_x = max_x.max(tx);
                min_y = min_y.min(ty);
                max_y = max_y.max(ty);
            }

            // No padding - the calculated bounds should be exact for stretch effect
            // Padding would cause sample_uv to go outside [0,1] range

            let center_offset_x = (min_x + max_x) / 2.0;
            let center_offset_y = (min_y + max_y) / 2.0;

            bevy::log::trace!(
                "[SpawnStretch] layer '{}' orig=({:.1},{:.1}) stretch_px={:.1} actual={:.1} offset=({:.2},{:.2})",
                layer.label,
                orig_width,
                orig_height,
                stretch_px,
                actual_stretch_px,
                center_offset_x,
                center_offset_y
            );

            (
                Some(Vec4::new(center_offset_x, center_offset_y, 0.0, 0.0)),
                Some((min_x, max_x, min_y, max_y)),
            )
        } else {
            (None, None)
        };

        // Get initial replace color params
        let initial_replace_color = if layer.animated.replace_old_color != Vec4::ZERO
            || layer.animated.replace_new_color.value.is_some()
            || !layer.animated.replace_new_color.keyframes.is_empty()
        {
            // Extract initial new_color value
            // If has keyframes, get color from first keyframe; otherwise use static value or default
            let new_color_srgb = if let Some(val) = layer.animated.replace_new_color.value {
                val
            } else if !layer.animated.replace_new_color.keyframes.is_empty() {
                // Parse first keyframe's color value (format: "r,g,b,a")
                let first_kf = &layer.animated.replace_new_color.keyframes[0];
                super::interpolation::parse_keyframe_color(&first_kf.value)
                    .unwrap_or(Vec4::new(1.0, 1.0, 1.0, 1.0))
            } else {
                Vec4::new(1.0, 1.0, 1.0, 1.0)
            };

            let threshold = layer.animated.replace_threshold.value.unwrap_or(0.25);
            let feather = layer.animated.replace_feather.value.unwrap_or(0.25);
            let alpha = layer.animated.replace_alpha.value.unwrap_or(1.0);
            let lock_lum = if layer.animated.replace_lock_luminance {
                1.0
            } else {
                0.0
            };

            let flags = Vec4::new(1.0, lock_lum, 0.0, 0.0); // enabled, lock_luminance
            let params = Vec4::new(threshold, feather, alpha, 0.0);

            // Pass colors directly in sRGB - shader will convert to linear
            Some((
                flags,
                layer.animated.replace_old_color,
                new_color_srgb,
                params,
            ))
        } else {
            None
        };

        add_visual_components(
            commands,
            meshes,
            unified_materials,
            sdf_materials,
            entity,
            &layer.spec,
            &layer.mask_info,
            layer.palette_params.as_ref(),
            images,
            fonts,
            white_pixel,
            &layer.label,
            layer.id,
            initial_scale,
            initial_wipe,
            initial_stretch,
            initial_blur,
            layer.embed_scene_size,
            size_scale,
            max_blur_radius,
            initial_mesh_offset,
            initial_stretch_mesh_bounds,
            1.0 / inv_fit_scale,            // fit_scale for mask coordinates
            layer.containing_embed_id != 0, // is_embed_content - force effect material for bounds clipping
            !layer.animated.scale.keyframes.is_empty(), // has_scale_animation - needs bounds clipping
            layer.animated.scale_assist_axis != 0, // has_scale_assist - needs UnifiedEffectMaterial for dynamic sizing
            layer.animated.repeat_count.value.is_some_and(|v| v > 0.0)
                || !layer.animated.repeat_count.keyframes.is_empty()
                || layer
                    .animated
                    .linear_repeat_count
                    .value
                    .is_some_and(|v| v > 0.0)
                || !layer.animated.linear_repeat_count.keyframes.is_empty(), // has_repeat - needs UnifiedEffectMaterial
            layer.animated.threshold_value.value.is_some()
                || !layer.animated.threshold_value.keyframes.is_empty(), // has_threshold - needs UnifiedEffectMaterial
            layer.animated.grid_spacing.value.is_some()
                || !layer.animated.grid_spacing.keyframes.is_empty(), // has_grid - needs UnifiedEffectMaterial
            global_time as u64,    // current playback time for mask initialization
            initial_replace_color, // replace color params
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
    if layer.containing_embed_id != 0 {
        // This is embed content - make it a child of its parent entity
        // This ensures proper Transform inheritance through the Bevy hierarchy
        commands.entity(parent_entity).add_child(entity);

        // Look up the embed entity and add marker for lifecycle management
        if let Some(&embed_entity) = spawned_entities.get(&layer.containing_embed_id) {
            commands
                .entity(entity)
                .insert(crate::scene::AmEmbedContentMarker {
                    embed_entity,
                    embed_id: layer.containing_embed_id,
                });
            bevy::log::debug!(
                "[Lifecycle] Embed content '{}' parented to {:?}, belongs to embed {} ({:?})",
                layer.label,
                parent_entity,
                layer.containing_embed_id,
                embed_entity
            );
        } else {
            // Even if embed lookup fails, still parent correctly for Transform inheritance
            bevy::log::debug!(
                "[Lifecycle] Embed content '{}' parented to {:?} (embed {} not in spawned_entities)",
                layer.label,
                parent_entity,
                layer.containing_embed_id
            );
        }
    } else {
        // Regular layer - add as child of parent
        commands.entity(parent_entity).add_child(entity);
    }

    entity
}

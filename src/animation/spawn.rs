//! # spawn.rs
//!
//! # 实体生成模块
//!
//! Entity spawning functions for AM layers including process_pending_layers,
//! spawn_layer_entity, and related helper functions.
//!
//! AM 图层的实体生成函数，包括 process_pending_layers、spawn_layer_entity 及相关辅助函数。

use bevy::asset::Assets;
use bevy::prelude::*;
use std::collections::HashMap;

use crate::scene::{AmPendingLayers, PendingLayer};
use crate::sdf_material::SdfMaterial;

use super::helpers::is_descendant_of;
use super::spawn_entity::spawn_layer_entity;

fn trace_lifecycle_enabled(layer_id: u64) -> bool {
    std::env::var_os("AM_TRACE_LIFECYCLE_IDS")
        .and_then(|value| value.into_string().ok())
        .is_some_and(|ids| {
            ids.split(',')
                .filter_map(|value| value.trim().parse::<u64>().ok())
                .any(|id| id == layer_id)
        })
}

/// Count total layers including nested ones.
///
/// 计算图层总数（包括嵌套图层）。
#[allow(dead_code)]
pub fn count_total_layers(layers: &[PendingLayer]) -> usize {
    layers
        .iter()
        .map(|l| 1 + count_total_layers(&l.children))
        .sum()
}

/// Process pending layers recursively.
pub(crate) fn process_pending_layers(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    unified_materials: &mut Assets<crate::masked_sprite::UnifiedEffectMaterial>,
    color_materials: &mut Assets<ColorMaterial>,
    sdf_materials: &mut Assets<SdfMaterial>,
    pending: &mut AmPendingLayers,
    images: &HashMap<String, Handle<Image>>,
    fonts: &HashMap<String, Handle<Font>>,
    white_pixel: Option<&Handle<Image>>,
    global_time: f32,
    parent_entity: Entity,
    time_offset: f32,
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
        _time_offset: f32,
    ) -> bool {
        is_ancestor_active_impl(layer_id, layers, global_time, _time_offset, &mut Vec::new())
    }

    fn is_ancestor_active_impl(
        layer_id: u64,
        layers: &[PendingLayer],
        global_time: f32,
        _time_offset: f32,
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

        // Use calc_local_time for visibility (applies accumulated parent speed).
        // start_time/end_time are in parent-scene time, so we need speed scaling.
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
        // Use calc_local_time for visibility (applies accumulated parent speed).
        // start_time/end_time are in parent-scene time, so we need speed scaling.
        let local_time = layer.animated.calc_local_time(global_time);

        // Check if layer should be active (considering both own time range and parent's time range)
        // Note: AM uses half-open interval [start, end) for layer visibility
        // Note: PendingLayer.end_time is already extended by echo_time_shift_ms
        // via extend_children_lifecycle() for echo/repeat copies.
        let own_time_active =
            local_time >= layer.start_time as f32 && local_time < layer.end_time as f32;

        // Check if all ancestors are active
        let ancestors_active =
            is_ancestor_active(layer.id, &pending.layers, global_time, time_offset);

        let should_be_active = own_time_active && ancestors_active;

        let is_spawned = pending.spawned_entities.contains_key(&layer.id);

        if trace_lifecycle_enabled(layer.id) {
            bevy::log::warn!(
                "[LifecycleTrace] id={} label='{}' parent={} global={:.1} local={:.1} range={}..{} own_active={} ancestors_active={} spawned={}",
                layer.id,
                layer.label,
                layer.parent,
                global_time,
                local_time,
                layer.start_time,
                layer.end_time,
                own_time_active,
                ancestors_active,
                is_spawned,
            );
        }

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
        let Some(entity) = pending.spawned_entities.remove(&layer_id) else {
            continue;
        };

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

        // Remove children from spawned_entities tracking (parent despawn will handle
        // the actual ECS despawn recursively)
        for child_id in children_to_remove {
            if let Some(_child_entity) = pending.spawned_entities.remove(&child_id)
                && let Some(child) = pending.layers.iter().find(|l| l.id == child_id)
            {
                bevy::log::trace!(
                    "    [Lifecycle] (cascade) Removing '{}' (id={}) from tracking",
                    child.label,
                    child_id
                );
            }
        }

        // Despawn the entity (and all ECS children recursively)
        commands.entity(entity).despawn();
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

    let trace_spawn_order = std::env::var_os("AM_SPAWN_ORDER_TRACE").is_some();
    if trace_spawn_order
        && to_spawn
            .iter()
            .any(|&idx| pending.layers[idx].label.contains("空"))
    {
        bevy::log::info!("[SPAWN_ORDER] Spawning {} layers:", to_spawn.len());
        for &idx in &to_spawn {
            let layer = &pending.layers[idx];
            let depth = {
                let mut visited = std::collections::HashSet::new();
                count_spawn_depth(layer.id, &pending.layers, &spawning_ids, &mut visited)
            };
            bevy::log::info!(
                "[SPAWN_ORDER]   depth={}: '{}' (id={}, parent={})",
                depth,
                layer.label,
                layer.id,
                layer.parent
            );
        }
    }

    // Spawn new entities in dependency order
    for idx in to_spawn {
        let layer = &pending.layers[idx];

        let perspective_parent_layer = pending
            .layers
            .iter()
            .find(|candidate| candidate.id == layer.parent)
            .filter(|parent_layer| parent_layer.is_perspective_null);
        // AM's generic layer-parenting applies ordinary parent transforms at sample time.
        // Only embedded scenes need flattening here so RTT/composite ownership stays sane.
        let flatten_under_perspective_parent = perspective_parent_layer.is_some()
            && matches!(layer.spec, crate::scene::AmLayerSpec::EmbedScene);
        let perspective_parent = if flatten_under_perspective_parent {
            pending
                .spawned_entities
                .get(&layer.parent)
                .copied()
                .map(|entity| crate::scene::AmPerspectiveParent {
                    entity,
                    layer_id: layer.parent,
                })
        } else {
            None
        };

        // Determine Bevy hierarchy parent for this entity.
        // Embedded scenes under perspective nulls stay detached so RTT/composite
        // systems do not treat them as ordinary nested embeds. Plain layers and
        // perspective-null chains keep the normal ECS hierarchy so Bevy parenting
        // matches AM's default layer-parenting semantics more closely.
        let actual_parent = if perspective_parent.is_some() {
            parent_entity
        } else if layer.parent != 0 {
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

        // Prefer the direct parent embed as the render-layer owner for content
        // nested immediately under an embed. This keeps nested group fill content
        // attached to the inner composite layer even if flatten remapping widened
        // containing_embed_id to an outer ancestor.
        let resolved_embed_owner_id = if layer.containing_embed_id == 0 {
            0
        } else {
            pending
                .layers
                .iter()
                .find(|candidate| candidate.id == layer.parent)
                .filter(|parent_layer| {
                    matches!(parent_layer.spec, crate::scene::AmLayerSpec::EmbedScene)
                })
                .map(|parent_layer| parent_layer.id)
                .unwrap_or(layer.containing_embed_id)
        };
        let has_child_layers = pending
            .layers
            .iter()
            .any(|candidate| candidate.parent == layer.id);

        let entity = spawn_layer_entity(
            commands,
            meshes,
            unified_materials,
            color_materials,
            sdf_materials,
            layer,
            images,
            fonts,
            white_pixel,
            actual_parent,
            perspective_parent,
            pending.embed_contents_container,
            pending.inv_fit_scale,
            resolved_embed_owner_id,
            has_child_layers,
            &pending.spawned_entities,
            global_time,
        );

        bevy::log::debug!(
            "[Lifecycle] Spawned '{}' (id={}, parent={}, embed={}, z={:.6}, time={}..{}ms) -> Entity {:?} parented to {:?}",
            layer.label,
            layer.id,
            layer.parent,
            resolved_embed_owner_id,
            layer.transform.translation.z,
            layer.start_time,
            layer.end_time,
            entity,
            actual_parent
        );

        let trace_debug_transform = std::env::var_os("AM_DEBUG_TRANSFORM_TRACE").is_some();
        if trace_debug_transform
            && (layer.label.contains("空") || layer.label.contains("Image_1699715690143"))
        {
            bevy::log::info!(
                "[DEBUG_TRANSFORM] '{}' (id={}, parent={}): local_pos=({:.1},{:.1}), rot={:.1}°, scale=({:.2},{:.2}), has_parent={}",
                layer.label,
                layer.id,
                layer.parent,
                layer.transform.translation.x,
                layer.transform.translation.y,
                layer
                    .transform
                    .rotation
                    .to_euler(bevy::math::EulerRot::ZYX)
                    .0
                    .to_degrees(),
                layer.transform.scale.x,
                layer.transform.scale.y,
                layer.animated.has_parent
            );
        }

        pending.spawned_entities.insert(layer.id, entity);

        // Insert AmPathRepeat for layers with path-repeat effect
        if layer.animated.path_repeat.is_some() {
            // Find the previous layer in the pending.layers list (by original XML order)
            if idx > 0 {
                let prev_layer = &pending.layers[idx - 1];
                let source_entity_opt = pending.spawned_entities.get(&prev_layer.id).copied();
                let source_shape_type = match &prev_layer.spec {
                    crate::scene::AmLayerSpec::SpriteShape { .. } => ".rect".to_string(),
                    crate::scene::AmLayerSpec::SdfShape { .. } => "sdf".to_string(),
                    _ => String::new(),
                };
                // Clone source's animated data so path positions can be computed
                // even after the source entity is despawned
                let source_animated = prev_layer.animated.clone();
                commands
                    .entity(entity)
                    .insert(crate::animation::AmPathRepeat {
                        source_entity: source_entity_opt.unwrap_or(Entity::PLACEHOLDER),
                        copy_entities: Vec::new(),
                        source_shape_type,
                        source_layer_id: prev_layer.id,
                        source_animated,
                    });
            }
        }
    }
}

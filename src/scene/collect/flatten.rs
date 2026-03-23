use bevy::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    sync::{Mutex, OnceLock},
};

use super::super::collect_echo::generate_unique_id;
use super::super::components::{AmLayerSpec, PendingLayer};

fn trace_flatten_once(key: impl Into<String>, message: impl FnOnce() -> String) {
    if std::env::var_os("AM_FLATTEN_TRACE").is_none() {
        return;
    }

    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let key = key.into();

    let should_log = {
        let mut guard = seen.lock().expect("flatten trace mutex poisoned");
        guard.insert(key)
    };

    if should_log {
        bevy::log::warn!("{}", message());
    }
}

fn remap_flattened_child(
    child: &mut PendingLayer,
    id_mappings: &[(u64, u64)],
    layer_id: u64,
    is_embed: bool,
    embed_bevy_pos: Vec3,
    child_embed_id: u64,
) {
    let original_parent = child.parent;

    if original_parent == 0 {
        child.parent = layer_id;

        if is_embed {
            child.animated.embed_offset = Vec2::new(embed_bevy_pos.x, embed_bevy_pos.y);
            bevy::log::trace!(
                "[FlattenDebug] Setting embed_offset for '{}' (id={}): offset=({:.1},{:.1})",
                child.label,
                child.id,
                embed_bevy_pos.x,
                embed_bevy_pos.y
            );
        }
    } else {
        let new_parent_id = id_mappings
            .iter()
            .find(|(old, _new)| *old == original_parent)
            .map(|(_, new)| *new);

        if let Some(new_parent_id) = new_parent_id {
            child.parent = new_parent_id;
            child.animated.parent_layer_id = new_parent_id;
        } else {
            bevy::log::trace!(
                "[Flatten] Parent {} not found in mapping for '{}', keeping as-is",
                original_parent,
                child.label
            );
        }
    }

    if child.containing_embed_id != 0 {
        if original_parent == 0 && child.containing_embed_id == child_embed_id && is_embed {
            child.containing_embed_id = layer_id;
            bevy::log::trace!(
                "[Flatten] Remapped containing_embed_id for '{}': {} -> {} (direct child of embed)",
                child.label,
                child_embed_id,
                layer_id
            );
        } else {
            let new_embed_id = id_mappings
                .iter()
                .find(|(old, _new)| *old == child.containing_embed_id)
                .map(|(_, new)| *new);

            if let Some(new_embed_id) = new_embed_id {
                child.containing_embed_id = new_embed_id;
                bevy::log::trace!(
                    "[Flatten] Remapped containing_embed_id for '{}' via id_mappings",
                    child.label
                );
            }
        }
    }

    child.animated.layer_id = child.id;

    if let Some(ref mut info) = child.mask_info {
        for mask in info.masks.iter_mut() {
            let new_mask_id = id_mappings
                .iter()
                .find(|(old, _new)| *old == mask.mask_layer_id)
                .map(|(_, new)| *new);

            if let Some(new_mask_id) = new_mask_id {
                bevy::log::debug!(
                    "[Flatten] Remapped mask_layer_id for '{}': {} -> {}",
                    child.label,
                    mask.mask_layer_id,
                    new_mask_id
                );
                mask.mask_layer_id = new_mask_id;
            }

            let new_mask_parent = id_mappings
                .iter()
                .find(|(old, _new)| *old == mask.mask_parent_layer_id)
                .map(|(_, new)| *new);

            if let Some(new_parent_id) = new_mask_parent {
                mask.mask_parent_layer_id = new_parent_id;
            } else if mask.mask_parent_layer_id == 0 && is_embed {
                mask.mask_parent_layer_id = layer_id;
            }
        }
    }
}

pub(super) fn flatten_pending_layers(
    layers: Vec<PendingLayer>,
    nesting_depth: u32,
) -> Vec<PendingLayer> {
    flatten_pending_layers_inner(layers, 0, 0, nesting_depth, 0)
}

fn flatten_pending_layers_inner(
    layers: Vec<PendingLayer>,
    current_embed_id: u64,
    embed_depth: u32,
    base_nesting_depth: u32,
    mut instance_counter: u64,
) -> Vec<PendingLayer> {
    let mut result = Vec::new();

    for layer in layers {
        let layer_id = layer.id;
        let children = layer.children.clone();
        let is_embed = matches!(layer.spec, AmLayerSpec::EmbedScene);
        let embed_bevy_pos = layer.transform.translation;

        let mut layer_without_children = layer;
        layer_without_children.children = Vec::new();

        let should_decouple = embed_depth >= 1 && !is_embed;
        let assigned_embed_id = if should_decouple { current_embed_id } else { 0 };
        layer_without_children.containing_embed_id = assigned_embed_id;

        if should_decouple && assigned_embed_id != 0 {
            bevy::log::debug!(
                "[Flatten] Content '{}' (id={}) assigned to embed {} (depth={})",
                layer_without_children.label,
                layer_without_children.id,
                assigned_embed_id,
                embed_depth
            );
        }

        result.push(layer_without_children);

        if !children.is_empty() {
            instance_counter += 1;
            let current_instance = instance_counter;

            let (child_embed_id, child_depth) = if is_embed {
                (layer_id, embed_depth + 1)
            } else {
                (current_embed_id, embed_depth)
            };

            let flattened_children = flatten_pending_layers_inner(
                children,
                child_embed_id,
                child_depth,
                base_nesting_depth,
                instance_counter,
            );

            let id_mappings: Vec<(u64, u64)> = flattened_children
                .iter()
                .map(|child| {
                    let old_id = child.id;
                    let new_id = generate_unique_id(
                        current_instance
                            .wrapping_mul(1_000_000)
                            .wrapping_add(child.id),
                    );
                    (old_id, new_id)
                })
                .collect();

            let z_map: HashMap<u64, f32> = flattened_children
                .iter()
                .map(|c| (c.id, c.transform.translation.z))
                .collect();
            let embed_parent_ids: HashSet<u64> = flattened_children
                .iter()
                .filter(|c| matches!(c.spec, AmLayerSpec::EmbedScene))
                .map(|c| c.id)
                .collect();

            for (idx, mut child) in flattened_children.into_iter().enumerate() {
                let original_parent = child.parent;
                let old_z = child.transform.translation.z;
                child.id = id_mappings[idx].1;
                remap_flattened_child(
                    &mut child,
                    &id_mappings,
                    layer_id,
                    is_embed,
                    embed_bevy_pos,
                    child_embed_id,
                );

                let inherit_parent_z =
                    original_parent != 0 && !embed_parent_ids.contains(&original_parent);
                let parent_z = match (inherit_parent_z, z_map.get(&original_parent).copied()) {
                    (true, Some(z)) => z,
                    _ => 0.0,
                };
                child.transform.translation.z -= parent_z;

                #[expect(clippy::excessive_nesting)]
                // reason: keep the targeted flatten trace beside the guarded label filter
                if child.label.starts_with("spr_s_boneloop_0.png Copy")
                    || child.label.starts_with("Rectangle 1 Copy")
                {
                    trace_flatten_once(format!("{}:{}", child.id, child.label), || {
                        format!(
                            "[FLATTEN] label='{}' old_parent={} new_parent={} old_z={:.4} parent_z={:.4} new_z={:.4} containing_embed={} is_embed_ctx={}",
                            child.label,
                            original_parent,
                            child.parent,
                            old_z,
                            parent_z,
                            child.transform.translation.z,
                            child.containing_embed_id,
                            is_embed,
                        )
                    });
                }

                result.push(child);
            }
        }
    }

    let _ = base_nesting_depth;
    result
}

#[cfg(test)]
mod tests {
    use super::remap_flattened_child;
    use crate::animation::AmAnimated;
    use crate::scene::{AmBlendingMode, AmLayerSpec, AmMaskEntry, AmMaskInfo, PendingLayer};
    use bevy::prelude::*;

    fn make_pending_layer(
        id: u64,
        parent: u64,
        containing_embed_id: u64,
        label: &str,
    ) -> PendingLayer {
        PendingLayer {
            id,
            label: label.to_string(),
            parent,
            start_time: 0,
            end_time: 0,
            transform: Transform::default(),
            animated: AmAnimated::default(),
            spec: AmLayerSpec::Null,
            z_index: 0.0,
            children: Vec::new(),
            blending_mode: AmBlendingMode::Normal,
            mask_info: None,
            palette_params: None,
            embed_scene_size: None,
            containing_embed_id,
            from_deeply_nested_scene: false,
            echo_runtime: None,
            group_fill: None,
            embed_requires_composite: false,
            embed_dynamic_resolution: false,
            embed_inner_total_time: None,
            hidden: false,
        }
    }

    #[test]
    fn remap_preserves_nested_embed_owner_for_nested_children() {
        let current_embed_layer_id = 102_373_407;
        let nested_embed_old_id = 12_372_971;
        let nested_embed_new_id = 1_000_000_000_021;
        let mut child = make_pending_layer(
            12_368_973,
            nested_embed_old_id,
            nested_embed_old_id,
            "spr_s_boneloop_0.png Copy 5",
        );

        remap_flattened_child(
            &mut child,
            &[(nested_embed_old_id, nested_embed_new_id)],
            current_embed_layer_id,
            true,
            Vec3::ZERO,
            nested_embed_old_id,
        );

        assert_eq!(child.parent, nested_embed_new_id);
        assert_eq!(child.animated.parent_layer_id, nested_embed_new_id);
        assert_eq!(child.containing_embed_id, nested_embed_new_id);
    }

    #[test]
    fn remap_assigns_current_embed_owner_for_direct_children() {
        let current_embed_layer_id = 102_373_407;
        let mut child = make_pending_layer(
            12_372_970,
            0,
            current_embed_layer_id,
            "spr_s_boneloop_0.png Copy",
        );

        remap_flattened_child(
            &mut child,
            &[],
            current_embed_layer_id,
            true,
            Vec3::ZERO,
            current_embed_layer_id,
        );

        assert_eq!(child.parent, current_embed_layer_id);
        assert_eq!(child.containing_embed_id, current_embed_layer_id);
    }

    #[test]
    fn remap_promotes_flattened_root_mask_parent_to_current_embed() {
        let current_embed_layer_id = 102_373_489;
        let original_mask_layer_id = 12_373_002;
        let remapped_mask_layer_id = 1_000_000_000_062;
        let mut child = make_pending_layer(
            12_373_493,
            0,
            current_embed_layer_id,
            "spr_s_boneloop_0.png Copy",
        );
        child.mask_info = Some(AmMaskInfo {
            masks: vec![AmMaskEntry {
                mask_layer_id: original_mask_layer_id,
                mask_parent_layer_id: 0,
                ..Default::default()
            }],
        });

        remap_flattened_child(
            &mut child,
            &[(original_mask_layer_id, remapped_mask_layer_id)],
            current_embed_layer_id,
            true,
            Vec3::ZERO,
            current_embed_layer_id,
        );

        let masks = &child.mask_info.as_ref().expect("mask info").masks;
        assert_eq!(masks.len(), 1);
        assert_eq!(masks[0].mask_layer_id, remapped_mask_layer_id);
        assert_eq!(masks[0].mask_parent_layer_id, current_embed_layer_id);
    }
}

//! # collect_echo.rs
//!
//! # Echo/Repeat ID 重映射与变换辅助
//!
//! Helper functions for echo/repeat copies: ID remapping and spatial transforms.
//! Echo/Repeat 副本的 ID 重映射和空间变换辅助函数。

use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use super::components::*;

/// Global counter for generating unique IDs during flatten
static UNIQUE_ID_COUNTER: AtomicU64 = AtomicU64::new(1_000_000_000_000);

/// Generate a unique ID that won't collide with original IDs
pub(crate) fn generate_unique_id(_base_id: u64) -> u64 {
    UNIQUE_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Remap all IDs in an echo PendingLayer tree to unique IDs.
/// This prevents echo copies from colliding with the original entity IDs.
/// The root's `parent` is NOT remapped (it references an entity in the outer scope).
pub(crate) fn remap_echo_pl_ids(pl: &mut PendingLayer) {
    let mut id_map = HashMap::new();
    collect_ids_for_remap(pl, &mut id_map);
    apply_id_remap(pl, &id_map, true);
}

/// Apply accumulated echo-copy spatial transform to all children of a pending layer.
pub(crate) fn apply_echo_copy_transform(
    pl: &mut PendingLayer,
    acc_scale: f32,
    acc_offset: Vec2,
    acc_angle: f32,
) {
    if (acc_scale - 1.0).abs() > 0.001 || acc_offset.length() > 0.001 || acc_angle.abs() > 0.001 {
        let angle_rad = (-acc_angle).to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        for child in &mut pl.children {
            let x = child.transform.translation.x;
            let y = child.transform.translation.y;
            child.transform.translation.x = (x * cos_a - y * sin_a) * acc_scale + acc_offset.x;
            child.transform.translation.y = (x * sin_a + y * cos_a) * acc_scale + acc_offset.y;
            child.transform.scale.x *= acc_scale;
            child.transform.scale.y *= acc_scale;
            child.transform.rotation *= Quat::from_rotation_z(angle_rad);
        }
    }
}

fn collect_ids_for_remap(pl: &PendingLayer, id_map: &mut HashMap<u64, u64>) {
    id_map
        .entry(pl.id)
        .or_insert_with(|| generate_unique_id(pl.id));
    for child in &pl.children {
        collect_ids_for_remap(child, id_map);
    }
}

fn apply_id_remap(pl: &mut PendingLayer, id_map: &HashMap<u64, u64>, is_root: bool) {
    if let Some(&new_id) = id_map.get(&pl.id) {
        pl.id = new_id;
        pl.animated.layer_id = new_id;
    }
    if !is_root {
        if let Some(&new_parent) = id_map.get(&pl.parent) {
            pl.parent = new_parent;
        }
        if let Some(&new_parent) = id_map.get(&pl.animated.parent_layer_id) {
            pl.animated.parent_layer_id = new_parent;
        }
    }
    if let Some(&new_embed) = id_map.get(&pl.containing_embed_id) {
        pl.containing_embed_id = new_embed;
    }
    if let Some(ref mut mask_info) = pl.mask_info {
        for entry in &mut mask_info.masks {
            if let Some(&new_mask) = id_map.get(&entry.mask_layer_id) {
                entry.mask_layer_id = new_mask;
            }
            if let Some(&new_parent) = id_map.get(&entry.mask_parent_layer_id) {
                entry.mask_parent_layer_id = new_parent;
            }
        }
    }
    for child in &mut pl.children {
        apply_id_remap(child, id_map, false);
    }
}

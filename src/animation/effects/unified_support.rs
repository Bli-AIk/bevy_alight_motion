use bevy::prelude::*;
use std::{
    collections::HashSet,
    sync::{Mutex, OnceLock},
};

use crate::animation::components::AmAnimated;
use crate::animation::interpolation::interpolate_vec2;

/// Convert sRGB component to linear for shader (colors from AM are sRGB).
pub(super) fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub(super) fn trace_stretch_once(layer_id: u64, message: impl FnOnce() -> String) {
    if std::env::var_os("AM_STRETCH_TRACE").is_none() {
        return;
    }

    static SEEN: OnceLock<Mutex<HashSet<u64>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let should_log = {
        let mut guard = seen.lock().expect("stretch trace mutex poisoned");
        guard.insert(layer_id)
    };

    if should_log {
        bevy::log::warn!("{}", message());
    }
}

pub(super) fn trace_unified_once(key: impl Into<String>, message: impl FnOnce() -> String) {
    if std::env::var_os("AM_UNIFIED_TRACE").is_none() {
        return;
    }

    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let key = key.into();

    let should_log = {
        let mut guard = seen.lock().expect("unified trace mutex poisoned");
        guard.insert(key)
    };

    if should_log {
        bevy::log::warn!("{}", message());
    }
}

/// Compute accumulated ancestor visual scale by walking up the entity hierarchy.
/// Only accumulates scale from ancestors that have UnifiedEffectMarker,
/// because those entities bake their animated scale into mesh size (not Transform.scale).
/// Regular group/shape parents put scale into Transform.scale, which children
/// already inherit through Bevy's transform hierarchy.
pub(super) fn compute_ancestor_scale(
    entity: Entity,
    parent_query: &Query<(&AmAnimated, Option<&ChildOf>)>,
    effect_check: &Query<(), With<crate::masked_sprite::UnifiedEffectMarker>>,
    global_time: f32,
) -> [f32; 2] {
    let mut acc_scale = [1.0f32, 1.0f32];

    let parent_entity = match parent_query.get(entity) {
        Ok((_, Some(child_of))) => child_of.parent(),
        _ => return acc_scale,
    };

    let mut current = parent_entity;
    while let Ok((animated, child_of_ref)) = parent_query.get(current) {
        if effect_check.contains(current) {
            let local_time = animated.calc_local_time(global_time);
            let layer_time = animated.calc_layer_time(local_time);
            let s = interpolate_vec2(&animated.scale, layer_time).unwrap_or([1.0, 1.0]);
            acc_scale[0] *= s[0];
            acc_scale[1] *= s[1];
        }

        if let Some(child_of) = child_of_ref {
            current = child_of.parent();
        } else {
            break;
        }
    }

    acc_scale
}

pub(super) fn insert_quad_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    entity: Entity,
    bounds: [f32; 4],
    uv_rect: [f32; 4],
) {
    let [min_x, max_x, min_y, max_y] = bounds;
    let [uv_left, uv_right, uv_top, uv_bottom] = uv_rect;

    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::RENDER_WORLD | bevy::asset::RenderAssetUsages::MAIN_WORLD,
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [min_x, min_y, 0.0],
            [max_x, min_y, 0.0],
            [max_x, max_y, 0.0],
            [min_x, max_y, 0.0],
        ],
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4]);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![
            [uv_left, uv_bottom],
            [uv_right, uv_bottom],
            [uv_right, uv_top],
            [uv_left, uv_top],
        ],
    );
    mesh.insert_indices(bevy::mesh::Indices::U32(vec![0u32, 1, 2, 0, 2, 3]));

    let mesh_handle = meshes.add(mesh);
    commands
        .entity(entity)
        .insert(bevy::mesh::Mesh2d(mesh_handle));
}

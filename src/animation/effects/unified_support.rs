//! Contains shared helper code for the unified-effect animation path.
//! It provides color conversion, one-shot tracing utilities, and a few reusable
//! mesh update helpers that multiple unified-effect modules rely on.
//!
//! 存放统一特效动画路径共用的辅助逻辑。它提供颜色空间转换、一次性追踪日志，
//! 以及多个统一特效模块都会复用的网格更新工具函数。

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

pub(super) fn trace_parenthelper_unified_state(
    marker: &crate::scene::AmLayerMarker,
    material: &crate::masked_sprite::UnifiedEffectMaterial,
    transform: &Transform,
    global_transform: &GlobalTransform,
    visibility: Option<&Visibility>,
    render_layers: Option<&bevy::camera::visibility::RenderLayers>,
    parent: Option<Entity>,
) {
    let global_pos = global_transform.translation();
    let has_texture = material.texture.is_some();
    trace_unified_once(format!("{}:{}", marker.id, marker.label), || {
        format!(
            "[UNIFIED:parenthelper] layer_id={} label='{}' parent={:?} has_texture={} color=({:.3},{:.3},{:.3},{:.3}) vis={:?} layers={:?} local=({:.2},{:.2},{:.4}) global=({:.2},{:.2},{:.4}) size=({:.2},{:.2},{:.2},{:.2}) effect_flags=({:.1},{:.1},{:.1},{:.1})",
            marker.id,
            marker.label,
            parent,
            has_texture,
            material.uniform_data.color.x,
            material.uniform_data.color.y,
            material.uniform_data.color.z,
            material.uniform_data.color.w,
            visibility,
            render_layers,
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
            global_pos.x,
            global_pos.y,
            global_pos.z,
            material.uniform_data.original_size.x,
            material.uniform_data.original_size.y,
            material.uniform_data.original_size.z,
            material.uniform_data.original_size.w,
            material.uniform_data.effect_flags.x,
            material.uniform_data.effect_flags.y,
            material.uniform_data.effect_flags.z,
            material.uniform_data.effect_flags.w,
        )
    });
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

pub(super) fn update_quad_mesh(
    meshes: &mut Assets<Mesh>,
    mesh_handle: &bevy::mesh::Mesh2d,
    bounds: [f32; 4],
    uv_rect: [f32; 4],
) {
    let [min_x, max_x, min_y, max_y] = bounds;
    let [uv_left, uv_right, uv_top, uv_bottom] = uv_rect;

    let Some(mesh) = meshes.get_mut(&mesh_handle.0) else {
        return;
    };

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
}

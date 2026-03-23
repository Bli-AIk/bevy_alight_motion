//! # visual_helpers.rs
//!
//! # 视觉辅助函数模块
//!
//! Helper functions for visual component creation (mask params, stretch mesh).
//! 视觉组件创建的辅助函数（遮罩参数、拉伸网格）。

use bevy::asset::Assets;
use bevy::prelude::*;
use std::{
    collections::HashSet,
    sync::{Mutex, OnceLock},
};

use crate::scene::AmMaskInfo;

pub(super) fn trace_visual_path_once(key: impl Into<String>, message: impl FnOnce() -> String) {
    if std::env::var_os("AM_VISUAL_PATH_TRACE").is_none() {
        return;
    }

    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let key = key.into();

    let should_log = {
        let mut guard = seen.lock().expect("visual path trace mutex poisoned");
        guard.insert(key)
    };

    if should_log {
        bevy::log::warn!("{}", message());
    }
}

/// Pre-calculate initial mask params from mask_info for first-frame correctness.
/// Returns (effect_flags_x, mask_params, mask2_flags_x, mask2_params).
pub(super) fn compute_initial_mask_params(
    mask_info: &Option<AmMaskInfo>,
    fit_scale: f32,
    global_time_ms: u64,
) -> (f32, Vec4, f32, Vec4) {
    let default_mask = (
        0.0,
        Vec4::new(0.0, 0.0, 10000.0, 10000.0),
        0.0,
        Vec4::new(0.0, 0.0, 10000.0, 10000.0),
    );
    let Some(mask_info) = mask_info else {
        return default_mask;
    };
    let active_masks = mask_info.get_active_masks(global_time_ms);
    if active_masks.is_empty() {
        bevy::log::trace!(
            "[MaterialInit] No active mask at time {}, mask_info has {} masks",
            global_time_ms,
            mask_info.masks.len()
        );
        return default_mask;
    }

    let mask1 = active_masks[0];
    let base_type1 = if mask1.is_circle { 2.0 } else { 1.0 };
    let mask1_type = if mask1.is_exclude {
        base_type1 + 2.0
    } else {
        base_type1
    };
    let mask1_params = Vec4::new(
        mask1.center.x * fit_scale,
        mask1.center.y * fit_scale,
        mask1.half_size.x.abs() * fit_scale,
        mask1.half_size.y.abs() * fit_scale,
    );

    let (mask2_type, mask2_params) = if active_masks.len() >= 2 {
        let mask2 = active_masks[1];
        let base_type2 = if mask2.is_circle { 2.0 } else { 1.0 };
        let m2_type = if mask2.is_exclude {
            base_type2 + 2.0
        } else {
            base_type2
        };
        let m2_params = Vec4::new(
            mask2.center.x * fit_scale,
            mask2.center.y * fit_scale,
            mask2.half_size.x.abs() * fit_scale,
            mask2.half_size.y.abs() * fit_scale,
        );
        bevy::log::trace!(
            "[MaterialInit] DUAL Mask init: mask1_type={}, mask2_type={}, fit_scale={:.4}",
            mask1_type,
            m2_type,
            fit_scale
        );
        (m2_type, m2_params)
    } else {
        bevy::log::trace!(
            "[MaterialInit] Mask init: effect_flags.x={}, center=({:.1},{:.1}), half_size=({:.1},{:.1}), fit_scale={:.4}",
            mask1_type,
            mask1.center.x * fit_scale,
            mask1.center.y * fit_scale,
            mask1.half_size.x.abs() * fit_scale,
            mask1.half_size.y.abs() * fit_scale,
            fit_scale
        );
        (0.0, Vec4::new(0.0, 0.0, 10000.0, 10000.0))
    };

    (mask1_type, mask1_params, mask2_type, mask2_params)
}

/// Create a mesh from stretch bounds (min_x, max_x, min_y, max_y).
pub(super) fn create_stretch_bounds_mesh(
    meshes: &mut Assets<Mesh>,
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
) -> Handle<Mesh> {
    let vertices = vec![
        [min_x, min_y, 0.0],
        [max_x, min_y, 0.0],
        [max_x, max_y, 0.0],
        [min_x, max_y, 0.0],
    ];
    let normals = vec![
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
    ];
    let uvs = vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
    let indices = vec![0u32, 1, 2, 0, 2, 3];

    let mut new_mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::RENDER_WORLD | bevy::asset::RenderAssetUsages::MAIN_WORLD,
    );
    new_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    new_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    new_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    new_mesh.insert_indices(bevy::mesh::Indices::U32(indices));
    meshes.add(new_mesh)
}

/// Extract fill color from AmFillColor.
///
/// - `no_fill`: When true (fillType="none"), always returns transparent regardless of fill_color.
/// - When false and `fill_color` is None, returns white as default.
/// - Otherwise extracts color from fill_color value or keyframes.
pub(crate) fn extract_fill_color(
    fill_color: &Option<crate::schema::AmFillColor>,
    no_fill: bool,
) -> Color {
    if no_fill {
        return Color::srgba(0.0, 0.0, 0.0, 0.0);
    }

    if let Some(fc) = fill_color {
        if !fc.value.is_empty() {
            if let Ok(c) = crate::schema::parse_color(&fc.value) {
                return Color::srgba(c[0], c[1], c[2], c[3]);
            }
        } else if !fc.keyframes.is_empty() {
            let mut sorted: Vec<_> = fc.keyframes.iter().collect();
            sorted.sort_by(|a, b| {
                a.time
                    .partial_cmp(&b.time)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            if let Ok(c) = crate::schema::parse_color(&sorted[0].value) {
                return Color::srgba(c[0], c[1], c[2], c[3]);
            }
        }
    }

    Color::WHITE
}

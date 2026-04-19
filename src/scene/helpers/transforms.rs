//! Contains transform and coordinate helpers used during scene collection.
//! It converts authored AM coordinates into Bevy space and extracts sensible
//! initial location, rotation, scale, opacity, and pivot values from animated
//! schema fields.
//!
//! 存放场景收集阶段使用的变换与坐标辅助函数。它负责把作者侧的 AM 坐标转换成
//! Bevy 空间，并从带动画的 schema 字段里提取合理的初始位置、旋转、缩放、透明度和
//! pivot 数值。

use bevy::prelude::*;

use super::super::components::AmSceneConfig;
use crate::schema::{AmAnimatedFloat, AmAnimatedVec2, AmAnimatedVec3};

pub fn am_to_bevy_coords(x: f32, y: f32, config: &AmSceneConfig) -> (f32, f32) {
    let bx = x - config.canvas_width / 2.0;
    let by = if config.flip_y {
        config.canvas_height / 2.0 - y
    } else {
        y - config.canvas_height / 2.0
    };
    (bx, by)
}

pub(crate) fn get_initial_location(
    prop: &AmAnimatedVec3,
    config: &AmSceneConfig,
    has_parent: bool,
) -> (f32, f32) {
    let (x, y) = if let Some(val) = &prop.value {
        (val[0], val[1])
    } else if !prop.keyframes.is_empty() {
        let mut sorted: Vec<_> = prop.keyframes.iter().collect();
        sorted.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        crate::schema::parse_vec3(&sorted[0].value)
            .map(|v| (v[0], v[1]))
            .unwrap_or((0.0, 0.0))
    } else if has_parent {
        (0.0, 0.0)
    } else {
        (config.canvas_width / 2.0, config.canvas_height / 2.0)
    };

    if has_parent {
        (x, -y)
    } else {
        am_to_bevy_coords(x, y, config)
    }
}

pub(crate) fn get_initial_rotation(prop: &AmAnimatedFloat) -> f32 {
    if let Some(val) = prop.value {
        -val
    } else if !prop.keyframes.is_empty() {
        let mut sorted: Vec<_> = prop.keyframes.iter().collect();
        sorted.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        -sorted[0].value.parse().unwrap_or(0.0)
    } else {
        0.0
    }
}

pub(crate) fn get_initial_scale(prop: &AmAnimatedVec2) -> (f32, f32) {
    if let Some(val) = &prop.value {
        (val[0], val[1])
    } else if !prop.keyframes.is_empty() {
        let mut sorted: Vec<_> = prop.keyframes.iter().collect();
        sorted.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        crate::schema::parse_vec2(&sorted[0].value)
            .unwrap_or([1.0, 1.0])
            .into()
    } else {
        (1.0, 1.0)
    }
}

pub(crate) fn get_initial_pivot(prop: &AmAnimatedVec2) -> (f32, f32) {
    if let Some(val) = &prop.value {
        (val[0], val[1])
    } else if !prop.keyframes.is_empty() {
        let mut sorted: Vec<_> = prop.keyframes.iter().collect();
        sorted.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        crate::schema::parse_vec2(&sorted[0].value)
            .unwrap_or([0.0, 0.0])
            .into()
    } else {
        (0.0, 0.0)
    }
}

pub(crate) fn calculate_embed_position_compensation(
    pivot: (f32, f32),
    scale: (f32, f32),
    rotation_deg: f32,
    has_parent: bool,
) -> (f32, f32) {
    let pivot_x = pivot.0;
    let pivot_y = if has_parent { pivot.1 } else { -pivot.1 };

    let scaled_offset_x = -pivot_x * scale.0;
    let scaled_offset_y = -pivot_y * scale.1;

    // For non-parented embeds, pivot_y is negated to match Bevy's Y-up coords.
    // The rotation must also be in Bevy space (rotation_deg is already Bevy-convention,
    // i.e. -AM_value).  Parented embeds keep AM's Y direction for the pivot, so the
    // rotation must revert to the original AM angle (-rotation_deg).
    let rotation_rad = if has_parent {
        (-rotation_deg).to_radians()
    } else {
        rotation_deg.to_radians()
    };
    let rotated_offset_x =
        scaled_offset_x * rotation_rad.cos() - scaled_offset_y * rotation_rad.sin();
    let rotated_offset_y =
        scaled_offset_x * rotation_rad.sin() + scaled_offset_y * rotation_rad.cos();

    let comp_x = rotated_offset_x + pivot_x;
    let comp_y = rotated_offset_y + pivot_y;

    (comp_x, comp_y)
}

#[allow(dead_code)]
pub(crate) fn calculate_pivot_compensation(
    pivot: (f32, f32),
    scale: (f32, f32),
    _rotation_deg: f32,
    has_parent: bool,
) -> (f32, f32) {
    let pivot_x = pivot.0;
    let pivot_y = pivot.1;

    let comp_x = pivot_x * (1.0 - scale.0);
    let comp_y = if has_parent {
        pivot_y * (1.0 - scale.1)
    } else {
        -pivot_y * (1.0 - scale.1)
    };

    (comp_x, comp_y)
}

pub(crate) fn get_initial_opacity(prop: &AmAnimatedFloat) -> f32 {
    if let Some(val) = prop.value {
        val
    } else if !prop.keyframes.is_empty() {
        let mut sorted: Vec<_> = prop.keyframes.iter().collect();
        sorted.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted[0].value.parse().unwrap_or(1.0)
    } else {
        1.0
    }
}

pub(crate) fn pivot_to_anchor_and_offset(
    pivot_x: f32,
    pivot_y: f32,
    width: f32,
    height: f32,
) -> (bevy::sprite::Anchor, f32, f32) {
    if pivot_x == 0.0 && pivot_y == 0.0 {
        return (bevy::sprite::Anchor::CENTER, 0.0, 0.0);
    }

    let anchor_x = if width > 0.0 { pivot_x / width } else { 0.0 };
    let anchor_y = if height > 0.0 { -pivot_y / height } else { 0.0 };

    let comp_x = anchor_x * width;
    let comp_y = anchor_y * height;

    (
        bevy::sprite::Anchor(Vec2::new(anchor_x, anchor_y)),
        comp_x,
        comp_y,
    )
}

pub(crate) fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

pub(crate) fn get_scale_at_normalized_time(
    prop: &crate::schema::AmAnimatedVec2,
    t: f32,
) -> (f32, f32) {
    if let Some(val) = &prop.value {
        return (val[0], val[1]);
    }

    if prop.keyframes.is_empty() {
        return (1.0, 1.0);
    }

    let mut sorted: Vec<_> = prop.keyframes.iter().collect();
    sorted.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if t <= sorted[0].time {
        return crate::schema::parse_vec2(&sorted[0].value)
            .map(|v| (v[0], v[1]))
            .unwrap_or((1.0, 1.0));
    }

    let last = sorted.last().unwrap();
    if t >= last.time {
        return crate::schema::parse_vec2(&last.value)
            .map(|v| (v[0], v[1]))
            .unwrap_or((1.0, 1.0));
    }

    for i in 0..sorted.len() - 1 {
        let kf_prev = sorted[i];
        let kf_next = sorted[i + 1];

        if t >= kf_prev.time && t <= kf_next.time {
            let v_prev = crate::schema::parse_vec2(&kf_prev.value).unwrap_or([1.0, 1.0]);
            let v_next = crate::schema::parse_vec2(&kf_next.value).unwrap_or([1.0, 1.0]);

            let span = kf_next.time - kf_prev.time;
            let local_t = if span > 0.0 {
                (t - kf_prev.time) / span
            } else {
                0.0
            };

            return (
                v_prev[0] + (v_next[0] - v_prev[0]) * local_t,
                v_prev[1] + (v_next[1] - v_prev[1]) * local_t,
            );
        }
    }

    (1.0, 1.0)
}

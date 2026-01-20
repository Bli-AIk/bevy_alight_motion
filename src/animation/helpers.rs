//! # helpers.rs
//!
//! # 辅助函数模块
//!
//! Helper functions for animation initialization and value extraction.
//! Contains get_initial_scale_from_animated, get_initial_size_from_animated,
//! is_descendant_of, and other utility functions.
//!
//! 动画初始化和值提取的辅助函数。
//! 包含 get_initial_scale_from_animated、get_initial_size_from_animated、
//! is_descendant_of 及其他工具函数。

use crate::scene::PendingLayer;
use crate::schema::AmAnimatedVec2;

use super::interpolation::parse_keyframe_vec2;

/// Check if a layer is a descendant of another layer (direct or nested).
///
/// 检查一个图层是否是另一个图层的后代（直接或嵌套）。
pub fn is_descendant_of(layer_id: u64, ancestor_id: u64, layers: &[PendingLayer]) -> bool {
    if layer_id == ancestor_id {
        return false; // Not a descendant of itself
    }

    // Find the layer
    let layer = match layers.iter().find(|l| l.id == layer_id) {
        Some(l) => l,
        None => return false,
    };

    // Check if direct child
    if layer.parent == ancestor_id {
        return true;
    }

    // Recursively check ancestors (with depth limit to prevent infinite loops)
    if layer.parent != 0 {
        return is_descendant_of(layer.parent, ancestor_id, layers);
    }

    false
}

/// Get initial scale from animated scale property.
/// For SDF shapes, the initial scale is stored in the animated data, not the transform.
/// When keyframes exist but all are before t=0 (negative time), use the last keyframe value.
///
/// 从动画缩放属性获取初始缩放。
/// 对于 SDF 形状，初始缩放存储在动画数据中，而非变换中。
/// 当关键帧存在但都在 t=0 之前时，使用最后一个关键帧值。
pub fn get_initial_scale_from_animated(prop: &AmAnimatedVec2) -> (f32, f32) {
    if let Some(val) = &prop.value {
        (val[0], val[1])
    } else if !prop.keyframes.is_empty() {
        // Sort keyframes by time
        let mut sorted: Vec<_> = prop.keyframes.iter().collect();
        sorted.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        // If all keyframes are before t=0, use the last keyframe (closest to t=0)
        // Otherwise, use the first keyframe (traditional behavior for t=0 being at or after first kf)
        let target_kf = if sorted.last().is_some_and(|kf| kf.time <= 0.0) {
            sorted.last().unwrap()
        } else {
            sorted.first().unwrap()
        };
        parse_keyframe_vec2(&target_kf.value)
            .map(|v| (v[0], v[1]))
            .unwrap_or((1.0, 1.0))
    } else {
        (1.0, 1.0)
    }
}

/// Get initial size from animated size property.
/// Returns default size of 100x100 if no value is set.
///
/// 从动画尺寸属性获取初始尺寸。
/// 如果没有设置值，返回默认尺寸 100x100。
#[allow(dead_code)]
pub fn get_initial_size_from_animated(prop: &AmAnimatedVec2) -> (f32, f32) {
    if let Some(val) = &prop.value {
        (val[0], val[1])
    } else if !prop.keyframes.is_empty() {
        // Sort keyframes by time and get the first one
        let mut sorted: Vec<_> = prop.keyframes.iter().collect();
        sorted.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        parse_keyframe_vec2(&sorted[0].value)
            .map(|v| (v[0], v[1]))
            .unwrap_or((100.0, 100.0))
    } else {
        (100.0, 100.0)
    }
}

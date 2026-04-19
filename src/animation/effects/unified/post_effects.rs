//! Handles post-style unified material updates such as replace color,
//! threshold, grid, and pixelate.
//!
//! 负责统一材质路径里偏后处理风格的 uniform 更新，例如颜色替换、
//! 阈值、网格和像素化。

use bevy::prelude::*;

use super::DebugEnvCache;
use crate::animation::components::AmAnimated;
use crate::animation::interpolation::{interpolate_color, interpolate_float, interpolate_vec2};

pub(super) fn update_replace_color(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    layer_time: f32,
    has_replace_color: bool,
    env_cache: &DebugEnvCache,
) {
    let trace_layer = env_cache.trace_effect(animated.layer_id);
    if env_cache.disable_replace_color(animated.layer_id) {
        material.uniform_data.replace_color_flags.x = 0.0;
        material.uniform_data.replace_color_flags.y = 0.0;
        material.uniform_data.replace_old_color = Vec4::ZERO;
        material.uniform_data.replace_new_color = Vec4::ZERO;
        material.uniform_data.replace_color_params = Vec4::ZERO;
        bevy::log::warn!(
            "[UnifiedTrace] layer={} replace-color disabled by AM_DISABLE_REPLACE_COLOR_IDS",
            animated.layer_id
        );
        return;
    }
    bevy::log::debug!(
        "[ReplaceColor Check] layer={} has_replace={} old_color={:?}",
        animated.layer_id,
        has_replace_color,
        animated.replace_old_color
    );
    if trace_layer {
        bevy::log::warn!(
            "[UnifiedTrace] layer={} replace-check has_replace={} old={:?} raw_new={:?}",
            animated.layer_id,
            has_replace_color,
            animated.replace_old_color,
            animated.replace_new_color.value
        );
    }
    if has_replace_color {
        let new_color = interpolate_color(&animated.replace_new_color, layer_time)
            .unwrap_or(animated.replace_old_color);
        let threshold = interpolate_float(&animated.replace_threshold, layer_time).unwrap_or(0.25);
        let feather = interpolate_float(&animated.replace_feather, layer_time).unwrap_or(0.25);
        let alpha = interpolate_float(&animated.replace_alpha, layer_time).unwrap_or(1.0);

        bevy::log::debug!(
            "[ReplaceColor Apply] layer={} old={:?} new={:?} threshold={:.3} feather={:.3} alpha={:.3}",
            animated.layer_id,
            animated.replace_old_color,
            new_color,
            threshold,
            feather,
            alpha
        );

        material.set_replace_color(
            animated.replace_old_color,
            new_color,
            threshold,
            feather,
            alpha,
            animated.replace_lock_luminance,
        );
        if trace_layer {
            bevy::log::warn!(
                "[UnifiedTrace] layer={} replace-apply new={:?} params={:?} flags={:?}",
                animated.layer_id,
                new_color,
                material.uniform_data.replace_color_params,
                material.uniform_data.replace_color_flags
            );
        }
    } else if trace_layer {
        bevy::log::warn!("[UnifiedTrace] layer={} replace skipped", animated.layer_id);
    }
}

pub(super) fn update_threshold(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    layer_time: f32,
    env_cache: &DebugEnvCache,
) {
    let trace_layer = env_cache.trace_effect(animated.layer_id);
    if env_cache.disable_threshold(animated.layer_id) {
        material.set_threshold(false, 0.5, 0.0, false, 0);
        bevy::log::warn!(
            "[UnifiedTrace] layer={} threshold disabled by AM_DISABLE_THRESHOLD_IDS",
            animated.layer_id
        );
        return;
    }
    let has_threshold =
        animated.threshold_value.value.is_some() || !animated.threshold_value.keyframes.is_empty();
    if has_threshold {
        let threshold = interpolate_float(&animated.threshold_value, layer_time).unwrap_or(0.5);
        let feather = interpolate_float(&animated.threshold_feather, layer_time).unwrap_or(0.0);
        material.set_threshold(
            true,
            threshold,
            feather,
            animated.threshold_invert,
            animated.threshold_blend_mode,
        );
        if trace_layer {
            bevy::log::warn!(
                "[UnifiedTrace] layer={} threshold enabled params={:?} flags={:?}",
                animated.layer_id,
                material.uniform_data.threshold_params,
                material.uniform_data.replace_color_flags
            );
        }
    } else if trace_layer {
        bevy::log::warn!(
            "[UnifiedTrace] layer={} threshold skipped",
            animated.layer_id
        );
    }
}

pub(super) fn update_grid(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    layer_time: f32,
) {
    let has_grid =
        animated.grid_spacing.value.is_some() || !animated.grid_spacing.keyframes.is_empty();
    if has_grid {
        let position = interpolate_vec2(&animated.grid_position, layer_time).unwrap_or([0.0, 0.0]);
        let spacing = interpolate_float(&animated.grid_spacing, layer_time).unwrap_or(0.1);
        let width = interpolate_float(&animated.grid_width, layer_time).unwrap_or(0.02);
        let smoothing = interpolate_float(&animated.grid_smoothing, layer_time).unwrap_or(0.0);
        let color = interpolate_color(&animated.grid_color, layer_time)
            .unwrap_or(Vec4::new(1.0, 1.0, 1.0, 1.0));

        material.set_grid(
            true,
            animated.grid_punchout,
            animated.grid_screen_space,
            position[0],
            position[1],
            spacing,
            width,
            smoothing,
            color,
        );
    }
}

pub(super) fn update_pixelate(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    layer_time: f32,
    global_transform: &GlobalTransform,
    root_scale: f32,
    has_pixelate: bool,
    disabled_effects: Option<&crate::plugin::DisabledEffects>,
    env_cache: &DebugEnvCache,
) {
    let trace_layer = env_cache.trace_effect(animated.layer_id);
    let globally_disabled = disabled_effects.is_some_and(|de| de.contains("pixelate"));
    if env_cache.disable_pixelate(animated.layer_id) || globally_disabled {
        material.set_pixelate(false, false, 1.0, 1.0, 1.0, 0.0, 0.0, 0.5, 1.0);
        material.uniform_data.pixelate_flags = Vec4::ZERO;
        material.uniform_data.pixelate_params1 = Vec4::ZERO;
        material.uniform_data.pixelate_params2 = Vec4::ZERO;
        if trace_layer {
            bevy::log::warn!(
                "[UnifiedTrace] layer={} pixelate disabled by blacklist",
                animated.layer_id
            );
        }
        return;
    }
    if has_pixelate {
        let size = interpolate_float(&animated.pixelate_size, layer_time).unwrap_or(1.0);
        let stretch =
            interpolate_vec2(&animated.pixelate_stretch, layer_time).unwrap_or([1.0, 1.0]);
        let angle = interpolate_float(&animated.pixelate_angle, layer_time).unwrap_or(0.0);
        let vignette = interpolate_float(&animated.pixelate_vignette, layer_time).unwrap_or(0.0);
        let threshold = interpolate_float(&animated.pixelate_threshold, layer_time).unwrap_or(0.5);
        let saturation =
            interpolate_float(&animated.pixelate_saturation, layer_time).unwrap_or(1.0);

        bevy::log::debug!(
            "[Pixelate] layer={} time={:.2} size={:.1} stretch=({:.2},{:.2}) angle={:.1}",
            animated.layer_id,
            layer_time,
            size,
            stretch[0],
            stretch[1],
            angle
        );

        material.set_pixelate(
            true,
            animated.pixelate_screen_space,
            size,
            stretch[0],
            stretch[1],
            angle,
            vignette,
            threshold,
            saturation,
        );

        let origin = global_transform.translation();
        let local_x_world = global_transform.transform_point(Vec3::X) - origin;
        let local_y_world = global_transform.transform_point(Vec3::Y) - origin;
        let scene_scale_x = local_x_world.length() / root_scale;
        let scene_scale_y = local_y_world.length() / root_scale;
        material.uniform_data.pixelate_flags.z = scene_scale_x;
        material.uniform_data.pixelate_flags.w = scene_scale_y;

        let local_x_world = global_transform.transform_point(Vec3::X) - origin;
        let scene_rotation = local_x_world.y.atan2(local_x_world.x);
        material.uniform_data.pixelate_params2.w = scene_rotation;
        if trace_layer {
            bevy::log::warn!(
                "[UnifiedTrace] layer={} pixelate enabled flags={:?} params1={:?} params2={:?}",
                animated.layer_id,
                material.uniform_data.pixelate_flags,
                material.uniform_data.pixelate_params1,
                material.uniform_data.pixelate_params2
            );
        }
    } else if trace_layer {
        bevy::log::warn!(
            "[UnifiedTrace] layer={} pixelate skipped",
            animated.layer_id
        );
    }
}

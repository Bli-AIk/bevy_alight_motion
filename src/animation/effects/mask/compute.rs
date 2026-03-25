//! Computes the geometric parameters for animated masks.
//! Given a mask entry and the currently spawned mask layer, it resolves center,
//! half-size, rotation, blend, and stretch data so the shader-side masking path
//! can mirror the source project faithfully.
//!
//! 负责计算动画遮罩的几何参数。它会根据 mask entry 和当前已生成的遮罩图层，
//! 求出中心、半尺寸、旋转、混合和拉伸信息，让 shader 端的遮罩路径能够尽量忠实地
//! 还原源项目的表现。

use bevy::prelude::*;

use crate::animation::components::AmAnimated;
use crate::animation::interpolation::{interpolate_float, interpolate_vec2};

use super::trace::trace_mask_once;

pub(super) struct MaskResult {
    pub(super) center: Vec2,
    pub(super) half_size: Vec2,
    pub(super) rotation: f32,
    pub(super) blend: Vec3,
    pub(super) sign_code: f32,
    pub(super) stretch1: Vec4,
    pub(super) stretch2: Vec4,
    pub(super) stretch_info: Vec4,
}

pub(super) fn compute_mask_params(
    mask: &crate::scene::AmMaskEntry,
    pending: &crate::scene::AmPendingLayers,
    mask_layer_query: &Query<(&GlobalTransform, &AmAnimated, &crate::scene::AmLayerSpec)>,
    playback_time: f32,
    fit_scale: f32,
) -> MaskResult {
    let fallback = MaskResult {
        center: mask.center * fit_scale,
        half_size: Vec2::new(mask.half_size.x.abs(), mask.half_size.y.abs()) * fit_scale,
        rotation: mask.rotation,
        blend: Vec3::new(1.0, 1.0, 0.0),
        sign_code: 0.0,
        stretch1: Vec4::ZERO,
        stretch2: Vec4::ZERO,
        stretch_info: Vec4::ZERO,
    };

    let Some(&mask_entity) = pending.spawned_entities.get(&mask.mask_layer_id) else {
        return fallback;
    };
    let Ok((mask_global_transform, animated, spec)) = mask_layer_query.get(mask_entity) else {
        return fallback;
    };

    let (base_width, base_height, pivot_x, pivot_y, fill_alpha, initial_sw, stroke_dir) = match spec
    {
        crate::scene::AmLayerSpec::SdfShape {
            width,
            height,
            pivot_x,
            pivot_y,
            fill_color,
            no_fill,
            stroke_width,
            stroke_direction,
            ..
        } => {
            let fa = if *no_fill {
                0.0
            } else if let Some(fc) = fill_color {
                if fc.value.len() >= 3 && fc.value.starts_with('#') {
                    let alpha_hex = &fc.value[1..3];
                    u8::from_str_radix(alpha_hex, 16).unwrap_or(255) as f32 / 255.0
                } else {
                    1.0
                }
            } else {
                1.0
            };
            (
                *width,
                *height,
                *pivot_x,
                *pivot_y,
                fa,
                *stroke_width,
                stroke_direction.as_str(),
            )
        }
        crate::scene::AmLayerSpec::SpriteShape { width, height, .. } => {
            (*width, *height, 0.0, 0.0, 1.0, 0.0, "centered")
        }
        _ => (
            mask.half_size.x * 2.0 / mask.scale.x,
            mask.half_size.y * 2.0 / mask.scale.y,
            0.0,
            0.0,
            1.0,
            0.0,
            "centered",
        ),
    };

    let local_time = animated.calc_local_time(playback_time);
    let layer_time = animated.calc_layer_time(local_time);

    let mask_opacity = interpolate_float(&animated.opacity, layer_time).unwrap_or(1.0);
    let current_sw = interpolate_float(&animated.stroke_width, layer_time).unwrap_or(initial_sw);

    let rotation_deg = interpolate_float(&animated.rotation, layer_time).unwrap_or(0.0);
    let rotation_rad = (-rotation_deg).to_radians();

    let [scale_x, scale_y] = interpolate_vec2(&animated.scale, layer_time).unwrap_or([1.0, 1.0]);

    let mask_parent_scale = if mask.mask_parent_layer_id != 0 {
        pending
            .spawned_entities
            .get(&mask.mask_parent_layer_id)
            .and_then(|&parent_entity| mask_layer_query.get(parent_entity).ok())
            .map(|(_, parent_animated, _)| {
                let parent_local_time = parent_animated.calc_local_time(playback_time);
                let parent_layer_time = parent_animated.calc_layer_time(parent_local_time);
                let [psx, psy] = interpolate_vec2(&parent_animated.scale, parent_layer_time)
                    .unwrap_or([1.0, 1.0]);
                Vec2::new(psx, psy)
            })
            .unwrap_or(Vec2::ONE)
    } else {
        Vec2::ONE
    };

    let (mask_global_scale, _, mask_translation) =
        mask_global_transform.to_scale_rotation_translation();
    let mask_sign_code = (if mask_global_scale.x < 0.0 { 1.0 } else { 0.0 })
        + (if mask_global_scale.y < 0.0 { 2.0 } else { 0.0 });

    let (center_x, center_y, trace_mask_pos, trace_parent_pos, trace_corrected_pos) =
        if mask.mask_parent_layer_id != 0 {
            let mask_pos = mask_global_transform.translation().truncate();
            let parent_pos = pending
                .spawned_entities
                .get(&mask.mask_parent_layer_id)
                .and_then(|&parent_entity| mask_layer_query.get(parent_entity).ok())
                .map(|(parent_gt, _, _)| parent_gt.translation().truncate())
                .unwrap_or(mask_pos);

            let corrected_pos = mask_pos;

            let scaled_offset_x = -pivot_x * scale_x * mask_parent_scale.x * fit_scale;
            let scaled_offset_y = pivot_y * scale_y * mask_parent_scale.y * fit_scale;
            let rotated_offset_x =
                scaled_offset_x * rotation_rad.cos() - scaled_offset_y * rotation_rad.sin();
            let rotated_offset_y =
                scaled_offset_x * rotation_rad.sin() + scaled_offset_y * rotation_rad.cos();

            (
                corrected_pos.x + rotated_offset_x,
                corrected_pos.y + rotated_offset_y,
                mask_pos,
                parent_pos,
                corrected_pos,
            )
        } else {
            let scaled_offset_x = -pivot_x * scale_x * mask_global_scale.x;
            let scaled_offset_y = pivot_y * scale_y * mask_global_scale.y;
            let rotated_offset_x =
                scaled_offset_x * rotation_rad.cos() - scaled_offset_y * rotation_rad.sin();
            let rotated_offset_y =
                scaled_offset_x * rotation_rad.sin() + scaled_offset_y * rotation_rad.cos();

            (
                mask_translation.x + rotated_offset_x,
                mask_translation.y + rotated_offset_y,
                mask_translation.truncate(),
                mask_translation.truncate(),
                mask_translation.truncate(),
            )
        };

    let [anim_size_x, anim_size_y] =
        interpolate_vec2(&animated.size, layer_time).unwrap_or([base_width, base_height]);

    let ext = |sw: f32| match stroke_dir {
        "inside" => 0.0,
        "outside" => sw,
        _ => sw * 0.5,
    };
    let initial_stroke_ext = ext(initial_sw);
    let stroke_delta = ext(current_sw) - initial_stroke_ext;
    let initial_stroke_ext_x = initial_stroke_ext;
    let initial_stroke_ext_y = initial_stroke_ext;
    let parent_abs_scale = Vec2::new(mask_parent_scale.x.abs(), mask_parent_scale.y.abs());
    let geom_half_w = if mask.mask_parent_layer_id != 0 {
        anim_size_x / 2.0 * scale_x * parent_abs_scale.x
    } else {
        anim_size_x / 2.0 * scale_x
    };
    let geom_half_h = if mask.mask_parent_layer_id != 0 {
        anim_size_y / 2.0 * scale_y * parent_abs_scale.y
    } else {
        anim_size_y / 2.0 * scale_y
    };
    let mut half_width = ((geom_half_w + initial_stroke_ext_x + stroke_delta) * fit_scale).abs();
    let mut half_height = ((geom_half_h + initial_stroke_ext_y + stroke_delta) * fit_scale).abs();

    let stretch_raw = interpolate_float(&animated.stretch_amount, layer_time).unwrap_or(0.0);
    if stretch_raw > 0.0 {
        let angle_deg = interpolate_float(&animated.stretch_angle, layer_time).unwrap_or(0.0);
        let angle_rad = angle_deg.to_radians();
        let adj = stretch_raw / 500.0;
        let scene_w = animated.canvas_width;
        let scene_h = animated.canvas_height;
        let dx = angle_rad.cos().abs() * adj * scene_w * fit_scale;
        let dy = angle_rad.sin().abs() * adj * scene_h * fit_scale;
        let rc = rotation_rad.cos().abs();
        let rs = rotation_rad.sin().abs();
        half_width += rc * dx + rs * dy;
        half_height += rs * dx + rc * dy;
    }
    let stretch2_raw = interpolate_float(&animated.stretch_seg2_amount, layer_time).unwrap_or(0.0);
    if stretch2_raw > 0.0 {
        let angle_deg = interpolate_float(&animated.stretch_seg2_angle, layer_time).unwrap_or(0.0);
        let angle_rad = angle_deg.to_radians();
        let adj = stretch2_raw / 500.0;
        let scene_w = animated.canvas_width;
        let scene_h = animated.canvas_height;
        let dx = angle_rad.cos().abs() * adj * scene_w * fit_scale;
        let dy = angle_rad.sin().abs() * adj * scene_h * fit_scale;
        let rc = rotation_rad.cos().abs();
        let rs = rotation_rad.sin().abs();
        half_width += rc * dx + rs * dy;
        half_height += rs * dx + rc * dy;
    }

    let sw_world = current_sw * fit_scale;

    let orig_half_w = ((geom_half_w + initial_stroke_ext_x + stroke_delta) * fit_scale).abs();
    let orig_half_h = ((geom_half_h + initial_stroke_ext_y + stroke_delta) * fit_scale).abs();

    let scene_w = animated.canvas_width;
    let scene_h = animated.canvas_height;

    let stretch1 = {
        let s = interpolate_float(&animated.stretch_amount, layer_time).unwrap_or(0.0);
        if s > 0.0 {
            let a = interpolate_float(&animated.stretch_angle, layer_time)
                .unwrap_or(0.0)
                .to_radians();
            let o = interpolate_float(&animated.stretch_offset, layer_time).unwrap_or(0.0) / 1000.0;
            let sm = interpolate_float(&animated.stretch_smooth, layer_time).unwrap_or(0.0);
            Vec4::new(a, s / 500.0, o, sm)
        } else {
            Vec4::ZERO
        }
    };
    let stretch2 = {
        let s = interpolate_float(&animated.stretch_seg2_amount, layer_time).unwrap_or(0.0);
        if s > 0.0 {
            let a = interpolate_float(&animated.stretch_seg2_angle, layer_time)
                .unwrap_or(0.0)
                .to_radians();
            let o = interpolate_float(&animated.stretch_seg2_offset, layer_time).unwrap_or(0.0)
                / 1000.0;
            let sm = interpolate_float(&animated.stretch_seg2_smooth, layer_time).unwrap_or(0.0);
            Vec4::new(a, s / 500.0, o, sm)
        } else {
            Vec4::ZERO
        }
    };

    trace_mask_once(format!("params:{}", mask.mask_layer_id), || {
        format!(
            "[MASK-PARAM] layer_id={} parent_id={} center=({:.2},{:.2}) half=({:.2},{:.2}) rot={:.4} sign={} blend=({:.2},{:.2},{:.2}) gscale=({:.3},{:.3}) pscale=({:.3},{:.3}) entry_center=({:.2},{:.2}) mask_pos=({:.2},{:.2}) parent_pos=({:.2},{:.2}) corrected=({:.2},{:.2})",
            mask.mask_layer_id,
            mask.mask_parent_layer_id,
            center_x,
            center_y,
            half_width.abs(),
            half_height.abs(),
            rotation_rad,
            mask_sign_code,
            fill_alpha,
            mask_opacity,
            sw_world,
            mask_global_scale.x,
            mask_global_scale.y,
            mask_parent_scale.x,
            mask_parent_scale.y,
            mask.center.x * fit_scale,
            mask.center.y * fit_scale,
            trace_mask_pos.x,
            trace_mask_pos.y,
            trace_parent_pos.x,
            trace_parent_pos.y,
            trace_corrected_pos.x,
            trace_corrected_pos.y,
        )
    });

    MaskResult {
        center: Vec2::new(center_x, center_y),
        half_size: Vec2::new(half_width.abs(), half_height.abs()),
        rotation: rotation_rad,
        blend: Vec3::new(fill_alpha, mask_opacity, sw_world),
        sign_code: mask_sign_code,
        stretch1,
        stretch2,
        stretch_info: Vec4::new(
            scene_w * fit_scale,
            scene_h * fit_scale,
            orig_half_w,
            orig_half_h,
        ),
    }
}

#[inline]
pub(super) fn mask_type_flag(is_circle: bool, is_exclude: bool) -> f32 {
    1.0 + is_circle as u8 as f32 + 2.0 * is_exclude as u8 as f32
}

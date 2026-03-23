//! This file evaluates repeat-related mask uniforms.
//! Mask layers can themselves carry repeat and linear-repeat animation data, so
//! the logic here extracts those values and writes the expanded mask-repeat state
//! that the unified shader expects.
//!
//! 这个文件负责求值与重复相关的遮罩 uniform。遮罩图层本身也可能带有 repeat 和
//! linear repeat 动画，因此这里会提取这些值，并写出统一 shader 所需的扩展遮罩
//! 重复状态。

use bevy::prelude::*;

use crate::animation::components::AmAnimated;
use crate::animation::effects::repeat::compute_java_random_state_packed;
use crate::animation::interpolation::{interpolate_float, interpolate_vec2};

use super::trace::trace_mask_once;

pub(super) fn set_mask_repeat_uniforms(
    mask_entry: &crate::scene::AmMaskEntry,
    pending: &crate::scene::AmPendingLayers,
    mask_layer_query: &Query<(&GlobalTransform, &AmAnimated, &crate::scene::AmLayerSpec)>,
    playback_time: f32,
    fit_scale: f32,
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
) {
    let disable_mask_linear_repeat = std::env::var_os("AM_DISABLE_MASK_LINEAR_REPEAT").is_some();

    let Some(&mask_entity) = pending.spawned_entities.get(&mask_entry.mask_layer_id) else {
        trace_mask_once(
            format!("repeat-missing:{}", mask_entry.mask_layer_id),
            || {
                format!(
                    "[MASK-RPT] mask entity NOT in spawned_entities for layer_id={}",
                    mask_entry.mask_layer_id
                )
            },
        );
        material.uniform_data.mask1_lr_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
        material.uniform_data.mask1_lr2_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
        material.uniform_data.mask1_repeat_params1 = Vec4::ZERO;
        material.uniform_data.mask1_rr_params1 = Vec4::ZERO;
        return;
    };
    let Ok((_gt, animated, _spec)) = mask_layer_query.get(mask_entity) else {
        trace_mask_once(format!("repeat-query-missing:{mask_entity:?}"), || {
            format!(
                "[MASK-RPT] mask entity {:?} missing query components (GT/Animated/Spec)",
                mask_entity
            )
        });
        material.uniform_data.mask1_lr_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
        material.uniform_data.mask1_lr2_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
        material.uniform_data.mask1_repeat_params1 = Vec4::ZERO;
        material.uniform_data.mask1_rr_params1 = Vec4::ZERO;
        return;
    };

    let local_time = animated.calc_local_time(playback_time);
    let layer_time = animated.calc_layer_time(local_time);
    let rp_count = interpolate_float(&animated.repeat_count, layer_time).unwrap_or(0.0);
    bevy::log::debug!(
        "[MASK-RPT] mask layer_id={} rp_count={:.1} lr_count={:.1}",
        mask_entry.mask_layer_id,
        rp_count,
        interpolate_float(&animated.linear_repeat_count, layer_time).unwrap_or(0.0)
    );
    if rp_count > 0.0 {
        let rp_offset = interpolate_vec2(&animated.repeat_offset, layer_time).unwrap_or([0.0, 0.0]);
        let rp_angle = interpolate_float(&animated.repeat_angle, layer_time).unwrap_or(0.0);
        let rp_scale = interpolate_float(&animated.repeat_scale, layer_time).unwrap_or(1.0);
        let rp_alpha = interpolate_float(&animated.repeat_alpha, layer_time).unwrap_or(1.0);

        let off_world_x = rp_offset[0] * fit_scale;
        let off_world_y = -rp_offset[1] * fit_scale;

        material.uniform_data.mask1_repeat_params1 =
            Vec4::new(rp_count.floor(), off_world_x, off_world_y, rp_angle);
        material.uniform_data.mask1_repeat_params2 = Vec4::new(rp_scale, rp_alpha, 0.0, 0.0);
    } else {
        material.uniform_data.mask1_repeat_params1 = Vec4::ZERO;
        material.uniform_data.mask1_repeat_params2 = Vec4::new(1.0, 1.0, 0.0, 0.0);
    }

    let count = interpolate_float(&animated.linear_repeat_count, layer_time)
        .unwrap_or(0.0)
        .round();
    if !disable_mask_linear_repeat && count > 0.0 {
        let pos =
            interpolate_vec2(&animated.linear_repeat_position, layer_time).unwrap_or([0.0, 0.0]);
        let off =
            interpolate_vec2(&animated.linear_repeat_offset, layer_time).unwrap_or([0.0, 0.0]);
        let angle = interpolate_float(&animated.linear_repeat_angle, layer_time).unwrap_or(0.0);
        let lr_scale = interpolate_float(&animated.linear_repeat_scale, layer_time).unwrap_or(1.0);
        let alpha = interpolate_float(&animated.linear_repeat_alpha, layer_time).unwrap_or(1.0);
        let start = interpolate_float(&animated.linear_repeat_start, layer_time).unwrap_or(0.0);
        let end = interpolate_float(&animated.linear_repeat_end, layer_time).unwrap_or(1.0);
        let phase = interpolate_float(&animated.linear_repeat_phase, layer_time).unwrap_or(0.0);
        let overlap = interpolate_float(&animated.linear_repeat_overlap, layer_time).unwrap_or(0.0);
        let ease_in = interpolate_float(&animated.linear_repeat_ease_in, layer_time).unwrap_or(0.0);
        let ease_out =
            interpolate_float(&animated.linear_repeat_ease_out, layer_time).unwrap_or(0.0);
        let sia = animated.linear_repeat_shape * 100
            + if animated.linear_repeat_invert { 10 } else { 0 }
            + if animated.linear_repeat_color_alt_copies {
                1
            } else {
                0
            };

        let pos_world_x = pos[0] * fit_scale;
        let pos_world_y = -pos[1] * fit_scale;
        let off_world_x = off[0] * fit_scale;
        let off_world_y = -off[1] * fit_scale;

        trace_mask_once(format!("repeat:{}", mask_entry.mask_layer_id), || {
            format!(
                "[MASK-REPEAT] layer_id={} count={:.0} pos_world=({:.2},{:.2}) off_world=({:.2},{:.2}) repeat_scale=({:.3},{:.3}) angle={:.2}",
                mask_entry.mask_layer_id,
                count,
                pos_world_x,
                pos_world_y,
                off_world_x,
                off_world_y,
                1.0,
                1.0,
                angle,
            )
        });

        material.uniform_data.mask1_lr_params1 = Vec4::new(count, pos_world_x, pos_world_y, angle);
        material.uniform_data.mask1_lr_params2 =
            Vec4::new(off_world_x, off_world_y, lr_scale, alpha);
        material.uniform_data.mask1_lr_params3 = Vec4::new(start, end, phase, overlap);
        material.uniform_data.mask1_lr_params4 = Vec4::new(ease_in, ease_out, 0.0, sia as f32);
        material.uniform_data.mask1_lr_params5 = if animated.linear_repeat_random_order {
            let seed = interpolate_float(&animated.linear_repeat_seed, layer_time).unwrap_or(0.0);
            let (lo, hi) = compute_java_random_state_packed(seed);
            Vec4::new(1.0, lo, hi, 0.0)
        } else {
            Vec4::ZERO
        };
    } else {
        material.uniform_data.mask1_lr_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
    }

    if let Some(ref lr2) = animated.linear_repeat2 {
        let count2 = interpolate_float(&lr2.count, layer_time)
            .unwrap_or(0.0)
            .round();
        if count2 > 0.0 {
            let pos2 = interpolate_vec2(&lr2.position, layer_time).unwrap_or([0.0, 0.0]);
            let off2 = interpolate_vec2(&lr2.offset, layer_time).unwrap_or([0.0, 0.0]);
            let angle2 = interpolate_float(&lr2.angle, layer_time).unwrap_or(0.0);
            let scale2 = interpolate_float(&lr2.scale, layer_time).unwrap_or(1.0);
            let alpha2 = interpolate_float(&lr2.alpha, layer_time).unwrap_or(1.0);
            let start2 = interpolate_float(&lr2.start, layer_time).unwrap_or(0.0);
            let end2 = interpolate_float(&lr2.end, layer_time).unwrap_or(1.0);
            let phase2 = interpolate_float(&lr2.phase, layer_time).unwrap_or(0.0);
            let overlap2 = interpolate_float(&lr2.overlap, layer_time).unwrap_or(0.0);
            let ease_in2 = interpolate_float(&lr2.ease_in, layer_time).unwrap_or(0.0);
            let ease_out2 = interpolate_float(&lr2.ease_out, layer_time).unwrap_or(0.0);
            let sia2 = lr2.shape * 100
                + if lr2.invert { 10 } else { 0 }
                + if lr2.color_alt_copies { 1 } else { 0 };

            let pos2_world_x = pos2[0] * fit_scale;
            let pos2_world_y = -pos2[1] * fit_scale;
            let off2_world_x = off2[0] * fit_scale;
            let off2_world_y = -off2[1] * fit_scale;

            material.uniform_data.mask1_lr2_params1 =
                Vec4::new(count2, pos2_world_x, pos2_world_y, angle2);
            material.uniform_data.mask1_lr2_params2 =
                Vec4::new(off2_world_x, off2_world_y, scale2, alpha2);
            material.uniform_data.mask1_lr2_params3 = Vec4::new(start2, end2, phase2, overlap2);
            material.uniform_data.mask1_lr2_params4 =
                Vec4::new(ease_in2, ease_out2, 0.0, sia2 as f32);
            material.uniform_data.mask1_lr2_params5 = if lr2.random_order {
                let seed2 = interpolate_float(&lr2.seed, layer_time).unwrap_or(0.0);
                let (lo2, hi2) = compute_java_random_state_packed(seed2);
                Vec4::new(1.0, lo2, hi2, 0.0)
            } else {
                Vec4::ZERO
            };
        } else {
            material.uniform_data.mask1_lr2_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
        }
    } else {
        material.uniform_data.mask1_lr2_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
    }

    let rr_count = interpolate_float(&animated.radial_repeat_count, layer_time)
        .unwrap_or(0.0)
        .round();
    if rr_count > 0.0 {
        let radius = interpolate_float(&animated.radial_repeat_radius, layer_time).unwrap_or(0.0);
        let orientation =
            interpolate_float(&animated.radial_repeat_orientation, layer_time).unwrap_or(0.0);
        let start_angle =
            interpolate_float(&animated.radial_repeat_start_angle, layer_time).unwrap_or(0.0);
        let sweep = interpolate_float(&animated.radial_repeat_sweep, layer_time).unwrap_or(360.0);
        let base_scale =
            interpolate_float(&animated.radial_repeat_base_scale, layer_time).unwrap_or(1.0);
        let offset =
            interpolate_vec2(&animated.radial_repeat_offset, layer_time).unwrap_or([0.0, 0.0]);
        let angle = interpolate_float(&animated.radial_repeat_angle, layer_time).unwrap_or(0.0);
        let rr_scale = interpolate_float(&animated.radial_repeat_scale, layer_time).unwrap_or(1.0);
        let alpha = interpolate_float(&animated.radial_repeat_alpha, layer_time).unwrap_or(1.0);
        let start = interpolate_float(&animated.radial_repeat_start, layer_time).unwrap_or(0.0);
        let end = interpolate_float(&animated.radial_repeat_end, layer_time).unwrap_or(1.0);
        let phase = interpolate_float(&animated.radial_repeat_phase, layer_time).unwrap_or(0.0);
        let overlap = interpolate_float(&animated.radial_repeat_overlap, layer_time).unwrap_or(0.0);
        let ease_in = interpolate_float(&animated.radial_repeat_ease_in, layer_time).unwrap_or(0.0);
        let ease_out =
            interpolate_float(&animated.radial_repeat_ease_out, layer_time).unwrap_or(0.0);

        let sia = animated.radial_repeat_shape * 100
            + if animated.radial_repeat_invert { 10 } else { 0 }
            + if animated.radial_repeat_color_alt_copies {
                1
            } else {
                0
            };

        let off_world_x = offset[0] * fit_scale;
        let off_world_y = -offset[1] * fit_scale;
        let radius_world = radius * fit_scale;

        material.uniform_data.mask1_rr_params1 =
            Vec4::new(rr_count, radius_world, orientation, start_angle);
        material.uniform_data.mask1_rr_params2 = Vec4::new(sweep, base_scale, angle, rr_scale);
        material.uniform_data.mask1_rr_params3 = Vec4::new(alpha, off_world_x, off_world_y, 0.0);
        material.uniform_data.mask1_rr_params4 = Vec4::new(start, end, phase, overlap);
        material.uniform_data.mask1_rr_params5 = Vec4::new(
            ease_in,
            ease_out,
            sia as f32,
            if animated.radial_repeat_random_order {
                animated.radial_repeat_seed + 0.5
            } else {
                animated.radial_repeat_seed
            },
        );
    } else {
        material.uniform_data.mask1_rr_params1 = Vec4::ZERO;
    }
}

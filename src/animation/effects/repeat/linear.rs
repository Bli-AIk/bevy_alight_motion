//! Evaluates the linear-repeat effect for unified visuals.
//! It turns the authored repeat count, offsets, ordering, color, and timing
//! parameters into uniform values and any supporting mesh/entity updates required
//! by the runtime.
//!
//! 负责为统一材质视觉对象求值 linear repeat 效果。它会把作者写下的重复次数、
//! 位移、顺序、颜色和时间参数转换成 uniform 数据，并在运行时需要时同步更新相关
//! 网格或辅助实体。

use bevy::prelude::*;

use crate::animation::components::AmAnimated;
use crate::animation::interpolation::{interpolate_color, interpolate_float, interpolate_vec2};

use super::java_random::compute_java_random_state_packed;

fn linear_repeat_trace_enabled(layer_id: u64) -> bool {
    if std::env::var_os("AM_TRACE_LINEAR_REPEAT_ALL").is_some() {
        return true;
    }
    std::env::var_os("AM_TRACE_LINEAR_REPEAT_IDS")
        .and_then(|value| value.into_string().ok())
        .is_some_and(|ids| {
            ids.split(',')
                .filter_map(|value| value.trim().parse::<u64>().ok())
                .any(|id| id == layer_id)
        })
}

pub(crate) fn process_linear_repeat_effect(
    animated: &AmAnimated,
    layer_time: f32,
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    orig_width: f32,
    orig_height: f32,
    mesh2d: &bevy::mesh::Mesh2d,
    meshes: &mut Assets<Mesh>,
) {
    let trace_enabled = linear_repeat_trace_enabled(animated.layer_id);
    if std::env::var_os("AM_DISABLE_LINEAR_REPEAT").is_some() {
        if trace_enabled {
            bevy::log::warn!(
                "[LinearRepeatTrace] layer={} disabled by AM_DISABLE_LINEAR_REPEAT",
                animated.layer_id
            );
        }
        material.uniform_data.linear_repeat_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
        material.uniform_data.linear_repeat_params2 = Vec4::new(0.0, 0.0, 1.0, 1.0);
        material.uniform_data.linear_repeat_params3 = Vec4::new(0.0, 1.0, 0.0, 0.0);
        material.uniform_data.linear_repeat_params4 = Vec4::ZERO;
        material.uniform_data.linear_repeat_params5 = Vec4::ZERO;
        material.uniform_data.linear_repeat_fill_color = Vec4::new(1.0, 1.0, 1.0, 1.0);
        material.uniform_data.linear_repeat2_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
        material.uniform_data.linear_repeat2_params2 = Vec4::new(0.0, 0.0, 1.0, 1.0);
        material.uniform_data.linear_repeat2_params3 = Vec4::new(0.0, 1.0, 0.0, 0.0);
        material.uniform_data.linear_repeat2_params4 = Vec4::ZERO;
        material.uniform_data.linear_repeat2_params5 = Vec4::ZERO;
        material.uniform_data.linear_repeat2_fill_color = Vec4::new(1.0, 1.0, 1.0, 1.0);
        return;
    }

    let has_linear_repeat = animated.linear_repeat_count.value.is_some_and(|v| v > 0.0)
        || animated
            .linear_repeat_count
            .keyframes
            .iter()
            .any(|kf| kf.value.parse::<f32>().unwrap_or(0.0) > 0.0);
    if !has_linear_repeat {
        material.uniform_data.linear_repeat_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
        material.uniform_data.linear_repeat_params2 = Vec4::new(0.0, 0.0, 1.0, 1.0);
        material.uniform_data.linear_repeat_params3 = Vec4::new(0.0, 1.0, 0.0, 0.0);
        material.uniform_data.linear_repeat_params4 = Vec4::ZERO;
        material.uniform_data.linear_repeat_params5 = Vec4::ZERO;
        material.uniform_data.linear_repeat_fill_color = Vec4::new(1.0, 1.0, 1.0, 1.0);
        material.uniform_data.linear_repeat2_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
        material.uniform_data.linear_repeat2_params2 = Vec4::new(0.0, 0.0, 1.0, 1.0);
        material.uniform_data.linear_repeat2_params3 = Vec4::new(0.0, 1.0, 0.0, 0.0);
        material.uniform_data.linear_repeat2_params4 = Vec4::ZERO;
        material.uniform_data.linear_repeat2_params5 = Vec4::ZERO;
        material.uniform_data.linear_repeat2_fill_color = Vec4::new(1.0, 1.0, 1.0, 1.0);
        return;
    }

    let count = interpolate_float(&animated.linear_repeat_count, layer_time).unwrap_or(0.0);
    let position =
        interpolate_vec2(&animated.linear_repeat_position, layer_time).unwrap_or([0.0, 0.0]);
    let offset = interpolate_vec2(&animated.linear_repeat_offset, layer_time).unwrap_or([0.0, 0.0]);
    let angle = interpolate_float(&animated.linear_repeat_angle, layer_time).unwrap_or(0.0);
    let scale = interpolate_float(&animated.linear_repeat_scale, layer_time).unwrap_or(1.0);
    let alpha = interpolate_float(&animated.linear_repeat_alpha, layer_time).unwrap_or(1.0);
    let fill_color_srgb = interpolate_color(&animated.linear_repeat_fill_color, layer_time)
        .unwrap_or(Vec4::new(1.0, 1.0, 1.0, 1.0));
    let fill_color = Vec4::new(
        fill_color_srgb.x.powf(2.2),
        fill_color_srgb.y.powf(2.2),
        fill_color_srgb.z.powf(2.2),
        fill_color_srgb.w,
    );
    let blend = interpolate_float(&animated.linear_repeat_blend, layer_time).unwrap_or(0.0);
    let start = interpolate_float(&animated.linear_repeat_start, layer_time).unwrap_or(0.0);
    let end = interpolate_float(&animated.linear_repeat_end, layer_time).unwrap_or(1.0);
    let phase = interpolate_float(&animated.linear_repeat_phase, layer_time).unwrap_or(0.0);
    let ease_in = interpolate_float(&animated.linear_repeat_ease_in, layer_time).unwrap_or(0.0);
    let ease_out = interpolate_float(&animated.linear_repeat_ease_out, layer_time).unwrap_or(0.0);
    let overlap = interpolate_float(&animated.linear_repeat_overlap, layer_time).unwrap_or(0.0);

    let shape_invert_alt = animated.linear_repeat_shape * 100
        + if animated.linear_repeat_invert { 10 } else { 0 }
        + if animated.linear_repeat_color_alt_copies {
            1
        } else {
            0
        };

    let count_rounded = count.round();

    material.uniform_data.linear_repeat_params1 =
        Vec4::new(count_rounded, position[0], position[1], angle);
    material.uniform_data.linear_repeat_params2 = Vec4::new(offset[0], offset[1], scale, alpha);
    material.uniform_data.linear_repeat_params3 = Vec4::new(start, end, phase, overlap);
    material.uniform_data.linear_repeat_params4 =
        Vec4::new(ease_in, ease_out, blend, shape_invert_alt as f32);
    material.uniform_data.linear_repeat_params5 = if animated.linear_repeat_random_order {
        let seed = interpolate_float(&animated.linear_repeat_seed, layer_time).unwrap_or(0.0);
        let (state_lo_bits, state_hi_bits) = compute_java_random_state_packed(seed);
        Vec4::new(1.0, state_lo_bits, state_hi_bits, 0.0)
    } else {
        Vec4::ZERO
    };
    material.uniform_data.linear_repeat_fill_color = fill_color;

    let (has_lr2, count2_rounded, position2, offset2, angle2, scale2) =
        if let Some(ref lr2) = animated.linear_repeat2 {
            let c2 = interpolate_float(&lr2.count, layer_time).unwrap_or(0.0);
            let p2 = interpolate_vec2(&lr2.position, layer_time).unwrap_or([0.0, 0.0]);
            let o2 = interpolate_vec2(&lr2.offset, layer_time).unwrap_or([0.0, 0.0]);
            let a2 = interpolate_float(&lr2.angle, layer_time).unwrap_or(0.0);
            let s2 = interpolate_float(&lr2.scale, layer_time).unwrap_or(1.0);
            let al2 = interpolate_float(&lr2.alpha, layer_time).unwrap_or(1.0);
            let fc2_srgb = interpolate_color(&lr2.fill_color, layer_time)
                .unwrap_or(Vec4::new(1.0, 1.0, 1.0, 1.0));
            let fc2 = Vec4::new(
                fc2_srgb.x.powf(2.2),
                fc2_srgb.y.powf(2.2),
                fc2_srgb.z.powf(2.2),
                fc2_srgb.w,
            );
            let bl2 = interpolate_float(&lr2.blend, layer_time).unwrap_or(0.0);
            let st2 = interpolate_float(&lr2.start, layer_time).unwrap_or(0.0);
            let en2 = interpolate_float(&lr2.end, layer_time).unwrap_or(1.0);
            let ph2 = interpolate_float(&lr2.phase, layer_time).unwrap_or(0.0);
            let ei2 = interpolate_float(&lr2.ease_in, layer_time).unwrap_or(0.0);
            let eo2 = interpolate_float(&lr2.ease_out, layer_time).unwrap_or(0.0);
            let ov2 = interpolate_float(&lr2.overlap, layer_time).unwrap_or(0.0);
            let sia2 = lr2.shape * 100
                + if lr2.invert { 10 } else { 0 }
                + if lr2.color_alt_copies { 1 } else { 0 };
            let c2r = c2.round();

            material.uniform_data.linear_repeat2_params1 = Vec4::new(c2r, p2[0], p2[1], a2);
            material.uniform_data.linear_repeat2_params2 = Vec4::new(o2[0], o2[1], s2, al2);
            material.uniform_data.linear_repeat2_params3 = Vec4::new(st2, en2, ph2, ov2);
            material.uniform_data.linear_repeat2_params4 = Vec4::new(ei2, eo2, bl2, sia2 as f32);
            material.uniform_data.linear_repeat2_params5 = if lr2.random_order {
                let lr2_seed = interpolate_float(&lr2.seed, layer_time).unwrap_or(0.0);
                let (state_lo_bits, state_hi_bits) = compute_java_random_state_packed(lr2_seed);
                Vec4::new(1.0, state_lo_bits, state_hi_bits, 0.0)
            } else {
                Vec4::ZERO
            };
            material.uniform_data.linear_repeat2_fill_color = fc2;
            (true, c2r, p2, o2, a2, s2)
        } else {
            material.uniform_data.linear_repeat2_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
            material.uniform_data.linear_repeat2_params2 = Vec4::new(0.0, 0.0, 1.0, 1.0);
            material.uniform_data.linear_repeat2_params3 = Vec4::new(0.0, 1.0, 0.0, 0.0);
            material.uniform_data.linear_repeat2_params4 = Vec4::ZERO;
            material.uniform_data.linear_repeat2_params5 = Vec4::ZERO;
            material.uniform_data.linear_repeat2_fill_color = Vec4::new(1.0, 1.0, 1.0, 1.0);
            (false, 0.0, [0.0, 0.0], [0.0, 0.0], 0.0, 1.0)
        };

    if trace_enabled {
        let progress = calc_linear_repeat_progress(
            0,
            count_rounded.max(1.0) as i32,
            start,
            end,
            phase,
            overlap,
            animated.linear_repeat_shape,
            animated.linear_repeat_invert,
            ease_in,
            ease_out,
        );
        let progress2 = has_lr2.then(|| {
            calc_linear_repeat_progress(
                0,
                count2_rounded.max(1.0) as i32,
                material.uniform_data.linear_repeat2_params3.x,
                material.uniform_data.linear_repeat2_params3.y,
                material.uniform_data.linear_repeat2_params3.z,
                material.uniform_data.linear_repeat2_params3.w,
                if let Some(ref lr2) = animated.linear_repeat2 {
                    lr2.shape
                } else {
                    0
                },
                if let Some(ref lr2) = animated.linear_repeat2 {
                    lr2.invert
                } else {
                    false
                },
                material.uniform_data.linear_repeat2_params4.x,
                material.uniform_data.linear_repeat2_params4.y,
            )
        });
        bevy::log::warn!(
            "[LinearRepeatTrace] layer={} layer_time={:.6} orig=({:.2},{:.2}) count={:.3}->{:.0} pos=({:.2},{:.2}) off=({:.2},{:.2}) angle={:.2} scale={:.3} alpha={:.3} start={:.3} end={:.3} phase={:.3} overlap={:.3} p0=({:.3},{:.3}) lr2={:?}",
            animated.layer_id,
            layer_time,
            orig_width,
            orig_height,
            count,
            count_rounded,
            position[0],
            position[1],
            offset[0],
            offset[1],
            angle,
            scale,
            alpha,
            start,
            end,
            phase,
            overlap,
            progress.0,
            progress.1,
            progress2,
        );
    }

    let n = count_rounded as i32;
    let angle_rad = angle.to_radians();
    let mut min_x = -orig_width / 2.0;
    let mut max_x = orig_width / 2.0;
    let mut min_y = -orig_height / 2.0;
    let mut max_y = orig_height / 2.0;
    let interp_progress = 1.0;

    let compute_displacement = |idx: i32, count: i32, pos: [f32; 2], off: [f32; 2]| -> (f32, f32) {
        let base = if count > 1 {
            idx as f32 / (count as f32 - 1.0)
        } else {
            0.0
        };
        (
            pos[0] * base + off[0] * interp_progress,
            pos[1] * base + off[1] * interp_progress,
        )
    };

    let n2 = if has_lr2 { count2_rounded as i32 } else { 1 };
    let angle2_rad = angle2.to_radians();

    for j in 0..n2 {
        let (d2x, d2y) = if has_lr2 {
            compute_displacement(j, n2, position2, offset2)
        } else {
            (0.0, 0.0)
        };
        let cum_scale2 = if has_lr2 {
            1.0 + (scale2 - 1.0) * interp_progress
        } else {
            1.0
        };
        let cum_angle2 = if has_lr2 {
            angle2_rad * interp_progress
        } else {
            0.0
        };

        for i in 0..n {
            let (d1x, d1y) = compute_displacement(i, n, position, offset);
            let cum_scale1 = 1.0 + (scale - 1.0) * interp_progress;
            let cum_angle1 = angle_rad * interp_progress;

            let scaled_d1x = d1x * cum_scale2;
            let scaled_d1y = d1y * cum_scale2;
            let (rot_d1x, rot_d1y) = if cum_angle2.abs() > 0.001 {
                let c = cum_angle2.cos();
                let s = cum_angle2.sin();
                (
                    scaled_d1x * c - scaled_d1y * s,
                    scaled_d1x * s + scaled_d1y * c,
                )
            } else {
                (scaled_d1x, scaled_d1y)
            };
            let total_dx = d2x + rot_d1x;
            let total_dy = d2y + rot_d1y;
            let cum_offset_x = total_dx;
            let cum_offset_y = -total_dy;
            let total_scale = cum_scale1 * cum_scale2;
            let total_angle = cum_angle1 + cum_angle2;

            let half_w = orig_width / 2.0 * total_scale;
            let half_h = orig_height / 2.0 * total_scale;
            let corners = [
                (-half_w, -half_h),
                (half_w, -half_h),
                (half_w, half_h),
                (-half_w, half_h),
            ];
            let cos_a = total_angle.cos();
            let sin_a = total_angle.sin();
            for (cx, cy) in corners {
                let rx = cx * cos_a - cy * sin_a + cum_offset_x;
                let ry = cx * sin_a + cy * cos_a + cum_offset_y;
                min_x = min_x.min(rx);
                max_x = max_x.max(rx);
                min_y = min_y.min(ry);
                max_y = max_y.max(ry);
            }
        }
    }

    let max_scale = scale.abs().max(1.0) * (if has_lr2 { scale2.abs().max(1.0) } else { 1.0 });
    let padding = 20.0 * max_scale
        + offset[0].abs()
        + offset[1].abs()
        + if has_lr2 {
            offset2[0].abs() + offset2[1].abs()
        } else {
            0.0
        };
    min_x -= padding;
    max_x += padding;
    min_y -= padding;
    max_y += padding;

    let new_width = max_x - min_x;
    let new_height = max_y - min_y;
    material.uniform_data.original_size = Vec4::new(orig_width, orig_height, new_width, new_height);

    let uv_min_x = min_x / orig_width + 0.5;
    let uv_max_x = max_x / orig_width + 0.5;
    let uv_at_bottom = 0.5 - min_y / orig_height;
    let uv_at_top = 0.5 - max_y / orig_height;

    let vertices = vec![
        [min_x, min_y, 0.0],
        [max_x, min_y, 0.0],
        [max_x, max_y, 0.0],
        [min_x, max_y, 0.0],
    ];
    let uvs = vec![
        [uv_min_x, uv_at_bottom],
        [uv_max_x, uv_at_bottom],
        [uv_max_x, uv_at_top],
        [uv_min_x, uv_at_top],
    ];
    let indices = vec![0u32, 1, 2, 0, 2, 3];

    super::overwrite_repeat_mesh(meshes, mesh2d, vertices, uvs, indices);
}

#[expect(dead_code)] // reason: kept as a CPU parity helper while SDF linear-repeat copy displacement is still partial
fn calc_linear_repeat_progress(
    index: i32,
    count: i32,
    start: f32,
    end: f32,
    phase: f32,
    overlap: f32,
    shape: i32,
    invert: bool,
    ease_in: f32,
    ease_out: f32,
) -> (f32, f32) {
    let fi = index as f32;
    let fcount = count as f32;
    let overlap_value = overlap + 1.0;
    let denominator = (2.0 * overlap_value) + fcount - 1.0;
    let step_width = 1.0 / denominator;
    let half_width = step_width * overlap_value;
    let base_position = (fi + overlap_value) / denominator + phase;
    let center_pos = base_position + half_width * 0.5;
    let base_progress = if count > 1 { fi / (fcount - 1.0) } else { 0.0 };

    let mut interp_progress = match shape {
        1 => {
            let in_fade = ((base_position - start) / half_width).clamp(0.0, 1.0);
            let out_fade = ((end - base_position) / half_width).clamp(0.0, 1.0);
            if start < end {
                in_fade.min(out_fade)
            } else {
                1.0 - in_fade.max(out_fade)
            }
        }
        2 => {
            if center_pos >= start && center_pos <= end {
                let x = (center_pos - start) / (end - start);
                let centered = (x - 0.5) * 2.0 * std::f32::consts::PI;
                (-centered * centered * 0.5).exp()
            } else {
                0.0
            }
        }
        3 => {
            if center_pos >= start && center_pos <= end {
                let x = (center_pos - start) / (end - start);
                if x < 0.5 { x * 2.0 } else { (1.0 - x) * 2.0 }
            } else {
                0.0
            }
        }
        _ => {
            let range = (end - start).max(0.001);
            (center_pos - start) / range
        }
    };

    if ease_in.abs() > 0.001 || ease_out.abs() > 0.001 {
        interp_progress = apply_repeat_easing(interp_progress.clamp(0.0, 1.0), ease_in, ease_out);
    }
    if invert {
        interp_progress = 1.0 - interp_progress;
    }
    interp_progress = interp_progress.clamp(0.0, 1.0);
    (base_progress, interp_progress)
}

fn apply_repeat_easing(progress: f32, ease_in: f32, ease_out: f32) -> f32 {
    if ease_in.abs() < 0.001 && ease_out.abs() < 0.001 {
        return progress;
    }
    let p1x = (ease_in * 0.5).max(0.0);
    let p1y = (-ease_in * 0.5).max(0.0);
    let p2x = 1.0 - (ease_out * 0.5).max(0.0);
    let p2y = 1.0 - (-ease_out * 0.5).max(0.0);
    cubic_bezier_2d(progress, p1x, p1y, p2x, p2y)
}

fn cubic_bezier_2d(t: f32, p1x: f32, p1y: f32, p2x: f32, p2y: f32) -> f32 {
    let mut guess = t;
    for _ in 0..8 {
        let x = cubic_bezier_1d(guess, p1x, p2x) - t;
        if x.abs() < 0.001 {
            break;
        }
        let dx = cubic_bezier_1d_derivative(guess, p1x, p2x);
        if dx.abs() < 0.0001 {
            break;
        }
        guess -= x / dx;
        guess = guess.clamp(0.0, 1.0);
    }
    cubic_bezier_1d(guess, p1y, p2y)
}

fn cubic_bezier_1d(t: f32, p1: f32, p2: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    3.0 * mt2 * t * p1 + 3.0 * mt * t2 * p2 + t3
}

fn cubic_bezier_1d_derivative(t: f32, p1: f32, p2: f32) -> f32 {
    let mt = 1.0 - t;
    3.0 * mt * mt * p1 + 6.0 * mt * t * (p2 - p1) + 3.0 * t * t * (1.0 - p2)
}

pub(crate) fn compute_sdf_linear_repeat_displacement(
    animated: &AmAnimated,
    layer_time: f32,
) -> Option<[f32; 2]> {
    let count = interpolate_float(&animated.linear_repeat_count, layer_time).unwrap_or(-1.0);
    let count_rounded = count.round() as i32;

    if count_rounded < 0 {
        return None;
    }
    if count_rounded == 0 {
        return Some([f32::NAN, f32::NAN]);
    }
    None
}

//! This file holds math helpers shared by multiple animation systems.
//! It centralizes reusable calculations such as perspective zoom, accumulated
//! frequencies, oscillation waveforms, and unwrapped rotation resolution so the
//! per-feature systems can stay focused on applying results to entities.
//!
//! 这个文件存放多个动画系统共用的数学辅助逻辑。它把透视缩放、频率累积、振荡波形、
//! 以及连续旋转角解析等可复用计算集中起来，让具体效果系统只需要专注于把结果应用到
//! 实体上。

use crate::animation::components::AmAnimated;
use crate::animation::interpolation::{interpolate_float, interpolate_float_reverse};

fn invert_if(val: f32, inv: bool) -> f32 {
    if inv { -val } else { val }
}

fn accumulate_hz(
    field: &crate::schema::AmAnimatedFloat,
    layer_time: f32,
    duration_sec: f32,
    non_keyed_factor: f32,
    step_divisor: f64,
) -> f32 {
    let time_sec = layer_time * duration_sec;
    if field.keyframes.is_empty() {
        let val = interpolate_float(field, layer_time).unwrap_or(0.0);
        val * time_sec * non_keyed_factor
    } else {
        let total_steps = (duration_sec * 120.0).round() as i32;
        let current_step = (120.0 * time_sec).round() as i32;
        let mut accum = 0.0f64;
        if total_steps > 0 {
            for i in 0..=current_step.min(total_steps) {
                let frac_t = i as f32 / total_steps as f32;
                let val_at_t = interpolate_float(field, frac_t).unwrap_or(0.0);
                accum += val_at_t as f64 / step_divisor;
            }
        }
        accum as f32
    }
}

fn compute_oscillate_wave(
    wave_type: i32,
    accumulated_freq: f32,
    phase: f32,
    sine_offset: f32,
    triangle_offset: f32,
) -> f32 {
    let sp = phase + sine_offset;
    match wave_type {
        0 => ((accumulated_freq * 2.0 + sp * 2.0) * std::f32::consts::PI).sin(),
        1 => {
            let tp = phase + triangle_offset;
            let x = (accumulated_freq * 2.0 + tp * 2.0) / 2.0 + tp;
            let x_mod = ((x + 0.75).rem_euclid(1.0)) - 0.5;
            x_mod.abs() * 4.0 - 1.0
        }
        _ => ((accumulated_freq * 2.0 + sp * 2.0) * std::f32::consts::PI).sin(),
    }
}

pub(crate) fn compute_perspective_zoom(
    z_offset: f32,
    canvas_width: f32,
    canvas_height: f32,
) -> f32 {
    if z_offset == 0.0 {
        return 1.0;
    }

    let cam_dist = canvas_width.max(canvas_height) / (2.0 * (30.0_f32).to_radians().tan());
    let denom = cam_dist + z_offset;
    if denom > 0.0 { cam_dist / denom } else { 0.001 }
}

pub(crate) fn resolve_unwrapped_rotation_deg(
    animated: &AmAnimated,
    layer_time: f32,
    frame_delta: f32,
) -> f32 {
    let base_rotation =
        interpolate_float_reverse(&animated.rotation, layer_time, frame_delta).unwrap_or(0.0);
    let mut final_rotation = -base_rotation;

    if let Some(swing_freq) = interpolate_float(&animated.swing_freq, layer_time)
        && swing_freq > 0.0
    {
        let swing_a1 = interpolate_float(&animated.swing_a1, layer_time).unwrap_or(0.0);
        let swing_a2 = interpolate_float(&animated.swing_a2, layer_time).unwrap_or(0.0);
        let swing_phase = interpolate_float(&animated.swing_phase, layer_time).unwrap_or(0.0);
        let duration_sec = (animated.end_time - animated.start_time) as f32 / 1000.0;
        let accumulated_freq =
            accumulate_hz(&animated.swing_freq, layer_time, duration_sec, 1.0, 120.0);

        let wave_value = match animated.swing_type {
            0 => ((accumulated_freq + swing_phase) * std::f32::consts::PI).sin(),
            1 => {
                let x = (accumulated_freq + swing_phase) / 2.0;
                let x_mod = ((x + 0.75).rem_euclid(1.0)) - 0.5;
                x_mod.abs() * 4.0 - 1.0
            }
            _ => ((accumulated_freq + swing_phase) * std::f32::consts::PI).sin(),
        };

        let swing_angle = ((swing_a2 - swing_a1) * ((wave_value + 1.0) / 2.0)) + swing_a1;
        final_rotation -= swing_angle;
    }

    if animated.spin_rpm.value.is_some() || !animated.spin_rpm.keyframes.is_empty() {
        let duration_sec = (animated.end_time - animated.start_time) as f32 / 1000.0;
        let spin_angle = accumulate_hz(&animated.spin_rpm, layer_time, duration_sec, 6.0, 20.0);
        final_rotation -= spin_angle;
    }

    if let Some(effect_angle) = interpolate_float(&animated.effect_angle, layer_time) {
        final_rotation -= invert_if(effect_angle, animated.effect_ainv);
    }

    for extra in &animated.extra_transform2 {
        let Some(extra_angle) = interpolate_float(&extra.angle, layer_time) else {
            continue;
        };
        final_rotation -= invert_if(extra_angle, extra.ainv);
    }

    final_rotation + animated.repeat_rotation_offset_deg
}

pub(crate) fn compute_normalized_frame_delta(animated: &AmAnimated) -> f32 {
    let element_duration_ms = (animated.end_time - animated.start_time) as f32;
    if element_duration_ms <= 0.0 {
        return 0.0;
    }

    let fps = animated.scene_fps.max(1.0);
    (1000.0 / fps) / element_duration_ms * animated.element_speed.abs()
}

pub(super) fn apply_oscillate(
    animated: &AmAnimated,
    layer_time: f32,
    bx: &mut f32,
    by: &mut f32,
) -> f32 {
    if animated.oscillate_freq.value.is_none() && animated.oscillate_freq.keyframes.is_empty() {
        return 1.0;
    }

    let duration_sec = (animated.end_time - animated.start_time) as f32 / 1000.0;
    let accumulated_freq = accumulate_hz(
        &animated.oscillate_freq,
        layer_time,
        duration_sec,
        1.0,
        120.0,
    );

    let phase = interpolate_float(&animated.oscillate_phase, layer_time).unwrap_or(0.0);
    let mag = interpolate_float(&animated.oscillate_mag, layer_time).unwrap_or(25.0);
    let angle_deg = interpolate_float(&animated.oscillate_angle, layer_time).unwrap_or(45.0);
    let a = (90.0 - angle_deg) * std::f32::consts::PI / 180.0;
    let dx = a.sin();
    let dy = a.cos();

    let m = compute_oscillate_wave(
        animated.oscillate_wave_type,
        accumulated_freq,
        phase,
        0.0,
        0.0,
    );

    let z_offset = match animated.oscillate_direction {
        1 => mag * m,
        2 => {
            *bx += dx * mag * m;
            *by -= dy * mag * m;
            let m2 = compute_oscillate_wave(
                animated.oscillate_wave_type,
                accumulated_freq,
                phase,
                0.25,
                0.125,
            );
            mag * m2
        }
        _ => {
            *bx += dx * mag * m;
            *by -= dy * mag * m;
            0.0
        }
    };

    let zoom = compute_perspective_zoom(z_offset, animated.canvas_width, animated.canvas_height);
    *bx *= zoom;
    *by *= zoom;
    zoom
}

pub(super) fn invert_transform_component(val: f32, inv: bool) -> f32 {
    invert_if(val, inv)
}

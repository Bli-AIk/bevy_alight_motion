//! # noise_effects.rs
//!
//! # 噪声效果计算模块
//!
//! Noise-based effect computation: jitter and simplex displace.
//! 基于噪声的效果计算：抖动和随机位移。

use super::components::AmAnimated;
use super::interpolation::interpolate_float;
use super::simplex_noise::simplex_noise_3d;
use super::systems::compute_perspective_zoom;

/// Compute jitter effect displacement (simplex noise along angle direction).
/// AM algorithm: time-based displacement with angle, frequency, magnitude, seed, slack.
/// AM 算法：基于时间的位移，使用角度、频率、幅度、种子、松弛参数
pub(crate) fn compute_jitter(animated: &AmAnimated, local_time: f32) -> (f32, f32, f32) {
    // AM uses integer millisecond scene time for both the jitter script
    // and effect parameter evaluation. Truncate to integer ms to match.
    let local_time_int = (local_time as f64).floor() as f32;
    let duration = (animated.end_time - animated.start_time) as f32;
    let am_layer_time = if duration > 0.0 {
        (local_time_int - animated.start_time as f32) / duration
    } else {
        0.0
    };

    // Interpolate jitter parameters at AM's integer-ms-based layer_time
    let jitter_freq_val = interpolate_float(&animated.jitter_freq, am_layer_time).unwrap_or(0.0);
    if jitter_freq_val <= 0.0 {
        return (0.0, 0.0, 1.0);
    }

    let jitter_angle_val = interpolate_float(&animated.jitter_angle, am_layer_time).unwrap_or(0.0);
    let jitter_mag_val = interpolate_float(&animated.jitter_mag, am_layer_time).unwrap_or(0.0);
    let jitter_seed_val = interpolate_float(&animated.jitter_seed, am_layer_time).unwrap_or(0.0);
    let jitter_slack_val = interpolate_float(&animated.jitter_slack, am_layer_time).unwrap_or(0.0);
    let jitter_zjitter_val =
        interpolate_float(&animated.jitter_zjitter, am_layer_time).unwrap_or(0.0);

    // Replicate AM's f32 precision chain for globalTime
    let duration_sec = (animated.end_time - animated.start_time) as f64 / 1000.0;
    let global_time = duration_sec * (am_layer_time as f64);
    let freq = jitter_freq_val as f64;
    let t = (global_time * freq).floor() / freq;

    let a = (jitter_angle_val as f64) * (std::f64::consts::PI / 180.0);
    let seed = jitter_seed_val as f64;
    let mag = jitter_mag_val as f64;

    // Primary displacement along angle direction
    let m = simplex_noise_3d(t * 637.729, 0.0, seed * 394.417);
    let mut dx_total = (a.sin() * mag * m) as f32;
    let mut dy_total = -((a.cos() * mag * m) as f32); // Y inverted for Bevy

    // Perpendicular slack displacement
    if jitter_slack_val > 0.0 {
        let a2 = a + std::f64::consts::FRAC_PI_2;
        let m2 = simplex_noise_3d(t * 951.217 + 149.231, 0.0, seed * 894.417 + 2773.908);
        let slack = jitter_slack_val as f64;
        dx_total += (a2.sin() * mag * m2 * slack) as f32;
        dy_total -= (a2.cos() * mag * m2 * slack) as f32;
    }

    // Z-axis jitter (affects perspective zoom like oscillate)
    let mut z_zoom = 1.0_f32;
    if jitter_zjitter_val > 0.0 {
        let zm = simplex_noise_3d(t * 637.729 + 241.386, 0.0, seed * 394.417 + 1729.361);
        let z_offset = (zm * jitter_zjitter_val as f64) as f32;
        z_zoom = compute_perspective_zoom(z_offset, animated.canvas_width, animated.canvas_height);
    }

    (dx_total, dy_total, z_zoom)
}

/// Compute simplex displace effect displacement.
/// AM algorithm: uses element's position as spatial noise input,
/// `evolution` as temporal input, and `scatter` as spatial frequency.
/// AM 算法：以元素位置为空间噪声输入，evolution 为时间输入，scatter 为空间频率
pub(crate) fn compute_simplex_displace(
    animated: &AmAnimated,
    layer_time: f32,
    bx: f32,
    by: f32,
) -> (f32, f32) {
    let mag = interpolate_float(&animated.sd_mag, layer_time).unwrap_or(50.0);
    if mag.abs() < f32::EPSILON {
        return (0.0, 0.0);
    }

    let evolution = interpolate_float(&animated.sd_evolution, layer_time).unwrap_or(0.0);
    let seed = interpolate_float(&animated.sd_seed, layer_time).unwrap_or(0.0);
    let scatter = interpolate_float(&animated.sd_scatter, layer_time).unwrap_or(0.5);

    // Convert Bevy coords back to AM scene coords (top-left origin, Y-down)
    let am_x = (bx + animated.canvas_width / 2.0) as f64;
    let am_y = (animated.canvas_height / 2.0 - by) as f64;
    let scatter = scatter as f64;
    let seed = seed as f64;
    let evolution = evolution as f64;
    let mag = mag as f64;

    // AM: dx = simplexNoise(x*scatter/50 + seed*54623.245, y*scatter/500, evolution + seed*49235.319798) * mag
    let dx = simplex_noise_3d(
        am_x * scatter / 50.0 + seed * 54623.245,
        am_y * scatter / 500.0,
        evolution + seed * 49235.319798,
    ) * mag;

    // AM: dy = simplexNoise(x*scatter/50, y*scatter/500 + seed*8723.5647, evolution+7468.329 + seed*19337.940385) * mag
    let dy = simplex_noise_3d(
        am_x * scatter / 50.0,
        am_y * scatter / 500.0 + seed * 8723.5647,
        evolution + 7468.329 + seed * 19337.940385,
    ) * mag;

    // dx maps directly to Bevy X; dy is negated (AM Y-down → Bevy Y-up)
    (dx as f32, -(dy as f32))
}

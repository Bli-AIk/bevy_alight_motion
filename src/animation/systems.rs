//! # systems.rs
//!
//! # 核心系统模块
//!
//! Core animation systems for transform, opacity, and playback control.
//! Contains animate_transform_system, animate_opacity_system, advance_playback_system, etc.
//!
//! 核心动画系统，用于变换、不透明度和播放控制。
//! 包含 animate_transform_system、animate_opacity_system、advance_playback_system 等。

use bevy::prelude::*;
use std::{
    collections::HashSet,
    sync::{Mutex, OnceLock},
};

use crate::scene::{AmForceHidden, AmLayerMarker};

use super::components::{AmAnimated, AmCameraLayer, AmPlayback, AmSdfShapeParent};
use super::interpolation::{
    interpolate_float, interpolate_float_reverse, interpolate_vec2, interpolate_vec2_reverse,
    interpolate_vec3, interpolate_vec3_reverse,
};
use super::noise_effects::{compute_jitter, compute_simplex_displace};

/// Negate value when `inv` is true.
fn invert_if(val: f32, inv: bool) -> f32 {
    if inv { -val } else { val }
}

/// Accumulate Hz-style parameter over time (used by oscillate, swing, spin).
/// For non-keyed: `value * time_sec * non_keyed_factor`.
/// For keyed: numerical integration at 120 steps/sec, each step adds `value / step_divisor`.
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

/// Compute oscillate wave value (sine or triangle).
/// `sine_offset` and `triangle_offset` shift the phase for orbit mode.
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

/// Compute perspective zoom from a z-offset (shared by oscillate and jitter).
pub(super) fn compute_perspective_zoom(
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

fn trace_layer_z_once(key: impl Into<String>, message: impl FnOnce() -> String) {
    if std::env::var_os("AM_LAYER_Z_TRACE").is_none() {
        return;
    }

    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let key = key.into();

    let should_log = {
        let mut guard = seen.lock().expect("layer z trace mutex poisoned");
        guard.insert(key)
    };

    if should_log {
        bevy::log::warn!("{}", message());
    }
}

pub fn debug_layer_global_z_system(
    query: Query<(
        Entity,
        &AmLayerMarker,
        &Transform,
        &GlobalTransform,
        Option<&ChildOf>,
    )>,
) {
    for (entity, marker, transform, global_transform, child_of) in query.iter() {
        let interesting = marker.label == "编组 2"
            || marker.label == "编组 2 Copy"
            || marker.label == "Rectangle 1 Copy"
            || marker.label == "Rectangle 1 Copy 3"
            || marker.label == "Rectangle 1 Copy 2"
            || marker.label == "spr_s_boneloop_0.png Copy";
        if !interesting {
            continue;
        }

        let parent = child_of.map(|c| c.parent());
        let global = global_transform.translation();
        trace_layer_z_once(format!("{}:{}", marker.id, marker.label), || {
            format!(
                "[LAYER-Z] entity={:?} layer_id={} label='{}' parent={:?} local_z={:.6} global_z={:.6} local_xy=({:.2},{:.2}) global_xy=({:.2},{:.2})",
                entity,
                marker.id,
                marker.label,
                parent,
                transform.translation.z,
                global.z,
                transform.translation.x,
                transform.translation.y,
                global.x,
                global.y,
            )
        });
    }
}

/// Apply linear repeat displacement for SDF shapes (CPU-side, since shaders don't support it).
fn apply_sdf_linear_repeat(
    sdf_parent: Option<&AmSdfShapeParent>,
    animated: &AmAnimated,
    layer_time: f32,
    bx: &mut f32,
    by: &mut f32,
) {
    if sdf_parent.is_none() {
        return;
    }
    let Some(d) =
        super::effects::repeat::compute_sdf_linear_repeat_displacement(animated, layer_time)
    else {
        return;
    };
    if d[0].is_nan() {
        // count == 0: hide the shape (set position offscreen)
        *bx = -99999.0;
        *by = -99999.0;
    } else {
        // Apply displacement (AM coords → Bevy: negate Y)
        *bx += d[0];
        *by -= d[1];
    }
}

/// Apply oscillate effect (position oscillation + z-depth perspective).
/// Returns the z_zoom multiplier.
fn apply_oscillate(animated: &AmAnimated, layer_time: f32, bx: &mut f32, by: &mut f32) -> f32 {
    if animated.oscillate_freq.value.is_none() && animated.oscillate_freq.keyframes.is_empty() {
        return 1.0;
    }

    let duration_sec = (animated.end_time - animated.start_time) as f32 / 1000.0;

    // Hz accumulation (same pattern as swing)
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

    // AM formula: a = (90 - angle) * PI / 180
    let a = (90.0 - angle_deg) * std::f32::consts::PI / 180.0;
    let dx = a.sin();
    let dy = a.cos();

    // Wave value: sin((freq*2 + phase*2) * PI) or triangle variant
    let m = compute_oscillate_wave(
        animated.oscillate_wave_type,
        accumulated_freq,
        phase,
        0.0,
        0.0,
    );

    let z_offset = match animated.oscillate_direction {
        1 => {
            // Depth (z): AM perspective camera maps z-offset to scale + position
            mag * m
        }
        2 => {
            // Orbit: offset x/y and z (90° phase shift)
            *bx += dx * mag * m;
            *by -= dy * mag * m; // Y inverted for Bevy
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
            // Direction 0 (angle-based): offset x/y
            *bx += dx * mag * m;
            *by -= dy * mag * m; // Y inverted for Bevy
            0.0
        }
    };

    // Apply z-depth perspective zoom
    let zoom = compute_perspective_zoom(z_offset, animated.canvas_width, animated.canvas_height);
    *bx *= zoom;
    *by *= zoom;
    zoom
}

/// Apply pivot offset compensation for different layer types.
fn apply_pivot_offset(
    animated: &AmAnimated,
    layer_time: f32,
    layer_spec: &crate::scene::AmLayerSpec,
    sdf_parent: Option<&AmSdfShapeParent>,
    current_scale: [f32; 2],
    bx: &mut f32,
    by: &mut f32,
) {
    let Some(pivot) = interpolate_vec2(&animated.pivot, layer_time) else {
        return;
    };
    let pivot_x = pivot[0];
    let pivot_y = pivot[1];

    // Check if this is an SDF shape by either having AmSdfShapeParent component
    // OR having SdfShape layer spec (mask layers don't have visual but still need SDF pivot handling)
    let is_sdf_shape =
        sdf_parent.is_some() || matches!(layer_spec, crate::scene::AmLayerSpec::SdfShape { .. });

    if is_sdf_shape {
        // SDF shapes: translation is at transform center (location + pivot)
        // Simply add pivot offset (Y flip for Bevy coordinates)
        *bx += pivot_x;
        *by -= pivot_y;
    } else if matches!(
        layer_spec,
        crate::scene::AmLayerSpec::EmbedScene | crate::scene::AmLayerSpec::Null
    ) {
        // Embed scenes & null objects: need rotation-aware pivot compensation
        // In AM, objects rotate/scale around (location + pivot)
        // Bevy rotates/scales around Transform.translation
        // We calculate where the visual center ends up after rotation/scale around pivot

        // Get current rotation (including repeat offset for correct pivot compensation)
        let rotation_deg = interpolate_float(&animated.rotation, layer_time).unwrap_or(0.0);
        let rotation_rad = (-rotation_deg + animated.repeat_rotation_offset_deg).to_radians();

        // Convert pivot to Bevy Y direction
        let pivot_bevy_y = -pivot_y;

        // Object offset from rotation center is -pivot
        // After scaling
        let scaled_offset_x = -pivot_x * current_scale[0];
        let scaled_offset_y = -pivot_bevy_y * current_scale[1];

        // After rotation
        let rotated_offset_x =
            scaled_offset_x * rotation_rad.cos() - scaled_offset_y * rotation_rad.sin();
        let rotated_offset_y =
            scaled_offset_x * rotation_rad.sin() + scaled_offset_y * rotation_rad.cos();

        // Compensation: rotated_offset - original_offset = rotated_offset + pivot
        *bx += rotated_offset_x + pivot_x;
        *by += rotated_offset_y + pivot_bevy_y;
    }
    // For effect sprites: no pivot compensation needed here
    // The mesh vertices in animate_unified_effect_system include anchor offset
    // which keeps the pivot point fixed as the mesh size changes
}

/// System to advance playback time.
pub fn advance_playback_system(time: Res<Time>, mut playback: ResMut<AmPlayback>) {
    if !playback.playing {
        return;
    }

    playback.current_time_ms += time.delta_secs() * 1000.0 * playback.speed;

    if playback.current_time_ms >= playback.total_time_ms {
        if playback.looping {
            playback.current_time_ms %= playback.total_time_ms;
        } else {
            playback.current_time_ms = playback.total_time_ms;
            playback.playing = false;
        }
    }
}

/// System to animate transforms based on keyframes.
/// Only skips updates when force_stopped is true (for inspector editing).
/// Normal pause still updates animations based on current time.
/// Note: Scale animation is skipped for SDF shape parents (handled by animate_sdf_scale).
/// Note: Scale animation is skipped for UnifiedEffectMarker entities (scale baked into mesh).
pub fn animate_transform_system(
    playback: Res<AmPlayback>,
    mut query: Query<(
        &AmAnimated,
        &mut Transform,
        &AmLayerMarker,
        &crate::scene::AmLayerSpec,
        Option<&AmSdfShapeParent>,
        Option<&crate::masked_sprite::UnifiedEffectMarker>,
        Option<&crate::scene::AmEmbedContentMarker>,
    )>,
) {
    // Skip animation only when force stopped (for inspector editing)
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (
        animated,
        mut transform,
        _marker,
        layer_spec,
        sdf_parent,
        effect_marker,
        embed_content_marker,
    ) in query.iter_mut()
    {
        // Calculate local time for animation interpolation (accounting for speed)
        // Note: half-frame centering for embed content is already baked into time_offset
        // by collect_embed/spawn_embed using render_fps at each nesting level.
        // See collect_embed.rs: time_offset includes -half_frame_ms/speed per level.
        let local_time = animated.calc_local_time(global_time);
        let is_embed_content = embed_content_marker.is_some();

        // Use local time for visibility check (affected by speed)
        // This ensures child layers respect parent's speed for start/end time

        if !animated.is_active(local_time) {
            continue;
        }

        // Calculate normalized time within layer duration
        let layer_time = animated.calc_layer_time(local_time);

        // Compute frame_delta in normalized time for AM's reverseInterpolateFirstFrame.
        // This enables smooth backward extrapolation for transform properties
        // when the first keyframe is near the element's start.
        let element_duration_ms = (animated.end_time - animated.start_time) as f32;
        let frame_delta = if element_duration_ms > 0.0 {
            let fps = animated.scene_fps.max(1.0);
            (1000.0 / fps) / element_duration_ms * animated.element_speed.abs()
        } else {
            0.0
        };

        // Get current scale for pivot compensation and flip detection
        // For SDF shapes and effect sprites, magnitude is handled separately, but we need sign for flipping
        let mut actual_scale = interpolate_vec2_reverse(&animated.scale, layer_time, frame_delta)
            .unwrap_or([1.0, 1.0]);

        // Apply scale_assist effect (multiplies scale based on axis)
        // Formula derived from reference video analysis:
        //   axis=1 (Y only): scale_y *= scale_param
        //   axis=2 (X only): scale_x *= scale_param
        //   axis=3 (Both):   scale_x *= scale_param
        //                    scale_y /= (scale_param^SCALE_POWER * damp_factor)
        //                    where damp_factor = damp^(1 + DAMP_COEFF*(damp-1)^DAMP_POWER)
        if animated.scale_assist_axis != 0
            && let Some(scale_param) = crate::animation::interpolation::interpolate_float(
                &animated.scale_assist,
                layer_time,
            )
        {
            // Get damp value (defaults to 1.0)
            let damp_param = crate::animation::interpolation::interpolate_float(
                &animated.scale_assist_damp,
                layer_time,
            )
            .unwrap_or(1.0);

            // Constants derived from empirical analysis of AM reference videos
            // Must match effects.rs for consistent scale calculations
            const SCALE_POWER: f32 = 1.71; // = ln(2) / ln(1.501), makes scale_y=0.5 when scale_param=1.501
            const DAMP_COEFF: f32 = 2.75;
            const DAMP_POWER: f32 = 1.93;

            match animated.scale_assist_axis {
                1 => {
                    // Y only (vertical stretch)
                    actual_scale[1] *= scale_param;
                }
                2 => {
                    // X only (horizontal stretch)
                    actual_scale[0] *= scale_param;
                }
                3 => {
                    // Both axes - X stretches, Y compresses
                    // This creates the characteristic "line stretch" effect
                    let damp_exp = 1.0 + DAMP_COEFF * (damp_param - 1.0).powf(DAMP_POWER);
                    let damp_factor = damp_param.powf(damp_exp);
                    let scale_divisor = scale_param.powf(SCALE_POWER) * damp_factor;
                    actual_scale[0] *= scale_param;
                    actual_scale[1] /= scale_divisor;
                }
                _ => {}
            }
        }

        // Apply transform2 posz as additive offset from identity (1.0).
        // Stacked transform2 effects contribute additively: combined = 1.0 + Σ(posz_i - 1.0)
        let mut posz_offset = 0.0_f32;
        if let Some(mut posz) = interpolate_float(&animated.effect_posz, layer_time) {
            if animated.effect_zinv {
                posz = 2.0 - posz;
            }
            posz_offset += posz - 1.0;
        }
        for extra in &animated.extra_transform2 {
            let Some(mut posz) = interpolate_float(&extra.pos_z, layer_time) else {
                continue;
            };
            if extra.zinv {
                posz = 2.0 - posz;
            }
            posz_offset += posz - 1.0;
        }
        let combined_posz = 1.0 + posz_offset;
        actual_scale[0] *= combined_posz;
        actual_scale[1] *= combined_posz;

        let current_scale = if sdf_parent.is_some() || effect_marker.is_some() || is_embed_content {
            [1.0_f32, 1.0_f32]
        } else {
            actual_scale
        };

        // Interpolate location and convert from AM to Bevy coordinates.
        // Child SDF layers such as mask/frame helpers often omit `location` and rely on a pivot-only
        // local transform. Rebuild them from (0,0) every frame so later SDF ancestor-scale
        // compensation does not keep multiplying the previous frame's already-compensated position.
        let mut oscillate_z_zoom = 1.0_f32;
        let loc =
            interpolate_vec3_reverse(&animated.location, layer_time, frame_delta).or_else(|| {
                if animated.has_parent
                    && sdf_parent.is_none()
                    && matches!(layer_spec, crate::scene::AmLayerSpec::SdfShape { .. })
                {
                    Some([0.0, 0.0, 0.0])
                } else {
                    None
                }
            });
        if let Some(loc) = loc {
            let (mut bx, mut by) = if animated.has_parent {
                // For layers with parents, use local coordinates
                // Only flip Y axis (AM Y-down -> Bevy Y-up)
                (loc[0], -loc[1])
            } else {
                // For root layers, convert from canvas coordinates
                // AM: Origin at top-left, Y increases downward
                // Bevy: Origin at center, Y increases upward
                (
                    loc[0] - animated.canvas_width / 2.0,
                    animated.canvas_height / 2.0 - loc[1],
                )
            };

            // Apply RTT alignment correction for embed content at animation start
            // This corrects a small positioning offset that appears in early animation frames
            // DISABLED: This correction was causing early frames to shift left, which increased
            // the position mismatch with reference videos. Analysis showed shot frames 0-2 were
            // already 6px left of their correct position.
            // if embed_content_marker.is_some() && layer_time < 0.02 {
            //     bx -= 5.0;
            // }

            // Debug: log position calculation for specific layers (trace level)
            if animated.layer_id == 347000343 {
                trace!(
                    "[PosCalc] layer={} is_embed_content={} speed_mul={:.2} time_offset={} | global_time={:.1} local_time={:.1} layer_time={:.4} | AM_loc=({:.2},{:.2}) canvas=({:.0},{:.0}) has_parent={} | bevy=({:.2},{:.2})",
                    animated.layer_id,
                    embed_content_marker.is_some(),
                    animated.speed_multiplier,
                    animated.time_offset,
                    global_time,
                    local_time,
                    layer_time,
                    loc[0],
                    loc[1],
                    animated.canvas_width,
                    animated.canvas_height,
                    animated.has_parent,
                    bx,
                    by
                );
            }

            apply_pivot_offset(
                animated,
                layer_time,
                layer_spec,
                sdf_parent,
                current_scale,
                &mut bx,
                &mut by,
            );

            // Apply effect position offsets (transform2 effect)
            if let Some(effect_x) = interpolate_float(&animated.effect_pos_x, layer_time) {
                bx += invert_if(effect_x, animated.effect_xinv);
            }
            if let Some(effect_y) = interpolate_float(&animated.effect_pos_y, layer_time) {
                by -= invert_if(effect_y, animated.effect_yinv); // Y is inverted
            }
            // Apply extra stacked transform2 position offsets
            for extra in &animated.extra_transform2 {
                bx += interpolate_float(&extra.pos_x, layer_time)
                    .map(|x| invert_if(x, extra.xinv))
                    .unwrap_or(0.0);
                by -= interpolate_float(&extra.pos_y, layer_time)
                    .map(|y| invert_if(y, extra.yinv))
                    .unwrap_or(0.0);
            }

            // Apply font Y offset for text layers (to compensate for different font metrics)
            // Only apply to root text layers; child text inherits offset from parent
            if !animated.has_parent {
                by -= animated.font_y_offset;
            }

            // Compensate for cosmic-text vs Android StaticLayout horizontal glyph positioning.
            // Android renders text to a bitmap with integer-pixel glyph positions, while
            // cosmic-text uses sub-pixel positioning that results in a consistent ~1px rightward
            // shift relative to AM's rendering. Apply correction scaled by inv_fit_scale to
            // get exactly 1 screen pixel adjustment regardless of scene resolution.
            if matches!(layer_spec, crate::scene::AmLayerSpec::Text { .. }) {
                bx -= animated.inv_fit_scale;
            }

            // Apply anchor offset compensation for SpriteShape with non-center pivot.
            // This keeps the sprite center at the AM location while pivot affects rotation/scale.
            // NOTE: Skip for SDF shapes - their pivot is already handled above via `by -= pivot_y`
            // Check layer_spec instead of sdf_parent because mask layers don't have visual but need SDF handling
            if !matches!(layer_spec, crate::scene::AmLayerSpec::SdfShape { .. })
                && sdf_parent.is_none()
            {
                bx += animated.anchor_offset.x;
                by += animated.anchor_offset.y;
            }

            // For embed content: coordinates are already in embed's internal canvas space
            // They render to RTT camera at origin - no scaling needed for position
            // The inv_fit_scale is only used for sprite SIZE compensation, not position

            // Apply oscillate effect (position oscillation + z-depth perspective)
            oscillate_z_zoom = apply_oscillate(animated, layer_time, &mut bx, &mut by);

            // Apply jitter effect (simplex noise-based position displacement)
            if animated.jitter_enabled {
                let (jdx, jdy, jz) = compute_jitter(animated, local_time);
                bx = (bx + jdx) * jz;
                by = (by + jdy) * jz;
                oscillate_z_zoom *= jz;
            }

            // Apply simplex displace effect (spatially-varying noise displacement)
            if animated.sd_enabled {
                let (sdx, sdy) = compute_simplex_displace(animated, layer_time, bx, by);
                bx += sdx;
                by += sdy;
            }

            // Apply repeat group position offset (accumulated per-copy offset)
            bx += animated.repeat_position_offset.x;
            by += animated.repeat_position_offset.y;

            // Apply linear repeat (repeat.line) displacement for SDF shapes.
            // SDF shaders don't support repeat.line, so we compute it CPU-side.
            apply_sdf_linear_repeat(sdf_parent, animated, layer_time, &mut bx, &mut by);

            transform.translation = Vec3::new(bx, by, transform.translation.z);
        }

        // Interpolate rotation (negate for Bevy's coordinate system)
        // Get base rotation - default to 0 if not animated
        let base_rotation =
            interpolate_float_reverse(&animated.rotation, layer_time, frame_delta).unwrap_or(0.0);
        let mut final_rotation = -base_rotation; // Negate for Bevy's coordinate system

        // Apply swing effect (oscillating rotation)
        // AM swing2: freq is Hz-accumulated (integral of freq over time),
        // sine uses sin((accum + phase) * π), triangle uses AM.triangle((accum + phase) / 2)
        if let Some(swing_freq) = interpolate_float(&animated.swing_freq, layer_time)
            && swing_freq > 0.0
        {
            let swing_a1 = interpolate_float(&animated.swing_a1, layer_time).unwrap_or(0.0);
            let swing_a2 = interpolate_float(&animated.swing_a2, layer_time).unwrap_or(0.0);
            let swing_phase = interpolate_float(&animated.swing_phase, layer_time).unwrap_or(0.0);

            // Hz accumulation: AM integrates freq over time for Hz-type parameters
            let duration_sec = (animated.end_time - animated.start_time) as f32 / 1000.0;
            let accumulated_freq =
                accumulate_hz(&animated.swing_freq, layer_time, duration_sec, 1.0, 120.0);

            // Waveform: AM script formula
            let wave_value = match animated.swing_type {
                0 => {
                    // Sine: sin((accumulated_freq + phase) * π)
                    ((accumulated_freq + swing_phase) * std::f32::consts::PI).sin()
                }
                1 => {
                    // Triangle: AM.triangle((accumulated_freq + phase) / 2.0)
                    // AM.triangle(x) = abs(((x + 0.75) % 1.0) - 0.5) * 4 - 1
                    let x = (accumulated_freq + swing_phase) / 2.0;
                    let x_mod = ((x + 0.75).rem_euclid(1.0)) - 0.5;
                    x_mod.abs() * 4.0 - 1.0
                }
                _ => ((accumulated_freq + swing_phase) * std::f32::consts::PI).sin(),
            };

            // AM angle formula: ((a2 - a1) * ((m + 1) / 2)) + a1
            let swing_angle = ((swing_a2 - swing_a1) * ((wave_value + 1.0) / 2.0)) + swing_a1;

            // Add swing angle to base rotation (swing is additive)
            // Negate for Bevy's coordinate system (like base rotation)
            final_rotation -= swing_angle;
        }

        // Apply spin effect (RPM-based continuous rotation)
        // AM spin: rpm is accumulated like Hz, but each step adds rpm/20 (degrees)
        // Non-keyed: accumulated = rpm * time_seconds * 6.0
        if animated.spin_rpm.value.is_some() || !animated.spin_rpm.keyframes.is_empty() {
            let duration_sec = (animated.end_time - animated.start_time) as f32 / 1000.0;
            let spin_angle = accumulate_hz(&animated.spin_rpm, layer_time, duration_sec, 6.0, 20.0);
            // Negate for Bevy's coordinate system
            final_rotation -= spin_angle;
        }

        // Apply transform2 effect angle (additional rotation in degrees)
        if let Some(effect_angle) = interpolate_float(&animated.effect_angle, layer_time) {
            final_rotation -= invert_if(effect_angle, animated.effect_ainv); // Negate for Bevy's coordinate system
        }
        // Apply extra stacked transform2 angles
        for extra in &animated.extra_transform2 {
            let Some(ea) = interpolate_float(&extra.angle, layer_time) else {
                continue;
            };
            final_rotation -= invert_if(ea, extra.ainv);
        }

        // Apply repeat group rotation offset (accumulated per-copy)
        final_rotation += animated.repeat_rotation_offset_deg;

        transform.rotation = Quat::from_rotation_z(final_rotation.to_radians());

        // Interpolate scale
        // Skip for SDF shapes (handled by animate_sdf_scale)
        // For effect sprites: magnitude is baked into mesh, but sign (flip) needs Transform
        // However, transform2 posz/angle/position must also be applied via Transform
        if sdf_parent.is_none() && effect_marker.is_none() && !is_embed_content {
            transform.scale = Vec3::new(
                current_scale[0] * oscillate_z_zoom * animated.repeat_scale_factor,
                current_scale[1] * oscillate_z_zoom * animated.repeat_scale_factor,
                1.0,
            );
        } else if effect_marker.is_some() || is_embed_content {
            // Effect sprites and embed content: base scale magnitude is baked into mesh.
            // But transform2 effects (posz) still need to be applied via Transform.scale,
            // since the unified effect system doesn't know about transform2.
            let sign_x = actual_scale[0].signum();
            let sign_y = actual_scale[1].signum();

            // Use the already-computed combined_posz (additive across stacked effects)
            transform.scale = Vec3::new(
                sign_x * combined_posz * oscillate_z_zoom,
                sign_y * combined_posz * oscillate_z_zoom,
                1.0,
            );
        }
    }
}

/// System to animate sprite opacity.
/// Only skips updates when force_stopped is true (for inspector editing).
pub fn animate_opacity_system(
    playback: Res<AmPlayback>,
    mut query: Query<(&AmAnimated, &mut Sprite)>,
) {
    // Skip animation only when force stopped (for inspector editing)
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (animated, mut sprite) in query.iter_mut() {
        // Use local time for visibility check (affected by speed)
        let local_time = animated.calc_local_time(global_time);
        if !animated.is_active(local_time) {
            sprite.color.set_alpha(0.0);
            continue;
        }

        // Use animation local time for interpolation
        let layer_time = animated.calc_layer_time(local_time);

        // Get opacity from animation data, default to 1.0 if not specified
        let opacity = interpolate_float(&animated.opacity, layer_time).unwrap_or(1.0);
        // Multiply by base_alpha to preserve original fill color transparency
        // e.g., if fillColor has alpha=0, the sprite should remain invisible regardless of opacity animation
        let mut final_alpha = (opacity * animated.base_alpha).clamp(0.0, 1.0);
        // Apply fade effect (fade in/out)
        final_alpha *= animated.calc_fade_alpha(layer_time);
        // Apply echo alpha (for echokf effect)
        if let Some(ref echo_cfg) = animated.echo_alpha_config {
            final_alpha *= echo_cfg.evaluate(global_time);
        }
        // AM composites opacity in sRGB space; Bevy blends in linear space.
        // Convert alpha sRGB→linear so GPU blend approximates AM's result.
        let corrected = if final_alpha > 0.001 && final_alpha < 0.999 {
            if final_alpha <= 0.04045 {
                final_alpha / 12.92
            } else {
                ((final_alpha + 0.055) / 1.055).powf(2.4)
            }
        } else {
            final_alpha
        };
        sprite.color.set_alpha(corrected);
    }
}

/// System to animate text opacity (handles Text2d entities).
/// Uses Visibility component for proper show/hide behavior and TextColor alpha for opacity animation.
/// Only skips updates when force_stopped is true (for inspector editing).
pub fn animate_text_opacity_system(
    playback: Res<AmPlayback>,
    mut query: Query<
        (
            &AmAnimated,
            &mut bevy::text::TextColor,
            &mut Visibility,
            &AmLayerMarker,
            Option<&AmForceHidden>,
        ),
        With<Text2d>,
    >,
) {
    // Skip animation only when force stopped (for inspector editing)
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;
    let text_count = query.iter().count();

    // Debug: log count of text entities being processed (only occasionally to avoid spam)
    static mut FRAME_COUNT: u32 = 0;
    unsafe {
        FRAME_COUNT += 1;
        if FRAME_COUNT % 300 == 1 {
            bevy::log::trace!(
                "[TEXT] Processing {} text entities at time={:.0}",
                text_count,
                global_time
            );
        }
    }

    for (animated, mut text_color, mut visibility, marker, force_hidden) in query.iter_mut() {
        // Use local time for visibility check (affected by speed)
        let local_time = animated.calc_local_time(global_time);

        // Check if layer is active
        if !animated.is_active(local_time) || force_hidden.is_some() {
            // Hide text when outside its time range
            if force_hidden.is_none() && *visibility != Visibility::Hidden {
                bevy::log::trace!(
                    "[TEXT] Hiding '{}' (id={}): time={:.0}, range=[{}, {}]",
                    marker.label,
                    marker.id,
                    local_time,
                    animated.start_time,
                    animated.end_time
                );
            }
            *visibility = Visibility::Hidden;
            text_color.0.set_alpha(0.0);
            continue;
        }

        // Show text when within its time range
        if *visibility == Visibility::Hidden {
            bevy::log::trace!(
                "[TEXT] Showing '{}' (id={}): time={:.0}, range=[{}, {}]",
                marker.label,
                marker.id,
                local_time,
                animated.start_time,
                animated.end_time
            );
        }
        *visibility = Visibility::Inherited;

        let layer_time = animated.calc_layer_time(local_time);

        // Get opacity from keyframes, or default to 1.0 if no opacity animation
        let opacity = interpolate_float(&animated.opacity, layer_time).unwrap_or(1.0);
        // Multiply by base_alpha to preserve original fill color transparency
        let mut final_alpha = opacity * animated.base_alpha;
        // Apply fade effect (fade in/out)
        final_alpha *= animated.calc_fade_alpha(layer_time);
        // Apply echo alpha (for echokf effect)
        if let Some(ref echo_cfg) = animated.echo_alpha_config {
            final_alpha *= echo_cfg.evaluate(global_time);
        }
        text_color.0.set_alpha(final_alpha.clamp(0.0, 1.0));
    }
}

/// System to animate shape size based on size property keyframes.
///
/// AM shapes have a `size` property (in properties list) that can be animated.
/// This is separate from scale animation - size changes the base dimensions
/// while scale is applied on top.
///
/// For SDF shapes: Updates SdfMaterial.params with new half-width/half-height.
/// For Sprite shapes: Updates Sprite.custom_size.
pub fn animate_size_system(
    playback: Res<AmPlayback>,
    // SDF shapes: parent entity has AmSdfShapeParent marker, child has SdfMaterial
    parent_query: Query<(&AmAnimated, &Children), With<AmSdfShapeParent>>,
    mut sdf_query: Query<(
        &bevy::prelude::MeshMaterial2d<crate::sdf_material::SdfMaterial>,
        &mut super::components::AmSdfParams,
    )>,
    mut materials: ResMut<Assets<crate::sdf_material::SdfMaterial>>,
    // Sprite shapes: entity has Sprite component directly
    mut sprite_query: Query<(&AmAnimated, &mut Sprite), Without<AmSdfShapeParent>>,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    // Handle SDF shapes (size is on parent, SdfMaterial is on child)
    for (animated, children) in parent_query.iter() {
        // Skip if no size animation
        if animated.size.keyframes.is_empty() && animated.size.value.is_none() {
            continue;
        }

        // Use local time for visibility check (affected by speed)
        let local_time = animated.calc_local_time(global_time);

        // Skip if outside active time range
        if !animated.is_active(local_time) {
            continue;
        }

        // Use animation local time for interpolation
        let layer_time = animated.calc_layer_time(local_time);

        // Interpolate size (already stored as full dimensions, need half for SDF)
        let Some(size) = interpolate_vec2(&animated.size, layer_time) else {
            continue;
        };
        let half_width = size[0].abs() / 2.0;
        let half_height = size[1].abs() / 2.0;

        // Update SDF children
        for child in children.iter() {
            let Ok((material_handle, mut sdf_params)) = sdf_query.get_mut(child) else {
                continue;
            };
            // Update base params (these will be further modified by scale in animate_sdf_scale)
            sdf_params.base_half_width = half_width;
            sdf_params.base_half_height = half_height;

            // Also update the actual material params directly
            // (animate_sdf_scale will run after and apply scale on top if needed)
            let Some(material) = materials.get_mut(&material_handle.0) else {
                continue;
            };
            material.uniform_data.params.x = half_width;
            material.uniform_data.params.y = half_height;
        }
    }

    // Handle Sprite shapes
    for (animated, mut sprite) in sprite_query.iter_mut() {
        // Skip if no size animation
        if animated.size.keyframes.is_empty() && animated.size.value.is_none() {
            continue;
        }

        // Use local time for visibility check (affected by speed)
        let local_time = animated.calc_local_time(global_time);

        // Skip if outside active time range
        if !animated.is_active(local_time) {
            continue;
        }

        // Use animation local time for interpolation
        let layer_time = animated.calc_layer_time(local_time);

        // Interpolate size (full dimensions for Sprite)
        if let Some(size) = interpolate_vec2(&animated.size, layer_time) {
            // Use original size - no scaling needed
            // For embed content, the final display size is affected by embed's inherited fit_scale
            sprite.custom_size = Some(Vec2::new(size[0].abs(), size[1].abs()));
        }
    }
}

/// Animate the Bevy Camera2d based on AM camera layer data.
/// Reads camera location/rotation/FOV and computes 2D pan, zoom, and rotation.
pub fn animate_am_camera_system(
    playback: Res<AmPlayback>,
    camera_query: Query<(&AmAnimated, &AmCameraLayer)>,
    pending_query: Query<&crate::scene::AmPendingLayers>,
    mut bevy_camera_query: Query<
        (&mut Transform, &mut Projection),
        (
            With<Camera2d>,
            Without<crate::effects::EmbedSceneRttCamera>,
            Without<crate::effects::LiftCompositeCameraMarker>,
        ),
    >,
) {
    if playback.force_stopped {
        return;
    }
    let global_time = playback.current_time_ms;

    for (animated, cam) in camera_query.iter() {
        let local_time = animated.calc_local_time(global_time);
        if !animated.is_active(local_time) {
            continue;
        }
        let layer_time = animated.calc_layer_time(local_time);

        // Interpolate camera location in AM coords
        let default_loc = [cam.scene_width / 2.0, cam.scene_height / 2.0, cam.base_z];
        let loc = interpolate_vec3(&animated.location, layer_time).unwrap_or(default_loc);

        // Interpolate rotation (degrees, clockwise positive in AM)
        let rotation_deg = interpolate_float(&animated.rotation, layer_time).unwrap_or(0.0);

        // Interpolate FOV (degrees)
        let fov_deg = interpolate_float(&cam.fov, layer_time).unwrap_or(60.0);

        // Convert pan from AM coords to Bevy coords
        let pan_x = loc[0] - cam.scene_width / 2.0;
        let pan_y = cam.scene_height / 2.0 - loc[1];

        // Compute zoom factor:
        // visible_half_w = |z| * tan(fov/2)
        // base_visible_half_w = |base_z| * tan(base_fov/2) = scene_width/2
        let base_fov_rad = 60.0_f32.to_radians();
        let current_fov_rad = fov_deg.to_radians();
        let z_abs = loc[2].abs();
        let base_z_abs = cam.base_z.abs();
        let zoom =
            (z_abs * (current_fov_rad / 2.0).tan()) / (base_z_abs * (base_fov_rad / 2.0).tan());

        // Get fit_scale from pending layers
        let fit_scale = pending_query
            .iter()
            .next()
            .map(|p| 1.0 / p.inv_fit_scale)
            .unwrap_or(1.0);

        // Apply to Bevy camera
        for (mut transform, mut projection) in bevy_camera_query.iter_mut() {
            transform.translation.x = pan_x * fit_scale;
            transform.translation.y = pan_y * fit_scale;
            // AM clockwise → Bevy counter-clockwise
            transform.rotation = Quat::from_rotation_z(-rotation_deg.to_radians());

            if let Projection::Orthographic(ref mut ortho) = *projection {
                ortho.scale = zoom;
            }
        }
    }
}

/// Runtime system for updating echokf echo entities with dynamic (keyframed) parameters.
/// Evaluates count/seconds/alpha keyframes per frame and updates echo time shifts and visibility.
/// Propagates updated values to all descendant entities in each echo subtree.
pub fn update_echo_runtime_system(
    playback: Res<AmPlayback>,
    mut echo_query: Query<(
        Entity,
        &super::components::AmEchoRuntime,
        &mut AmAnimated,
        &mut Visibility,
        Option<&AmForceHidden>,
    )>,
    children_query: Query<&Children>,
    mut child_animated_query: Query<&mut AmAnimated, Without<super::components::AmEchoRuntime>>,
) {
    for (entity, echo_rt, mut animated, mut visibility, force_hidden) in echo_query.iter_mut() {
        if force_hidden.is_some() {
            *visibility = Visibility::Hidden;
            continue;
        }

        // Compute parent element's fractional time (0-1)
        let global_time = playback.current_time_ms;
        let parent_local = (global_time - echo_rt.embed_time_offset) * echo_rt.embed_speed;
        let parent_duration = echo_rt.embed_end - echo_rt.embed_start;
        let frac_t = if parent_duration > 0.0 {
            ((parent_local - echo_rt.embed_start) / parent_duration).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Evaluate keyframed count
        let current_count = interpolate_float(&echo_rt.count_kf, frac_t)
            .unwrap_or(1.0)
            .round() as u32;

        // Hide echoes beyond current count
        if echo_rt.echo_index >= current_count {
            *visibility = Visibility::Hidden;
            continue;
        } else {
            *visibility = Visibility::Inherited;
        }

        // Evaluate keyframed seconds
        let current_seconds = interpolate_float(&echo_rt.seconds_kf, frac_t).unwrap_or(0.5);

        // Compute echo fraction and time shift based on current count
        let r0 = if current_count > 0 {
            echo_rt.echo_index as f32 / current_count as f32
        } else {
            0.0
        };

        let time_shift_ms = (1.0 - r0) * current_seconds * 1000.0;

        // Update root entity
        animated.echo_time_shift_ms = time_shift_ms;

        // Evaluate keyframed alpha and build echo_alpha_config
        let current_alpha = interpolate_float(&echo_rt.alpha_kf, frac_t).unwrap_or(1.0);
        let mix = current_alpha * (1.0 - r0) + r0;
        let echo_cfg = super::components::EchoAlphaConfig {
            alpha_keyframes: crate::schema::AmAnimatedFloat {
                value: Some(mix),
                keyframes: Vec::new(),
            },
            fraction: 0.0, // Already computed into mix
            parent_start: echo_rt.embed_start as i32,
            parent_end: echo_rt.embed_end as i32,
            parent_time_offset: echo_rt.embed_time_offset,
            parent_speed: echo_rt.embed_speed,
        };
        animated.echo_alpha_config = Some(echo_cfg.clone());

        // Propagate to all descendant entities in the echo subtree
        propagate_echo_to_descendants(
            entity,
            time_shift_ms,
            &echo_cfg,
            &children_query,
            &mut child_animated_query,
        );
    }
}

/// Recursively propagate echo_time_shift_ms and echo_alpha_config to all descendants.
fn propagate_echo_to_descendants(
    parent: Entity,
    time_shift_ms: f32,
    echo_cfg: &super::components::EchoAlphaConfig,
    children_query: &Query<&Children>,
    child_animated_query: &mut Query<&mut AmAnimated, Without<super::components::AmEchoRuntime>>,
) {
    let Ok(children) = children_query.get(parent) else {
        return;
    };
    for child in children.iter() {
        if let Ok(mut child_animated) = child_animated_query.get_mut(child) {
            child_animated.echo_time_shift_ms = time_shift_ms;
            child_animated.echo_alpha_config = Some(echo_cfg.clone());
        }
        // Recurse into grandchildren
        propagate_echo_to_descendants(
            child,
            time_shift_ms,
            echo_cfg,
            children_query,
            child_animated_query,
        );
    }
}

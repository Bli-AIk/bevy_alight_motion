//! # systems.rs
//!
//! # 核心系统模块
//!
//! Core animation systems for transform, opacity, and playback control.
//! Contains animate_transform_system, animate_opacity_system, advance_playback_system, etc.
//!
//! 核心动画系统，用于变换、不透明度和播放控制。
//! 包含 animate_transform_system、animate_opacity_system、advance_playback_system 等。

use bevy::math::EulerRot;
use bevy::prelude::*;

use crate::scene::AmLayerMarker;

use super::components::{AmAnimated, AmCameraLayer, AmPlayback, AmSdfShapeParent};
use super::interpolation::{
    interpolate_float, interpolate_vec2, interpolate_vec3, interpolate_vec3_with_extrapolation,
};

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
#[allow(clippy::type_complexity)]
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
        let mut local_time = animated.calc_local_time(global_time);

        // For embed content, add 0.5 frame offset to match AM's internal timing
        // This compensates for the difference between video frame edges and centers
        // Note: only apply offset when animation is not frozen (speed_multiplier != 0)
        let is_embed_content = embed_content_marker.is_some();
        if is_embed_content && animated.speed_multiplier != 0.0 {
            let frame_duration_ms = 1000.0 / 30.0; // Assuming 30fps
            let offset = frame_duration_ms * 0.35;
            local_time += offset;
        }

        // Use local time for visibility check (affected by speed)
        // This ensures child layers respect parent's speed for start/end time
        if !animated.is_active(local_time) {
            continue;
        }

        // Calculate normalized time within layer duration
        let layer_time = animated.calc_layer_time(local_time);

        // Get current scale for pivot compensation and flip detection
        // For SDF shapes and effect sprites, magnitude is handled separately, but we need sign for flipping
        let mut actual_scale = interpolate_vec2(&animated.scale, layer_time).unwrap_or([1.0, 1.0]);

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

        let current_scale = if sdf_parent.is_some() || effect_marker.is_some() {
            [1.0_f32, 1.0_f32]
        } else {
            actual_scale
        };

        // Interpolate location and convert from AM to Bevy coordinates
        // Use extrapolation for location to improve accuracy before first keyframe
        let mut oscillate_z_zoom = 1.0_f32;
        if let Some(loc) = interpolate_vec3_with_extrapolation(&animated.location, layer_time) {
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

            // Apply pivot offset and compensation
            // AM transforms around (location + pivot), Bevy transforms around entity origin
            // For SDF shapes: translation should be at transform center (location + pivot)
            // For embed scenes: need full rotation-aware pivot compensation
            // For effect sprites (scale_assist): skip pivot compensation - mesh anchor offset handles it
            // For other shapes: need pivot compensation for non-unit scale
            if let Some(pivot) = interpolate_vec2(&animated.pivot, layer_time) {
                let pivot_x = pivot[0];
                let pivot_y = pivot[1];

                // Check if this is an SDF shape by either having AmSdfShapeParent component
                // OR having SdfShape layer spec (mask layers don't have visual but still need SDF pivot handling)
                let is_sdf_shape = sdf_parent.is_some()
                    || matches!(layer_spec, crate::scene::AmLayerSpec::SdfShape { .. });

                if is_sdf_shape {
                    // SDF shapes: translation is at transform center (location + pivot)
                    // Simply add pivot offset (Y flip for Bevy coordinates)
                    bx += pivot_x;
                    by -= pivot_y;
                } else if matches!(layer_spec, crate::scene::AmLayerSpec::EmbedScene) {
                    // Embed scenes: need rotation-aware pivot compensation
                    // In AM, objects rotate/scale around (location + pivot)
                    // Bevy rotates/scales around Transform.translation
                    // We calculate where the visual center ends up after rotation/scale around pivot

                    // Get current rotation
                    let rotation_deg =
                        interpolate_float(&animated.rotation, layer_time).unwrap_or(0.0);
                    let rotation_rad = (-rotation_deg).to_radians(); // Bevy uses opposite rotation direction

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
                    bx += rotated_offset_x + pivot_x;
                    by += rotated_offset_y + pivot_bevy_y;
                } else if effect_marker.is_none() {
                    // Standard shapes: pivot offset is already applied in the initial transform
                    // No additional compensation needed here
                }
                // For effect sprites: no pivot compensation needed here
                // The mesh vertices in animate_unified_effect_system include anchor offset
                // which keeps the pivot point fixed as the mesh size changes
            }

            // Apply effect position offsets (transform2 effect)
            if let Some(effect_x) = interpolate_float(&animated.effect_pos_x, layer_time) {
                bx += effect_x;
            }
            if let Some(effect_y) = interpolate_float(&animated.effect_pos_y, layer_time) {
                by -= effect_y; // Y is inverted
            }

            // Apply font Y offset for text layers (to compensate for different font metrics)
            // Only apply to root text layers; child text inherits offset from parent
            if !animated.has_parent {
                by -= animated.font_y_offset;
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

            // Apply oscillate effect (position oscillation)
            // AM oscillate3: freq is Hz-accumulated, applies positional offset
            let mut oscillate_z_offset: f32 = 0.0;
            if animated.oscillate_freq.value.is_some()
                || !animated.oscillate_freq.keyframes.is_empty()
            {
                let duration_sec = (animated.end_time - animated.start_time) as f32 / 1000.0;
                let time_sec = layer_time * duration_sec;

                // Hz accumulation (same pattern as swing)
                let accumulated_freq = if animated.oscillate_freq.keyframes.is_empty() {
                    let freq =
                        interpolate_float(&animated.oscillate_freq, layer_time).unwrap_or(0.0);
                    freq * time_sec
                } else {
                    let total_steps = (duration_sec * 120.0).round() as i32;
                    let current_step = (120.0 * time_sec).round() as i32;
                    let mut accum = 0.0f64;
                    if total_steps > 0 {
                        for i in 0..=current_step.min(total_steps) {
                            let frac_t = i as f32 / total_steps as f32;
                            let freq_at_t =
                                interpolate_float(&animated.oscillate_freq, frac_t).unwrap_or(0.0);
                            accum += freq_at_t as f64 / 120.0;
                        }
                    }
                    accum as f32
                };

                let phase = interpolate_float(&animated.oscillate_phase, layer_time).unwrap_or(0.0);
                let mag = interpolate_float(&animated.oscillate_mag, layer_time).unwrap_or(25.0);
                let angle_deg =
                    interpolate_float(&animated.oscillate_angle, layer_time).unwrap_or(45.0);

                // AM formula: a = (90 - angle) * PI / 180
                let a = (90.0 - angle_deg) * std::f32::consts::PI / 180.0;
                let dx = a.sin();
                let dy = a.cos();

                // Wave value: sin((freq*2 + phase*2) * PI) or triangle variant
                let m = match animated.oscillate_wave_type {
                    0 => ((accumulated_freq * 2.0 + phase * 2.0) * std::f32::consts::PI).sin(),
                    1 => {
                        // AM.triangle((freq*2 + phase*2) / 2 + phase)
                        let x = (accumulated_freq * 2.0 + phase * 2.0) / 2.0 + phase;
                        let x_mod = ((x + 0.75).rem_euclid(1.0)) - 0.5;
                        x_mod.abs() * 4.0 - 1.0
                    }
                    _ => ((accumulated_freq * 2.0 + phase * 2.0) * std::f32::consts::PI).sin(),
                };

                match animated.oscillate_direction {
                    1 => {
                        // Depth (z): AM perspective camera maps z-offset to scale + position
                        oscillate_z_offset = mag * m;
                    }
                    2 => {
                        // Orbit: offset x/y and z (90° phase shift)
                        bx += dx * mag * m;
                        by -= dy * mag * m; // Y inverted for Bevy

                        let m2 = match animated.oscillate_wave_type {
                            0 => ((accumulated_freq * 2.0 + (phase + 0.25) * 2.0)
                                * std::f32::consts::PI)
                                .sin(),
                            1 => {
                                let x = (accumulated_freq * 2.0 + (phase + 0.125) * 2.0) / 2.0
                                    + phase
                                    + 0.125;
                                let x_mod = ((x + 0.75).rem_euclid(1.0)) - 0.5;
                                x_mod.abs() * 4.0 - 1.0
                            }
                            _ => ((accumulated_freq * 2.0 + (phase + 0.25) * 2.0)
                                * std::f32::consts::PI)
                                .sin(),
                        };
                        oscillate_z_offset = mag * m2;
                    }
                    _ => {
                        // Direction 0 (angle-based): offset x/y
                        bx += dx * mag * m;
                        by -= dy * mag * m; // Y inverted for Bevy
                    }
                }
            }

            // Apply z-depth perspective effect from oscillate direction=1/2
            // AM default camera: Perspective, FOV=60°, at scene center z=-base_cam_dist
            // zoom = base_cam_dist / (base_cam_dist + z_offset)
            // Position scales from center, element scale multiplied by zoom
            oscillate_z_zoom = if oscillate_z_offset != 0.0 {
                let cam_dist = animated.canvas_width.max(animated.canvas_height)
                    / (2.0 * (30.0_f32).to_radians().tan());
                let denom = cam_dist + oscillate_z_offset;
                if denom > 0.0 {
                    let zoom = cam_dist / denom;
                    bx *= zoom;
                    by *= zoom;
                    zoom
                } else {
                    0.001 // element behind camera, nearly invisible
                }
            } else {
                1.0
            };

            transform.translation = Vec3::new(bx, by, transform.translation.z);
        }

        // Interpolate rotation (negate for Bevy's coordinate system)
        // Get base rotation - default to 0 if not animated
        let base_rotation = interpolate_float(&animated.rotation, layer_time).unwrap_or(0.0);
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
            let time_sec = layer_time * duration_sec;
            let accumulated_freq = if animated.swing_freq.keyframes.is_empty() {
                // Non-keyed: simple multiplication
                swing_freq * time_sec
            } else {
                // Keyed: numerical integration at 120 steps/sec (matches AM)
                let total_steps = (duration_sec * 120.0).round() as i32;
                let current_step = (120.0 * time_sec).round() as i32;
                let mut accum = 0.0f64;
                if total_steps > 0 {
                    for i in 0..=current_step.min(total_steps) {
                        let frac_t = i as f32 / total_steps as f32;
                        let freq_at_t =
                            interpolate_float(&animated.swing_freq, frac_t).unwrap_or(0.0);
                        accum += freq_at_t as f64 / 120.0;
                    }
                }
                accum as f32
            };

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
            let time_sec = layer_time * duration_sec;
            let spin_angle = if animated.spin_rpm.keyframes.is_empty() {
                // Non-keyed: rpm * time_seconds * 6.0
                let rpm = interpolate_float(&animated.spin_rpm, layer_time).unwrap_or(0.0);
                rpm * time_sec * 6.0
            } else {
                // Keyed: numerical integration at 120 steps/sec, each step adds rpm(t) / 20.0
                let total_steps = (duration_sec * 120.0).round() as i32;
                let current_step = (120.0 * time_sec).round() as i32;
                let mut accum = 0.0f64;
                if total_steps > 0 {
                    for i in 0..=current_step.min(total_steps) {
                        let frac_t = i as f32 / total_steps as f32;
                        let rpm_at_t = interpolate_float(&animated.spin_rpm, frac_t).unwrap_or(0.0);
                        accum += rpm_at_t as f64 / 20.0;
                    }
                }
                accum as f32
            };
            // Negate for Bevy's coordinate system
            final_rotation -= spin_angle;
        }

        transform.rotation = Quat::from_rotation_z(final_rotation.to_radians());

        // DEBUG: Log rotation for 空2 layers to verify correct value
        if _marker.label.contains("空 2") {
            bevy::log::info!(
                "[DEBUG_ROT] '{}' (id={}): base_rotation={:.1}°, final_rotation={:.1}°, transform.rotation={:?}",
                _marker.label,
                animated.layer_id,
                base_rotation,
                final_rotation,
                transform.rotation
            );
        }

        // Interpolate scale
        // Skip for SDF shapes (handled by animate_sdf_scale)
        // For effect sprites: magnitude is baked into mesh, but sign (flip) needs Transform
        if sdf_parent.is_none() && effect_marker.is_none() {
            transform.scale = Vec3::new(
                current_scale[0] * oscillate_z_zoom,
                current_scale[1] * oscillate_z_zoom,
                1.0,
            );
        } else if effect_marker.is_some() {
            // Effect sprites: apply only the sign of scale for flipping
            // The magnitude is already baked into the mesh by animate_unified_effect_system
            let sign_x = actual_scale[0].signum();
            let sign_y = actual_scale[1].signum();
            transform.scale = Vec3::new(sign_x, sign_y, 1.0);
        }
    }
}

/// DEBUG: System to print GlobalTransform for debugging parent-child transforms
#[allow(dead_code)]
fn debug_global_transform_system(
    playback: Res<AmPlayback>,
    query: Query<(&AmAnimated, &GlobalTransform, &Transform, &AmLayerMarker)>,
) {
    use std::sync::atomic::{AtomicU32, Ordering};
    static LAST_FRAME: AtomicU32 = AtomicU32::new(999);

    // Print on frame 0, 10, 30 (30 is t=0.5s where animation should be stable)
    let frame = (playback.current_time_ms / 16.667).round() as u32;
    let last = LAST_FRAME.load(Ordering::Relaxed);
    if (frame == 0 || frame == 10 || frame == 30) && frame != last {
        LAST_FRAME.store(frame, Ordering::Relaxed);
        info!(
            "[DEBUG_GLOBAL] === Frame {} (t={:.1}ms) ===",
            frame, playback.current_time_ms
        );
        for (animated, global_transform, local_transform, marker) in query.iter() {
            let (g_scale, g_rot, g_trans) = global_transform.to_scale_rotation_translation();
            let g_rot_deg = g_rot.to_euler(EulerRot::ZYX).0.to_degrees();
            let (l_scale, l_rot, l_trans) = (
                local_transform.scale,
                local_transform.rotation,
                local_transform.translation,
            );
            let l_rot_deg = l_rot.to_euler(EulerRot::ZYX).0.to_degrees();
            if marker.label.contains("空") || marker.label.contains("Image_1699715690143") {
                info!(
                    "[DEBUG_GLOBAL] '{}' (id={}, parent={}): LOCAL pos=({:.1},{:.1}), rot={:.1}°, scale=({:.2},{:.2}) | GLOBAL pos=({:.1},{:.1}), rot={:.1}°, scale=({:.2},{:.2})",
                    marker.label,
                    animated.layer_id,
                    animated.parent_layer_id,
                    l_trans.x,
                    l_trans.y,
                    l_rot_deg,
                    l_scale.x,
                    l_scale.y,
                    g_trans.x,
                    g_trans.y,
                    g_rot_deg,
                    g_scale.x,
                    g_scale.y
                );
            }
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
        let final_alpha = opacity * animated.base_alpha;
        sprite.color.set_alpha(final_alpha.clamp(0.0, 1.0));
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

    for (animated, mut text_color, mut visibility, marker) in query.iter_mut() {
        // Use local time for visibility check (affected by speed)
        let local_time = animated.calc_local_time(global_time);

        // Check if layer is active
        if !animated.is_active(local_time) {
            // Hide text when outside its time range
            if *visibility != Visibility::Hidden {
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
        let final_alpha = opacity * animated.base_alpha;
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
        if let Some(size) = interpolate_vec2(&animated.size, layer_time) {
            let half_width = size[0].abs() / 2.0;
            let half_height = size[1].abs() / 2.0;

            // Update SDF children
            for child in children.iter() {
                if let Ok((material_handle, mut sdf_params)) = sdf_query.get_mut(child) {
                    // Update base params (these will be further modified by scale in animate_sdf_scale)
                    sdf_params.base_half_width = half_width;
                    sdf_params.base_half_height = half_height;

                    // Also update the actual material params directly
                    // (animate_sdf_scale will run after and apply scale on top if needed)
                    if let Some(material) = materials.get_mut(&material_handle.0) {
                        material.uniform_data.params.x = half_width;
                        material.uniform_data.params.y = half_height;
                    }
                }
            }
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
        (With<Camera2d>, Without<crate::effects::EmbedSceneRttCamera>),
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

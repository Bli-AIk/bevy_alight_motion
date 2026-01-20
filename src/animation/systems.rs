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

use crate::scene::AmLayerMarker;

use super::components::{AmAnimated, AmPlayback, AmSdfShapeParent};
use super::interpolation::{
    interpolate_float, interpolate_vec2, interpolate_vec3_with_extrapolation,
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
        if embed_content_marker.is_some() && animated.speed_multiplier != 0.0 {
            let frame_duration_ms = 1000.0 / 30.0; // Assuming 30fps
            local_time += frame_duration_ms * 0.35; // Adjusted based on testing
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
        let actual_scale = interpolate_vec2(&animated.scale, layer_time).unwrap_or([1.0, 1.0]);
        let current_scale = if sdf_parent.is_some() || effect_marker.is_some() {
            [1.0_f32, 1.0_f32]
        } else {
            actual_scale
        };

        // Interpolate location and convert from AM to Bevy coordinates
        // Use extrapolation for location to improve accuracy before first keyframe
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
            if embed_content_marker.is_some() && layer_time < 0.02 {
                bx -= 5.0;
            }

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
            // For other shapes: need pivot compensation for non-unit scale
            if let Some(pivot) = interpolate_vec2(&animated.pivot, layer_time) {
                let pivot_x = pivot[0];
                let pivot_y = pivot[1];

                if sdf_parent.is_some() {
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
                } else {
                    // Non-SDF shapes: need pivot compensation for scale
                    // Formula: pivot * (1 - scale) compensates for scale around pivot
                    bx += pivot_x * (1.0 - current_scale[0]);
                    by -= pivot_y * (1.0 - current_scale[1]);
                }
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
            if sdf_parent.is_none() {
                bx += animated.anchor_offset.x;
                by += animated.anchor_offset.y;
            }

            // For embed content: coordinates are already in embed's internal canvas space
            // They render to RTT camera at origin - no scaling needed for position
            // The inv_fit_scale is only used for sprite SIZE compensation, not position

            transform.translation = Vec3::new(bx, by, transform.translation.z);
        }

        // Interpolate rotation (negate for Bevy's coordinate system)
        if let Some(rot) = interpolate_float(&animated.rotation, layer_time) {
            transform.rotation = Quat::from_rotation_z((-rot).to_radians());
        }

        // Interpolate scale
        // Skip for SDF shapes (handled by animate_sdf_scale)
        // For effect sprites: magnitude is baked into mesh, but sign (flip) needs Transform
        if sdf_parent.is_none() && effect_marker.is_none() {
            transform.scale = Vec3::new(current_scale[0], current_scale[1], 1.0);
        } else if effect_marker.is_some() {
            // Effect sprites: apply only the sign of scale for flipping
            // The magnitude is already baked into the mesh by animate_unified_effect_system
            let sign_x = actual_scale[0].signum();
            let sign_y = actual_scale[1].signum();
            transform.scale = Vec3::new(sign_x, sign_y, 1.0);
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

//! # animation.rs
//!
//! # animation.rs 文件
//!
//! ## Module Overview
//!
//! ## 模块概述
//!
//! Animation systems for interpolating keyframes in Alight Motion projects.
//!
//! 用于在 Alight Motion 项目中插值关键帧的动画系统。
//!
//! ## Source File Overview
//!
//! ## 源文件概述
//!
//! This file contains the core animation components and systems for playback control,
//! transform animation, opacity animation, SDF shape animation, and layer lifecycle management.
//!
//! 本文件包含核心动画组件和系统，用于播放控制、变换动画、不透明度动画、SDF形状动画和图层生命周期管理。

use bevy::ecs::relationship::Relationship;
use bevy::prelude::*;

use crate::scene::AmLayerMarker;
use crate::schema::{AmAnimatedFloat, AmAnimatedVec2, AmAnimatedVec3, AmKeyframe, Easing};

/// Component marking an entity as part of an AM animation.
///
/// 标记实体为 AM 动画一部分的组件。
#[derive(Component, Debug, Clone)]
pub struct AmAnimated {
    /// Unique layer ID from AM.
    ///
    /// AM 中的唯一图层 ID。
    pub layer_id: u64,
    /// Start time in milliseconds (relative to time_offset).
    ///
    /// 开始时间（毫秒，相对于时间偏移）。
    pub start_time: i32,
    /// End time in milliseconds (relative to time_offset).
    pub end_time: i32,
    /// Time offset from parent scene (for embedded scenes).
    pub time_offset: i32,
    /// Location animation data.
    pub location: AmAnimatedVec3,
    /// Pivot/anchor point animation data.
    pub pivot: AmAnimatedVec2,
    /// Rotation animation data.
    pub rotation: AmAnimatedFloat,
    /// Scale animation data.
    pub scale: AmAnimatedVec2,
    /// Opacity animation data.
    pub opacity: AmAnimatedFloat,
    /// Canvas width for coordinate conversion.
    pub canvas_width: f32,
    /// Canvas height for coordinate conversion.
    pub canvas_height: f32,
    /// Whether this layer has a parent (uses local coordinates).
    pub has_parent: bool,
    /// Effect position X offset (from transform2 effect).
    pub effect_pos_x: AmAnimatedFloat,
    /// Effect position Y offset (from transform2 effect).
    pub effect_pos_y: AmAnimatedFloat,
    /// Font Y offset for text layers (to compensate for different font metrics).
    pub font_y_offset: f32,
    /// Size animation data (for shapes). AM size is half-extents, stored as full dimensions.
    pub size: AmAnimatedVec2,
    /// Position compensation for anchor offset (Bevy coords).
    /// When anchor is not CENTER, sprite position needs adjustment to keep center at AM location.
    pub anchor_offset: Vec2,
    /// Wipe effect start (0.0-1.0 percentage, default 0.0).
    pub wipe_start: AmAnimatedFloat,
    /// Wipe effect end (0.0-1.0 percentage, default 1.0).
    pub wipe_end: AmAnimatedFloat,
    /// Wipe effect angle in radians (0 = left-to-right).
    pub wipe_angle: AmAnimatedFloat,
    /// Wipe effect feather (softness of edge, 0.0 = sharp).
    pub wipe_feather: AmAnimatedFloat,
    /// Stretch segment effect angle in degrees (0 = horizontal split).
    pub stretch_angle: AmAnimatedFloat,
    /// Stretch segment effect stretch amount (pixels, normalized to UV).
    pub stretch_amount: AmAnimatedFloat,
    /// Stretch segment effect offset (position of split line).
    pub stretch_offset: AmAnimatedFloat,
    /// Stretch segment effect smooth width (0 = hard edge).
    pub stretch_smooth: AmAnimatedFloat,
    /// Speed multiplier from parent embed scenes.
    /// Local time = (global_time - time_offset) * speed_multiplier
    pub speed_multiplier: f32,
    /// Embed parent offset (Bevy coords) for coordinate adjustment.
    /// When this layer is a child of an embed scene, this stores the embed's
    /// Bevy position so the animation system can compensate for it.
    pub embed_offset: Vec2,
    /// Inverse fit scale for embed children coordinate adjustment.
    /// When the project is scaled to fit window, embed children need their coordinates
    /// scaled by 1/fit_scale to compensate for the root scaling.
    pub inv_fit_scale: f32,
}

/// Resource to control animation playback.
#[derive(Resource, Debug, Clone)]
pub struct AmPlayback {
    /// Current time in milliseconds.
    pub current_time_ms: f32,
    /// Total duration in milliseconds.
    pub total_time_ms: f32,
    /// Is playing.
    pub playing: bool,
    /// Playback speed (1.0 = normal).
    pub speed: f32,
    /// Loop playback.
    pub looping: bool,
    /// Force stopped - when true, animation systems won't update transforms.
    /// Use this for debugging/inspector editing. Normal pause still updates animations.
    pub force_stopped: bool,
}

impl Default for AmPlayback {
    fn default() -> Self {
        Self {
            current_time_ms: 0.0,
            total_time_ms: 2000.0,
            playing: true,
            speed: 1.0,
            looping: true,
            force_stopped: false,
        }
    }
}

impl AmPlayback {
    /// Create with specific duration.
    pub fn with_duration(total_time_ms: f32) -> Self {
        Self {
            total_time_ms,
            ..Default::default()
        }
    }

    /// Reset to beginning.
    pub fn reset(&mut self) {
        self.current_time_ms = 0.0;
    }

    /// Toggle play/pause.
    pub fn toggle(&mut self) {
        self.playing = !self.playing;
    }

    /// Toggle force stop - freezes all animation updates for inspector editing.
    pub fn toggle_force_stop(&mut self) {
        self.force_stopped = !self.force_stopped;
    }
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
    )>,
) {
    // Skip animation only when force stopped (for inspector editing)
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (animated, mut transform, _marker, layer_spec, sdf_parent, effect_marker) in
        query.iter_mut()
    {
        // Calculate local time (accounting for time offset from parent scene)
        let local_time = (global_time - animated.time_offset as f32) * animated.speed_multiplier;

        // Check if layer is active at current local time
        if local_time < animated.start_time as f32 || local_time > animated.end_time as f32 {
            continue;
        }

        // Calculate normalized time within layer duration
        let layer_duration = (animated.end_time - animated.start_time) as f32;
        let layer_time = (local_time - animated.start_time as f32) / layer_duration;

        // Get current scale for pivot compensation
        // For SDF shapes and effect sprites, scale is handled separately, so use (1, 1)
        let current_scale = if sdf_parent.is_some() || effect_marker.is_some() {
            [1.0_f32, 1.0_f32]
        } else {
            interpolate_vec2(&animated.scale, layer_time).unwrap_or([1.0, 1.0])
        };

        // Interpolate location and convert from AM to Bevy coordinates
        if let Some(loc) = interpolate_vec3(&animated.location, layer_time) {
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
            bx += animated.anchor_offset.x;
            by += animated.anchor_offset.y;

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
        // Skip for effect sprites (scale is baked into mesh, handled by animate_unified_effect)
        if sdf_parent.is_none() && effect_marker.is_none() {
            transform.scale = Vec3::new(current_scale[0], current_scale[1], 1.0);
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
        // Calculate local time (accounting for time offset from parent scene)
        let local_time = (global_time - animated.time_offset as f32) * animated.speed_multiplier;

        // Check if layer is active
        if local_time < animated.start_time as f32 || local_time > animated.end_time as f32 {
            sprite.color.set_alpha(0.0);
            continue;
        }

        let layer_duration = (animated.end_time - animated.start_time) as f32;
        let layer_time = (local_time - animated.start_time as f32) / layer_duration;

        // Get opacity from animation data, default to 1.0 if not specified
        let opacity = interpolate_float(&animated.opacity, layer_time).unwrap_or(1.0);
        sprite.color.set_alpha(opacity.clamp(0.0, 1.0));
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
        // Calculate local time (accounting for time offset from parent scene)
        let local_time = (global_time - animated.time_offset as f32) * animated.speed_multiplier;

        // Check if layer is active
        if local_time < animated.start_time as f32 || local_time > animated.end_time as f32 {
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

        let layer_duration = (animated.end_time - animated.start_time) as f32;
        let layer_time = (local_time - animated.start_time as f32) / layer_duration;

        // Get opacity from keyframes, or default to 1.0 if no opacity animation
        let opacity = interpolate_float(&animated.opacity, layer_time).unwrap_or(1.0);
        text_color.0.set_alpha(opacity.clamp(0.0, 1.0));
    }
}

/// System to animate SDF shape opacity (handles SmudShape entities).
/// Uses Visibility component for proper show/hide behavior and SmudShape color alpha for opacity animation.
/// Only skips updates when force_stopped is true (for inspector editing).
pub fn animate_sdf_opacity_system(
    playback: Res<AmPlayback>,
    mut query: Query<(
        &AmAnimated,
        &mut bevy_smud::SmudShape,
        &mut Visibility,
        &AmLayerMarker,
    )>,
) {
    // Skip animation only when force stopped (for inspector editing)
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (animated, mut smud_shape, mut visibility, _marker) in query.iter_mut() {
        // Calculate local time (accounting for time offset from parent scene)
        let local_time = (global_time - animated.time_offset as f32) * animated.speed_multiplier;

        // Check if layer is active
        if local_time < animated.start_time as f32 || local_time > animated.end_time as f32 {
            // Hide shape when outside its time range
            *visibility = Visibility::Hidden;
            smud_shape.color.set_alpha(0.0);
            continue;
        }

        // Show shape when within its time range
        *visibility = Visibility::Inherited;

        let layer_duration = (animated.end_time - animated.start_time) as f32;
        let layer_time = (local_time - animated.start_time as f32) / layer_duration;

        // Get opacity from keyframes, or default to 1.0 if no opacity animation
        let opacity = interpolate_float(&animated.opacity, layer_time).unwrap_or(1.0);
        smud_shape.color.set_alpha(opacity.clamp(0.0, 1.0));
    }
}

/// System to update SDF shape dimensions based on parent scale animation.
///
/// ## New Approach (parametric SDF)
/// Instead of using Transform.scale (which bevy_smud only supports uniformly),
/// we update SmudShape.params to change the SDF dimensions:
/// - params.x = base_half_width * animation_scale_x
/// - params.y = base_half_height * animation_scale_y
/// - params.z = stroke_width (constant)
/// - params.w = packed_stroke_color (constant)
///
/// This allows non-uniform scaling while keeping stroke width constant.
///
/// Also updates the child transform translation to account for pivot scaling.
/// Since the parent (Pivot) is not scaled, we must move the child (Center)
/// to simulate scaling around the pivot.
pub fn animate_sdf_scale_system(
    playback: Res<AmPlayback>,
    parent_query: Query<(&AmAnimated, &Children), With<AmSdfShapeParent>>,
    mut sdf_query: Query<(&mut SmudShape, &AmSdfParams, &mut Transform)>,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (animated, children) in parent_query.iter() {
        // Calculate local time
        let local_time = (global_time - animated.time_offset as f32) * animated.speed_multiplier;

        // Skip if outside active time range
        if local_time < animated.start_time as f32 || local_time > animated.end_time as f32 {
            continue;
        }

        // Calculate normalized time within layer duration
        let layer_duration = (animated.end_time - animated.start_time) as f32;
        let layer_time = (local_time - animated.start_time as f32) / layer_duration;

        // Get animation scale from keyframes
        let anim_scale = interpolate_vec2(&animated.scale, layer_time).unwrap_or([1.0, 1.0]);

        // Update SDF child's params to reflect scaled dimensions
        for child in children.iter() {
            if let Ok((mut smud_shape, sdf_params, mut transform)) = sdf_query.get_mut(child) {
                // Calculate scaled dimensions
                let scaled_half_width = sdf_params.base_half_width * anim_scale[0];
                let scaled_half_height = sdf_params.base_half_height * anim_scale[1];

                // Update params: (half_width, half_height, stroke_width, packed_stroke)
                smud_shape.params = Vec4::new(
                    scaled_half_width,
                    scaled_half_height,
                    sdf_params.stroke_width,
                    sdf_params.packed_stroke,
                );

                // Update translation to simulate scaling around pivot
                // Center position = -Pivot * Scale
                // Account for Y-flip: AM pivot_y is down (+), Bevy Y is up
                // So translation.y = pivot_y * scale_y (positive pivot_y moves center UP relative to pivot)
                transform.translation.x = -sdf_params.base_pivot_x * anim_scale[0];
                transform.translation.y = sdf_params.base_pivot_y * anim_scale[1];
            }
        }
    }
}

/// System to animate shape size based on size property keyframes.
///
/// AM shapes have a `size` property (in properties list) that can be animated.
/// This is separate from scale animation - size changes the base dimensions
/// while scale is applied on top.
///
/// For SDF shapes: Updates SmudShape.params with new half-width/half-height.
/// For Sprite shapes: Updates Sprite.custom_size.
pub fn animate_size_system(
    playback: Res<AmPlayback>,
    // SDF shapes: parent entity has AmSdfShapeParent marker, child has SmudShape
    parent_query: Query<(&AmAnimated, &Children), With<AmSdfShapeParent>>,
    mut sdf_query: Query<(&mut SmudShape, &mut AmSdfParams)>,
    // Sprite shapes: entity has Sprite component directly
    mut sprite_query: Query<(&AmAnimated, &mut Sprite), Without<AmSdfShapeParent>>,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    // Handle SDF shapes (size is on parent, SmudShape is on child)
    for (animated, children) in parent_query.iter() {
        // Skip if no size animation
        if animated.size.keyframes.is_empty() && animated.size.value.is_none() {
            continue;
        }

        // Calculate local time
        let local_time = (global_time - animated.time_offset as f32) * animated.speed_multiplier;

        // Skip if outside active time range
        if local_time < animated.start_time as f32 || local_time > animated.end_time as f32 {
            continue;
        }

        // Calculate normalized time within layer duration
        let layer_duration = (animated.end_time - animated.start_time) as f32;
        let layer_time = (local_time - animated.start_time as f32) / layer_duration;

        // Interpolate size (already stored as full dimensions, need half for SDF)
        if let Some(size) = interpolate_vec2(&animated.size, layer_time) {
            let half_width = size[0].abs() / 2.0;
            let half_height = size[1].abs() / 2.0;

            // Update SDF children
            for child in children.iter() {
                if let Ok((mut smud_shape, mut sdf_params)) = sdf_query.get_mut(child) {
                    // Update base params (these will be further modified by scale in animate_sdf_scale)
                    sdf_params.base_half_width = half_width;
                    sdf_params.base_half_height = half_height;

                    // Also update the actual SmudShape params directly
                    // (animate_sdf_scale will run after and apply scale on top if needed)
                    smud_shape.params.x = half_width;
                    smud_shape.params.y = half_height;
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

        // Calculate local time
        let local_time = (global_time - animated.time_offset as f32) * animated.speed_multiplier;

        // Skip if outside active time range
        if local_time < animated.start_time as f32 || local_time > animated.end_time as f32 {
            continue;
        }

        // Calculate normalized time within layer duration
        let layer_duration = (animated.end_time - animated.start_time) as f32;
        let layer_time = (local_time - animated.start_time as f32) / layer_duration;

        // Interpolate size (full dimensions for Sprite)
        if let Some(size) = interpolate_vec2(&animated.size, layer_time) {
            // Use original size - no scaling needed
            // For embed content, the final display size is affected by embed's inherited fit_scale
            sprite.custom_size = Some(Vec2::new(size[0].abs(), size[1].abs()));
        }
    }
}

/// Interpolate a Vec3 property at normalized time t.
pub fn interpolate_vec3(prop: &AmAnimatedVec3, t: f32) -> Option<[f32; 3]> {
    if prop.keyframes.is_empty() {
        return prop.value;
    }

    let (kf_prev, kf_next, local_t) = find_keyframes(&prop.keyframes, t);

    let v_prev = parse_keyframe_vec3(&kf_prev.value).unwrap_or([0.0, 0.0, 0.0]);
    let v_next = parse_keyframe_vec3(&kf_next.value).unwrap_or(v_prev);

    // Easing is defined on the "target" keyframe (describes how to arrive at it)
    let easing = kf_next
        .easing
        .as_ref()
        .map(|e| Easing::parse(e))
        .unwrap_or_default();
    let eased_t = easing.evaluate(local_t);

    Some([
        lerp(v_prev[0], v_next[0], eased_t),
        lerp(v_prev[1], v_next[1], eased_t),
        lerp(v_prev[2], v_next[2], eased_t),
    ])
}

/// Interpolate a Vec2 property at normalized time t.
pub fn interpolate_vec2(prop: &AmAnimatedVec2, t: f32) -> Option<[f32; 2]> {
    if prop.keyframes.is_empty() {
        return prop.value;
    }

    let (kf_prev, kf_next, local_t) = find_keyframes(&prop.keyframes, t);

    let v_prev = parse_keyframe_vec2(&kf_prev.value).unwrap_or([1.0, 1.0]);
    let v_next = parse_keyframe_vec2(&kf_next.value).unwrap_or(v_prev);

    // Easing is defined on the "target" keyframe (describes how to arrive at it)
    let easing = kf_next
        .easing
        .as_ref()
        .map(|e| Easing::parse(e))
        .unwrap_or_default();
    let eased_t = easing.evaluate(local_t);

    Some([
        lerp(v_prev[0], v_next[0], eased_t),
        lerp(v_prev[1], v_next[1], eased_t),
    ])
}

/// Interpolate a float property at normalized time t.
pub fn interpolate_float(prop: &AmAnimatedFloat, t: f32) -> Option<f32> {
    if prop.keyframes.is_empty() {
        return prop.value;
    }

    let (kf_prev, kf_next, local_t) = find_keyframes(&prop.keyframes, t);

    let v_prev: f32 = kf_prev.value.parse().unwrap_or(0.0);
    let v_next: f32 = kf_next.value.parse().unwrap_or(v_prev);

    // Easing is defined on the "target" keyframe (describes how to arrive at it)
    let easing = kf_next
        .easing
        .as_ref()
        .map(|e| Easing::parse(e))
        .unwrap_or_default();
    let eased_t = easing.evaluate(local_t);

    Some(lerp(v_prev, v_next, eased_t))
}

/// Find the surrounding keyframes for a given time.
fn find_keyframes(keyframes: &[AmKeyframe], t: f32) -> (&AmKeyframe, &AmKeyframe, f32) {
    // Sort keyframes by time (in case they're not sorted)
    let mut sorted: Vec<_> = keyframes.iter().collect();
    sorted.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Handle edge cases
    if sorted.len() == 1 {
        return (sorted[0], sorted[0], 0.0);
    }

    // Find surrounding keyframes
    for i in 0..sorted.len() - 1 {
        let kf_prev = sorted[i];
        let kf_next = sorted[i + 1];

        if t >= kf_prev.time && t <= kf_next.time {
            let span = kf_next.time - kf_prev.time;
            let local_t = if span > 0.0 {
                (t - kf_prev.time) / span
            } else {
                0.0
            };
            return (kf_prev, kf_next, local_t);
        }
    }

    // Before first keyframe
    if t < sorted[0].time {
        return (sorted[0], sorted[0], 0.0);
    }

    // After last keyframe
    let last = sorted.last().unwrap();
    (last, last, 0.0)
}

/// Linear interpolation.
#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Parse Vec3 from keyframe value string.
fn parse_keyframe_vec3(s: &str) -> Option<[f32; 3]> {
    crate::schema::parse_vec3(s).ok()
}

/// Parse Vec2 from keyframe value string.
fn parse_keyframe_vec2(s: &str) -> Option<[f32; 2]> {
    crate::schema::parse_vec2(s).ok()
}

// ============================================================================
// Layer Lifecycle Management System
// ============================================================================

use crate::loader::AmProject;
use crate::plugin::AmWhitePixel;
use crate::scene::{
    AmBlendingMode, AmLayerSpec, AmMaskInfo, AmPendingLayers, AmVisualSpawned, PendingLayer,
};
use bevy::asset::Assets;
use bevy_smud::prelude::*;
use std::collections::HashMap;

/// System to manage layer lifecycle based on playback time.
/// - Creates entities when layers enter their time range
/// - Destroys entities when layers exit their time range
/// - Implements true lazy spawning where no entities exist until needed
#[allow(clippy::too_many_arguments)]
pub fn manage_layer_lifecycle_system(
    mut commands: Commands,
    playback: Res<AmPlayback>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut unified_materials: ResMut<Assets<crate::masked_sprite::UnifiedEffectMaterial>>,
    sdf_shaders: Res<crate::sdf::AmSdfShaders>,
    white_pixel: Option<Res<AmWhitePixel>>,
    projects: Res<Assets<AmProject>>,
    mut project_query: Query<(Entity, &crate::scene::AmProjectRoot, &mut AmPendingLayers)>,
) {
    // Skip if force stopped
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    // Debug logging
    static mut FRAME_COUNT: u32 = 0;
    unsafe {
        FRAME_COUNT += 1;
    }

    for (project_entity, root, mut pending) in project_query.iter_mut() {
        let Some(project) = projects.get(&root.handle) else {
            continue;
        };

        let white_pixel_handle = white_pixel.as_ref().map(|wp| wp.0.clone());

        // Process all pending layers (including nested ones)
        process_pending_layers(
            &mut commands,
            &mut shaders,
            &mut meshes,
            &mut unified_materials,
            &sdf_shaders,
            &mut pending,
            &project.images,
            &project.fonts,
            white_pixel_handle.as_ref(),
            global_time,
            project_entity,
            0, // root time offset
        );

        // Log stats occasionally
        unsafe {
            if FRAME_COUNT % 300 == 1 {
                let spawned_count = pending.spawned_entities.len();
                let total_layers = count_total_layers(&pending.layers);
                bevy::log::trace!(
                    "[Lifecycle] time={:.0}ms | spawned={}/{} entities",
                    global_time,
                    spawned_count,
                    total_layers
                );
            }
        }
    }
}

/// Count total layers including nested ones.
fn count_total_layers(layers: &[PendingLayer]) -> usize {
    layers
        .iter()
        .map(|l| 1 + count_total_layers(&l.children))
        .sum()
}

/// Process pending layers recursively.
#[allow(clippy::too_many_arguments)]
fn process_pending_layers(
    commands: &mut Commands,
    shaders: &mut Assets<Shader>,
    meshes: &mut Assets<Mesh>,
    unified_materials: &mut Assets<crate::masked_sprite::UnifiedEffectMaterial>,
    sdf_shaders: &crate::sdf::AmSdfShaders,
    pending: &mut AmPendingLayers,
    images: &HashMap<String, Handle<Image>>,
    fonts: &HashMap<String, Handle<Font>>,
    white_pixel: Option<&Handle<Image>>,
    global_time: f32,
    parent_entity: Entity,
    time_offset: i32,
) {
    // We need to collect actions to avoid borrowing issues
    let mut to_spawn: Vec<usize> = Vec::new(); // indices of layers to spawn
    let mut to_despawn: Vec<u64> = Vec::new(); // layer_id

    // Helper function to check if an ancestor is active
    fn is_ancestor_active(
        layer_id: u64,
        layers: &[PendingLayer],
        global_time: f32,
        time_offset: i32,
    ) -> bool {
        let layer = match layers.iter().find(|l| l.id == layer_id) {
            Some(l) => l,
            None => return true, // If not found, assume active (root)
        };

        if layer.parent == 0 {
            return true; // No parent, always considered active from parent perspective
        }

        // Check parent's active status
        let parent = match layers.iter().find(|l| l.id == layer.parent) {
            Some(p) => p,
            None => return true, // Parent not in our list, assume active
        };

        let parent_local_time = (global_time - (parent.animated.time_offset + time_offset) as f32)
            * parent.animated.speed_multiplier;
        let parent_active = parent_local_time >= parent.start_time as f32
            && parent_local_time <= parent.end_time as f32;

        if !parent_active {
            return false; // Parent is not active
        }

        // Recursively check grandparent
        is_ancestor_active(layer.parent, layers, global_time, time_offset)
    }

    for (idx, layer) in pending.layers.iter().enumerate() {
        // Calculate local time with speed multiplier
        let local_time = (global_time - (layer.animated.time_offset + time_offset) as f32)
            * layer.animated.speed_multiplier;

        // Check if layer should be active (considering both own time range and parent's time range)
        let own_time_active =
            local_time >= layer.start_time as f32 && local_time <= layer.end_time as f32;

        // Check if all ancestors are active
        let ancestors_active =
            is_ancestor_active(layer.id, &pending.layers, global_time, time_offset);

        let should_be_active = own_time_active && ancestors_active;

        let is_spawned = pending.spawned_entities.contains_key(&layer.id);

        if should_be_active && !is_spawned {
            to_spawn.push(idx);
        } else if !should_be_active && is_spawned {
            to_despawn.push(layer.id);
        }
    }

    // Despawn entities that are no longer active
    for layer_id in to_despawn {
        if let Some(entity) = pending.spawned_entities.remove(&layer_id) {
            // Find layer info for logging
            if let Some(layer) = pending.layers.iter().find(|l| l.id == layer_id) {
                bevy::log::trace!(
                    "  [Lifecycle] Despawning '{}' (id={})",
                    layer.label,
                    layer_id
                );
            }

            // Find all children of this layer (direct and nested) and despawn them first
            let children_to_remove: Vec<u64> = pending
                .layers
                .iter()
                .filter(|l| is_descendant_of(l.id, layer_id, &pending.layers))
                .map(|l| l.id)
                .collect();

            // Despawn children (deepest first would be ideal, but order doesn't matter much
            // since we're despawning them all)
            for child_id in children_to_remove {
                if let Some(child_entity) = pending.spawned_entities.remove(&child_id) {
                    if let Some(child) = pending.layers.iter().find(|l| l.id == child_id) {
                        bevy::log::trace!(
                            "    [Lifecycle] (cascade) Despawning child '{}' (id={})",
                            child.label,
                            child_id
                        );
                    }
                    commands.entity(child_entity).despawn();
                }
            }

            // Despawn the entity itself
            commands.entity(entity).despawn();
        }
    }

    // Sort layers to spawn by dependency (parents before children) using topological sort
    // Build a set of layer IDs being spawned this frame
    let spawning_ids: std::collections::HashSet<u64> =
        to_spawn.iter().map(|&idx| pending.layers[idx].id).collect();

    // Helper function to count dependency depth (how many ancestors are also being spawned)
    // For embed content, we also need to consider containing_embed_id as a dependency
    fn count_spawn_depth(
        layer_id: u64,
        layers: &[PendingLayer],
        spawning_ids: &std::collections::HashSet<u64>,
        visited: &mut std::collections::HashSet<u64>,
    ) -> usize {
        if visited.contains(&layer_id) {
            return 0; // Prevent infinite loop
        }
        visited.insert(layer_id);

        let layer = match layers.iter().find(|l| l.id == layer_id) {
            Some(l) => l,
            None => return 0,
        };

        // Calculate depth from parent chain
        let parent_depth = if layer.parent == 0 || !spawning_ids.contains(&layer.parent) {
            0
        } else {
            1 + count_spawn_depth(layer.parent, layers, spawning_ids, visited)
        };

        // For embed content, containing_embed_id must also be spawned first
        let embed_depth = if layer.containing_embed_id == 0
            || !spawning_ids.contains(&layer.containing_embed_id)
        {
            0
        } else {
            1 + count_spawn_depth(layer.containing_embed_id, layers, spawning_ids, visited)
        };

        // Return the maximum depth to ensure all dependencies are spawned first
        parent_depth.max(embed_depth)
    }

    // Sort by depth (lower depth = spawn first)
    to_spawn.sort_by_key(|&idx| {
        let layer_id = pending.layers[idx].id;
        let mut visited = std::collections::HashSet::new();
        count_spawn_depth(layer_id, &pending.layers, &spawning_ids, &mut visited)
    });

    // Spawn new entities in dependency order
    for idx in to_spawn {
        let layer = &pending.layers[idx];

        // Determine parent for this entity
        let actual_parent = if layer.parent != 0 {
            match pending.spawned_entities.get(&layer.parent) {
                Some(&e) => e,
                None => {
                    bevy::log::warn!(
                        "[Lifecycle] WARNING: Parent {} not found for '{}' (id={}), using root",
                        layer.parent,
                        layer.label,
                        layer.id
                    );
                    parent_entity
                }
            }
        } else {
            parent_entity
        };

        let entity = spawn_layer_entity(
            commands,
            shaders,
            meshes,
            unified_materials,
            sdf_shaders,
            layer,
            images,
            fonts,
            white_pixel,
            actual_parent,
            parent_entity, // project_entity for embed content attachment
            pending.inv_fit_scale,
            &pending.spawned_entities,
        );

        bevy::log::info!(
            "[Lifecycle] Spawning '{}' (id={}, parent={}, embed={}, z={:.6}, time={}..{}ms)",
            layer.label,
            layer.id,
            layer.parent,
            layer.containing_embed_id,
            layer.transform.translation.z,
            layer.start_time,
            layer.end_time
        );

        pending.spawned_entities.insert(layer.id, entity);
    }
}

/// Check if a layer is a descendant of another layer (direct or nested).
fn is_descendant_of(layer_id: u64, ancestor_id: u64, layers: &[PendingLayer]) -> bool {
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
fn get_initial_scale_from_animated(prop: &AmAnimatedVec2) -> (f32, f32) {
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
            .unwrap_or((1.0, 1.0))
    } else {
        (1.0, 1.0)
    }
}

/// Spawn a complete entity from a PendingLayer.
///
/// For spatial decoupling of embed content:
/// - If `containing_embed_id != 0`, the entity is NOT made a Bevy child of its parent
/// - Instead, it's attached to the project root with `AmEmbedContentMarker`
/// - This prevents Transform inheritance from embed entity (no double rotation)
#[allow(clippy::too_many_arguments)]
fn spawn_layer_entity(
    commands: &mut Commands,
    shaders: &mut Assets<Shader>,
    meshes: &mut Assets<Mesh>,
    unified_materials: &mut Assets<crate::masked_sprite::UnifiedEffectMaterial>,
    sdf_shaders: &crate::sdf::AmSdfShaders,
    layer: &PendingLayer,
    images: &HashMap<String, Handle<Image>>,
    fonts: &HashMap<String, Handle<Font>>,
    white_pixel: Option<&Handle<Image>>,
    parent_entity: Entity,
    _project_entity: Entity, // Unused for embed content (they exist in world space)
    inv_fit_scale: f32,
    spawned_entities: &HashMap<u64, Entity>,
) -> Entity {
    let entity_name = format!("Layer[{}]: {}", layer.id, layer.label);

    // Check if layer has any effects that need scale baking
    let has_wipe = layer.animated.wipe_end.value != Some(1.0)
        || !layer.animated.wipe_end.keyframes.is_empty()
        || layer.animated.wipe_start.value.is_some()
        || !layer.animated.wipe_start.keyframes.is_empty();

    let has_stretch = layer.animated.stretch_amount.value.is_some()
        || !layer.animated.stretch_amount.keyframes.is_empty()
        || layer.animated.stretch_angle.value.is_some()
        || !layer.animated.stretch_angle.keyframes.is_empty()
        || layer.animated.stretch_offset.value.is_some()
        || !layer.animated.stretch_offset.keyframes.is_empty()
        || layer.animated.stretch_smooth.value.is_some()
        || !layer.animated.stretch_smooth.keyframes.is_empty();

    let has_mask = layer.mask_info.is_some();
    let needs_effect = has_wipe || has_stretch || has_mask;

    // If layer has effects, bake scale into a separate variable and use identity scale for transform
    // This is because effects need actual pixel sizes, not scaled coordinates
    let transform_to_use =
        if needs_effect && layer.blending_mode != crate::scene::AmBlendingMode::Mask {
            let mut t = layer.transform;
            t.scale = Vec3::ONE; // Reset scale - it will be baked into mesh size
            t
        } else {
            layer.transform
        };

    // Clone animated component and set inv_fit_scale for embed children
    let mut animated = layer.animated.clone();
    if animated.embed_offset != Vec2::ZERO {
        animated.inv_fit_scale = inv_fit_scale;
    }

    // Create base entity with common components
    let entity = commands
        .spawn((
            Name::new(entity_name),
            AmLayerMarker {
                id: layer.id,
                label: layer.label.clone(),
            },
            animated,
            layer.spec.clone(),
            transform_to_use,
            GlobalTransform::default(),
            Visibility::Inherited,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Add mask info component if this layer is affected by a mask
    if let Some(mask_info) = &layer.mask_info {
        commands.entity(entity).insert(mask_info.clone());
        bevy::log::debug!(
            "[Lifecycle] Layer '{}' has mask: center=({:.1},{:.1}), half_size=({:.1},{:.1})",
            layer.label,
            mask_info.center.x,
            mask_info.center.y,
            mask_info.half_size.x,
            mask_info.half_size.y
        );
    }

    // Add visual components based on spec (skip for mask layers)
    if layer.blending_mode != AmBlendingMode::Mask {
        // Extract initial scale from animated data for SDF shapes
        // (transform.scale is set to 1.0 for SDF shapes, actual scale is in animated)
        let initial_scale = get_initial_scale_from_animated(&layer.animated.scale);

        // Check if layer has wipe effect
        let has_wipe = layer.animated.wipe_end.value != Some(1.0)
            || !layer.animated.wipe_end.keyframes.is_empty()
            || layer.animated.wipe_start.value.is_some()
            || !layer.animated.wipe_start.keyframes.is_empty();

        // Check if layer has stretch segment effect
        let has_stretch = layer.animated.stretch_amount.value.is_some()
            || !layer.animated.stretch_amount.keyframes.is_empty()
            || layer.animated.stretch_angle.value.is_some()
            || !layer.animated.stretch_angle.keyframes.is_empty()
            || layer.animated.stretch_offset.value.is_some()
            || !layer.animated.stretch_offset.keyframes.is_empty()
            || layer.animated.stretch_smooth.value.is_some()
            || !layer.animated.stretch_smooth.keyframes.is_empty();

        // Get initial wipe params
        let initial_wipe = if has_wipe {
            let wipe_start = layer.animated.wipe_start.value.unwrap_or(0.0);
            let wipe_end = layer.animated.wipe_end.value.unwrap_or(1.0);
            let wipe_angle = layer.animated.wipe_angle.value.unwrap_or(0.0);
            let wipe_feather = layer.animated.wipe_feather.value.unwrap_or(0.0);
            Some(Vec4::new(wipe_start, wipe_end, wipe_angle, wipe_feather))
        } else {
            None
        };

        // Get initial stretch segment params
        let initial_stretch = if has_stretch {
            let angle_deg = layer.animated.stretch_angle.value.unwrap_or(0.0);
            let angle_rad = angle_deg.to_radians();
            let stretch_px = layer.animated.stretch_amount.value.unwrap_or(0.0);
            let stretch_uv = stretch_px / 500.0;
            let offset_px = layer.animated.stretch_offset.value.unwrap_or(0.0);
            let offset_uv = offset_px / 500.0;
            let smooth = layer.animated.stretch_smooth.value.unwrap_or(0.0);
            let smooth_width = smooth * 0.3;
            Some(Vec4::new(angle_rad, stretch_uv, offset_uv, smooth_width))
        } else {
            None
        };

        // For embed content rendered to RTT, use original size (no scaling)
        // The final display size will be affected by embed's inherited fit_scale
        let size_scale = 1.0;

        add_visual_components(
            commands,
            shaders,
            meshes,
            unified_materials,
            sdf_shaders,
            entity,
            &layer.spec,
            &layer.mask_info,
            images,
            fonts,
            white_pixel,
            &layer.label,
            layer.id,
            initial_scale,
            initial_wipe,
            initial_stretch,
            layer.embed_scene_size,
            size_scale,
        );
    } else {
        bevy::log::info!(
            "[Lifecycle] Skipping visual for mask layer '{}' (id={})",
            layer.label,
            layer.id
        );
    }

    // Spatial decoupling: embed content is NOT made a Bevy child of the embed entity
    // This prevents Transform inheritance (no double rotation/scale)
    if layer.containing_embed_id != 0 {
        // This is embed content - DO NOT attach to any parent entity
        // The content exists in world space and renders to the embed's RTT camera
        // NOTE: We explicitly do NOT call add_child() here - this is spatial decoupling!
        // The entity remains a root-level entity in world space.

        // Look up the embed entity and add marker for lifecycle management
        if let Some(&embed_entity) = spawned_entities.get(&layer.containing_embed_id) {
            commands
                .entity(entity)
                .insert(crate::scene::AmEmbedContentMarker {
                    embed_entity,
                    embed_id: layer.containing_embed_id,
                });
            bevy::log::info!(
                "[Lifecycle] Embed content '{}' (entity {:?}) exists in world space, belongs to embed {} ({:?})",
                layer.label,
                entity,
                layer.containing_embed_id,
                embed_entity
            );
        } else {
            bevy::log::warn!(
                "[Lifecycle] Embed {} not found for content '{}', marker not added",
                layer.containing_embed_id,
                layer.label
            );
        }
    } else {
        // Regular layer - add as child of parent
        commands.entity(parent_entity).add_child(entity);
    }

    entity
}

/// Add visual components to an entity based on layer spec.
/// Uses UnifiedEffectMaterial for all effects (RTT-ready architecture).
#[allow(clippy::too_many_arguments)]
fn add_visual_components(
    commands: &mut Commands,
    shaders: &mut Assets<Shader>,
    meshes: &mut Assets<Mesh>,
    unified_materials: &mut Assets<crate::masked_sprite::UnifiedEffectMaterial>,
    sdf_shaders: &crate::sdf::AmSdfShaders,
    entity: Entity,
    spec: &AmLayerSpec,
    mask_info: &Option<AmMaskInfo>,
    images: &HashMap<String, Handle<Image>>,
    fonts: &HashMap<String, Handle<Font>>,
    white_pixel: Option<&Handle<Image>>,
    label: &str,
    id: u64,
    initial_scale: (f32, f32),
    wipe_params: Option<Vec4>,
    stretch_params: Option<Vec4>,
    embed_scene_size: Option<(f32, f32)>,
    size_scale: f32,
) {
    use crate::masked_sprite::{UnifiedEffectMarker, UnifiedEffectMaterial};
    use bevy::mesh::Mesh2d;

    // Determine which effects are needed
    let needs_stretch = stretch_params.is_some();
    let needs_wipe = wipe_params.is_some();
    let needs_mask = mask_info.is_some();
    let needs_any_effect = needs_stretch || needs_wipe || needs_mask;

    // Helper function to create a rectangle mesh with anchor offset
    fn create_anchored_rectangle(
        meshes: &mut Assets<Mesh>,
        width: f32,
        height: f32,
        anchor: &bevy::sprite::Anchor,
    ) -> Handle<Mesh> {
        let anchor_vec = anchor.as_vec();
        let offset_x = -anchor_vec.x * width;
        let offset_y = -anchor_vec.y * height;
        let half_w = width / 2.0;
        let half_h = height / 2.0;

        let vertices = vec![
            [offset_x - half_w, offset_y - half_h, 0.0],
            [offset_x + half_w, offset_y - half_h, 0.0],
            [offset_x + half_w, offset_y + half_h, 0.0],
            [offset_x - half_w, offset_y + half_h, 0.0],
        ];

        let normals = vec![
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ];

        let uvs = vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

        let indices = vec![0, 1, 2, 0, 2, 3];

        let mut mesh = Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_indices(bevy::mesh::Indices::U32(indices));

        meshes.add(mesh)
    }

    // Helper to create unified material with effects
    fn create_unified_material(
        unified_materials: &mut Assets<UnifiedEffectMaterial>,
        texture: Handle<Image>,
        color: LinearRgba,
        width: f32,
        height: f32,
        mask_info: &Option<AmMaskInfo>,
        wipe_params: Option<Vec4>,
        stretch_params: Option<Vec4>,
    ) -> Handle<UnifiedEffectMaterial> {
        let mut material = UnifiedEffectMaterial {
            color,
            effect_flags: Vec4::ZERO,
            mask_params: Vec4::new(0.0, 0.0, 10000.0, 10000.0),
            wipe_params: Vec4::new(0.0, 1.0, 0.0, 0.0),
            stretch_params: Vec4::ZERO,
            original_size: Vec4::new(width, height, width, height),
            mesh_offset: Vec4::ZERO,
            texture: Some(texture),
        };

        // Enable mask if present
        if let Some(mask) = mask_info {
            material.effect_flags.x = 1.0;
            material.mask_params = Vec4::new(
                mask.center.x,
                mask.center.y,
                mask.half_size.x,
                mask.half_size.y,
            );
        }

        // Enable wipe if present
        if let Some(wp) = wipe_params {
            material.effect_flags.y = 1.0;
            material.wipe_params = wp;
        }

        // Enable stretch if present
        if let Some(sp) = stretch_params {
            material.effect_flags.z = 1.0;
            material.stretch_params = sp;
        }

        unified_materials.add(material)
    }

    match spec {
        AmLayerSpec::SpriteShape {
            image_uri,
            is_media,
            fill_color,
            width,
            height,
            anchor,
        } => {
            // Apply size_scale for embed children to compensate for fit_scale
            let base_width = *width * size_scale;
            let base_height = *height * size_scale;

            // For effects, use scaled dimensions (scale is applied to mesh, not transform)
            let scaled_width = base_width * initial_scale.0;
            let scaled_height = base_height * initial_scale.1;

            if *is_media && !image_uri.is_empty() {
                if let Some(handle) = images.get(image_uri) {
                    if needs_any_effect {
                        // Use UnifiedEffectMaterial for all effect cases
                        // Create mesh with SCALED dimensions since effects need actual pixel sizes
                        let mesh =
                            create_anchored_rectangle(meshes, scaled_width, scaled_height, anchor);
                        let material = create_unified_material(
                            unified_materials,
                            handle.clone(),
                            LinearRgba::WHITE,
                            scaled_width,
                            scaled_height,
                            mask_info,
                            wipe_params,
                            stretch_params,
                        );

                        // Scale is already baked into mesh in spawn_layer_entity
                        commands.entity(entity).insert((
                            Mesh2d(mesh),
                            MeshMaterial2d(material),
                            UnifiedEffectMarker,
                            AmVisualSpawned,
                        ));

                        bevy::log::info!(
                            "[Visual] Spawned sprite '{}' with unified effect: base_size=({:.1},{:.1}), scaled=({:.1},{:.1}), mask={}, wipe={}, stretch={}",
                            label,
                            base_width,
                            base_height,
                            scaled_width,
                            scaled_height,
                            needs_mask,
                            needs_wipe,
                            needs_stretch
                        );
                    } else {
                        // No effects - use normal sprite
                        commands.entity(entity).insert((
                            Sprite {
                                image: handle.clone(),
                                color: Color::WHITE,
                                custom_size: Some(Vec2::new(base_width, base_height)),
                                ..default()
                            },
                            *anchor,
                            AmVisualSpawned,
                        ));
                    }
                }
            } else if let Some(wp) = white_pixel {
                let color = extract_fill_color(fill_color);
                if needs_any_effect {
                    let mesh =
                        create_anchored_rectangle(meshes, scaled_width, scaled_height, anchor);
                    let material = create_unified_material(
                        unified_materials,
                        wp.clone(),
                        color.to_linear(),
                        scaled_width,
                        scaled_height,
                        mask_info,
                        wipe_params,
                        stretch_params,
                    );

                    // Reset entity's scale to 1 since we baked it into the mesh
                    // Scale is already baked into mesh in spawn_layer_entity
                    commands.entity(entity).insert((
                        Mesh2d(mesh),
                        MeshMaterial2d(material),
                        UnifiedEffectMarker,
                        AmVisualSpawned,
                    ));

                    bevy::log::info!(
                        "[Visual] Spawned fill sprite '{}' with unified effect: scaled=({:.1},{:.1})",
                        label,
                        scaled_width,
                        scaled_height
                    );
                } else {
                    commands.entity(entity).insert((
                        Sprite {
                            image: wp.clone(),
                            color,
                            custom_size: Some(Vec2::new(base_width, base_height)),
                            ..default()
                        },
                        *anchor,
                        AmVisualSpawned,
                    ));
                }
            }
        }
        AmLayerSpec::SdfShape {
            fill_color,
            stroke_color_value,
            stroke_width,
            stroke_join,
            width,
            height,
            pivot_x,
            pivot_y,
            shape_type,
        } => {
            spawn_sdf_visual(
                commands,
                shaders,
                sdf_shaders,
                entity,
                fill_color,
                stroke_color_value,
                *stroke_width,
                stroke_join,
                *width,
                *height,
                *pivot_x,
                *pivot_y,
                shape_type,
                &AmLayerMarker {
                    id,
                    label: label.to_string(),
                },
                initial_scale,
            );
        }
        AmLayerSpec::Image {
            image_uri,
            width,
            height,
            anchor,
        } => {
            // Apply size_scale for embed children to compensate for fit_scale
            let base_width = *width * size_scale;
            let base_height = *height * size_scale;

            // For effects, use scaled dimensions
            let scaled_width = base_width * initial_scale.0;
            let scaled_height = base_height * initial_scale.1;

            if let Some(handle) = images.get(image_uri) {
                if needs_any_effect {
                    let mesh =
                        create_anchored_rectangle(meshes, scaled_width, scaled_height, anchor);
                    let material = create_unified_material(
                        unified_materials,
                        handle.clone(),
                        LinearRgba::WHITE,
                        scaled_width,
                        scaled_height,
                        mask_info,
                        wipe_params,
                        stretch_params,
                    );

                    // Scale is already baked into mesh in spawn_layer_entity
                    commands.entity(entity).insert((
                        Mesh2d(mesh),
                        MeshMaterial2d(material),
                        UnifiedEffectMarker,
                        AmVisualSpawned,
                    ));

                    bevy::log::info!(
                        "[Visual] Spawned image '{}' with unified effect: scaled=({:.1},{:.1})",
                        label,
                        scaled_width,
                        scaled_height
                    );
                } else {
                    commands.entity(entity).insert((
                        Sprite {
                            image: handle.clone(),
                            color: Color::WHITE,
                            custom_size: Some(Vec2::new(base_width, base_height)),
                            ..default()
                        },
                        *anchor,
                        AmVisualSpawned,
                    ));
                }
            }
        }
        AmLayerSpec::Text {
            content,
            font_name,
            font_size,
            align,
            fill_color,
        } => {
            use bevy::text::Justify;

            let color = extract_fill_color(fill_color);
            let justify = match align.as_str() {
                "center" => Justify::Center,
                "right" => Justify::Right,
                _ => Justify::Left,
            };

            let font = fonts
                .get(font_name)
                .cloned()
                .unwrap_or_else(Handle::default);

            commands.entity(entity).insert((
                Text2d::new(content.clone()),
                TextFont {
                    font,
                    font_size: *font_size,
                    ..default()
                },
                TextLayout::new_with_justify(justify),
                TextColor(color),
                bevy::sprite::Anchor(Vec2::new(-0.5, 0.0)),
                AmVisualSpawned,
            ));
        }
        AmLayerSpec::Null => {
            commands.entity(entity).insert(AmVisualSpawned);
        }
        AmLayerSpec::EmbedScene => {
            // Add RTT setup marker if scene size is available
            if let Some((width, height)) = embed_scene_size {
                commands.entity(entity).insert((
                    crate::effects::NeedsEmbedSceneRtt {
                        scene_width: width,
                        scene_height: height,
                    },
                    AmVisualSpawned,
                ));
            } else {
                commands.entity(entity).insert(AmVisualSpawned);
            }
        }
    }
}

/// Extract fill color from AmFillColor.
fn extract_fill_color(fill_color: &Option<crate::schema::AmFillColor>) -> Color {
    if let Some(fc) = fill_color {
        if !fc.value.is_empty() {
            if let Ok(c) = crate::schema::parse_color(&fc.value) {
                return Color::srgba(c[0], c[1], c[2], c[3]);
            }
        } else if !fc.keyframes.is_empty() {
            let mut sorted: Vec<_> = fc.keyframes.iter().collect();
            sorted.sort_by(|a, b| {
                a.time
                    .partial_cmp(&b.time)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            if let Ok(c) = crate::schema::parse_color(&sorted[0].value) {
                return Color::srgba(c[0], c[1], c[2], c[3]);
            }
        }
    }
    Color::WHITE
}

/// Spawn SDF visual components as children of the layer entity.
///
/// ## AM Behavior (what we're matching)
/// AM draws stroked rectangles by:
/// 1. Drawing a base 100x100 square with stroke
/// 2. Applying scale transform to stretch it
/// 3. Stroke width remains constant (not scaled)
///
/// ## Our Implementation
/// 1. Use a fixed 50x50 half-extent SDF box as the base
/// 2. Calculate the scale to stretch it to target dimensions
/// 3. Apply scale as a child transform (SDF entity)
/// 4. Use stroked_fill shader with constant stroke_width parameter
///
/// ## bevy_smud Non-Uniform Scale Limitation
/// bevy_smud only supports uniform scaling (single f32 scale value extracted from Transform).
/// To achieve non-uniform scaling, we use a **parametric SDF** that reads dimensions from params:
/// - params.x = half_width
/// - params.y = half_height  
/// - params.z = stroke_width
/// - params.w = packed_stroke_color
///
/// The frame size is calculated to encompass the shape + stroke at maximum expected scale.
#[allow(clippy::too_many_arguments)]
fn spawn_sdf_visual(
    commands: &mut Commands,
    shaders: &mut Assets<Shader>,
    sdf_shaders: &crate::sdf::AmSdfShaders,
    parent_entity: Entity,
    fill_color: &Option<crate::schema::AmFillColor>,
    stroke_color_value: &str,
    stroke_width: f32,
    stroke_join: &str,
    width: f32,
    height: f32,
    pivot_x: f32,
    pivot_y: f32,
    shape_type: &str,
    marker: &AmLayerMarker,
    initial_scale: (f32, f32),
) {
    use crate::sdf::pack_color;

    let fill = extract_fill_color(fill_color);
    let stroke = if !stroke_color_value.is_empty() {
        crate::schema::parse_color(stroke_color_value)
            .map(|c| Color::srgba(c[0], c[1], c[2], c[3]))
            .unwrap_or(Color::WHITE)
    } else {
        Color::WHITE
    };

    // Target dimensions from shape properties (base size before animation scale)
    let target_half_width = width / 2.0;
    let target_half_height = height / 2.0;

    // Use a dummy SDF expression because our custom fill shader handles distance calculation explicitly.
    // We pass 0.0 to satisfy the parser, but it won't be used for distance.
    // However, we must ensure 'params' is available in the shader context.
    // bevy_smud generates: fn sdf(p, params) -> f32 { <EXPR> }
    // We can just use a simple expression like "0.0" as we don't use the result 'd' in our fill shader.
    let parametric_sdf = shaders.add_sdf_expr("0.0");

    // Select the correct fill shader based on shape type and stroke join
    // Current mapping:
    // .circle -> stroked_fill_circle (ignores join type as circles naturally have round joins)
    // .rect ->
    //   - join="miter" -> stroked_fill_box_miter (Square corners)
    //   - join="round" -> stroked_fill_box (Round corners)
    //   - join="bevel" -> stroked_fill_box_bevel (Bevel corners)
    //   - join="" (default) -> stroked_fill_box_bevel (Assume bevel if unspecified, per user request)
    let stroked_fill = if shape_type == ".circle" {
        sdf_shaders
            .stroked_fill_circle
            .clone()
            .expect("Circle fill shader not initialized")
    } else {
        match stroke_join {
            "miter" => sdf_shaders
                .stroked_fill_box_miter
                .clone()
                .expect("Box miter shader not initialized"),
            "round" => sdf_shaders
                .stroked_fill_box
                .clone()
                .expect("Box round shader not initialized"),
            "bevel" | "" => sdf_shaders
                .stroked_fill_box_bevel
                .clone()
                .expect("Box bevel shader not initialized"),
            _ => sdf_shaders
                .stroked_fill_box
                .clone()
                .expect("Box default shader not initialized"),
        }
    };

    bevy::log::trace!("[SDF] Spawning {} with join='{}'", shape_type, stroke_join);

    // Pack stroke color into u32 bits stored as f32
    let packed_stroke = pack_color(stroke);

    // Frame size for rendering - must be large enough for the largest expected shape.
    // Since we scale via params, the frame needs to accommodate the max size + stroke.
    // We use a conservative estimate based on the target size * reasonable max scale factor.
    // AM animations typically don't exceed 10x scale, so use that as a safety margin.
    let max_scale_factor = 10.0;
    let frame_half =
        (target_half_width.max(target_half_height) * max_scale_factor) + stroke_width * 2.0;
    let frame_size = frame_half * 2.0;

    // Calculate initial translation based on pivot and initial scale (with Y-flip for Bevy)
    // Pivot (px, py) in AM means Center is at (-px, -py) relative to Pivot.
    // Bevy Y is flipped, so Center Y is -(-py) = py.
    // Apply initial scale to the pivot offset so the child is correctly positioned from the start.
    let initial_translation = Vec3::new(-pivot_x * initial_scale.0, pivot_y * initial_scale.1, 0.0);

    // Spawn single SDF entity with both fill and stroke
    // Note: Transform.scale is set to 1.0 because we handle sizing via params
    // SDF child uses default transform (z=0 relative to parent) - z-ordering is handled by parent's z value
    let sdf_entity = commands
        .spawn((
            Name::new(format!("SdfShape[{}]: {}", marker.id, marker.label)),
            Transform::from_translation(initial_translation),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            SmudShape {
                color: fill,
                sdf: parametric_sdf,
                frame: Frame::Quad(frame_size),
                fill: stroked_fill,
                // params: half_width, half_height, stroke_width, packed_stroke_color
                params: Vec4::new(
                    target_half_width,
                    target_half_height,
                    stroke_width,
                    packed_stroke,
                ),
                ..default()
            },
            // Store base params for animation
            AmSdfParams {
                base_half_width: target_half_width,
                base_half_height: target_half_height,
                stroke_width,
                packed_stroke,
                base_pivot_x: pivot_x,
                base_pivot_y: pivot_y,
            },
        ))
        .id();

    // Add as child and mark parent
    commands
        .entity(parent_entity)
        .add_child(sdf_entity)
        .insert((AmVisualSpawned, AmSdfShapeParent));

    bevy::log::info!(
        "[SDF] Created shape for '{}': size={}x{}, stroke_width={}, frame={}, pivot=({:.1},{:.1}), initial_scale=({:.2},{:.2}), initial_translation=({:.1},{:.1},{:.1})",
        marker.label,
        width,
        height,
        stroke_width,
        frame_size,
        pivot_x,
        pivot_y,
        initial_scale.0,
        initial_scale.1,
        initial_translation.x,
        initial_translation.y,
        initial_translation.z
    );
}

/// Component to store SDF shape parameters for animation.
/// Used by animate_sdf_scale to update SmudShape.params based on animation scale.
#[derive(Component, Debug, Clone)]
pub struct AmSdfParams {
    /// Base half width of the shape (before animation scale)
    pub base_half_width: f32,
    /// Base half height of the shape (before animation scale)
    pub base_half_height: f32,
    /// Stroke width in pixels (constant, not scaled)
    pub stroke_width: f32,
    /// Packed stroke color (stored to preserve during updates)
    pub packed_stroke: f32,
    /// Base pivot X in pixels
    pub base_pivot_x: f32,
    /// Base pivot Y in pixels
    pub base_pivot_y: f32,
}

// Keep legacy types for now to avoid breaking changes in case they're referenced elsewhere
/// Component to store original SDF fill parameters for animation.
/// @deprecated Use AmSdfParams instead
#[derive(Component, Debug, Clone)]
pub struct AmSdfFillParams {
    /// Base half width of the shape (without scale)
    pub base_half_width: f32,
    /// Base half height of the shape (without scale)
    pub base_half_height: f32,
    /// Half of the stroke width (used to inset the fill)
    pub stroke_half_width: f32,
}

/// Component to store original SDF stroke parameters for animation.
/// @deprecated Use AmSdfParams instead
#[derive(Component, Debug, Clone)]
pub struct AmSdfStrokeParams {
    /// Base half width of the shape (without scale)
    pub base_half_width: f32,
    /// Base half height of the shape (without scale)
    pub base_half_height: f32,
    /// Half of the stroke width (used to offset the stroke)
    pub stroke_half_width: f32,
}

/// Marker component to identify entities that are SDF shape parents.
/// Used to skip scale animation in animate_transform (scale is handled by animate_sdf_scale).
#[derive(Component, Debug, Clone, Default)]
pub struct AmSdfShapeParent;

/// System to apply mask clipping to layers that have an AmMaskInfo component.
/// This system checks if the sprite/layer is within the mask bounds and hides it if outside.
/// Note: This is a simplified implementation that only checks the sprite center against the mask.
/// For precise pixel-level masking, a custom shader would be needed.
pub fn apply_mask_clipping_system(
    mut query: Query<(
        &GlobalTransform,
        &ChildOf,
        &AmMaskInfo,
        &mut Visibility,
        &AmLayerMarker,
    )>,
    parent_query: Query<&GlobalTransform>,
) {
    for (global_transform, parent, mask_info, mut visibility, marker) in query.iter_mut() {
        let world_pos: Vec3 = global_transform.translation();

        // Calculate position relative to parent (mask coordinate space)
        let local_pos = if let Ok(parent_transform) = parent_query.get(parent.get()) {
            parent_transform
                .to_matrix()
                .inverse()
                .transform_point3(world_pos)
                .truncate()
        } else {
            world_pos.truncate()
        };

        // Check if sprite center is inside the mask rectangle
        // Note: This doesn't account for mask rotation, treating it as axis-aligned
        let rel_pos = local_pos - mask_info.center;
        let inside_mask =
            rel_pos.x.abs() <= mask_info.half_size.x && rel_pos.y.abs() <= mask_info.half_size.y;

        // Update visibility based on mask check
        if inside_mask {
            if *visibility == Visibility::Hidden {
                *visibility = Visibility::Inherited;
                bevy::log::trace!("[MASK] Layer '{}' now visible (inside mask)", marker.label);
            }
        } else if *visibility != Visibility::Hidden {
            *visibility = Visibility::Hidden;
            bevy::log::trace!(
                "[MASK] Layer '{}' hidden (outside mask at {:.1},{:.1})",
                marker.label,
                world_pos.x,
                world_pos.y
            );
        }
    }
}

// ============================================================================
// Unified Effect Material Animation System
// ============================================================================

/// System to animate effects on sprites using UnifiedEffectMaterial.
/// This system handles all effect types (wipe, stretch segment, mask) in a single pass.
/// It is designed for the RTT architecture where effects are stackable.
pub fn animate_unified_effect_system(
    playback: Res<AmPlayback>,
    mut commands: Commands,
    query: Query<(
        Entity,
        &AmAnimated,
        &MeshMaterial2d<crate::masked_sprite::UnifiedEffectMaterial>,
        &Transform,
    )>,
    mut materials: ResMut<Assets<crate::masked_sprite::UnifiedEffectMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (entity, animated, material_handle, transform) in query.iter() {
        // Calculate local time
        let local_time = (global_time - animated.time_offset as f32) * animated.speed_multiplier;
        if local_time < animated.start_time as f32 || local_time > animated.end_time as f32 {
            continue;
        }

        let layer_duration = (animated.end_time - animated.start_time) as f32;
        let layer_time = (local_time - animated.start_time as f32) / layer_duration;

        // Get sprite base size and scale
        let sprite_size = interpolate_vec2(&animated.size, layer_time).unwrap_or([100.0, 100.0]);
        let scale = interpolate_vec2(&animated.scale, layer_time).unwrap_or([1.0, 1.0]);
        // Actual rendered size = base size * scale
        // Use abs() because negative size in AM behaves same as positive (no flip)
        let orig_width = (sprite_size[0] * scale[0]).abs().max(1.0);
        let orig_height = (sprite_size[1] * scale[1]).abs().max(1.0);

        // Get transform rotation angle for effect compensation
        // In Bevy, rotation is stored as Quat, extract Z rotation
        let (_, _, transform_rotation_rad) = transform.rotation.to_euler(bevy::math::EulerRot::XYZ);

        // Calculate "world-space" dimensions for stretch calculations
        // When element is rotated, its local width/height swap in world space
        let rot_cos = transform_rotation_rad.cos().abs();
        let rot_sin = transform_rotation_rad.sin().abs();
        let world_width = orig_width * rot_cos + orig_height * rot_sin;
        let world_height = orig_width * rot_sin + orig_height * rot_cos;

        // Check which effects are active
        let has_wipe = animated.wipe_end.value != Some(1.0)
            || !animated.wipe_end.keyframes.is_empty()
            || animated.wipe_start.value.is_some()
            || !animated.wipe_start.keyframes.is_empty();

        let has_stretch = animated.stretch_amount.value.is_some()
            || !animated.stretch_amount.keyframes.is_empty()
            || animated.stretch_angle.value.is_some()
            || !animated.stretch_angle.keyframes.is_empty()
            || animated.stretch_offset.value.is_some()
            || !animated.stretch_offset.keyframes.is_empty()
            || animated.stretch_smooth.value.is_some()
            || !animated.stretch_smooth.keyframes.is_empty();

        if let Some(material) = materials.get_mut(&material_handle.0) {
            // Update wipe parameters if needed
            if has_wipe {
                material.set_wipe_enabled(true);
                let wipe_start = interpolate_float(&animated.wipe_start, layer_time).unwrap_or(0.0);
                let wipe_end = interpolate_float(&animated.wipe_end, layer_time).unwrap_or(1.0);
                let wipe_angle = interpolate_float(&animated.wipe_angle, layer_time).unwrap_or(0.0);
                let wipe_feather =
                    interpolate_float(&animated.wipe_feather, layer_time).unwrap_or(0.0);
                material.wipe_params = Vec4::new(wipe_start, wipe_end, wipe_angle, wipe_feather);
            } else {
                material.set_wipe_enabled(false);
            }

            // Update stretch segment parameters if needed
            if has_stretch {
                material.set_stretch_enabled(true);

                let angle_deg =
                    interpolate_float(&animated.stretch_angle, layer_time).unwrap_or(0.0);
                // Compensate for transform rotation: subtract transform rotation from effect angle
                // This ensures the stretch effect is applied in world space, not local space
                // Note: transform rotation is already negated in animate_transform_system (for Bevy's coord system)
                // So we add it back here to get the original AM rotation value
                let angle_rad = angle_deg.to_radians() + transform_rotation_rad;
                let stretch_px =
                    interpolate_float(&animated.stretch_amount, layer_time).unwrap_or(0.0);
                let offset_px =
                    interpolate_float(&animated.stretch_offset, layer_time).unwrap_or(0.0);
                let smooth = interpolate_float(&animated.stretch_smooth, layer_time).unwrap_or(0.0);
                let smooth_width = smooth * 0.3;

                // Calculate mesh expansion (same logic as animate_stretch_segment_system)
                // Use world_width for base_divisor calculation because AM's stretch effect
                // operates in world space, not local space
                let base_divisor = world_width / 5.76;
                let stretch_factor = 1.0 + stretch_px / base_divisor;
                let actual_stretch_px = orig_width * stretch_factor - orig_width;

                let angle_factor = 1.0 - 0.25 * angle_rad.sin().abs();
                let half_gap = actual_stretch_px * 0.5 * angle_factor;

                let rotate = |x: f32, y: f32, angle: f32| -> (f32, f32) {
                    let c = angle.cos();
                    let s = angle.sin();
                    (x * c - y * s, x * s + y * c)
                };

                let transform_vertex = |vx: f32, vy: f32| -> (f32, f32) {
                    let (rx, ry) = rotate(vx, vy, angle_rad);
                    let shifted_x = rx + offset_px;
                    let pushed_x = rx + shifted_x.signum() * half_gap;
                    rotate(pushed_x, ry, -angle_rad)
                };

                let hw = orig_width / 2.0;
                let hh = orig_height / 2.0;
                let corners = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)];

                let mut min_x = f32::MAX;
                let mut max_x = f32::MIN;
                let mut min_y = f32::MAX;
                let mut max_y = f32::MIN;

                for (cx, cy) in corners {
                    let (tx, ty) = transform_vertex(cx, cy);
                    min_x = min_x.min(tx);
                    max_x = max_x.max(tx);
                    min_y = min_y.min(ty);
                    max_y = max_y.max(ty);
                }

                let new_width = max_x - min_x;
                let new_height = max_y - min_y;
                let center_offset_x = (min_x + max_x) / 2.0;
                let center_offset_y = (min_y + max_y) / 2.0;

                // Update material parameters
                material.stretch_params =
                    Vec4::new(angle_rad, actual_stretch_px, offset_px, smooth_width);
                material.original_size = Vec4::new(orig_width, orig_height, new_width, new_height);
                material.mesh_offset = Vec4::new(center_offset_x, center_offset_y, 0.0, 0.0);

                // Create new mesh with expanded bounds
                let vertices = vec![
                    [min_x, min_y, 0.0],
                    [max_x, min_y, 0.0],
                    [max_x, max_y, 0.0],
                    [min_x, max_y, 0.0],
                ];
                let normals = vec![
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0],
                ];
                let uvs = vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
                let indices = vec![0u32, 1, 2, 0, 2, 3];

                let mut new_mesh = Mesh::new(
                    bevy::mesh::PrimitiveTopology::TriangleList,
                    bevy::asset::RenderAssetUsages::RENDER_WORLD
                        | bevy::asset::RenderAssetUsages::MAIN_WORLD,
                );
                new_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
                new_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                new_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                new_mesh.insert_indices(bevy::mesh::Indices::U32(indices));

                let new_mesh_handle = meshes.add(new_mesh);
                commands
                    .entity(entity)
                    .insert(bevy::mesh::Mesh2d(new_mesh_handle));
            } else {
                material.set_stretch_enabled(false);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_keyframe(t: f32, v: &str, e: Option<&str>) -> AmKeyframe {
        AmKeyframe {
            time: t,
            value: v.to_string(),
            easing: e.map(String::from),
        }
    }

    #[test]
    fn test_interpolate_float_static() {
        let prop = AmAnimatedFloat {
            value: Some(0.5),
            keyframes: vec![],
        };
        assert_eq!(interpolate_float(&prop, 0.0), Some(0.5));
        assert_eq!(interpolate_float(&prop, 0.5), Some(0.5));
        assert_eq!(interpolate_float(&prop, 1.0), Some(0.5));
    }

    #[test]
    fn test_interpolate_float_linear() {
        let prop = AmAnimatedFloat {
            value: None,
            keyframes: vec![
                make_keyframe(0.0, "0.0", None),
                make_keyframe(1.0, "1.0", None),
            ],
        };

        let v = interpolate_float(&prop, 0.0).unwrap();
        assert!((v - 0.0).abs() < 0.001);

        let v = interpolate_float(&prop, 0.5).unwrap();
        assert!((v - 0.5).abs() < 0.001);

        let v = interpolate_float(&prop, 1.0).unwrap();
        assert!((v - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_interpolate_float_step() {
        // Easing is on the target keyframe (describes how to arrive at it)
        let prop = AmAnimatedFloat {
            value: None,
            keyframes: vec![
                make_keyframe(0.0, "1.0", None),
                make_keyframe(1.0, "0.0", Some("step 1.0 0.0")),
            ],
        };

        let v = interpolate_float(&prop, 0.0).unwrap();
        assert!((v - 1.0).abs() < 0.001, "At t=0.0, expected 1.0, got {}", v);

        let v = interpolate_float(&prop, 0.5).unwrap();
        assert!(
            (v - 1.0).abs() < 0.001,
            "At t=0.5, expected 1.0 (step), got {}",
            v
        );

        let v = interpolate_float(&prop, 0.99).unwrap();
        assert!(
            (v - 1.0).abs() < 0.001,
            "At t=0.99, expected 1.0 (step), got {}",
            v
        );
    }

    #[test]
    fn test_interpolate_vec3_linear() {
        let prop = AmAnimatedVec3 {
            value: None,
            keyframes: vec![
                make_keyframe(0.0, "0.0,0.0,0.0", None),
                make_keyframe(1.0, "100.0,200.0,0.0", None),
            ],
        };

        let v = interpolate_vec3(&prop, 0.5).unwrap();
        assert!((v[0] - 50.0).abs() < 0.1);
        assert!((v[1] - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_interpolate_boundary() {
        let prop = AmAnimatedFloat {
            value: None,
            keyframes: vec![
                make_keyframe(0.2, "0.0", None),
                make_keyframe(0.8, "1.0", None),
            ],
        };

        // Before first keyframe
        let v = interpolate_float(&prop, 0.0).unwrap();
        assert!((v - 0.0).abs() < 0.001);

        // After last keyframe
        let v = interpolate_float(&prop, 1.0).unwrap();
        assert!((v - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_interpolate_cubic_bezier() {
        // Easing is on the target keyframe (describes how to arrive at it)
        let prop = AmAnimatedFloat {
            value: None,
            keyframes: vec![
                make_keyframe(0.0, "0.0", None),
                make_keyframe(1.0, "100.0", Some("cubicBezier 0.0 0.0 0.58 1.0")),
            ],
        };

        let v_mid = interpolate_float(&prop, 0.5).unwrap();
        // ease-out should be faster at the start, so at t=0.5, value should be > 50
        assert!(v_mid > 50.0, "Expected > 50.0 for ease-out, got {}", v_mid);
    }
}

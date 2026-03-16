//! # spawn_embed.rs
//!
//! Embedded scene spawning.
//! 嵌入场景（EmbedScene）的实体生成。

use bevy::asset::Assets;
use bevy::prelude::*;
use std::collections::HashMap;

use crate::animation::AmAnimated;
use crate::effects::{AmGroupFill, GroupFillType, NeedsStrategyEvaluation};
use crate::loader::FontMetrics;
use crate::schema::{AmAnimatedFloat, AmAnimatedVec2};
use crate::sdf::AmSdfShaders;

use super::components::*;
use super::effects::*;
use super::helpers::*;
use super::spawn::spawn_scene;

/// Parse a color string into a linear `Vec4`, returning `fallback` on failure.
fn parse_color_to_linear_vec4(color_str: &str, fallback: Vec4) -> Vec4 {
    let Ok(c) = crate::schema::parse_color(color_str) else {
        return fallback;
    };
    let srgb = bevy::color::Color::srgba(c[0], c[1], c[2], c[3]);
    let linear = srgb.to_linear();
    Vec4::new(linear.red, linear.green, linear.blue, linear.alpha)
}

/// Spawn an embedded scene.
pub(crate) fn spawn_embed_scene(
    commands: &mut Commands,
    shaders: &mut Assets<Shader>,
    embed: &crate::schema::AmEmbedScene,
    images: &HashMap<String, Handle<Image>>,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    white_pixel: &Handle<Image>,
    sdf_shaders: &AmSdfShaders,
    config: &AmSceneConfig,
    z: f32,
) -> Entity {
    let has_parent = embed.parent != 0;
    let (mut tx, mut ty) = get_initial_location(&embed.transform.location, config, has_parent);
    let mut rotation = get_initial_rotation(&embed.transform.rotation);
    let (mut sx, mut sy) = get_initial_scale(&embed.transform.scale);
    let pivot = get_initial_pivot(&embed.transform.pivot);

    // Apply repeat effect transform offsets (if any)
    // Rotation: additive (AM degrees, negate for Bevy)
    rotation += -config.repeat_rotation_deg;
    // Scale: multiplicative
    sx *= config.repeat_scale_factor;
    sy *= config.repeat_scale_factor;

    // Apply pivot compensation for initial position (uses modified rotation/scale)
    let (comp_x, comp_y) = calculate_pivot_compensation(pivot, (sx, sy), rotation, has_parent);
    tx += comp_x;
    ty += comp_y;

    // Apply repeat position offset (already in Bevy Y-up coords)
    tx += config.repeat_offset.x;
    ty += config.repeat_offset.y;

    bevy::log::trace!(
        "Registering embedScene '{}' (id={}, parent={}): pos=({:.1},{:.1}), pivot=({:.1},{:.1}), scale=({:.2},{:.2}), start_time={}, time_offset={}",
        embed.label,
        embed.id,
        embed.parent,
        tx,
        ty,
        pivot.0,
        pivot.1,
        sx,
        sy,
        embed.start_time,
        config.time_offset
    );

    // Extract transform2 effects from embed
    let mut all_embed_transform2 = extract_all_transform2_effects(&embed.effects);
    bevy::log::info!(
        "[EMBED_T2] '{}' (id={}): {} effects parsed, {} transform2 extracted, primary posz kf={}",
        embed.label,
        embed.id,
        embed.effects.len(),
        all_embed_transform2.len(),
        all_embed_transform2
            .first()
            .map(|t| t.pos_z.keyframes.len())
            .unwrap_or(0)
    );
    let embed_transform2 = if all_embed_transform2.is_empty() {
        Transform2Params::default()
    } else {
        all_embed_transform2.remove(0)
    };
    let embed_extra_transform2 = all_embed_transform2;
    let fade_effect = extract_fade_effect(&embed.effects);
    let wavewarp2_effect = extract_wavewarp2_effect(&embed.effects);
    let mirror_effect = extract_mirror_effect(&embed.effects);
    let lift_effect = extract_lift_effect(&embed.effects);
    let rays_effect = extract_rays_effect(&embed.effects);

    let transform = Transform {
        translation: Vec3::new(tx, ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    // Create entity name for inspector identification
    let entity_name = format!("Embed[{}]: {}", embed.id, embed.label);

    let entity = commands
        .spawn((
            Name::new(entity_name),
            AmLayerMarker {
                id: embed.id,
                label: embed.label.clone(),
            },
            AmAnimated {
                layer_id: embed.id,
                start_time: embed.start_time,
                end_time: embed.end_time,
                time_offset: config.time_offset,
                lifecycle_offset: config.lifecycle_offset,
                location: embed.transform.location.clone(),
                pivot: embed.transform.pivot.clone(),
                rotation: embed.transform.rotation.clone(),
                scale: embed.transform.scale.clone(),
                opacity: embed.transform.opacity.clone(),
                canvas_width: config.canvas_width,
                canvas_height: config.canvas_height,
                has_parent,
                parent_layer_id: embed.parent,
                effect_pos_x: embed_transform2.pos_x,
                effect_pos_y: embed_transform2.pos_y,
                effect_posz: embed_transform2.pos_z,
                effect_angle: embed_transform2.angle,
                effect_xinv: embed_transform2.xinv,
                effect_yinv: embed_transform2.yinv,
                effect_zinv: embed_transform2.zinv,
                effect_ainv: embed_transform2.ainv,
                extra_transform2: embed_extra_transform2,
                font_y_offset: 0.0,
                size: AmAnimatedVec2::default(),
                anchor_offset: Vec2::ZERO,
                wipe_start: AmAnimatedFloat::default(),
                wipe_end: AmAnimatedFloat {
                    value: Some(1.0),
                    keyframes: vec![],
                },
                wipe_angle: AmAnimatedFloat::default(),
                wipe_feather: AmAnimatedFloat::default(),
                stretch_angle: AmAnimatedFloat::default(),
                stretch_amount: AmAnimatedFloat::default(),
                stretch_offset: AmAnimatedFloat::default(),
                stretch_smooth: AmAnimatedFloat::default(),
                stretch_seg2_angle: AmAnimatedFloat::default(),
                stretch_seg2_amount: AmAnimatedFloat::default(),
                stretch_seg2_offset: AmAnimatedFloat::default(),
                stretch_seg2_smooth: AmAnimatedFloat::default(),
                blur_strength: AmAnimatedFloat::default(),
                speed_multiplier: config.speed_multiplier,
                element_speed: 1.0,
                scene_fps: config.scene_fps,
                embed_offset: Vec2::ZERO,
                inv_fit_scale: 1.0,
                stroke_width: AmAnimatedFloat::default(),
                base_alpha: get_base_alpha(&embed.fill_color, false) * config.repeat_alpha_factor,
                fade_in_time: fade_effect.in_time,
                fade_out_time: fade_effect.out_time,
                fade_layer_duration_ms: (embed.end_time - embed.start_time) as f32,
                palette_alpha: AmAnimatedFloat::default(),
                scale_assist: AmAnimatedFloat::default(),
                scale_assist_damp: AmAnimatedFloat::default(),
                scale_assist_axis: 0,
                stretch2_scale: AmAnimatedFloat::default(),
                stretch2_angle: AmAnimatedFloat::default(),
                stretch2_content_only: false,
                wavewarp2_phase: wavewarp2_effect.phase,
                wavewarp2_a1d: wavewarp2_effect.a1d,
                wavewarp2_m1: wavewarp2_effect.m1,
                wavewarp2_m2: wavewarp2_effect.m2,
                wavewarp2_a2d: wavewarp2_effect.a2d,
                wavewarp2_damping: wavewarp2_effect.damping,
                wavewarp2_damping_space: wavewarp2_effect.damping_space,
                wavewarp2_damping_origin: wavewarp2_effect.damping_origin,
                wavewarp2_screen_space: wavewarp2_effect.screen_space,
                wavewarp2_has_effect: wavewarp2_effect.has_effect,
                mirror_type: mirror_effect.mirror_type,
                mirror_blend_mode: mirror_effect.blend_mode,
                mirror_alpha: mirror_effect.alpha,
                mirror_offset: mirror_effect.offset,
                mirror_has_effect: mirror_effect.has_effect,
                lift_fill: lift_effect.fill,
                lift_has_effect: lift_effect.has_effect,
                rays_center_x: rays_effect.center_x,
                rays_center_y: rays_effect.center_y,
                rays_strength: rays_effect.strength,
                rays_intensity: rays_effect.intensity,
                rays_threshold: rays_effect.threshold,
                rays_threshold_color: rays_effect.threshold_color,
                rays_fill_color: rays_effect.fill_color,
                rays_blend: rays_effect.blend,
                rays_quality: rays_effect.quality,
                rays_has_effect: rays_effect.has_effect,
                replace_old_color: Vec4::ZERO,
                replace_new_color: crate::schema::AmAnimatedColor::default(),
                replace_threshold: AmAnimatedFloat::default(),
                replace_feather: AmAnimatedFloat::default(),
                replace_alpha: AmAnimatedFloat::default(),
                replace_lock_luminance: false,
                repeat_count: AmAnimatedFloat::default(),
                repeat_offset: AmAnimatedVec2::default(),
                repeat_angle: AmAnimatedFloat::default(),
                repeat_scale: AmAnimatedFloat::default(),
                repeat_alpha: AmAnimatedFloat::default(),
                // Linear repeat effect
                linear_repeat_count: AmAnimatedFloat::default(),
                linear_repeat_position: AmAnimatedVec2::default(),
                linear_repeat_offset: AmAnimatedVec2::default(),
                linear_repeat_angle: AmAnimatedFloat::default(),
                linear_repeat_scale: AmAnimatedFloat::default(),
                linear_repeat_alpha: AmAnimatedFloat::default(),
                linear_repeat_fill_color: crate::schema::AmAnimatedColor::default(),
                linear_repeat_blend: AmAnimatedFloat::default(),
                linear_repeat_color_alt_copies: false,
                linear_repeat_start: AmAnimatedFloat::default(),
                linear_repeat_end: AmAnimatedFloat {
                    value: Some(1.0),
                    keyframes: vec![],
                },
                linear_repeat_phase: AmAnimatedFloat::default(),
                linear_repeat_ease_in: AmAnimatedFloat::default(),
                linear_repeat_ease_out: AmAnimatedFloat::default(),
                linear_repeat_overlap: AmAnimatedFloat::default(),
                linear_repeat_shape: 0,
                linear_repeat_invert: false,
                linear_repeat_random_order: false,
                linear_repeat_seed: AmAnimatedFloat::default(),
                linear_repeat2: None,
                // Radial repeat effect (defaults for embed scene)
                radial_repeat_count: AmAnimatedFloat::default(),
                radial_repeat_radius: AmAnimatedFloat::default(),
                radial_repeat_orientation: AmAnimatedFloat::default(),
                radial_repeat_start_angle: AmAnimatedFloat::default(),
                radial_repeat_sweep: AmAnimatedFloat::default(),
                radial_repeat_base_scale: AmAnimatedFloat::default(),
                radial_repeat_offset: AmAnimatedVec2::default(),
                radial_repeat_angle: AmAnimatedFloat::default(),
                radial_repeat_scale: AmAnimatedFloat::default(),
                radial_repeat_alpha: AmAnimatedFloat::default(),
                radial_repeat_fill_color: crate::schema::AmAnimatedColor::default(),
                radial_repeat_blend: AmAnimatedFloat::default(),
                radial_repeat_color_alt_copies: false,
                radial_repeat_start: AmAnimatedFloat::default(),
                radial_repeat_end: AmAnimatedFloat {
                    value: Some(1.0),
                    ..Default::default()
                },
                radial_repeat_phase: AmAnimatedFloat::default(),
                radial_repeat_ease_in: AmAnimatedFloat::default(),
                radial_repeat_ease_out: AmAnimatedFloat::default(),
                radial_repeat_overlap: AmAnimatedFloat::default(),
                radial_repeat_shape: 0,
                radial_repeat_invert: false,
                radial_repeat_random_order: false,
                radial_repeat_seed: 0.0,
                // Swing effect (defaults for embed scene)
                swing_freq: AmAnimatedFloat::default(),
                swing_a1: AmAnimatedFloat::default(),
                swing_a2: AmAnimatedFloat::default(),
                swing_phase: AmAnimatedFloat::default(),
                swing_type: 0,
                // Oscillate effect (defaults for embed scene)
                oscillate_direction: 0,
                oscillate_angle: AmAnimatedFloat::default(),
                oscillate_freq: AmAnimatedFloat::default(),
                oscillate_mag: AmAnimatedFloat::default(),
                oscillate_wave_type: 0,
                oscillate_phase: AmAnimatedFloat::default(),
                // Spin effect (defaults for embed scene)
                spin_rpm: AmAnimatedFloat::default(),
                // Threshold effect (defaults for embed scene)
                threshold_value: AmAnimatedFloat::default(),
                threshold_feather: AmAnimatedFloat::default(),
                threshold_invert: false,
                threshold_blend_mode: 0,
                // Grid effect (defaults for embed scene)
                grid_position: AmAnimatedVec2::default(),
                grid_spacing: AmAnimatedFloat::default(),
                grid_width: AmAnimatedFloat::default(),
                grid_color: crate::schema::AmAnimatedColor::default(),
                grid_punchout: false,
                grid_smoothing: AmAnimatedFloat::default(),
                grid_screen_space: false,
                // Pixelate effect (defaults for embed scene)
                pixelate_size: AmAnimatedFloat::default(),
                pixelate_stretch: AmAnimatedVec2::default(),
                pixelate_angle: AmAnimatedFloat::default(),
                pixelate_vignette: AmAnimatedFloat::default(),
                pixelate_threshold: AmAnimatedFloat::default(),
                pixelate_saturation: AmAnimatedFloat::default(),
                pixelate_screen_space: false,
                solid_color: Default::default(),
                solid_color_alpha: Default::default(),
                solid_color_blend_mode: 0,
                base_fill_color: [0.0; 4],
                fill_color: Default::default(),
                path_repeat: None,
                textspacing_letter: Default::default(),
                textspacing_line: AmAnimatedFloat {
                    value: Some(1.0),
                    keyframes: vec![],
                },
                textprogress_start: Default::default(),
                textprogress_end: AmAnimatedFloat {
                    value: Some(1.0),
                    keyframes: vec![],
                },
                textprogress_cursor: 0,
                textprogress_blink: false,
                counter_offset: AmAnimatedFloat::default(),
                counter_scale: AmAnimatedFloat::default(),
                shape_props: Default::default(),
                shape_points: Default::default(),
                jitter_enabled: false,
                jitter_angle: AmAnimatedFloat::default(),
                jitter_freq: AmAnimatedFloat::default(),
                jitter_mag: AmAnimatedFloat::default(),
                jitter_seed: AmAnimatedFloat::default(),
                jitter_slack: AmAnimatedFloat::default(),
                jitter_zjitter: AmAnimatedFloat::default(),
                sd_enabled: false,
                sd_mag: AmAnimatedFloat::default(),
                sd_evolution: AmAnimatedFloat::default(),
                sd_seed: AmAnimatedFloat::default(),
                sd_scatter: AmAnimatedFloat::default(),
                rgb_split_enabled: false,
                rgb_split_strength: AmAnimatedFloat::default(),
                rgb_split_angle: AmAnimatedFloat::default(),
                rgb_split_center: 1,
                rgb_split_mode: 2,
                exposure_value: AmAnimatedFloat::default(),
                exposure_gamma: AmAnimatedFloat::default(),
                exposure_offset: AmAnimatedFloat::default(),
                exposure_has_effect: false,
                chromakey_enabled: false,
                chromakey_key_color: crate::schema::AmAnimatedColor::default(),
                chromakey_threshold: AmAnimatedFloat::default(),
                chromakey_feather: AmAnimatedFloat::default(),
                chromakey_defringe: false,
                chromakey_invert: false,
                blend_mode: AmBlendingMode::default(),
                retime: config.retime.clone(),
                echo_time_shift_ms: config.echo_time_shift_ms,
                echo_alpha_config: config.echo_alpha_config.clone(),
                repeat_rotation_offset_deg: -config.repeat_rotation_deg,
                repeat_scale_factor: config.repeat_scale_factor,
                repeat_position_offset: config.repeat_offset,
                embed_inner_total_time: None,
            },
            AmLayerSpec::EmbedScene,
            // Mark for render strategy evaluation (Hybrid Pipeline)
            // The evaluate_render_strategy_system will determine if this embed
            // needs Direct (no RTT), Stencil, or Composite (full RTT) rendering.
            NeedsStrategyEvaluation {
                scene_width: embed.scene.width as f32,
                scene_height: embed.scene.height as f32,
                has_scale_animation: !embed.transform.scale.keyframes.is_empty(),
            },
            transform,
            GlobalTransform::default(),
            Visibility::Hidden, // Start hidden, lifecycle system will show when active
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Insert group fill component if this embed has a fill type
    match embed.fill_type.as_str() {
        "none" => {
            commands.entity(entity).insert(AmGroupFill {
                fill_type: GroupFillType::None,
                fill_color: Vec4::ZERO,
            });
        }
        "color" => {
            let color = match embed.fill_color {
                Some(ref fc) => parse_color_to_linear_vec4(&fc.value, Vec4::ONE),
                None => Vec4::ONE,
            };
            commands.entity(entity).insert(AmGroupFill {
                fill_type: GroupFillType::Color,
                fill_color: color,
            });
        }
        "gradient" => {
            if let Some(ref g) = embed.gradient {
                let gradient_type = match g.gradient_type.as_str() {
                    "linear" => 1u8,
                    "radial" => 2u8,
                    "sweep" => 3u8,
                    _ => 1u8,
                };
                let start_color = parse_color_to_linear_vec4(&g.start_color, Vec4::ZERO);
                let end_color = parse_color_to_linear_vec4(&g.end_color, Vec4::ONE);
                let start_pt = g.start.unwrap_or([0.5, 0.0]);
                let end_pt = g.end.unwrap_or([0.5, 1.0]);
                commands.entity(entity).insert(AmGroupFill {
                    fill_type: GroupFillType::Gradient {
                        gradient_type,
                        start_color,
                        end_color,
                        points: Vec4::new(start_pt[0], start_pt[1], end_pt[0], end_pt[1]),
                    },
                    fill_color: Vec4::ONE,
                });
            }
        }
        _ => {}
    }

    // Mark embed mask layers for Composite rendering strategy
    if embed.blending == "mask" || embed.blending == "exclude" {
        commands.entity(entity).insert(crate::effects::AmEmbedMask);
        bevy::log::debug!(
            "[spawn_embed] Marked embed '{}' (id={}) as mask (blending={})",
            embed.label,
            embed.id,
            embed.blending
        );
    }

    // Recursively spawn nested scene with accumulated time offset
    // The nested scene's layers use times relative to the embed's start_time
    //
    // Calculate the internal time offset for the embedded scene.
    // When the parent timeline reaches startTime, the embedded scene should be at inTime.
    //
    // The formula for local_time in the animation system is:
    //   local_time = (global_time - time_offset) * speed_multiplier
    //
    // When global_time = embed.start_time, we want local_time = inTime:
    //   inTime = (embed.start_time - time_offset) * speed
    //   time_offset = embed.start_time - inTime / speed
    //
    // Note: This handles the case where speed != 1.0, which affects internal time flow.
    //
    // Nested scenes use smaller z_spacing to keep all children within
    // the parent's z-range (between parent and next sibling)
    // Using /100 instead of /1000 for better numerical precision
    let in_time = embed.in_time.unwrap_or(0) as f32;
    let effective_speed = config.speed_multiplier * embed.speed;

    // embed.start_time is relative to PARENT's internal time, not global time.
    // When parent's internal time = embed.start_time, child should start.
    // Parent internal time = (global_time - parent_time_offset) * parent_speed
    // global_start = parent_time_offset + embed.start_time / parent_speed
    let global_start = if config.speed_multiplier > 0.0 {
        config.time_offset + embed.start_time as f32 / config.speed_multiplier
    } else {
        config.time_offset + embed.start_time as f32
    };
    let time_offset_with_in_time = if effective_speed > 0.0 {
        // AM's retimeNestedScene computes the parent time via
        //   timeFromFrameNumber(parentFrame, parentFPHS)
        // which includes a +50000/fphs half-frame offset (the frame CENTER time).
        // See NestedSceneElementKt.java:103 and TimeKt.java timeFromFrameNumber.
        let half_frame_ms = if config.render_fps > 0.0 {
            500.0 / config.render_fps
        } else {
            0.0
        };
        global_start - in_time / effective_speed - half_frame_ms / effective_speed
    } else {
        global_start
    };
    // Lifecycle offset also needs to account for parent speed
    let lifecycle_offset_with_in_time = global_start - in_time;
    let nested_z_spacing = config.z_spacing / 100.0;

    // Parse retime mode (same as collect path)
    let retime_mode = crate::animation::RetimeMode::parse(&embed.scene.retime);
    let retime_info = if retime_mode != crate::animation::RetimeMode::Off {
        let container_duration = (embed.end_time - embed.start_time) as f32;
        let nested_total = embed.scene.total_time as f32;
        Some(crate::animation::AmRetimeInfo {
            mode: retime_mode,
            embed_global_start: global_start,
            container_duration_ms: container_duration,
            nested_total_time_ms: nested_total,
            embed_speed: effective_speed,
        })
    } else {
        config.retime.clone()
    };

    // Calculate nested render fps (same as collect path)
    let element_duration = (embed.end_time - embed.start_time) as f64;
    let inner_total_time = embed.scene.total_time as f64;
    let x_z = if inner_total_time > 0.0 {
        (element_duration / inner_total_time).max(1.0).ceil() as u32
    } else {
        1
    };
    let coerce_at_least = if effective_speed < 0.99999 {
        (1.0 / effective_speed.max(1e-6)).round().max(1.0) as u32
    } else {
        1
    };
    let parent_fphs = (config.render_fps * 100.0) as u32;
    let nested_fphs = (parent_fphs * x_z * coerce_at_least * 16).min(192000);
    let nested_render_fps = nested_fphs as f32 / 100.0;

    let nested_config = AmSceneConfig {
        canvas_width: embed.scene.width as f32,
        canvas_height: embed.scene.height as f32,
        time_offset: time_offset_with_in_time,
        lifecycle_offset: lifecycle_offset_with_in_time as i32,
        z_spacing: nested_z_spacing,
        nesting_depth: config.nesting_depth + 1,
        speed_multiplier: effective_speed,
        scene_fps: embed.scene.fps as f32,
        scene_total_time: embed.scene.total_time as f32,
        retime: retime_info,
        render_fps: nested_render_fps,
        // Reset repeat spatial transforms — they apply only to THIS embed, not children
        repeat_offset: Vec2::ZERO,
        repeat_rotation_deg: 0.0,
        repeat_scale_factor: 1.0,
        ..config.clone()
    };

    spawn_scene(
        commands,
        shaders,
        &embed.scene,
        images,
        fonts,
        font_metrics,
        white_pixel,
        sdf_shaders,
        entity,
        &nested_config,
    );

    entity
}

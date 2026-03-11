//! # spawn_visual.rs
//!
//! # 视觉元素生成模块
//!
//! Entity spawning functions for visual AM layers (image, text).
//! 视觉 AM 图层（图片、文字）的实体生成函数。

use bevy::prelude::*;
use bevy::sprite::Text2d;
use bevy::text::{TextColor, TextFont, TextLayout};
use std::collections::HashMap;

use crate::animation::AmAnimated;
use crate::loader::FontMetrics;
use crate::schema::{AmAnimatedFloat, AmAnimatedVec2, AmText};

use super::components::*;
use super::effects::*;
use super::helpers::*;

pub(crate) fn spawn_image(
    commands: &mut Commands,
    image: &crate::schema::AmImage,
    _images: &HashMap<String, Handle<Image>>,
    config: &AmSceneConfig,
    z: f32,
) -> Entity {
    let has_parent = image.parent != 0;
    let (tx, ty) = get_initial_location(&image.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&image.transform.rotation);
    let (sx, sy) = get_initial_scale(&image.transform.scale);
    let mut all_transform2 = extract_all_transform2_effects(&image.effects);
    let transform2 = if all_transform2.is_empty() {
        Transform2Params::default()
    } else {
        all_transform2.remove(0)
    };
    let extra_transform2 = all_transform2;
    let wipe_effect = extract_wipe_effect(&image.effects);
    let all_stretch_segments = extract_all_stretch_segment_effects(&image.effects);
    let stretch_segment = all_stretch_segments.first().cloned().unwrap_or_default();
    let gaussian_blur = extract_gaussian_blur_effect(&image.effects);
    let scale_assist = extract_scale_assist_effect(&image.effects);
    let stretch2_effect = extract_stretch2_effect(&image.effects);
    let replace_color = extract_replace_color_effect(&image.effects);
    let repeat_effect = extract_repeat_effect(&image.effects);
    let (linear_repeat_effect, linear_repeat_effect2) =
        extract_linear_repeat_effects(&image.effects);
    let radial_repeat_effect = extract_radial_repeat_effect(&image.effects);
    let swing_effect = extract_swing_effect(&image.effects);
    let oscillate_effect = extract_oscillate_effect(&image.effects);
    let spin_rpm = extract_spin_rpm(&image.effects);
    let threshold_effect = extract_threshold_effect(&image.effects);
    let grid_effect = extract_grid_effect(&image.effects);
    let pixelate_effect = extract_pixelate_effect(&image.effects);
    let solid_color_effect = extract_solid_color_effect(&image.effects);
    let (pivot_x, pivot_y) = get_initial_pivot(&image.transform.pivot);
    let palette_map = extract_palette_map_effect(&image.effects);
    let fade_effect = extract_fade_effect(&image.effects);
    let wavewarp2_effect = extract_wavewarp2_effect(&image.effects);
    let mirror_effect = extract_mirror_effect(&image.effects);

    // Get size from properties
    let (width, height) = get_shape_size(&image.properties, &image.fill_type);

    // Calculate anchor and position compensation
    let (anchor, comp_x, comp_y) = pivot_to_anchor_and_offset(pivot_x, pivot_y, width, height);
    let (final_tx, final_ty) = (tx + comp_x, ty + comp_y);

    bevy::log::trace!(
        "Registering image '{}' (id={}, parent={}): pos=({:.1},{:.1}), scale=({:.2},{:.2}), size=({:.0},{:.0}), pivot=({:.1},{:.1}), fill={}",
        image.label,
        image.id,
        image.parent,
        final_tx,
        final_ty,
        sx,
        sy,
        width,
        height,
        pivot_x,
        pivot_y,
        image.fill_image
    );

    let transform = Transform {
        translation: Vec3::new(final_tx, final_ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    // Create entity name for inspector identification
    let entity_name = format!("Image[{}]: {}", image.id, image.label);

    let entity = commands
        .spawn((
            Name::new(entity_name),
            AmLayerMarker {
                id: image.id,
                label: image.label.clone(),
            },
            AmAnimated {
                layer_id: image.id,
                start_time: image.start_time,
                end_time: image.end_time,
                time_offset: config.time_offset,
                lifecycle_offset: config.lifecycle_offset,
                location: image.transform.location.clone(),
                pivot: image.transform.pivot.clone(),
                rotation: image.transform.rotation.clone(),
                scale: image.transform.scale.clone(),
                opacity: image.transform.opacity.clone(),
                canvas_width: config.canvas_width,
                canvas_height: config.canvas_height,
                has_parent,
                parent_layer_id: image.parent,
                effect_pos_x: transform2.pos_x,
                effect_pos_y: transform2.pos_y,
                effect_posz: transform2.pos_z,
                effect_angle: transform2.angle,
                effect_xinv: transform2.xinv,
                effect_yinv: transform2.yinv,
                effect_zinv: transform2.zinv,
                effect_ainv: transform2.ainv,
                extra_transform2,
                font_y_offset: 0.0,
                size: AmAnimatedVec2::default(),
                anchor_offset: Vec2::new(comp_x, comp_y),
                wipe_start: wipe_effect.start,
                wipe_end: wipe_effect.end,
                wipe_angle: wipe_effect.angle,
                wipe_feather: wipe_effect.feather,
                stretch_angle: stretch_segment.angle,
                stretch_amount: stretch_segment.stretch,
                stretch_offset: stretch_segment.offset,
                stretch_smooth: stretch_segment.smooth,
                stretch_seg2_angle: all_stretch_segments
                    .get(1)
                    .map_or_else(AmAnimatedFloat::default, |s| s.angle.clone()),
                stretch_seg2_amount: all_stretch_segments
                    .get(1)
                    .map_or_else(AmAnimatedFloat::default, |s| s.stretch.clone()),
                stretch_seg2_offset: all_stretch_segments
                    .get(1)
                    .map_or_else(AmAnimatedFloat::default, |s| s.offset.clone()),
                stretch_seg2_smooth: all_stretch_segments
                    .get(1)
                    .map_or_else(AmAnimatedFloat::default, |s| s.smooth.clone()),
                blur_strength: gaussian_blur.strength,
                speed_multiplier: config.speed_multiplier,
                element_speed: 1.0,
                scene_fps: config.scene_fps,
                embed_offset: Vec2::ZERO,
                inv_fit_scale: 1.0,
                stroke_width: AmAnimatedFloat::default(),
                base_alpha: 1.0, // Image layers are fully opaque
                fade_in_time: fade_effect.in_time,
                fade_out_time: fade_effect.out_time,
                fade_layer_duration_ms: (image.end_time - image.start_time) as f32,
                palette_alpha: palette_map.alpha.clone(),
                scale_assist: scale_assist.scale,
                scale_assist_damp: scale_assist.damp,
                scale_assist_axis: scale_assist.axis,
                stretch2_scale: stretch2_effect.scale,
                stretch2_angle: stretch2_effect.angle,
                stretch2_content_only: stretch2_effect.content_only,
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
                replace_old_color: replace_color.old_color,
                replace_new_color: replace_color.new_color,
                replace_threshold: replace_color.threshold,
                replace_feather: replace_color.feather,
                replace_alpha: replace_color.alpha,
                replace_lock_luminance: replace_color.lock_luminance,
                repeat_count: repeat_effect.count,
                repeat_offset: repeat_effect.offset,
                repeat_angle: repeat_effect.angle,
                repeat_scale: repeat_effect.scale,
                repeat_alpha: repeat_effect.alpha,
                // Linear repeat effect
                linear_repeat_count: linear_repeat_effect.count,
                linear_repeat_position: linear_repeat_effect.position,
                linear_repeat_offset: linear_repeat_effect.offset,
                linear_repeat_angle: linear_repeat_effect.angle,
                linear_repeat_scale: linear_repeat_effect.scale,
                linear_repeat_alpha: linear_repeat_effect.alpha,
                linear_repeat_fill_color: linear_repeat_effect.fill_color,
                linear_repeat_blend: linear_repeat_effect.blend,
                linear_repeat_color_alt_copies: linear_repeat_effect.color_alt_copies,
                linear_repeat_start: linear_repeat_effect.start,
                linear_repeat_end: linear_repeat_effect.end,
                linear_repeat_phase: linear_repeat_effect.phase,
                linear_repeat_ease_in: linear_repeat_effect.ease_in,
                linear_repeat_ease_out: linear_repeat_effect.ease_out,
                linear_repeat_overlap: linear_repeat_effect.overlap,
                linear_repeat_shape: linear_repeat_effect.shape,
                linear_repeat_invert: linear_repeat_effect.invert,
                linear_repeat_random_order: linear_repeat_effect.random_order,
                linear_repeat_seed: linear_repeat_effect.seed,
                linear_repeat2: linear_repeat_effect2.map(Box::new),
                // Radial repeat effect
                radial_repeat_count: radial_repeat_effect.count,
                radial_repeat_radius: radial_repeat_effect.radius,
                radial_repeat_orientation: radial_repeat_effect.orientation,
                radial_repeat_start_angle: radial_repeat_effect.start_angle,
                radial_repeat_sweep: radial_repeat_effect.sweep,
                radial_repeat_base_scale: radial_repeat_effect.base_scale,
                radial_repeat_offset: radial_repeat_effect.offset,
                radial_repeat_angle: radial_repeat_effect.angle,
                radial_repeat_scale: radial_repeat_effect.scale,
                radial_repeat_alpha: radial_repeat_effect.alpha,
                radial_repeat_fill_color: radial_repeat_effect.fill_color,
                radial_repeat_blend: radial_repeat_effect.blend,
                radial_repeat_color_alt_copies: radial_repeat_effect.color_alt_copies,
                radial_repeat_start: radial_repeat_effect.start,
                radial_repeat_end: radial_repeat_effect.end,
                radial_repeat_phase: radial_repeat_effect.phase,
                radial_repeat_ease_in: radial_repeat_effect.ease_in,
                radial_repeat_ease_out: radial_repeat_effect.ease_out,
                radial_repeat_overlap: radial_repeat_effect.overlap,
                radial_repeat_shape: radial_repeat_effect.shape,
                radial_repeat_invert: radial_repeat_effect.invert,
                radial_repeat_random_order: radial_repeat_effect.random_order,
                radial_repeat_seed: radial_repeat_effect.seed,
                // Swing effect
                swing_freq: swing_effect.freq,
                swing_a1: swing_effect.a1,
                swing_a2: swing_effect.a2,
                swing_phase: swing_effect.phase,
                swing_type: swing_effect.swing_type,
                // Oscillate effect
                oscillate_direction: oscillate_effect.direction,
                oscillate_angle: oscillate_effect.angle,
                oscillate_freq: oscillate_effect.freq,
                oscillate_mag: oscillate_effect.mag,
                oscillate_wave_type: oscillate_effect.wave_type,
                oscillate_phase: oscillate_effect.phase,
                // Spin effect
                spin_rpm,
                // Threshold effect
                threshold_value: threshold_effect.threshold,
                threshold_feather: threshold_effect.feather,
                threshold_invert: threshold_effect.invert,
                threshold_blend_mode: threshold_effect.blend_mode,
                // Grid effect
                grid_position: grid_effect.position,
                grid_spacing: grid_effect.spacing,
                grid_width: grid_effect.width,
                grid_color: grid_effect.color,
                grid_punchout: grid_effect.punchout,
                grid_smoothing: grid_effect.smoothing,
                grid_screen_space: grid_effect.screen_space,
                // Pixelate effect
                pixelate_size: pixelate_effect.size,
                pixelate_stretch: pixelate_effect.stretch,
                pixelate_angle: pixelate_effect.angle,
                pixelate_vignette: pixelate_effect.vignette,
                pixelate_threshold: pixelate_effect.threshold,
                pixelate_saturation: pixelate_effect.saturation,
                pixelate_screen_space: pixelate_effect.screen_space,
                solid_color: solid_color_effect.color,
                solid_color_alpha: solid_color_effect.alpha,
                solid_color_blend_mode: solid_color_effect.blend_mode,
                base_fill_color: [0.0; 4],
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
                shape_props: Default::default(),
                shape_points: Default::default(),
                jitter_enabled: false,
                jitter_angle: AmAnimatedFloat::default(),
                jitter_freq: AmAnimatedFloat::default(),
                jitter_mag: AmAnimatedFloat::default(),
                jitter_seed: AmAnimatedFloat::default(),
                jitter_slack: AmAnimatedFloat::default(),
                jitter_zjitter: AmAnimatedFloat::default(),
                retime: config.retime.clone(),
                echo_time_shift_ms: config.echo_time_shift_ms,
                echo_alpha_config: config.echo_alpha_config.clone(),
            },
            AmLayerSpec::Image {
                image_uri: image.fill_image.clone(),
                width,
                height,
                anchor,
            },
            transform,
            GlobalTransform::default(),
            Visibility::Hidden, // Start hidden, lifecycle system will show when active
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Add palette map params if effect is present
    if palette_map.has_effect() {
        commands
            .entity(entity)
            .insert(AmPaletteMapParams::from_params(&palette_map));
    }

    entity
}

/// Spawn a text layer.
pub(crate) fn spawn_text(
    commands: &mut Commands,
    text: &AmText,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    config: &AmSceneConfig,
    z: f32,
) -> Entity {
    let has_parent = text.parent != 0;
    let (tx, ty) = get_initial_location(&text.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&text.transform.rotation);
    let (sx, sy) = get_initial_scale(&text.transform.scale);
    let opacity = get_initial_opacity(&text.transform.opacity);

    let wrap_width = text.wrap_width;

    // AM text position is at the CENTER of the wrapWidth box for all alignments.
    // Use Anchor::CENTER and no position offset.
    let wrap_offset_x = 0.0;

    // Get font size (default to 16.0 if not specified)
    const TEXT_SIZE_MULTIPLIER: f32 = 3.0;
    let font_size = if text.size > 0.0 {
        text.size * TEXT_SIZE_MULTIPLIER
    } else {
        48.0
    };

    // Parse font name from "imported?name=FontName.ttf" format
    let font_name = text
        .font
        .strip_prefix("imported?name=")
        .unwrap_or(&text.font)
        .to_string();

    let font_y_offset = font_metrics
        .get(&font_name)
        .map(|m| {
            let n_lines = text.content.chars().filter(|c| *c == '\n').count() as f32 + 1.0;
            let damping = (2.0_f32 / n_lines).min(1.0);
            m.include_pad_y_offset(font_size) * damping
        })
        .unwrap_or(0.0);

    // Get text color from fill_color
    let color = if let Some(fill_color) = &text.fill_color {
        if !fill_color.value.is_empty() {
            crate::schema::parse_color(&fill_color.value)
                .map(|c| Color::srgba(c[0], c[1], c[2], c[3] * opacity))
                .unwrap_or(Color::srgba(1.0, 1.0, 1.0, opacity))
        } else {
            Color::srgba(1.0, 1.0, 1.0, opacity)
        }
    } else {
        Color::srgba(1.0, 1.0, 1.0, opacity)
    };

    let transform = Transform {
        translation: Vec3::new(tx + wrap_offset_x, ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    // Apply wrap_offset to animated location for keyframe-based animations
    let mut modified_location = text.transform.location.clone();
    if wrap_offset_x != 0.0 {
        if let Some(ref mut val) = modified_location.value {
            val[0] += wrap_offset_x;
        }
        for kf in &mut modified_location.keyframes {
            if let Ok(mut parsed) = crate::schema::parse_vec3(&kf.value) {
                parsed[0] += wrap_offset_x;
                kf.value = format!("{},{},{}", parsed[0], parsed[1], parsed[2]);
            }
        }
    }

    // Create entity name for inspector identification
    let entity_name = if text.label.is_empty() {
        format!("Text[{}]: {}", text.id, truncate_string(&text.content, 20))
    } else {
        format!("Text[{}]: {}", text.id, text.label)
    };

    let mut entity = commands.spawn((
        Name::new(entity_name),
        AmLayerMarker {
            id: text.id,
            label: text.label.clone(),
        },
        AmAnimated {
            layer_id: text.id,
            start_time: text.start_time,
            end_time: text.end_time,
            time_offset: config.time_offset,
            lifecycle_offset: config.lifecycle_offset,
            location: modified_location,
            pivot: text.transform.pivot.clone(),
            rotation: text.transform.rotation.clone(),
            scale: text.transform.scale.clone(),
            opacity: text.transform.opacity.clone(),
            canvas_width: config.canvas_width,
            canvas_height: config.canvas_height,
            has_parent,
            parent_layer_id: text.parent,
            effect_pos_x: AmAnimatedFloat::default(),
            effect_pos_y: AmAnimatedFloat::default(),
            effect_posz: AmAnimatedFloat::default(),
            effect_angle: AmAnimatedFloat::default(),
            effect_xinv: false,
            effect_yinv: false,
            effect_zinv: false,
            effect_ainv: false,
            extra_transform2: vec![],
            font_y_offset,
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
            base_alpha: get_base_alpha(&text.fill_color, false),
            fade_in_time: AmAnimatedFloat::default(),
            fade_out_time: AmAnimatedFloat::default(),
            fade_layer_duration_ms: (text.end_time - text.start_time) as f32,
            palette_alpha: AmAnimatedFloat::default(),
            scale_assist: AmAnimatedFloat::default(),
            scale_assist_damp: AmAnimatedFloat::default(),
            scale_assist_axis: 0,
            stretch2_scale: AmAnimatedFloat::default(),
            stretch2_angle: AmAnimatedFloat::default(),
            stretch2_content_only: false,
            wavewarp2_phase: AmAnimatedFloat::default(),
            wavewarp2_a1d: AmAnimatedFloat::default(),
            wavewarp2_m1: AmAnimatedFloat::default(),
            wavewarp2_m2: AmAnimatedFloat::default(),
            wavewarp2_a2d: AmAnimatedFloat::default(),
            wavewarp2_damping: AmAnimatedFloat::default(),
            wavewarp2_damping_space: AmAnimatedFloat::default(),
            wavewarp2_damping_origin: AmAnimatedFloat::default(),
            wavewarp2_screen_space: false,
            wavewarp2_has_effect: false,
            mirror_type: 0,
            mirror_blend_mode: 0,
            mirror_alpha: AmAnimatedFloat::default(),
            mirror_offset: AmAnimatedFloat::default(),
            mirror_has_effect: false,
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
            // Linear repeat effect (defaults)
            linear_repeat_count: AmAnimatedFloat::default(),
            linear_repeat_position: AmAnimatedVec2::default(),
            linear_repeat_offset: AmAnimatedVec2::default(),
            linear_repeat_angle: AmAnimatedFloat::default(),
            linear_repeat_scale: AmAnimatedFloat {
                value: Some(1.0),
                keyframes: vec![],
            },
            linear_repeat_alpha: AmAnimatedFloat {
                value: Some(1.0),
                keyframes: vec![],
            },
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
            // Radial repeat effect (defaults for text)
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
            // Swing effect (defaults for text)
            swing_freq: AmAnimatedFloat::default(),
            swing_a1: AmAnimatedFloat::default(),
            swing_a2: AmAnimatedFloat::default(),
            swing_phase: AmAnimatedFloat::default(),
            swing_type: 0,
            // Oscillate effect (defaults for text)
            oscillate_direction: 0,
            oscillate_angle: AmAnimatedFloat::default(),
            oscillate_freq: AmAnimatedFloat::default(),
            oscillate_mag: AmAnimatedFloat::default(),
            oscillate_wave_type: 0,
            oscillate_phase: AmAnimatedFloat::default(),
            // Spin effect (defaults for text)
            spin_rpm: AmAnimatedFloat::default(),
            // Threshold effect (defaults for text)
            threshold_value: AmAnimatedFloat::default(),
            threshold_feather: AmAnimatedFloat::default(),
            threshold_invert: false,
            threshold_blend_mode: 0,
            // Grid effect (defaults for text)
            grid_position: AmAnimatedVec2::default(),
            grid_spacing: AmAnimatedFloat::default(),
            grid_width: AmAnimatedFloat::default(),
            grid_color: crate::schema::AmAnimatedColor::default(),
            grid_punchout: false,
            grid_smoothing: AmAnimatedFloat::default(),
            grid_screen_space: false,
            // Pixelate effect (defaults for text)
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
            shape_props: Default::default(),
            shape_points: Default::default(),
            jitter_enabled: false,
            jitter_angle: AmAnimatedFloat::default(),
            jitter_freq: AmAnimatedFloat::default(),
            jitter_mag: AmAnimatedFloat::default(),
            jitter_seed: AmAnimatedFloat::default(),
            jitter_slack: AmAnimatedFloat::default(),
            jitter_zjitter: AmAnimatedFloat::default(),
            retime: config.retime.clone(),
            echo_time_shift_ms: config.echo_time_shift_ms,
            echo_alpha_config: config.echo_alpha_config.clone(),
        },
        transform,
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
    ));

    // Add Text2d component with embedded font or Bevy's default font
    // 使用嵌入字体或 Bevy 默认字体添加 Text2d 组件
    let text_font = if let Some(font_handle) = fonts.get(&font_name) {
        bevy::log::trace!("  -> Using embedded font: {}", font_name);
        TextFont {
            font: font_handle.clone(),
            font_size,
            ..default()
        }
    } else {
        bevy::log::warn!(
            "Font '{}' not available for text '{}' (id={}), using Bevy's default font",
            font_name,
            text.label,
            text.id
        );
        // Use TextFont::default() which points to Bevy's built-in FiraMono font
        // when the default_font feature is enabled (which is the default)
        TextFont {
            font_size,
            ..TextFont::default()
        }
    };

    // Determine text justification based on align attribute
    let justify = match text.align.as_str() {
        "center" => bevy::text::Justify::Center,
        "right" => bevy::text::Justify::Right,
        _ => bevy::text::Justify::Left,
    };

    // AM text element position is always the CENTER of the text box
    let anchor = bevy::sprite::Anchor::CENTER;

    // Text layers have visual components spawned immediately but use visibility for lifecycle
    entity.insert((
        Text2d::new(&text.content),
        text_font,
        TextColor(color),
        TextLayout::new_with_justify(justify),
        bevy::text::TextBounds::new_horizontal(wrap_width),
        anchor,
        AmLayerSpec::Text {
            content: text.content.clone(),
            font_name: font_name.clone(),
            font_size,
            align: text.align.clone(),
            fill_color: text.fill_color.clone(),
            wrap_width: text.wrap_width,
            line_height_ratio: font_metrics
                .get(&font_name)
                .map(|m| m.am_line_height_ratio(font_size))
                .unwrap_or(1.2),
        },
        AmVisualSpawned,
    ));

    entity.id()
}

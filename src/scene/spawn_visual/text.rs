//! This file spawns text layers into Bevy text entities.
//! It resolves fonts, initial text styling, positioning, and the runtime animation
//! payload so authored AM text layers become Bevy text nodes that later systems
//! can animate.
//!
//! 这个文件负责把文本图层生成成 Bevy 文本实体。它会解析字体、初始文本样式、位置以及
//! 运行时动画载荷，让作者写下的 AM 文本图层变成后续系统可以继续驱动的 Bevy 文本节点。

use bevy::prelude::*;
use bevy::sprite::Text2d;
use bevy::text::{TextColor, TextFont, TextLayout};
use std::collections::HashMap;

use crate::animation::AmAnimated;
use crate::loader::FontMetrics;
use crate::schema::{AmAnimatedFloat, AmAnimatedVec2, AmText};

use super::super::components::*;
use super::super::helpers::*;

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
    let wrap_offset_x = 0.0;

    const TEXT_SIZE_MULTIPLIER: f32 = 3.0;
    let font_size = if text.size > 0.0 {
        text.size * TEXT_SIZE_MULTIPLIER
    } else {
        48.0
    };

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
            base_alpha: get_base_alpha(&text.fill_color, false) * config.repeat_alpha_factor,
            fade_in_time: AmAnimatedFloat::default(),
            fade_out_time: AmAnimatedFloat::default(),
            fade_layer_duration_ms: (text.end_time - text.start_time) as f32,
            palette_alpha: AmAnimatedFloat::default(),
            scale_assist: AmAnimatedFloat::default(),
            scale_assist_damp: AmAnimatedFloat::default(),
            scale_assist_axis: 0,
            parenthelper_scale_mode: 0,
            parenthelper_rotate_mode: 0,
            parenthelper_scale_weight: AmAnimatedFloat::default(),
            parenthelper_rotate_weight: AmAnimatedFloat::default(),
            parenthelper_auto_rotate: 0,
            parenthelper_radius_adjust: AmAnimatedFloat::default(),
            parenthelper_has_effect: false,
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
            lift_fill: AmAnimatedFloat::default(),
            lift_has_effect: false,
            rays_center_x: AmAnimatedFloat::default(),
            rays_center_y: AmAnimatedFloat::default(),
            rays_strength: AmAnimatedFloat::default(),
            rays_intensity: AmAnimatedFloat::default(),
            rays_threshold: AmAnimatedFloat::default(),
            rays_threshold_color: Vec4::ZERO,
            rays_fill_color: Vec4::ZERO,
            rays_blend: AmAnimatedFloat::default(),
            rays_quality: AmAnimatedFloat::default(),
            rays_has_effect: false,
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
            swing_freq: AmAnimatedFloat::default(),
            swing_a1: AmAnimatedFloat::default(),
            swing_a2: AmAnimatedFloat::default(),
            swing_phase: AmAnimatedFloat::default(),
            swing_type: 0,
            oscillate_direction: 0,
            oscillate_angle: AmAnimatedFloat::default(),
            oscillate_freq: AmAnimatedFloat::default(),
            oscillate_mag: AmAnimatedFloat::default(),
            oscillate_wave_type: 0,
            oscillate_phase: AmAnimatedFloat::default(),
            spin_rpm: AmAnimatedFloat::default(),
            threshold_value: AmAnimatedFloat::default(),
            threshold_feather: AmAnimatedFloat::default(),
            threshold_invert: false,
            threshold_blend_mode: 0,
            grid_position: AmAnimatedVec2::default(),
            grid_spacing: AmAnimatedFloat::default(),
            grid_width: AmAnimatedFloat::default(),
            grid_color: crate::schema::AmAnimatedColor::default(),
            grid_punchout: false,
            grid_smoothing: AmAnimatedFloat::default(),
            grid_screen_space: false,
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
            repeat_rotation_offset_deg: 0.0,
            repeat_scale_factor: 1.0,
            repeat_position_offset: Vec2::ZERO,
            embed_inner_total_time: None,
        },
        transform,
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
    ));

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
        TextFont {
            font_size,
            ..TextFont::default()
        }
    };

    let justify = match text.align.as_str() {
        "center" => bevy::text::Justify::Center,
        "right" => bevy::text::Justify::Right,
        _ => bevy::text::Justify::Left,
    };

    let anchor = bevy::sprite::Anchor::CENTER;

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

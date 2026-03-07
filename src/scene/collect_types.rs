//! # collect_types.rs
//!
//! # 类型收集模块
//!
//! Functions for collecting specific layer types (shape, null, embed, text, image).
//! 特定图层类型（形状、空对象、嵌入、文字、图片）的收集函数。

use bevy::prelude::*;
use std::collections::HashMap;

use crate::animation::AmAnimated;
use crate::loader::FontMetrics;
use crate::schema::{AmAnimatedFloat, AmAnimatedVec2, AmText};

use super::components::*;
use super::effects::*;
use super::helpers::*;

/// Collect a null object's data.
pub(crate) fn collect_null(
    null: &crate::schema::AmNullObj,
    config: &AmSceneConfig,
    z: f32,
) -> Option<PendingLayer> {
    let has_parent = null.parent != 0;
    let (tx, ty) = get_initial_location(&null.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&null.transform.rotation);
    let (sx, sy) = get_initial_scale(&null.transform.scale);
    let mut all_transform2 = extract_all_transform2_effects(&null.effects);
    let transform2 = if all_transform2.is_empty() {
        Transform2Params::default()
    } else {
        all_transform2.remove(0)
    };
    let extra_transform2 = all_transform2;
    let wipe_effect = extract_wipe_effect(&null.effects);
    let all_stretch_segments = extract_all_stretch_segment_effects(&null.effects);
    let stretch_segment = all_stretch_segments.first().cloned().unwrap_or_default();
    let gaussian_blur = extract_gaussian_blur_effect(&null.effects);
    let scale_assist = extract_scale_assist_effect(&null.effects);
    let stretch2_effect = extract_stretch2_effect(&null.effects);
    let replace_color = extract_replace_color_effect(&null.effects);
    let repeat_effect = extract_repeat_effect(&null.effects);
    let (linear_repeat_effect, linear_repeat_effect2) =
        extract_linear_repeat_effects(&null.effects);
    let radial_repeat_effect = extract_radial_repeat_effect(&null.effects);
    let swing_effect = extract_swing_effect(&null.effects);
    let oscillate_effect = extract_oscillate_effect(&null.effects);
    let jitter_effect = extract_jitter_effect(&null.effects);
    let spin_rpm = extract_spin_rpm(&null.effects);
    let threshold_effect = extract_threshold_effect(&null.effects);
    let grid_effect = extract_grid_effect(&null.effects);
    let pixelate_effect = extract_pixelate_effect(&null.effects);
    let solid_color_effect = extract_solid_color_effect(&null.effects);

    let transform = Transform {
        translation: Vec3::new(tx, ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    Some(PendingLayer {
        id: null.id,
        label: null.label.clone(),
        parent: null.parent,
        start_time: null.start_time,
        end_time: null.end_time,
        transform,
        animated: AmAnimated {
            layer_id: null.id,
            start_time: null.start_time,
            end_time: null.end_time,
            time_offset: config.time_offset,
            lifecycle_offset: config.lifecycle_offset,
            location: null.transform.location.clone(),
            pivot: null.transform.pivot.clone(),
            rotation: null.transform.rotation.clone(),
            scale: null.transform.scale.clone(),
            opacity: null.transform.opacity.clone(),
            canvas_width: config.canvas_width,
            canvas_height: config.canvas_height,
            has_parent,
            parent_layer_id: null.parent,
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
            anchor_offset: Vec2::ZERO,
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
            base_alpha: 1.0, // Null objects are fully opaque
            palette_alpha: AmAnimatedFloat::default(),
            scale_assist: scale_assist.scale,
            scale_assist_damp: scale_assist.damp,
            scale_assist_axis: scale_assist.axis,
            stretch2_scale: stretch2_effect.scale,
            stretch2_angle: stretch2_effect.angle,
            stretch2_content_only: stretch2_effect.content_only,
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
            radial_repeat_count: radial_repeat_effect.count.clone(),
            radial_repeat_radius: radial_repeat_effect.radius.clone(),
            radial_repeat_orientation: radial_repeat_effect.orientation.clone(),
            radial_repeat_start_angle: radial_repeat_effect.start_angle.clone(),
            radial_repeat_sweep: radial_repeat_effect.sweep.clone(),
            radial_repeat_base_scale: radial_repeat_effect.base_scale.clone(),
            radial_repeat_offset: radial_repeat_effect.offset.clone(),
            radial_repeat_angle: radial_repeat_effect.angle.clone(),
            radial_repeat_scale: radial_repeat_effect.scale.clone(),
            radial_repeat_alpha: radial_repeat_effect.alpha.clone(),
            radial_repeat_fill_color: radial_repeat_effect.fill_color.clone(),
            radial_repeat_blend: radial_repeat_effect.blend.clone(),
            radial_repeat_color_alt_copies: radial_repeat_effect.color_alt_copies,
            radial_repeat_start: radial_repeat_effect.start.clone(),
            radial_repeat_end: radial_repeat_effect.end.clone(),
            radial_repeat_phase: radial_repeat_effect.phase.clone(),
            radial_repeat_ease_in: radial_repeat_effect.ease_in.clone(),
            radial_repeat_ease_out: radial_repeat_effect.ease_out.clone(),
            radial_repeat_overlap: radial_repeat_effect.overlap.clone(),
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
            oscillate_angle: oscillate_effect.angle.clone(),
            oscillate_freq: oscillate_effect.freq.clone(),
            oscillate_mag: oscillate_effect.mag.clone(),
            oscillate_wave_type: oscillate_effect.wave_type,
            oscillate_phase: oscillate_effect.phase.clone(),
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
            // Jitter effect
            jitter_enabled: jitter_effect.enabled,
            jitter_angle: jitter_effect.angle,
            jitter_freq: jitter_effect.freq,
            jitter_mag: jitter_effect.mag,
            jitter_seed: jitter_effect.seed,
            jitter_slack: jitter_effect.slack,
            jitter_zjitter: jitter_effect.zjitter,
            retime: config.retime.clone(),
            echo_time_shift_ms: config.echo_time_shift_ms,
            echo_alpha_config: config.echo_alpha_config.clone(),
        },
        spec: AmLayerSpec::Null,
        z_index: z,
        children: Vec::new(),
        blending_mode: AmBlendingMode::Normal,
        mask_info: None,
        palette_params: None,
        embed_scene_size: None,
        containing_embed_id: 0,
        from_deeply_nested_scene: config.nesting_depth > 1,
        echo_runtime: None,
        group_fill: None,
    })
}

/// Apply mask information to layers that are below mask layers.
/// A mask layer affects all layers with lower z-index (parent=0 only) until end of scope.
///
/// This function works on a **flattened** list of layers (from `flatten_pending_layers`).
/// Since children are extracted into the flat list with remapped parent IDs, we need to:
/// 1. Find mask layers (parent=0 and blending_mode=Mask)
/// 2. Find root-level layers (parent=0) that are below the mask (lower z-index)
/// 3. Propagate mask to all descendants by following the parent chain
pub(crate) fn collect_text(
    text: &AmText,
    _fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    config: &AmSceneConfig,
    z: f32,
) -> Option<PendingLayer> {
    let has_parent = text.parent != 0;
    let (tx, ty) = get_initial_location(&text.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&text.transform.rotation);
    let (sx, sy) = get_initial_scale(&text.transform.scale);

    // Font name parsing
    let font_name = text
        .font
        .strip_prefix("imported?name=")
        .unwrap_or(&text.font)
        .to_string();

    const TEXT_SIZE_MULTIPLIER: f32 = 3.0;
    let font_size = if text.size > 0.0 {
        text.size * TEXT_SIZE_MULTIPLIER
    } else {
        48.0
    };

    // AM text position is at the CENTER of the wrapWidth box for all alignments.
    let _wrap_width = text.wrap_width;
    let wrap_offset_x = 0.0;

    // Compensate for AM's StaticLayout includePad(true) centering vs Bevy's Anchor::CENTER.
    // AM adds asymmetric padding (win metrics vs hhea metrics) at first/last lines,
    // then centers the padded box. This shifts the visual text center.
    // For multi-line multi-script text, font fallback changes the effective metrics,
    // so we dampen the offset for text with many lines.
    let font_y_offset = if let Some(metrics) = font_metrics.get(&font_name) {
        let n_lines = text.content.chars().filter(|c| *c == '\n').count() as f32 + 1.0;
        let damping = (2.0_f32 / n_lines).min(1.0);
        metrics.include_pad_y_offset(font_size) * damping
    } else {
        0.0
    };

    let transform = Transform {
        translation: Vec3::new(tx + wrap_offset_x, ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    // Apply wrap_offset to animated location
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

    Some(PendingLayer {
        id: text.id,
        label: text.label.clone(),
        parent: text.parent,
        start_time: text.start_time,
        end_time: text.end_time,
        transform,
        animated: AmAnimated {
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
            palette_alpha: AmAnimatedFloat::default(),
            scale_assist: AmAnimatedFloat::default(),
            scale_assist_damp: AmAnimatedFloat::default(),
            scale_assist_axis: 0,
            stretch2_scale: AmAnimatedFloat::default(),
            stretch2_angle: AmAnimatedFloat::default(),
            stretch2_content_only: false,
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
            // Linear repeat effect (defaults for text)
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
            // Radial repeat effect (defaults)
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
            // Oscillate effect (defaults)
            oscillate_direction: 0,
            oscillate_angle: AmAnimatedFloat::default(),
            oscillate_freq: AmAnimatedFloat::default(),
            oscillate_mag: AmAnimatedFloat::default(),
            oscillate_wave_type: 0,
            oscillate_phase: AmAnimatedFloat::default(),
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
            textspacing_letter: extract_text_spacing_effect(&text.effects).letter_spacing,
            textspacing_line: extract_text_spacing_effect(&text.effects).line_spacing,
            textprogress_start: extract_text_progress_effect(&text.effects).start,
            textprogress_end: extract_text_progress_effect(&text.effects).end,
            textprogress_cursor: extract_text_progress_effect(&text.effects).cursor,
            textprogress_blink: extract_text_progress_effect(&text.effects).blink,
            shape_props: Default::default(),
            shape_points: Default::default(),
            // Jitter effect (not extracted for text - defaults)
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
        spec: AmLayerSpec::Text {
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
        z_index: z,
        children: Vec::new(),
        blending_mode: AmBlendingMode::Normal,
        mask_info: None,
        palette_params: None,
        embed_scene_size: None,
        containing_embed_id: 0,
        from_deeply_nested_scene: config.nesting_depth > 1,
        echo_runtime: None,
        group_fill: None,
    })
}

/// Collect an image layer's data.
pub(crate) fn collect_image(
    image: &crate::schema::AmImage,
    config: &AmSceneConfig,
    z: f32,
) -> Option<PendingLayer> {
    let has_parent = image.parent != 0;
    let (tx, ty) = get_initial_location(&image.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&image.transform.rotation);
    let (sx, sy) = get_initial_scale(&image.transform.scale);
    let (pivot_x, pivot_y) = get_initial_pivot(&image.transform.pivot);
    let wipe_effect = extract_wipe_effect(&image.effects);
    let all_stretch_segments = extract_all_stretch_segment_effects(&image.effects);
    let stretch_segment = all_stretch_segments.first().cloned().unwrap_or_default();
    let gaussian_blur = extract_gaussian_blur_effect(&image.effects);
    let palette_map = extract_palette_map_effect(&image.effects);
    let scale_assist = extract_scale_assist_effect(&image.effects);
    let stretch2_effect = extract_stretch2_effect(&image.effects);
    let replace_color = extract_replace_color_effect(&image.effects);
    let repeat_effect = extract_repeat_effect(&image.effects);
    let (linear_repeat_effect, linear_repeat_effect2) =
        extract_linear_repeat_effects(&image.effects);
    let radial_repeat_effect = extract_radial_repeat_effect(&image.effects);
    let swing_effect = extract_swing_effect(&image.effects);
    let oscillate_effect = extract_oscillate_effect(&image.effects);
    let jitter_effect = extract_jitter_effect(&image.effects);
    let spin_rpm = extract_spin_rpm(&image.effects);
    let threshold_effect = extract_threshold_effect(&image.effects);
    let grid_effect = extract_grid_effect(&image.effects);
    let pixelate_effect = extract_pixelate_effect(&image.effects);
    let solid_color_effect = extract_solid_color_effect(&image.effects);

    // Get size from properties
    let (width, height) = get_shape_size(&image.properties, &image.fill_type);

    // Calculate anchor and position compensation
    let (anchor, comp_x, comp_y) = pivot_to_anchor_and_offset(pivot_x, pivot_y, width, height);
    let (final_tx, final_ty) = (tx + comp_x, ty + comp_y);

    let transform = Transform {
        translation: Vec3::new(final_tx, final_ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    Some(PendingLayer {
        id: image.id,
        label: image.label.clone(),
        parent: image.parent,
        start_time: image.start_time,
        end_time: image.end_time,
        transform,
        animated: AmAnimated {
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
            effect_pos_x: AmAnimatedFloat::default(),
            effect_pos_y: AmAnimatedFloat::default(),
            effect_posz: AmAnimatedFloat::default(),
            effect_angle: AmAnimatedFloat::default(),
            effect_xinv: false,
            effect_yinv: false,
            effect_zinv: false,
            effect_ainv: false,
            extra_transform2: vec![],
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
            palette_alpha: palette_map.alpha.clone(),
            scale_assist: scale_assist.scale,
            scale_assist_damp: scale_assist.damp,
            scale_assist_axis: scale_assist.axis,
            stretch2_scale: stretch2_effect.scale,
            stretch2_angle: stretch2_effect.angle,
            stretch2_content_only: stretch2_effect.content_only,
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
            radial_repeat_count: radial_repeat_effect.count.clone(),
            radial_repeat_radius: radial_repeat_effect.radius.clone(),
            radial_repeat_orientation: radial_repeat_effect.orientation.clone(),
            radial_repeat_start_angle: radial_repeat_effect.start_angle.clone(),
            radial_repeat_sweep: radial_repeat_effect.sweep.clone(),
            radial_repeat_base_scale: radial_repeat_effect.base_scale.clone(),
            radial_repeat_offset: radial_repeat_effect.offset.clone(),
            radial_repeat_angle: radial_repeat_effect.angle.clone(),
            radial_repeat_scale: radial_repeat_effect.scale.clone(),
            radial_repeat_alpha: radial_repeat_effect.alpha.clone(),
            radial_repeat_fill_color: radial_repeat_effect.fill_color.clone(),
            radial_repeat_blend: radial_repeat_effect.blend.clone(),
            radial_repeat_color_alt_copies: radial_repeat_effect.color_alt_copies,
            radial_repeat_start: radial_repeat_effect.start.clone(),
            radial_repeat_end: radial_repeat_effect.end.clone(),
            radial_repeat_phase: radial_repeat_effect.phase.clone(),
            radial_repeat_ease_in: radial_repeat_effect.ease_in.clone(),
            radial_repeat_ease_out: radial_repeat_effect.ease_out.clone(),
            radial_repeat_overlap: radial_repeat_effect.overlap.clone(),
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
            oscillate_angle: oscillate_effect.angle.clone(),
            oscillate_freq: oscillate_effect.freq.clone(),
            oscillate_mag: oscillate_effect.mag.clone(),
            oscillate_wave_type: oscillate_effect.wave_type,
            oscillate_phase: oscillate_effect.phase.clone(),
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
            // Jitter effect
            jitter_enabled: jitter_effect.enabled,
            jitter_angle: jitter_effect.angle,
            jitter_freq: jitter_effect.freq,
            jitter_mag: jitter_effect.mag,
            jitter_seed: jitter_effect.seed,
            jitter_slack: jitter_effect.slack,
            jitter_zjitter: jitter_effect.zjitter,
            retime: config.retime.clone(),
            echo_time_shift_ms: config.echo_time_shift_ms,
            echo_alpha_config: config.echo_alpha_config.clone(),
        },
        spec: AmLayerSpec::Image {
            image_uri: image.fill_image.clone(),
            width,
            height,
            anchor,
        },
        z_index: z,
        children: Vec::new(),
        blending_mode: AmBlendingMode::Normal,
        mask_info: None,
        palette_params: if palette_map.has_effect() {
            Some(AmPaletteMapParams::from_params(&palette_map))
        } else {
            None
        },
        embed_scene_size: None,
        containing_embed_id: 0,
        from_deeply_nested_scene: config.nesting_depth > 1,
        echo_runtime: None,
        group_fill: None,
    })
}

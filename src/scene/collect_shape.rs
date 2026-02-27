//! # collect_shape.rs
//!
//! # 形状图层收集
//!
//! Functions for collecting shape layer data into PendingLayer.
//! 形状图层数据收集为 PendingLayer 的函数。

use super::helpers;
use bevy::prelude::*;
use std::collections::HashMap;

use crate::animation::AmAnimated;
use crate::schema::{AmAnimatedFloat, AmAnimatedVec2, AmShape};

use super::components::*;
use super::effects::*;
use super::helpers::*;

pub(crate) fn collect_shape(
    shape: &AmShape,
    config: &AmSceneConfig,
    z: f32,
) -> Option<PendingLayer> {
    let has_parent = shape.parent != 0;
    let (tx, ty) = get_initial_location(&shape.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&shape.transform.rotation);
    let (sx, sy) = get_initial_scale(&shape.transform.scale);

    bevy::log::debug!(
        "[collect_shape] '{}': has_parent={}, canvas={}x{}, time_offset={}, bevy_pos=({:.1},{:.1})",
        shape.label,
        has_parent,
        config.canvas_width,
        config.canvas_height,
        config.time_offset,
        tx,
        ty
    );
    let mut all_transform2 = extract_all_transform2_effects(&shape.effects);
    let transform2 = if all_transform2.is_empty() {
        Transform2Params::default()
    } else {
        all_transform2.remove(0)
    };
    let extra_transform2 = all_transform2;
    let wipe_effect = extract_wipe_effect(&shape.effects);
    let stretch_segment = extract_stretch_segment_effect(&shape.effects);
    let gaussian_blur = extract_gaussian_blur_effect(&shape.effects);
    let palette_map = extract_palette_map_effect(&shape.effects);
    let scale_assist = extract_scale_assist_effect(&shape.effects);
    let stretch2_effect = extract_stretch2_effect(&shape.effects);
    if stretch2_effect.scale.value.is_some() {
        bevy::log::warn!(
            "[collect_shape] '{}' has stretch2: scale={:?}",
            shape.label,
            stretch2_effect.scale.value
        );
    }
    let replace_color = extract_replace_color_effect(&shape.effects);
    let repeat_effect = extract_repeat_effect(&shape.effects);
    let (linear_repeat_effect, linear_repeat_effect2) =
        extract_linear_repeat_effects(&shape.effects);
    let radial_repeat_effect = extract_radial_repeat_effect(&shape.effects);
    let swing_effect = extract_swing_effect(&shape.effects);
    let oscillate_effect = extract_oscillate_effect(&shape.effects);
    let jitter_effect = extract_jitter_effect(&shape.effects);
    let spin_rpm = extract_spin_rpm(&shape.effects);
    let threshold_effect = extract_threshold_effect(&shape.effects);
    let grid_effect = extract_grid_effect(&shape.effects);
    let pixelate_effect = extract_pixelate_effect(&shape.effects);
    let solid_color_effect = extract_solid_color_effect(&shape.effects);
    if scale_assist.axis != 0 {
        bevy::log::info!(
            "[COLLECT] Shape '{}' has scale_assist: axis={}, has_keyframes={}",
            shape.label,
            scale_assist.axis,
            !scale_assist.scale.keyframes.is_empty()
        );
    }
    let (pivot_x, pivot_y) = get_initial_pivot(&shape.transform.pivot);
    let (width, height) = get_shape_size(&shape.properties, &shape.fill_type);
    let size_animation = get_shape_size_animation(&shape.properties);

    let has_stroke_or_border = shape.stroke.as_ref().is_some_and(|s| {
        s.size
            .as_ref()
            .is_some_and(|sz| sz.value.unwrap_or(0.0) > 0.0 || !sz.keyframes.is_empty())
            || s.end_size > 0.0
    }) || shape.borders.iter().any(|b| {
        b.size
            .as_ref()
            .is_some_and(|sz| sz.value.unwrap_or(0.0) > 0.0 || !sz.keyframes.is_empty())
            || b.end_size > 0.0
    });
    let needs_sdf = shape.fill_type == "gradient"
        || ((shape.fill_type == "color" || shape.fill_type == "none")
            && (shape.shape_type != ".rect" || has_stroke_or_border));

    // Calculate anchor and position compensation for non-SDF shapes
    let (anchor, comp_x, comp_y) = pivot_to_anchor_and_offset(pivot_x, pivot_y, width, height);

    // For SpriteShape, we need to compensate position when anchor is not CENTER
    // For SDF shapes, parent should be at pivot point (for rotation/scale around pivot)
    let (final_tx, final_ty) = if needs_sdf {
        // SDF parent is at pivot point: AM center + pivot offset (with Y flip)
        // pivot is relative to center in AM coords, so pivot_point = center + (pivot_x, -pivot_y) in Bevy
        (tx + pivot_x, ty - pivot_y)
    } else {
        (tx + comp_x, ty + comp_y)
    };

    // For SDF shapes, we don't apply scale to the transform because:
    // 1. Scale will be applied to SDF params instead (to avoid stretching stroke width)
    // 2. The SDF dimensions are updated dynamically via animate_sdf_scale system
    let transform = Transform {
        translation: Vec3::new(final_tx, final_ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: if needs_sdf {
            Vec3::new(1.0, 1.0, 1.0)
        } else {
            Vec3::new(sx, sy, 1.0)
        },
    };

    // Extract animated shape properties (for SDF shapes; defaults for others)
    let (shape_props, shape_points) = if needs_sdf {
        helpers::extract_shape_animations(&shape.shape_type, &shape.properties)
    } else {
        Default::default()
    };

    let spec = if needs_sdf {
        let default_stroke = crate::schema::AmStroke::default();
        let has_path_stroke = shape.stroke.is_some();
        // Use path-stroke if available, otherwise fall back to first border
        let stroke = shape
            .stroke
            .as_ref()
            .unwrap_or_else(|| shape.borders.first().unwrap_or(&default_stroke));

        // AM's border effect uses a pixel-scanning shader hardcoded to 2048-wide buffers.
        // Border sizes are in this 2048-normalized space, so we scale to composition pixels:
        //   effective_size = xml_size * (comp_width / 2048)
        // Path-stroke and centered borders use NanoVG strokeWidth (no scaling needed).
        // AM filters out centered borders for shapes and renders them as NanoVG path strokes.
        let border_scale = if has_path_stroke {
            1.0
        } else {
            let direction = shape
                .borders
                .first()
                .map(|b| b.direction.as_str())
                .unwrap_or("centered");
            if direction == "centered" {
                1.0 // Centered borders rendered via NanoVG, not pixel-scan effect
            } else {
                config.canvas_width / 2048.0
            }
        };

        let has_any_stroke = has_path_stroke || !shape.borders.is_empty();

        // Get initial stroke width: only use default 4.0 if shape actually has a stroke
        let stroke_width = if has_any_stroke {
            stroke
                .size
                .as_ref()
                .and_then(|s| {
                    s.value
                        .or_else(|| s.keyframes.first().and_then(|kf| kf.value.parse().ok()))
                })
                .unwrap_or(4.0)
                * border_scale
        } else {
            0.0
        };
        let stroke_color_value = stroke
            .color
            .as_ref()
            .map(|c| c.value.clone())
            .unwrap_or_default();

        // Track whether this is a "no fill" shape (fillType="none")
        // This is different from having no fillColor value (defaults to white)
        let no_fill = shape.fill_type == "none";

        // Extract second border data if present (always uses border scaling)
        let border2 = shape.borders.get(1);
        let border2_scale = config.canvas_width / 2048.0;
        let border2_width = border2
            .and_then(|b| {
                b.size.as_ref().and_then(|s| {
                    s.value
                        .or_else(|| s.keyframes.first().and_then(|kf| kf.value.parse().ok()))
                })
            })
            .unwrap_or(0.0)
            * border2_scale;
        let border2_color_value = border2
            .and_then(|b| b.color.as_ref().map(|c| c.value.clone()))
            .unwrap_or_default();
        let border2_direction = border2.map(|b| b.direction.clone()).unwrap_or_default();

        // Extract shape-specific extra parameters based on shape type
        let (
            shape_extra,
            shape_extra2,
            shape_extra3,
            shape_extra4,
            shape_extra5,
            shape_extra6,
            shape_extra7,
        ) = extract_shape_extras(
            &shape.shape_type,
            &shape.properties,
            shape
                .path_element
                .as_ref()
                .map(|p| p.d.as_str())
                .unwrap_or(""),
        );

        // Extract gradient data
        let (gradient_type, gradient_start_color, gradient_end_color, gradient_points) =
            super::spawn::extract_gradient_data(&shape.gradient);

        AmLayerSpec::SdfShape {
            fill_color: shape.fill_color.clone(),
            stroke_color_value,
            stroke_width,
            stroke_join: stroke.join.clone(),
            stroke_direction: stroke.direction.clone(),
            border2_color_value,
            border2_width,
            border2_direction,
            width,
            height,
            pivot_x,
            pivot_y,
            shape_type: shape.shape_type.clone(),
            no_fill,
            shape_extra,
            shape_extra2,
            shape_extra3,
            shape_extra4,
            shape_extra5,
            shape_extra6,
            shape_extra7,
            gradient_type,
            gradient_start_color,
            gradient_end_color,
            gradient_points,
        }
    } else if shape.fill_type == "media" && !shape.fill_image.is_empty() {
        AmLayerSpec::SpriteShape {
            image_uri: shape.fill_image.clone(),
            is_media: true,
            fill_color: None,
            width,
            height,
            anchor,
        }
    } else {
        AmLayerSpec::SpriteShape {
            image_uri: String::new(),
            is_media: false,
            fill_color: shape.fill_color.clone(),
            width,
            height,
            anchor,
        }
    };

    // For SDF shapes, anchor_offset moves parent from center to pivot point
    // For SpriteShape, use the computed compensation
    let anchor_offset = if needs_sdf {
        // SDF parent needs to be offset from center to pivot point
        Vec2::new(pivot_x, -pivot_y)
    } else {
        Vec2::new(comp_x, comp_y)
    };

    let stroke_width_anim = {
        let has_path_stroke = shape.stroke.is_some();
        let border_scale = if has_path_stroke {
            1.0
        } else {
            let direction = shape
                .borders
                .first()
                .map(|b| b.direction.as_str())
                .unwrap_or("centered");
            if direction == "centered" {
                1.0
            } else {
                config.canvas_width / 2048.0
            }
        };
        let mut anim =
            get_stroke_width_animation(shape.stroke.as_ref().or_else(|| shape.borders.first()));
        // Scale border-sourced keyframe values by comp_width/2048
        if border_scale != 1.0 {
            if let Some(ref mut v) = anim.value {
                *v *= border_scale;
            }
            for kf in &mut anim.keyframes {
                if let Ok(val) = kf.value.parse::<f32>() {
                    kf.value = (val * border_scale).to_string();
                }
            }
        }
        anim
    };

    Some(PendingLayer {
        id: shape.id,
        label: shape.label.clone(),
        parent: shape.parent,
        start_time: shape.start_time,
        end_time: shape.end_time,
        transform,
        animated: AmAnimated {
            layer_id: shape.id,
            start_time: shape.start_time,
            end_time: shape.end_time,
            time_offset: config.time_offset,
            lifecycle_offset: config.lifecycle_offset,
            location: shape.transform.location.clone(),
            pivot: shape.transform.pivot.clone(),
            rotation: shape.transform.rotation.clone(),
            scale: shape.transform.scale.clone(),
            opacity: shape.transform.opacity.clone(),
            canvas_width: config.canvas_width,
            canvas_height: config.canvas_height,
            has_parent,
            parent_layer_id: shape.parent,
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
            size: size_animation,
            anchor_offset,
            wipe_start: wipe_effect.start,
            wipe_end: wipe_effect.end,
            wipe_angle: wipe_effect.angle,
            wipe_feather: wipe_effect.feather,
            stretch_angle: stretch_segment.angle,
            stretch_amount: stretch_segment.stretch,
            stretch_offset: stretch_segment.offset,
            stretch_smooth: stretch_segment.smooth,
            blur_strength: gaussian_blur.strength,
            speed_multiplier: config.speed_multiplier,
            element_speed: shape.speed,
            scene_fps: config.scene_fps,
            embed_offset: Vec2::ZERO,
            inv_fit_scale: 1.0,
            stroke_width: stroke_width_anim,
            // If shape is marked hidden in AM, force base_alpha to 0 so it's never visible
            base_alpha: if shape.hidden {
                0.0
            } else {
                get_base_alpha(&shape.fill_color, shape.fill_type == "none")
            },
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
            base_fill_color: get_initial_fill_color_rgba(
                &shape.fill_color,
                shape.fill_type == "none",
            ),
            path_repeat: {
                let pr = extract_path_repeat_effect(&shape.effects);
                if pr.has_effect() { Some(pr) } else { None }
            },
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
            shape_props,
            shape_points,
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
        spec,
        z_index: z,
        children: Vec::new(),
        blending_mode: match shape.blending.as_str() {
            "mask" => AmBlendingMode::Mask,
            "exclude" => AmBlendingMode::Exclude,
            _ => AmBlendingMode::Normal,
        },
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
    })
}

/// Extract shape-specific extra parameters based on shape type.
/// Returns (shape_extra, shape_extra2, ..., shape_extra7) as Vec4 values.
pub(crate) fn extract_shape_extras(
    shape_type: &str,
    properties: &[crate::schema::AmProperty],
    path_data: &str,
) -> (Vec4, Vec4, Vec4, Vec4, Vec4, Vec4, Vec4) {
    use super::helpers::*;
    let z = Vec4::ZERO;
    match shape_type {
        ".roundrect" => {
            let corner_radius = get_shape_float_property(properties, "cornerRadius", 25.0);
            (Vec4::new(corner_radius, 0.0, 0.0, 0.0), z, z, z, z, z, z)
        }
        ".poly" => {
            let side_count = get_shape_float_property(properties, "sideCount", 6.0);
            let radius = get_shape_float_property(properties, "radius", 100.0);
            let offset_angle = get_shape_float_property(properties, "offsetAngle", 0.0);
            (
                Vec4::new(side_count, radius, offset_angle, 0.0),
                z,
                z,
                z,
                z,
                z,
                z,
            )
        }
        ".star" => {
            let point_count = get_shape_float_property(properties, "pointCount", 5.0);
            let outer_radius = get_shape_float_property(properties, "outerRadius", 100.0);
            let inner_radius = get_shape_float_property(properties, "innerRadius", 50.0);
            let offset_angle = get_shape_float_property(properties, "offsetAngle", 0.0);
            (
                Vec4::new(point_count, outer_radius, inner_radius, offset_angle),
                z,
                z,
                z,
                z,
                z,
                z,
            )
        }
        ".pie" => {
            let start_angle = get_shape_float_property(properties, "startAngle", 0.0);
            let end_angle = get_shape_float_property(properties, "endAngle", 90.0);
            let radius = get_shape_float_property(properties, "radius", 100.0);
            (
                Vec4::new(start_angle, end_angle, radius, 0.0),
                z,
                z,
                z,
                z,
                z,
                z,
            )
        }
        ".plus" => {
            let stem_size = get_shape_float_property(properties, "stemSize", 50.0);
            (Vec4::new(stem_size, 0.0, 0.0, 0.0), z, z, z, z, z, z)
        }
        ".multifoil" => {
            let point_count = get_shape_float_property(properties, "pointCount", 5.0);
            let outer_radius = get_shape_float_property(properties, "outerRadius", 100.0);
            let inner_radius = get_shape_float_property(properties, "innerRadius", 50.0);
            let offset_angle = get_shape_float_property(properties, "offsetAngle", 0.0);
            (
                Vec4::new(point_count, outer_radius, inner_radius, offset_angle),
                z,
                z,
                z,
                z,
                z,
                z,
            )
        }
        ".line" => {
            let p1 = get_shape_vec2_property(properties, "p1", [-100.0, 0.0]);
            let p2 = get_shape_vec2_property(properties, "p2", [100.0, 0.0]);
            (Vec4::new(p1[0], p1[1], p2[0], p2[1]), z, z, z, z, z, z)
        }
        ".arc" => {
            let start_angle = get_shape_float_property(properties, "startAngle", 0.0);
            let end_angle = get_shape_float_property(properties, "endAngle", 90.0);
            let radius = get_shape_float_property(properties, "radius", 100.0);
            (
                Vec4::new(start_angle, end_angle, radius, 0.0),
                z,
                z,
                z,
                z,
                z,
                z,
            )
        }
        ".triangle" => {
            let p1 = get_shape_vec2_property(properties, "p1", [-100.0, 100.0]);
            let p2 = get_shape_vec2_property(properties, "p2", [0.0, -100.0]);
            let p3 = get_shape_vec2_property(properties, "p3", [100.0, 100.0]);
            (
                Vec4::new(p1[0], p1[1], p2[0], p2[1]),
                Vec4::new(p3[0], p3[1], 0.0, 0.0),
                z,
                z,
                z,
                z,
                z,
            )
        }
        ".quad" => {
            let p1 = get_shape_vec2_property(properties, "p1", [-100.0, -100.0]);
            let p2 = get_shape_vec2_property(properties, "p2", [100.0, -100.0]);
            let p3 = get_shape_vec2_property(properties, "p3", [100.0, 100.0]);
            let p4 = get_shape_vec2_property(properties, "p4", [-100.0, 100.0]);
            (
                Vec4::new(p1[0], p1[1], p2[0], p2[1]),
                Vec4::new(p3[0], p3[1], p4[0], p4[1]),
                z,
                z,
                z,
                z,
                z,
            )
        }
        ".penta" => {
            let p1 = get_shape_vec2_property(properties, "p1", [-100.0, -100.0]);
            let p2 = get_shape_vec2_property(properties, "p2", [0.0, -100.0]);
            let p3 = get_shape_vec2_property(properties, "p3", [0.0, 0.0]);
            let p4 = get_shape_vec2_property(properties, "p4", [100.0, 100.0]);
            let p5 = get_shape_vec2_property(properties, "p5", [-100.0, 100.0]);
            (
                Vec4::new(p1[0], p1[1], p2[0], p2[1]),
                Vec4::new(p3[0], p3[1], p4[0], p4[1]),
                Vec4::new(p5[0], p5[1], 0.0, 0.0),
                z,
                z,
                z,
                z,
            )
        }
        _ if shape_type.is_empty() && !path_data.is_empty() => {
            // Freeform path: parse path data into vertices
            parse_path_extras(path_data)
        }
        _ => (z, z, z, z, z, z, z),
    }
}

/// Parse SVG-like path data into vertex vec4s for the shader.
/// Supports M (move) and L (line) commands. Stores up to 13 vertices + vertex count.
pub(crate) fn parse_path_extras(path_data: &str) -> (Vec4, Vec4, Vec4, Vec4, Vec4, Vec4, Vec4) {
    let mut vertices: Vec<f32> = Vec::new();
    // Pre-process: insert spaces around SVG command letters
    let mut cleaned = String::with_capacity(path_data.len() + 20);
    for c in path_data.chars() {
        if c.is_ascii_alphabetic() {
            cleaned.push(' ');
            cleaned.push(c);
            cleaned.push(' ');
        } else {
            cleaned.push(c);
        }
    }
    let tokens: Vec<&str> = cleaned.split_whitespace().collect();
    let mut i = 0;
    while i < tokens.len() {
        match tokens[i] {
            "M" | "L" | "m" | "l" => {
                if i + 2 < tokens.len() {
                    if let (Ok(x), Ok(y)) =
                        (tokens[i + 1].parse::<f32>(), tokens[i + 2].parse::<f32>())
                        && vertices.len() < 26
                    {
                        // 13 vertices max (26 floats)
                        vertices.push(x);
                        vertices.push(y);
                    }
                    i += 3;
                } else {
                    i += 1;
                }
            }
            "Z" | "z" => {
                i += 1;
            }
            _ => {
                // Try parsing as coordinate pair (implicit L command)
                if i + 1 < tokens.len()
                    && let (Ok(x), Ok(y)) = (tokens[i].parse::<f32>(), tokens[i + 1].parse::<f32>())
                {
                    if vertices.len() < 26 {
                        vertices.push(x);
                        vertices.push(y);
                    }
                    i += 2;
                    continue;
                }
                i += 1;
            }
        }
    }
    let vertex_count = (vertices.len() / 2) as f32;
    // Pad to 26 floats (13 vertices)
    while vertices.len() < 26 {
        vertices.push(0.0);
    }
    (
        Vec4::new(vertices[0], vertices[1], vertices[2], vertices[3]),
        Vec4::new(vertices[4], vertices[5], vertices[6], vertices[7]),
        Vec4::new(vertices[8], vertices[9], vertices[10], vertices[11]),
        Vec4::new(vertices[12], vertices[13], vertices[14], vertices[15]),
        Vec4::new(vertices[16], vertices[17], vertices[18], vertices[19]),
        Vec4::new(vertices[20], vertices[21], vertices[22], vertices[23]),
        Vec4::new(vertices[24], vertices[25], vertex_count, 0.0),
    )
}

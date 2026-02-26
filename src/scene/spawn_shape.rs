//! # spawn_shape.rs
//!
//! # 形状图层生成
//!
//! Shape layer spawning functions.
//! 形状图层的实体生成函数。

use bevy::prelude::*;
use std::collections::HashMap;

use crate::animation::AmAnimated;
use crate::effects::NeedsStrategyEvaluation;
use crate::schema::{AmAnimatedFloat, AmAnimatedVec2, AmShape};
use crate::sdf::AmSdfShaders;

use super::components::*;
use super::effects::*;
use super::helpers::*;
use super::spawn::extract_gradient_data;

/// Spawn a shape layer (lazy - visual components spawned later by lifecycle system).
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_shape(
    commands: &mut Commands,
    _shaders: &mut Assets<Shader>,
    shape: &AmShape,
    _images: &HashMap<String, Handle<Image>>,
    _white_pixel: &Handle<Image>,
    _sdf_shaders: &AmSdfShaders,
    config: &AmSceneConfig,
    z: f32,
) -> Entity {
    // Get initial transform values - use local coords if has parent
    let has_parent = shape.parent != 0;
    let (tx, ty) = get_initial_location(&shape.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&shape.transform.rotation);
    let (sx, sy) = get_initial_scale(&shape.transform.scale);
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
    let scale_assist = extract_scale_assist_effect(&shape.effects);
    let stretch2_effect = extract_stretch2_effect(&shape.effects);
    let repeat_effect = extract_repeat_effect(&shape.effects);
    let (linear_repeat_effect, linear_repeat_effect2) =
        extract_linear_repeat_effects(&shape.effects);
    let radial_repeat_effect = extract_radial_repeat_effect(&shape.effects);
    let swing_effect = extract_swing_effect(&shape.effects);
    let oscillate_effect = extract_oscillate_effect(&shape.effects);
    let spin_rpm = extract_spin_rpm(&shape.effects);
    let threshold_effect = extract_threshold_effect(&shape.effects);
    let grid_effect = extract_grid_effect(&shape.effects);
    let pixelate_effect = extract_pixelate_effect(&shape.effects);
    let solid_color_effect = extract_solid_color_effect(&shape.effects);
    let path_repeat_effect = extract_path_repeat_effect(&shape.effects);
    let (pivot_x, pivot_y) = get_initial_pivot(&shape.transform.pivot);

    // Get size from properties
    let (width, height) = get_shape_size(&shape.properties, &shape.fill_type);

    // AM location points to object CENTER, not pivot. No position compensation needed.
    // Pivot only affects rotation/scale center, which is handled by Anchor.
    bevy::log::trace!(
        "Registering shape '{}' (id={}, parent={}, hidden={}): pos=({:.1},{:.1}), z={:.1}, scale=({:.2},{:.2}), size=({:.0},{:.0}), pivot=({:.1},{:.1}), fill={}, image={}",
        shape.label,
        shape.id,
        shape.parent,
        shape.hidden,
        tx,
        ty,
        z,
        sx,
        sy,
        width,
        height,
        pivot_x,
        pivot_y,
        shape.fill_type,
        shape.fill_image
    );

    // Create entity name for inspector identification
    let entity_name = format!("Shape[{}]: {}", shape.id, shape.label);

    // Check if this is a stroked shape that needs SDF rendering
    // Also use SDF for circles (better quality than sprite rect)
    // fillType="none" also needs SDF for stroke-only rendering (no fill)
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
    // because Bevy draws sprite with anchor point at translation position
    // For SDF shapes, parent should be at pivot point (for rotation/scale around pivot)
    let (final_tx, final_ty) = if needs_sdf {
        // SDF parent is at pivot point: AM center + pivot offset (with Y flip)
        (tx + pivot_x, ty - pivot_y)
    } else {
        // SpriteShape: compensate position so center stays at AM location
        (tx + comp_x, ty + comp_y)
    };

    let transform = Transform {
        translation: Vec3::new(final_tx, final_ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    // Create the layer spec for lazy spawning
    let layer_spec = if needs_sdf {
        let default_stroke = crate::schema::AmStroke::default();
        // Use path-stroke if available, otherwise fall back to first border
        let stroke = shape
            .stroke
            .as_ref()
            .unwrap_or_else(|| shape.borders.first().unwrap_or(&default_stroke));
        // Get initial stroke width: first check <size> element, then fall back to @end-size attribute
        let stroke_width = stroke
            .size
            .as_ref()
            .and_then(|s| {
                // Prefer static value, fall back to first keyframe value
                s.value
                    .or_else(|| s.keyframes.first().and_then(|kf| kf.value.parse().ok()))
            })
            .unwrap_or({
                // Fall back: AM's default stroke size for path-stroke is 4.0
                // (from KeyableEdgeDecoration.NO_STROKE template)
                // end-size is a separate attribute (end cap size multiplier), not stroke width
                4.0
            });
        let stroke_color_value = stroke
            .color
            .as_ref()
            .map(|c| c.value.clone())
            .unwrap_or_default();

        // Track whether this is a "no fill" shape (fillType="none")
        // This is different from having no fillColor value (defaults to white)
        let no_fill = shape.fill_type == "none";

        // Extract second border data if present
        let border2 = shape.borders.get(1);
        let border2_width = border2
            .and_then(|b| {
                b.size.as_ref().and_then(|s| {
                    s.value
                        .or_else(|| s.keyframes.first().and_then(|kf| kf.value.parse().ok()))
                })
            })
            .unwrap_or(0.0);
        let border2_color_value = border2
            .and_then(|b| b.color.as_ref().map(|c| c.value.clone()))
            .unwrap_or_default();
        let border2_direction = border2.map(|b| b.direction.clone()).unwrap_or_default();

        if border2_width > 0.0 {
            bevy::log::debug!(
                "[SPAWN] '{}': border2 width={}, color='{}', direction='{}', borders_count={}",
                shape.label,
                border2_width,
                border2_color_value,
                border2_direction,
                shape.borders.len()
            );
        }

        // Extract shape-specific extra parameters
        let (
            shape_extra,
            shape_extra2,
            shape_extra3,
            shape_extra4,
            shape_extra5,
            shape_extra6,
            shape_extra7,
        ) = super::collect_shape::extract_shape_extras(
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
            extract_gradient_data(&shape.gradient);

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
        // Color fill
        AmLayerSpec::SpriteShape {
            image_uri: String::new(),
            is_media: false,
            fill_color: shape.fill_color.clone(),
            width,
            height,
            anchor,
        }
    };

    // Spawn the layer entity without visual components (they'll be added by lifecycle system)
    // For SDF shapes, anchor_offset moves parent from center to pivot point
    // For SpriteShape, use the computed compensation
    let anchor_offset = if needs_sdf {
        // SDF parent needs to be offset from center to pivot point
        Vec2::new(pivot_x, -pivot_y)
    } else {
        Vec2::new(comp_x, comp_y)
    };

    let stroke_width_anim =
        get_stroke_width_animation(shape.stroke.as_ref().or_else(|| shape.borders.first()));
    let no_fill = shape.fill_type == "none";
    // If shape is marked hidden in AM, force base_alpha to 0 so it's never visible
    let base_alpha = if shape.hidden {
        0.0
    } else {
        get_base_alpha(&shape.fill_color, no_fill)
    };
    let palette_map = extract_palette_map_effect(&shape.effects);
    let replace_color = extract_replace_color_effect(&shape.effects);

    let entity = commands
        .spawn((
            Name::new(entity_name),
            AmLayerMarker {
                id: shape.id,
                label: shape.label.clone(),
            },
            AmAnimated {
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
                size: get_shape_size_animation(&shape.properties),
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
                base_alpha,
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
                base_fill_color: get_initial_fill_color_rgba(&shape.fill_color, no_fill),
                path_repeat: if path_repeat_effect.has_effect() {
                    Some(path_repeat_effect)
                } else {
                    None
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
                shape_props: Default::default(),
                shape_points: Default::default(),
                retime: config.retime.clone(),
            },
            layer_spec,
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

    // If shape is marked as hidden in AM, force it to stay hidden
    if shape.hidden {
        commands.entity(entity).insert(AmForceHidden);
    }

    entity
}

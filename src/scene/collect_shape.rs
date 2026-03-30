//! # collect_shape.rs
//!
//! # 形状图层收集
//!
//! Functions for collecting shape layer data into PendingLayer.
//! 形状图层数据收集为 PendingLayer 的函数。

mod shape_extras;
mod spec;

use super::helpers;
use bevy::prelude::*;

use crate::animation::AmAnimated;
use crate::schema::{AmAnimatedFloat, AmShape};

use self::spec::build_shape_spec;
use super::components::*;
use super::effects::*;
use super::helpers::*;
pub(crate) use shape_extras::extract_shape_extras;

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
    let all_stretch_segments = extract_all_stretch_segment_effects(&shape.effects);
    let stretch_segment = all_stretch_segments.first().cloned().unwrap_or_default();
    let gaussian_blur = extract_gaussian_blur_effect(&shape.effects);
    let palette_map = extract_palette_map_effect(&shape.effects);
    let scale_assist = extract_scale_assist_effect(&shape.effects);
    let parent_helper = extract_parent_helper_effect(&shape.effects);
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
    let linear_repeat_after_stretch_segment = linear_repeat_after_stretch_segment(&shape.effects);
    let radial_repeat_effect = extract_radial_repeat_effect(&shape.effects);
    let swing_effect = extract_swing_effect(&shape.effects);
    let oscillate_effect = extract_oscillate_effect(&shape.effects);
    let jitter_effect = extract_jitter_effect(&shape.effects);
    let sd_effect = extract_simplex_displace_effect(&shape.effects);
    let rgb_split_effect = extract_rgb_split_effect(&shape.effects);
    let exposure_gamma_effect = extract_exposure_gamma_effect(&shape.effects);
    let chromakey_effect = extract_chromakey_effect(&shape.effects);
    let spin_rpm = extract_spin_rpm(&shape.effects);
    let threshold_effect = extract_threshold_effect(&shape.effects);
    let grid_effect = extract_grid_effect(&shape.effects);
    let pixelate_effect = extract_pixelate_effect(&shape.effects);
    let solid_color_effect = extract_solid_color_effect(&shape.effects);
    let fade_effect = extract_fade_effect(&shape.effects);
    let wavewarp2_effect = extract_wavewarp2_effect(&shape.effects);
    let mirror_effect = extract_mirror_effect(&shape.effects);
    let lift_effect = extract_lift_effect(&shape.effects);
    let rays_effect = extract_rays_effect(&shape.effects);
    if scale_assist.axis != 0 {
        bevy::log::info!(
            "[COLLECT] Shape '{}' has scale_assist: axis={}, has_keyframes={}",
            shape.label,
            scale_assist.axis,
            !scale_assist.scale.keyframes.is_empty()
        );
    }
    let (pivot_x, pivot_y) = get_initial_pivot(&shape.transform.pivot);
    let (width, height) = get_shape_size(&shape.properties, &shape.shape_type, &shape.fill_type);
    let size_animation = get_shape_size_animation(&shape.properties, &shape.shape_type);

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

    let spec = build_shape_spec(
        shape, config, needs_sdf, width, height, pivot_x, pivot_y, anchor,
    );

    // For SDF shapes, anchor_offset moves parent from center to pivot point
    // For SpriteShape, use the computed compensation
    let anchor_offset = if needs_sdf {
        // SDF parent needs to be offset from center to pivot point
        Vec2::new(pivot_x, -pivot_y)
    } else {
        Vec2::new(comp_x, comp_y)
    };

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
    let mut stroke_width_anim =
        get_stroke_width_animation(shape.stroke.as_ref().or_else(|| shape.borders.first()));
    // Scale border-sourced keyframe values by comp_width/2048
    if border_scale != 1.0 {
        if let Some(ref mut v) = stroke_width_anim.value {
            *v *= border_scale;
        }
        for kf in &mut stroke_width_anim.keyframes {
            let Ok(val) = kf.value.parse::<f32>() else {
                continue;
            };
            kf.value = (val * border_scale).to_string();
        }
    }

    Some(PendingLayer {
        id: shape.id,
        label: shape.label.clone(),
        parent: shape.parent,
        is_perspective_null: false,
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
            scale_baked_into_mesh: needs_sdf,
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
            // AM serializes media speed via SceneElement speedMap/timeMapping, which
            // affects source-time trimming rather than transform/effect keyframes.
            // Applying it to layer_time makes still media shapes animate too slowly.
            element_speed: if shape.fill_type == "media" {
                1.0
            } else {
                shape.speed
            },
            scene_fps: config.scene_fps,
            embed_offset: Vec2::ZERO,
            inv_fit_scale: 1.0,
            stroke_width: stroke_width_anim,
            // If shape is marked hidden in AM, force base_alpha to 0 so it's never visible
            base_alpha: if shape.hidden {
                0.0
            } else {
                get_base_alpha(&shape.fill_color, shape.fill_type == "none")
                    * config.repeat_alpha_factor
            },
            fade_in_time: fade_effect.in_time,
            fade_out_time: fade_effect.out_time,
            fade_layer_duration_ms: (shape.end_time - shape.start_time) as f32,
            palette_alpha: palette_map.alpha.clone(),
            scale_assist: scale_assist.scale,
            scale_assist_damp: scale_assist.damp,
            scale_assist_axis: scale_assist.axis,
            parenthelper_scale_mode: parent_helper.scale_mode,
            parenthelper_rotate_mode: parent_helper.rotate_mode,
            parenthelper_scale_weight: parent_helper.scale_weight,
            parenthelper_rotate_weight: parent_helper.rotate_weight,
            parenthelper_auto_rotate: parent_helper.auto_rotate,
            parenthelper_radius_adjust: parent_helper.radius_adjust,
            parenthelper_has_effect: parent_helper.has_effect,
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
            linear_repeat_after_stretch_segment,
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
            fill_color: fill_color_to_animated(&shape.fill_color),
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
            counter_offset: AmAnimatedFloat::default(),
            counter_scale: AmAnimatedFloat::default(),
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
            // Simplex displace effect
            sd_enabled: sd_effect.enabled,
            sd_mag: sd_effect.mag,
            sd_evolution: sd_effect.evolution,
            sd_seed: sd_effect.seed,
            sd_scatter: sd_effect.scatter,
            rgb_split_enabled: rgb_split_effect.enabled,
            rgb_split_strength: rgb_split_effect.strength,
            rgb_split_angle: rgb_split_effect.angle,
            rgb_split_center: rgb_split_effect.center_channel,
            rgb_split_mode: rgb_split_effect.mode,
            exposure_value: exposure_gamma_effect.exposure,
            exposure_gamma: exposure_gamma_effect.gamma,
            exposure_offset: exposure_gamma_effect.offset,
            exposure_has_effect: exposure_gamma_effect.has_effect,
            chromakey_enabled: chromakey_effect.enabled,
            chromakey_key_color: chromakey_effect.key_color,
            chromakey_threshold: chromakey_effect.threshold,
            chromakey_feather: chromakey_effect.feather,
            chromakey_defringe: chromakey_effect.defringe,
            chromakey_invert: chromakey_effect.invert,
            blend_mode: AmBlendingMode::default(),
            retime: config.retime.clone(),
            echo_time_shift_ms: config.echo_time_shift_ms,
            echo_alpha_config: config.echo_alpha_config.clone(),
            repeat_rotation_offset_deg: 0.0,
            repeat_scale_factor: 1.0,
            repeat_position_offset: Vec2::ZERO,
            embed_inner_total_time: None,
        },
        spec,
        z_index: z,
        children: Vec::new(),
        blending_mode: AmBlendingMode::parse_am(shape.blending.as_str()),
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
        embed_render_plan: None,
        embed_inner_total_time: None,
        hidden: shape.hidden,
    })
}

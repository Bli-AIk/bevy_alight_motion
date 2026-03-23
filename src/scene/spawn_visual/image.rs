use bevy::prelude::*;
use std::collections::HashMap;

use crate::animation::AmAnimated;
use crate::schema::{AmAnimatedFloat, AmAnimatedVec2};

use super::super::components::*;
use super::super::effects::*;
use super::super::helpers::*;

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
    let parent_helper = extract_parent_helper_effect(&image.effects);
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
    let lift_effect = extract_lift_effect(&image.effects);
    let rays_effect = extract_rays_effect(&image.effects);

    let (width, height) = get_shape_size(&image.properties, "", &image.fill_type);
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
                base_alpha: config.repeat_alpha_factor,
                fade_in_time: fade_effect.in_time,
                fade_out_time: fade_effect.out_time,
                fade_layer_duration_ms: (image.end_time - image.start_time) as f32,
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
                swing_freq: swing_effect.freq,
                swing_a1: swing_effect.a1,
                swing_a2: swing_effect.a2,
                swing_phase: swing_effect.phase,
                swing_type: swing_effect.swing_type,
                oscillate_direction: oscillate_effect.direction,
                oscillate_angle: oscillate_effect.angle,
                oscillate_freq: oscillate_effect.freq,
                oscillate_mag: oscillate_effect.mag,
                oscillate_wave_type: oscillate_effect.wave_type,
                oscillate_phase: oscillate_effect.phase,
                spin_rpm,
                threshold_value: threshold_effect.threshold,
                threshold_feather: threshold_effect.feather,
                threshold_invert: threshold_effect.invert,
                threshold_blend_mode: threshold_effect.blend_mode,
                grid_position: grid_effect.position,
                grid_spacing: grid_effect.spacing,
                grid_width: grid_effect.width,
                grid_color: grid_effect.color,
                grid_punchout: grid_effect.punchout,
                grid_smoothing: grid_effect.smoothing,
                grid_screen_space: grid_effect.screen_space,
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
            AmLayerSpec::Image {
                image_uri: image.fill_image.clone(),
                width,
                height,
                anchor,
            },
            transform,
            GlobalTransform::default(),
            Visibility::Hidden,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    if palette_map.has_effect() {
        commands
            .entity(entity)
            .insert(AmPaletteMapParams::from_params(&palette_map));
    }

    entity
}

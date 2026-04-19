//! Collects Alight Motion null layers into pending runtime layers.
//! Nulls do not render by themselves, but they carry transform hierarchies and
//! many reusable motion effects, so this collector converts them into the same
//! animated runtime representation used by visible layers.
//!
//! 负责把 Alight Motion 的 null 图层收集成待生成的运行时图层。Null
//! 自身不直接渲染，但它们承载层级变换和大量可复用的运动效果，因此这个收集器会把
//! 它们转换成与可见图层一致的动画运行时表示。

use bevy::prelude::*;

use crate::animation::AmAnimated;
use crate::schema::{AmAnimatedFloat, AmAnimatedVec2};

use super::super::components::*;
use super::super::effects::*;
use super::super::helpers::*;

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
    let parent_helper = extract_parent_helper_effect(&null.effects);
    let stretch2_effect = extract_stretch2_effect(&null.effects);
    let replace_color = extract_replace_color_effect(&null.effects);
    let repeat_effect = extract_repeat_effect(&null.effects);
    let (linear_repeat_effect, linear_repeat_effect2) =
        extract_linear_repeat_effects(&null.effects);
    let linear_repeat_after_stretch_segment = false;
    let radial_repeat_effect = extract_radial_repeat_effect(&null.effects);
    let swing_effect = extract_swing_effect(&null.effects);
    let oscillate_effect = extract_oscillate_effect(&null.effects);
    let jitter_effect = extract_jitter_effect(&null.effects);
    let sd_effect = extract_simplex_displace_effect(&null.effects);
    let rgb_split_effect = extract_rgb_split_effect(&null.effects);
    let spin_rpm = extract_spin_rpm(&null.effects);
    let threshold_effect = extract_threshold_effect(&null.effects);
    let grid_effect = extract_grid_effect(&null.effects);
    let pixelate_effect = extract_pixelate_effect(&null.effects);
    let solid_color_effect = extract_solid_color_effect(&null.effects);
    let fade_effect = extract_fade_effect(&null.effects);
    let wavewarp2_effect = extract_wavewarp2_effect(&null.effects);
    let mirror_effect = extract_mirror_effect(&null.effects);
    let lift_effect = extract_lift_effect(&null.effects);
    let rays_effect = extract_rays_effect(&null.effects);
    let exposure_gamma_effect = extract_exposure_gamma_effect(&null.effects);
    let chromakey_effect = extract_chromakey_effect(&null.effects);
    let transform = Transform {
        translation: Vec3::new(tx, ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    Some(PendingLayer {
        id: null.id,
        label: null.label.clone(),
        parent: null.parent,
        is_perspective_null: null.obj_type == "perspective",
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
            scale_baked_into_mesh: false,
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
            base_alpha: config.repeat_alpha_factor,
            fade_in_time: fade_effect.in_time,
            fade_out_time: fade_effect.out_time,
            fade_layer_duration_ms: (null.end_time - null.start_time) as f32,
            palette_alpha: AmAnimatedFloat::default(),
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
            wavewarp2_phase: wavewarp2_effect.phase.clone(),
            wavewarp2_a1d: wavewarp2_effect.a1d.clone(),
            wavewarp2_m1: wavewarp2_effect.m1.clone(),
            wavewarp2_m2: wavewarp2_effect.m2.clone(),
            wavewarp2_a2d: wavewarp2_effect.a2d.clone(),
            wavewarp2_damping: wavewarp2_effect.damping.clone(),
            wavewarp2_damping_space: wavewarp2_effect.damping_space.clone(),
            wavewarp2_damping_origin: wavewarp2_effect.damping_origin.clone(),
            wavewarp2_screen_space: wavewarp2_effect.screen_space,
            wavewarp2_has_effect: wavewarp2_effect.has_effect,
            mirror_type: mirror_effect.mirror_type,
            mirror_blend_mode: mirror_effect.blend_mode,
            mirror_alpha: mirror_effect.alpha.clone(),
            mirror_offset: mirror_effect.offset.clone(),
            mirror_has_effect: mirror_effect.has_effect,
            lift_fill: lift_effect.fill.clone(),
            lift_has_effect: lift_effect.has_effect,
            rays_center_x: rays_effect.center_x.clone(),
            rays_center_y: rays_effect.center_y.clone(),
            rays_strength: rays_effect.strength.clone(),
            rays_intensity: rays_effect.intensity.clone(),
            rays_threshold: rays_effect.threshold.clone(),
            rays_threshold_color: rays_effect.threshold_color,
            rays_fill_color: rays_effect.fill_color,
            rays_blend: rays_effect.blend.clone(),
            rays_quality: rays_effect.quality.clone(),
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
            linear_repeat_after_stretch_segment,
            linear_repeat2: linear_repeat_effect2.map(Box::new),
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
            swing_freq: swing_effect.freq,
            swing_a1: swing_effect.a1,
            swing_a2: swing_effect.a2,
            swing_phase: swing_effect.phase,
            swing_type: swing_effect.swing_type,
            oscillate_direction: oscillate_effect.direction,
            oscillate_angle: oscillate_effect.angle.clone(),
            oscillate_freq: oscillate_effect.freq.clone(),
            oscillate_mag: oscillate_effect.mag.clone(),
            oscillate_wave_type: oscillate_effect.wave_type,
            oscillate_phase: oscillate_effect.phase.clone(),
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
            jitter_enabled: jitter_effect.enabled,
            jitter_angle: jitter_effect.angle,
            jitter_freq: jitter_effect.freq,
            jitter_mag: jitter_effect.mag,
            jitter_seed: jitter_effect.seed,
            jitter_slack: jitter_effect.slack,
            jitter_zjitter: jitter_effect.zjitter,
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
        embed_render_plan: None,
        embed_inner_total_time: None,
        hidden: null.hidden,
    })
}

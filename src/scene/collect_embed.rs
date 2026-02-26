//! # collect_embed.rs
//!
//! # 嵌入场景收集
//!
//! Functions for collecting embed scene layer data into PendingLayer.
//! 嵌入场景图层数据收集为 PendingLayer 的函数。

use bevy::prelude::*;
use std::collections::HashMap;

use crate::animation::AmAnimated;
use crate::loader::FontMetrics;
use crate::schema::{AmAnimatedFloat, AmAnimatedVec2};

use super::collect::{apply_mask_to_children, collect_pending_layers};
use super::components::*;
use super::effects::*;
use super::helpers::*;


/// Collect an embed scene's data recursively.
pub(crate) fn collect_embed_scene(
    embed: &crate::schema::AmEmbedScene,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    config: &AmSceneConfig,
    z: f32,
) -> PendingLayer {
    let has_parent = embed.parent != 0;
    let (mut tx, mut ty) = get_initial_location(&embed.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&embed.transform.rotation);
    let (sx, sy) = get_initial_scale(&embed.transform.scale);
    let pivot = get_initial_pivot(&embed.transform.pivot);

    // For embed scenes with rotation/scale and non-zero pivot, we need to calculate
    // the correct position compensation. In AM, objects rotate/scale around (location + pivot).
    // Bevy rotates/scales around the Transform.translation, so we need to adjust.
    let (comp_x, comp_y) =
        calculate_embed_position_compensation(pivot, (sx, sy), rotation, has_parent);
    tx += comp_x;
    ty += comp_y;

    let transform = Transform {
        translation: Vec3::new(tx, ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    // Collect children with nested config
    // Nested scenes use smaller z_spacing to keep all children within
    // the parent's z-range (between parent and next sibling)
    // Using /100 instead of /1000 for better numerical precision
    let nested_z_spacing = config.z_spacing / 100.0;

    // Calculate the internal time offset for the embedded scene.
    // When the parent timeline reaches startTime, the embedded scene should be at inTime.
    //
    // The formula for local_time in the animation system is:
    //   local_time = (global_time - time_offset) * speed_multiplier
    //
    // embed.start_time is relative to PARENT's internal time, not global time.
    // When parent's internal time = embed.start_time, child should start.
    // Parent internal time = (global_time - parent_time_offset) * parent_speed
    // global_start = parent_time_offset + embed.start_time / parent_speed
    let in_time = embed.in_time.unwrap_or(0) as f32;
    let effective_speed = config.speed_multiplier * embed.speed;
    let global_start = if config.speed_multiplier > 0.0 {
        config.time_offset as f32 + embed.start_time as f32 / config.speed_multiplier
    } else {
        config.time_offset as f32 + embed.start_time as f32
    };
    let time_offset_with_in_time = if effective_speed > 0.0 {
        global_start - in_time / effective_speed
    } else {
        global_start
    };

    // Lifecycle offset also needs to account for parent speed, since spawn/despawn
    // uses lifecycle_time = global_time - lifecycle_offset and compares with start_time/end_time.
    // When global_time = global_start, lifecycle_time should be 0 (or in_time if specified).
    // lifecycle_offset = global_start - in_time
    let lifecycle_offset_with_in_time = global_start - in_time;

    // Note: retime="off" means "don't retime" - use normal animation speed
    // It does NOT mean freeze animations. The parent's speed still applies.
    let nested_speed = effective_speed;

    bevy::log::trace!(
        "  [TimeOffset] embed '{}': parent_offset={}, start_time={}, in_time={}, speed={}, nested_offset={}, lifecycle_offset={}, nested_speed={}",
        embed.label,
        config.time_offset,
        embed.start_time,
        in_time,
        effective_speed,
        time_offset_with_in_time,
        lifecycle_offset_with_in_time,
        nested_speed
    );

    let nested_config = AmSceneConfig {
        canvas_width: embed.scene.width as f32,
        canvas_height: embed.scene.height as f32,
        time_offset: time_offset_with_in_time as i32,
        lifecycle_offset: lifecycle_offset_with_in_time as i32,
        z_spacing: nested_z_spacing,
        nesting_depth: config.nesting_depth + 1,
        speed_multiplier: nested_speed,
        scene_fps: embed.scene.fps as f32,
        ..config.clone()
    };

    let mut children = collect_pending_layers(&embed.scene, fonts, font_metrics, &nested_config);

    // Process mask relationships within this embed scene
    apply_mask_to_children(&mut children);

    // Propagate embed's replaceColor effect to children that don't have their own.
    // In AM, group effects apply after compositing children into an FBO.
    // For direct rendering, we approximate by applying the embed's replaceColor to each child.
    let embed_replace = extract_replace_color_effect(&embed.effects);
    if embed_replace.old_color != Vec4::ZERO {
        for child in &mut children {
            if child.animated.replace_old_color == Vec4::ZERO {
                child.animated.replace_old_color = embed_replace.old_color;
                child.animated.replace_new_color = embed_replace.new_color.clone();
                child.animated.replace_threshold = embed_replace.threshold.clone();
                child.animated.replace_feather = embed_replace.feather.clone();
                child.animated.replace_alpha = embed_replace.alpha.clone();
                child.animated.replace_lock_luminance = embed_replace.lock_luminance;
            }
        }
    }

    // Propagate embed's pixelate effect to children.
    // In AM, the embed renders children to FBO then applies its own pixelate as a post-process.
    // For children that already have their own pixelate with the same grid origin (centered shapes),
    // the group pixelation on the composited FBO doesn't change already-uniform cells.
    // We keep the child's own size rather than multiplying, which is more accurate for aligned grids.
    let embed_pixelate = extract_pixelate_effect(&embed.effects);
    if let Some(embed_pix_size) = embed_pixelate.size.value
        && embed_pix_size > 1.0
    {
        for child in &mut children {
            if child.animated.pixelate_size.value.is_some() {
                // Child already has pixelate: keep child's own size.
                // When both grids share the same origin (common for centered shapes),
                // the group re-pixelation on aligned uniform cells is a no-op.
            } else if child.animated.pixelate_size.keyframes.is_empty() {
                // Child has no pixelate: apply embed's pixelate directly
                child.animated.pixelate_size = embed_pixelate.size.clone();
                child.animated.pixelate_stretch = embed_pixelate.stretch.clone();
                child.animated.pixelate_angle = embed_pixelate.angle.clone();
                child.animated.pixelate_vignette = embed_pixelate.vignette.clone();
                child.animated.pixelate_threshold = embed_pixelate.threshold.clone();
                child.animated.pixelate_saturation = embed_pixelate.saturation.clone();
                child.animated.pixelate_screen_space = embed_pixelate.screen_space;
            }
        }
    }

    // Extract transform2 effects from embed
    let mut all_embed_transform2 = extract_all_transform2_effects(&embed.effects);
    let embed_transform2 = if all_embed_transform2.is_empty() {
        Transform2Params::default()
    } else {
        all_embed_transform2.remove(0)
    };
    let embed_extra_transform2 = all_embed_transform2;

    PendingLayer {
        id: embed.id,
        label: embed.label.clone(),
        parent: embed.parent,
        start_time: embed.start_time,
        end_time: embed.end_time,
        transform,
        animated: AmAnimated {
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
            blur_strength: AmAnimatedFloat::default(),
            speed_multiplier: config.speed_multiplier,
            element_speed: 1.0,
            scene_fps: config.scene_fps,
            embed_offset: Vec2::ZERO,
            inv_fit_scale: 1.0,
            stroke_width: AmAnimatedFloat::default(),
            base_alpha: get_base_alpha(&embed.fill_color, false),
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
            // Linear repeat effect (defaults for embed)
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
            // Swing effect (defaults for embed)
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
            // Threshold effect (defaults for embed)
            threshold_value: AmAnimatedFloat::default(),
            threshold_feather: AmAnimatedFloat::default(),
            threshold_invert: false,
            threshold_blend_mode: 0,
            // Grid effect (defaults for embed)
            grid_position: AmAnimatedVec2::default(),
            grid_spacing: AmAnimatedFloat::default(),
            grid_width: AmAnimatedFloat::default(),
            grid_color: crate::schema::AmAnimatedColor::default(),
            grid_punchout: false,
            grid_smoothing: AmAnimatedFloat::default(),
            grid_screen_space: false,
            // Pixelate effect (defaults for embed)
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
        },
        spec: AmLayerSpec::EmbedScene,
        z_index: z,
        children,
        blending_mode: AmBlendingMode::Normal,
        mask_info: None,
        palette_params: None,
        embed_scene_size: Some((embed.scene.width as f32, embed.scene.height as f32)),
        containing_embed_id: 0,
        from_deeply_nested_scene: config.nesting_depth > 1,
    }
}

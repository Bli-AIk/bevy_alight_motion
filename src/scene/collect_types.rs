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
use crate::schema::{
    AmAnimatedFloat, AmAnimatedVec2, AmShape, AmText,
};

use super::collect::{apply_mask_to_children, collect_pending_layers};
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
    let (effect_pos_x, effect_pos_y) = extract_effect_animations(&shape.effects);
    let wipe_effect = extract_wipe_effect(&shape.effects);
    let stretch_segment = extract_stretch_segment_effect(&shape.effects);
    let gaussian_blur = extract_gaussian_blur_effect(&shape.effects);
    let palette_map = extract_palette_map_effect(&shape.effects);
    let (pivot_x, pivot_y) = get_initial_pivot(&shape.transform.pivot);
    let (width, height) = get_shape_size(&shape.properties, &shape.fill_type);
    let size_animation = get_shape_size_animation(&shape.properties);

    let needs_sdf = shape.fill_type == "color"
        && (shape.shape_type == ".circle"
            || shape.stroke.as_ref().is_some_and(|s| {
                // Check if stroke has a size > 0 (either via <size> element or @end-size attribute)
                s.size
                    .as_ref()
                    .is_some_and(|sz| sz.value.unwrap_or(0.0) > 0.0 || !sz.keyframes.is_empty())
                    || s.end_size > 0.0
            }));

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

    let spec = if needs_sdf {
        let default_stroke = crate::schema::AmStroke::default();
        let stroke = shape.stroke.as_ref().unwrap_or(&default_stroke);
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
                // Fall back to @end-size attribute if no <size> element
                // Note: end-size appears to use a different scale than <size> element
                // AM shows stroke=2.0 as minimum visible, suggesting end-size needs scaling
                if stroke.end_size > 0.0 {
                    stroke.end_size * 1.5
                } else {
                    0.0
                }
            });
        let stroke_color_value = stroke
            .color
            .as_ref()
            .map(|c| c.value.clone())
            .unwrap_or_default();
        AmLayerSpec::SdfShape {
            fill_color: shape.fill_color.clone(),
            stroke_color_value,
            stroke_width,
            stroke_join: stroke.join.clone(),
            width,
            height,
            pivot_x,
            pivot_y,
            shape_type: shape.shape_type.clone(),
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

    let stroke_width_anim = get_stroke_width_animation(shape.stroke.as_ref());

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
            effect_pos_x,
            effect_pos_y,
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
            embed_offset: Vec2::ZERO,
            inv_fit_scale: 1.0,
            stroke_width: stroke_width_anim,
            base_alpha: get_base_alpha(&shape.fill_color),
            palette_alpha: palette_map.alpha.clone(),
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
    })
}

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
    let (effect_pos_x, effect_pos_y) = extract_effect_animations(&null.effects);
    let wipe_effect = extract_wipe_effect(&null.effects);
    let stretch_segment = extract_stretch_segment_effect(&null.effects);
    let gaussian_blur = extract_gaussian_blur_effect(&null.effects);

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
            effect_pos_x,
            effect_pos_y,
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
            blur_strength: gaussian_blur.strength,
            speed_multiplier: config.speed_multiplier,
            embed_offset: Vec2::ZERO,
            inv_fit_scale: 1.0,
            stroke_width: AmAnimatedFloat::default(),
            base_alpha: 1.0, // Null objects are fully opaque
            palette_alpha: AmAnimatedFloat::default(),
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
    })
}

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

    bevy::log::info!(
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
        ..config.clone()
    };

    let mut children = collect_pending_layers(&embed.scene, fonts, font_metrics, &nested_config);

    // Process mask relationships within this embed scene
    apply_mask_to_children(&mut children);

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
            effect_pos_x: AmAnimatedFloat::default(),
            effect_pos_y: AmAnimatedFloat::default(),
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
            embed_offset: Vec2::ZERO,
            inv_fit_scale: 1.0,
            stroke_width: AmAnimatedFloat::default(),
            base_alpha: get_base_alpha(&embed.fill_color),
            palette_alpha: AmAnimatedFloat::default(),
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

    // Calculate wrap offset for text positioning
    // AM text position is based on the CENTER of the wrapWidth box
    // We need to offset to get the LEFT edge for left-aligned text
    let wrap_width = text.wrap_width;
    let wrap_offset_x = if has_parent {
        0.0 // Child text uses relative positioning, no wrap offset
    } else {
        match text.align.as_str() {
            "left" => -wrap_width / 2.0, // Move left by half of wrapWidth
            "right" => wrap_width / 2.0, // Move right by half of wrapWidth
            _ => 0.0,                    // Center - no offset needed
        }
    };

    // Calculate Y offset based on font metrics
    const REFERENCE_WIN_ASCENT: f32 = 1.1285;
    let font_y_offset = if let Some(metrics) = font_metrics.get(&font_name) {
        let ascent_diff = REFERENCE_WIN_ASCENT - metrics.win_ascent;
        ascent_diff * font_size * 0.43
    } else {
        0.0
    };

    let y_offset_to_apply = if has_parent { 0.0 } else { font_y_offset };

    let transform = Transform {
        translation: Vec3::new(tx + wrap_offset_x, ty - y_offset_to_apply, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    // Create a modified location with wrap_offset applied (for animations)
    let mut modified_location = text.transform.location.clone();
    if let Some(ref mut val) = modified_location.value {
        val[0] += wrap_offset_x;
    }
    // Also modify keyframes if present
    for kf in &mut modified_location.keyframes {
        if let Ok(mut parsed) = crate::schema::parse_vec3(&kf.value) {
            parsed[0] += wrap_offset_x;
            kf.value = format!("{},{},{}", parsed[0], parsed[1], parsed[2]);
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
            location: modified_location, // Use modified location with wrap offset
            pivot: text.transform.pivot.clone(),
            rotation: text.transform.rotation.clone(),
            scale: text.transform.scale.clone(),
            opacity: text.transform.opacity.clone(),
            canvas_width: config.canvas_width,
            canvas_height: config.canvas_height,
            has_parent,
            effect_pos_x: AmAnimatedFloat::default(),
            effect_pos_y: AmAnimatedFloat::default(),
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
            blur_strength: AmAnimatedFloat::default(),
            speed_multiplier: config.speed_multiplier,
            embed_offset: Vec2::ZERO,
            inv_fit_scale: 1.0,
            stroke_width: AmAnimatedFloat::default(),
            base_alpha: get_base_alpha(&text.fill_color),
            palette_alpha: AmAnimatedFloat::default(),
        },
        spec: AmLayerSpec::Text {
            content: text.content.clone(),
            font_name,
            font_size,
            align: text.align.clone(),
            fill_color: text.fill_color.clone(),
        },
        z_index: z,
        children: Vec::new(),
        blending_mode: AmBlendingMode::Normal,
        mask_info: None,
        palette_params: None,
        embed_scene_size: None,
        containing_embed_id: 0,
        from_deeply_nested_scene: config.nesting_depth > 1,
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
    let stretch_segment = extract_stretch_segment_effect(&image.effects);
    let gaussian_blur = extract_gaussian_blur_effect(&image.effects);
    let palette_map = extract_palette_map_effect(&image.effects);

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
            effect_pos_x: AmAnimatedFloat::default(),
            effect_pos_y: AmAnimatedFloat::default(),
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
            blur_strength: gaussian_blur.strength,
            speed_multiplier: config.speed_multiplier,
            embed_offset: Vec2::ZERO,
            inv_fit_scale: 1.0,
            stroke_width: AmAnimatedFloat::default(),
            base_alpha: 1.0, // Image layers are fully opaque
            palette_alpha: palette_map.alpha.clone(),
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
    })
}

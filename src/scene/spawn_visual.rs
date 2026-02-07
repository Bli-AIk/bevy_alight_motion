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
    let (effect_pos_x, effect_pos_y) = extract_effect_animations(&image.effects);
    let wipe_effect = extract_wipe_effect(&image.effects);
    let stretch_segment = extract_stretch_segment_effect(&image.effects);
    let gaussian_blur = extract_gaussian_blur_effect(&image.effects);
    let scale_assist = extract_scale_assist_effect(&image.effects);
    let replace_color = extract_replace_color_effect(&image.effects);
    let (pivot_x, pivot_y) = get_initial_pivot(&image.transform.pivot);
    let palette_map = extract_palette_map_effect(&image.effects);

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
                effect_pos_x,
                effect_pos_y,
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
                scale_assist: scale_assist.scale,
                scale_assist_damp: scale_assist.damp,
                scale_assist_axis: scale_assist.axis,
                replace_old_color: replace_color.old_color,
                replace_new_color: replace_color.new_color,
                replace_threshold: replace_color.threshold,
                replace_feather: replace_color.feather,
                replace_alpha: replace_color.alpha,
                replace_lock_luminance: replace_color.lock_luminance,
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

    // AM text position is based on the CENTER of the wrapWidth box
    // We need to offset to get the LEFT edge for left-aligned text
    // 在AM中，文本位置是基于wrapWidth框的中心
    // 对于左对齐文本，我们需要偏移到左边缘
    // But for text with parent, don't apply wrap offset since position is relative
    // 但是对于有父对象的文本，不应用wrapWidth偏移，因为位置是相对的
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

    // Get font size (default to 16.0 if not specified)
    // AM font sizes appear to be in a different scale - use a larger multiplier
    // 文本大小乘数 - 调整这个值来修改字体大小
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

    // Calculate Y offset based on font metrics
    // 基于字体度量计算 Y 偏移
    //
    // AM 的文本定位似乎基于某个参考字体的 win_ascent 值
    // 当字体的 win_ascent 与参考值不同时，需要根据差值调整 Y 位置
    //
    // 通过实验确定：
    // - 8-bit Operator + Bold (win_ascent=1.1285) 显示位置正确
    // - Mars Needs Cunnilingus (win_ascent=0.7500) 需要向下偏移约 16.3px (font_size=48)
    // - 偏移量 = (REFERENCE_WIN_ASCENT - win_ascent) * font_size * factor
    //
    // 经计算: factor ≈ 0.897 使得两个字体都能正确显示
    // 但为了简化，使用 (1.1285 - win_ascent) / 2 * font_size 作为偏移
    const REFERENCE_WIN_ASCENT: f32 = 1.1285; // 8-bit Operator + Bold 作为参考
    let font_y_offset = if let Some(metrics) = font_metrics.get(&font_name) {
        // 当 win_ascent 小于参考值时，文本需要向下移动（负Y方向）
        // offset 为正值时减去它会使 Y 变小（向下）
        let ascent_diff = REFERENCE_WIN_ASCENT - metrics.win_ascent;
        let offset = ascent_diff * font_size * 0.43; // factor 经验值

        // 计算基础Y位置（未应用偏移）
        let base_y = ty;
        let final_y = base_y - offset;

        bevy::log::trace!(
            "  Font metrics for '{}': win_ascent={:.4}, win_descent={:.4}",
            font_name,
            metrics.win_ascent,
            metrics.win_descent
        );
        bevy::log::trace!(
            "  Y calculation: base_y={:.2}, ascent_diff={:.4}, offset={:.2}, final_y={:.2}",
            base_y,
            ascent_diff,
            offset,
            final_y
        );
        offset
    } else {
        bevy::log::trace!(
            "  No font metrics found for '{}', using offset=0",
            font_name
        );
        0.0
    };

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

    bevy::log::trace!(
        "Spawning text '{}' (id={}, parent={}): pos=({:.1},{:.1}), wrapWidth={:.1}, wrapOffset={:.1}, size={:.1}, font={}, y_offset={:.1}, content='{}'",
        text.label,
        text.id,
        text.parent,
        tx,
        ty,
        wrap_width,
        wrap_offset_x,
        font_size,
        font_name,
        font_y_offset,
        text.content
    );

    // Only apply font_y_offset to root text layers; child text inherits offset from parent
    let y_offset_to_apply = if has_parent { 0.0 } else { font_y_offset };

    let transform = Transform {
        translation: Vec3::new(tx + wrap_offset_x, ty - y_offset_to_apply, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    // Create a modified location with wrap_offset applied (no Y offset)
    // 创建一个带有wrapWidth偏移的location副本（无Y偏移）
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
            base_alpha: get_base_alpha(&text.fill_color, false),
            palette_alpha: AmAnimatedFloat::default(),
            scale_assist: AmAnimatedFloat::default(),
            scale_assist_damp: AmAnimatedFloat::default(),
            scale_assist_axis: 0,
            replace_old_color: Vec4::ZERO,
            replace_new_color: crate::schema::AmAnimatedColor::default(),
            replace_threshold: AmAnimatedFloat::default(),
            replace_feather: AmAnimatedFloat::default(),
            replace_alpha: AmAnimatedFloat::default(),
            replace_lock_luminance: false,
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

    // Text layers have visual components spawned immediately but use visibility for lifecycle
    entity.insert((
        Text2d::new(&text.content),
        text_font,
        TextColor(color),
        TextLayout::new_with_justify(justify),
        // Use left-center anchor for text - AM uses center Y as the reference point
        // With center anchor, the Y coordinate points to the vertical center of the text
        bevy::sprite::Anchor(Vec2::new(-0.5, 0.0)),
        AmLayerSpec::Text {
            content: text.content.clone(),
            font_name: font_name.clone(),
            font_size,
            align: text.align.clone(),
            fill_color: text.fill_color.clone(),
        },
        AmVisualSpawned, // Mark as already spawned
    ));

    entity.id()
}

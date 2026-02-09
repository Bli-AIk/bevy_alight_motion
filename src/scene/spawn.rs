//! # spawn.rs
//!
//! # 实体生成模块
//!
//! Entity spawning functions for AM scene layers.
//! AM 场景图层的实体生成函数。

use bevy::asset::Assets;
use bevy::prelude::*;
use std::collections::HashMap;

use crate::animation::AmAnimated;
use crate::effects::NeedsStrategyEvaluation;
use crate::loader::FontMetrics;
use crate::schema::{AmAnimatedFloat, AmAnimatedVec2, AmLayer, AmScene, AmShape};
use crate::sdf::AmSdfShaders;

use super::components::*;
use super::effects::*;
use super::helpers::*;
use super::spawn_visual::{spawn_image, spawn_text};

#[allow(clippy::too_many_arguments)]
pub fn spawn_scene(
    commands: &mut Commands,
    shaders: &mut Assets<Shader>,
    scene: &AmScene,
    images: &HashMap<String, Handle<Image>>,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    white_pixel: &Handle<Image>,
    sdf_shaders: &AmSdfShaders,
    parent: Entity,
    config: &AmSceneConfig,
) -> HashMap<u64, Entity> {
    let mut entity_map: HashMap<u64, Entity> = HashMap::new();
    let mut parent_relations: Vec<(Entity, u64)> = Vec::new();

    // In AM, layers at the END of the XML are rendered on top (higher z).
    // Last layer in XML = highest z (on top), first layer = lowest z (on bottom).
    let layer_count = scene.layers.len();

    bevy::log::trace!(
        "spawn_scene: layer_count={}, z_spacing={}, nesting_depth={}",
        layer_count,
        config.z_spacing,
        config.nesting_depth
    );

    // For nested scenes, start at a small offset above parent to avoid z-fighting
    // Root scene (depth 0) starts at z=0, nested scenes start at a small offset
    let z_base = if config.nesting_depth > 0 {
        config.z_spacing * 0.1 // Small offset to be above parent
    } else {
        0.0
    };

    // First pass: create all entities and collect parent relationships
    for (idx, layer) in scene.layers.iter().enumerate() {
        // Simple sequential z allocation
        let z = z_base + idx as f32 * config.z_spacing;

        // Log z value for each layer
        let layer_label = match layer {
            AmLayer::Shape(s) => &s.label,
            AmLayer::Nullobj(n) => &n.label,
            AmLayer::EmbedScene(e) => &e.label,
            AmLayer::Text(t) => &t.label,
            AmLayer::Audio(a) => &a.label,
            AmLayer::Bookmark(b) => &b.label,
            AmLayer::Image(i) => &i.label,
            AmLayer::Camera(c) => &c.label,
            AmLayer::Video(v) => &v.label,
        };
        bevy::log::trace!(
            "[Z-ORDER] depth={}, idx={}, z={:.4}, label='{}'",
            config.nesting_depth,
            idx,
            z,
            layer_label
        );

        match layer {
            AmLayer::Shape(shape) => {
                let entity = spawn_shape(
                    commands,
                    shaders,
                    shape,
                    images,
                    white_pixel,
                    sdf_shaders,
                    config,
                    z,
                );
                entity_map.insert(shape.id, entity);
                if shape.parent != 0 {
                    parent_relations.push((entity, shape.parent));
                } else {
                    commands.entity(parent).add_child(entity);
                }
            }
            AmLayer::Nullobj(null) => {
                let entity = spawn_null(commands, null, config, z);
                entity_map.insert(null.id, entity);
                if null.parent != 0 {
                    parent_relations.push((entity, null.parent));
                } else {
                    commands.entity(parent).add_child(entity);
                }
            }
            AmLayer::EmbedScene(embed) => {
                let entity = spawn_embed_scene(
                    commands,
                    shaders,
                    embed,
                    images,
                    fonts,
                    font_metrics,
                    white_pixel,
                    sdf_shaders,
                    config,
                    z,
                );
                entity_map.insert(embed.id, entity);
                if embed.parent != 0 {
                    parent_relations.push((entity, embed.parent));
                } else {
                    commands.entity(parent).add_child(entity);
                }
            }
            AmLayer::Bookmark(_) => {
                // Bookmarks are non-visual timeline markers, skip them
            }
            AmLayer::Text(text) => {
                let entity = spawn_text(commands, text, fonts, font_metrics, config, z);
                entity_map.insert(text.id, entity);
                if text.parent != 0 {
                    parent_relations.push((entity, text.parent));
                } else {
                    commands.entity(parent).add_child(entity);
                }
            }
            AmLayer::Audio(audio) => {
                // TODO: Audio playback is not yet implemented, skip for now
                bevy::log::trace!(
                    "Skipping audio layer '{}' (id={}) - audio not implemented",
                    audio.label,
                    audio.id
                );
            }
            AmLayer::Camera(camera) => {
                // TODO: Camera layer is not yet implemented, skip for now
                bevy::log::trace!(
                    "Skipping camera layer '{}' (id={}) - camera not implemented",
                    camera.label,
                    camera.id
                );
            }
            AmLayer::Image(image) => {
                let entity = spawn_image(commands, image, images, config, z);
                entity_map.insert(image.id, entity);
                if image.parent != 0 {
                    parent_relations.push((entity, image.parent));
                } else {
                    commands.entity(parent).add_child(entity);
                }
            }
            AmLayer::Video(video) => {
                // TODO: Video playback is not yet implemented, skip for now
                bevy::log::trace!(
                    "Skipping video layer '{}' (id={}) - video not implemented",
                    video.label,
                    video.id
                );
            }
        }
    }

    // Second pass: set up parent-child relationships
    for (child_entity, parent_id) in parent_relations {
        if let Some(&parent_entity) = entity_map.get(&parent_id) {
            commands.entity(parent_entity).add_child(child_entity);
        } else {
            // Parent not found in this scene, attach to scene root
            commands.entity(parent).add_child(child_entity);
        }
    }

    entity_map
}

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
    let (effect_pos_x, effect_pos_y) = extract_effect_animations(&shape.effects);
    let wipe_effect = extract_wipe_effect(&shape.effects);
    let stretch_segment = extract_stretch_segment_effect(&shape.effects);
    let gaussian_blur = extract_gaussian_blur_effect(&shape.effects);
    let scale_assist = extract_scale_assist_effect(&shape.effects);
    let repeat_effect = extract_repeat_effect(&shape.effects);
    let linear_repeat_effect = extract_linear_repeat_effect(&shape.effects);
    let swing_effect = extract_swing_effect(&shape.effects);
    let threshold_effect = extract_threshold_effect(&shape.effects);
    let grid_effect = extract_grid_effect(&shape.effects);
    let (pivot_x, pivot_y) = get_initial_pivot(&shape.transform.pivot);

    // Get size from properties
    let (width, height) = get_shape_size(&shape.properties, &shape.fill_type);

    // AM location points to object CENTER, not pivot. No position compensation needed.
    // Pivot only affects rotation/scale center, which is handled by Anchor.
    bevy::log::trace!(
        "Registering shape '{}' (id={}, parent={}): pos=({:.1},{:.1}), z={:.1}, scale=({:.2},{:.2}), size=({:.0},{:.0}), pivot=({:.1},{:.1}), fill={}, image={}",
        shape.label,
        shape.id,
        shape.parent,
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
    let needs_sdf = (shape.fill_type == "color" || shape.fill_type == "none")
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
                // Scale by ~20x to match <size> element behavior
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

        // Track whether this is a "no fill" shape (fillType="none")
        // This is different from having no fillColor value (defaults to white)
        let no_fill = shape.fill_type == "none";

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
            no_fill,
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

    let stroke_width_anim = get_stroke_width_animation(shape.stroke.as_ref());
    let no_fill = shape.fill_type == "none";
    let base_alpha = get_base_alpha(&shape.fill_color, no_fill);
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
                effect_pos_x,
                effect_pos_y,
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
                embed_offset: Vec2::ZERO,
                inv_fit_scale: 1.0,
                stroke_width: stroke_width_anim,
                base_alpha,
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
                // Swing effect
                swing_freq: swing_effect.freq,
                swing_a1: swing_effect.a1,
                swing_a2: swing_effect.a2,
                swing_phase: swing_effect.phase,
                swing_type: swing_effect.swing_type,
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

    entity
}

/// Spawn a null object.
pub(crate) fn spawn_null(
    commands: &mut Commands,
    null: &crate::schema::AmNullObj,
    config: &AmSceneConfig,
    z: f32,
) -> Entity {
    let has_parent = null.parent != 0;
    let (tx, ty) = get_initial_location(&null.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&null.transform.rotation);
    let (sx, sy) = get_initial_scale(&null.transform.scale);
    let (effect_pos_x, effect_pos_y) = extract_effect_animations(&null.effects);
    let wipe_effect = extract_wipe_effect(&null.effects);
    let stretch_segment = extract_stretch_segment_effect(&null.effects);
    let gaussian_blur = extract_gaussian_blur_effect(&null.effects);
    let scale_assist = extract_scale_assist_effect(&null.effects);
    let replace_color = extract_replace_color_effect(&null.effects);
    let repeat_effect = extract_repeat_effect(&null.effects);
    let linear_repeat_effect = extract_linear_repeat_effect(&null.effects);
    let swing_effect = extract_swing_effect(&null.effects);
    let threshold_effect = extract_threshold_effect(&null.effects);
    let grid_effect = extract_grid_effect(&null.effects);

    bevy::log::trace!(
        "Registering nullobj '{}' (id={}, parent={}): pos=({:.1},{:.1}), scale=({:.2},{:.2})",
        null.label,
        null.id,
        null.parent,
        tx,
        ty,
        sx,
        sy
    );

    let transform = Transform {
        translation: Vec3::new(tx, ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    // Create entity name for inspector identification
    let entity_name = format!("Null[{}]: {}", null.id, null.label);

    commands
        .spawn((
            Name::new(entity_name),
            AmLayerMarker {
                id: null.id,
                label: null.label.clone(),
            },
            AmAnimated {
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
                scale_assist: scale_assist.scale,
                scale_assist_damp: scale_assist.damp,
                scale_assist_axis: scale_assist.axis,
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
                // Swing effect
                swing_freq: swing_effect.freq,
                swing_a1: swing_effect.a1,
                swing_a2: swing_effect.a2,
                swing_phase: swing_effect.phase,
                swing_type: swing_effect.swing_type,
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
            },
            AmLayerSpec::Null,
            transform,
            GlobalTransform::default(),
            Visibility::Hidden, // Start hidden, lifecycle system will show when active
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id()
}

/// Spawn an embedded scene.
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_embed_scene(
    commands: &mut Commands,
    shaders: &mut Assets<Shader>,
    embed: &crate::schema::AmEmbedScene,
    images: &HashMap<String, Handle<Image>>,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    white_pixel: &Handle<Image>,
    sdf_shaders: &AmSdfShaders,
    config: &AmSceneConfig,
    z: f32,
) -> Entity {
    let has_parent = embed.parent != 0;
    let (mut tx, mut ty) = get_initial_location(&embed.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&embed.transform.rotation);
    let (sx, sy) = get_initial_scale(&embed.transform.scale);
    let pivot = get_initial_pivot(&embed.transform.pivot);

    // Apply pivot compensation for initial position
    let (comp_x, comp_y) = calculate_pivot_compensation(pivot, (sx, sy), rotation, has_parent);
    tx += comp_x;
    ty += comp_y;

    bevy::log::trace!(
        "Registering embedScene '{}' (id={}, parent={}): pos=({:.1},{:.1}), pivot=({:.1},{:.1}), scale=({:.2},{:.2}), start_time={}, time_offset={}",
        embed.label,
        embed.id,
        embed.parent,
        tx,
        ty,
        pivot.0,
        pivot.1,
        sx,
        sy,
        embed.start_time,
        config.time_offset
    );

    let transform = Transform {
        translation: Vec3::new(tx, ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    // Create entity name for inspector identification
    let entity_name = format!("Embed[{}]: {}", embed.id, embed.label);

    let entity = commands
        .spawn((
            Name::new(entity_name),
            AmLayerMarker {
                id: embed.id,
                label: embed.label.clone(),
            },
            AmAnimated {
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
                base_alpha: get_base_alpha(&embed.fill_color, false),
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
                repeat_count: AmAnimatedFloat::default(),
                repeat_offset: AmAnimatedVec2::default(),
                repeat_angle: AmAnimatedFloat::default(),
                repeat_scale: AmAnimatedFloat::default(),
                repeat_alpha: AmAnimatedFloat::default(),
                // Linear repeat effect
                linear_repeat_count: AmAnimatedFloat::default(),
                linear_repeat_position: AmAnimatedVec2::default(),
                linear_repeat_offset: AmAnimatedVec2::default(),
                linear_repeat_angle: AmAnimatedFloat::default(),
                linear_repeat_scale: AmAnimatedFloat::default(),
                linear_repeat_alpha: AmAnimatedFloat::default(),
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
                // Swing effect (defaults for embed scene)
                swing_freq: AmAnimatedFloat::default(),
                swing_a1: AmAnimatedFloat::default(),
                swing_a2: AmAnimatedFloat::default(),
                swing_phase: AmAnimatedFloat::default(),
                swing_type: 0,
                // Threshold effect (defaults for embed scene)
                threshold_value: AmAnimatedFloat::default(),
                threshold_feather: AmAnimatedFloat::default(),
                threshold_invert: false,
                threshold_blend_mode: 0,
                // Grid effect (defaults for embed scene)
                grid_position: AmAnimatedVec2::default(),
                grid_spacing: AmAnimatedFloat::default(),
                grid_width: AmAnimatedFloat::default(),
                grid_color: crate::schema::AmAnimatedColor::default(),
                grid_punchout: false,
                grid_smoothing: AmAnimatedFloat::default(),
                grid_screen_space: false,
            },
            AmLayerSpec::EmbedScene,
            // Mark for render strategy evaluation (Hybrid Pipeline)
            // The evaluate_render_strategy_system will determine if this embed
            // needs Direct (no RTT), Stencil, or Composite (full RTT) rendering.
            NeedsStrategyEvaluation {
                scene_width: embed.scene.width as f32,
                scene_height: embed.scene.height as f32,
                has_scale_animation: !embed.transform.scale.keyframes.is_empty(),
            },
            transform,
            GlobalTransform::default(),
            Visibility::Hidden, // Start hidden, lifecycle system will show when active
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Recursively spawn nested scene with accumulated time offset
    // The nested scene's layers use times relative to the embed's start_time
    //
    // Calculate the internal time offset for the embedded scene.
    // When the parent timeline reaches startTime, the embedded scene should be at inTime.
    //
    // The formula for local_time in the animation system is:
    //   local_time = (global_time - time_offset) * speed_multiplier
    //
    // When global_time = embed.start_time, we want local_time = inTime:
    //   inTime = (embed.start_time - time_offset) * speed
    //   time_offset = embed.start_time - inTime / speed
    //
    // Note: This handles the case where speed != 1.0, which affects internal time flow.
    //
    // Nested scenes use smaller z_spacing to keep all children within
    // the parent's z-range (between parent and next sibling)
    // Using /100 instead of /1000 for better numerical precision
    let in_time = embed.in_time.unwrap_or(0) as f32;
    let effective_speed = config.speed_multiplier * embed.speed;
    // embed.start_time is relative to PARENT's internal time, not global time.
    // When parent's internal time = embed.start_time, child should start.
    // Parent internal time = (global_time - parent_time_offset) * parent_speed
    // global_start = parent_time_offset + embed.start_time / parent_speed
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
    // Lifecycle offset also needs to account for parent speed
    let lifecycle_offset_with_in_time = global_start - in_time;
    let nested_z_spacing = config.z_spacing / 100.0;
    let nested_config = AmSceneConfig {
        canvas_width: embed.scene.width as f32,
        canvas_height: embed.scene.height as f32,
        time_offset: time_offset_with_in_time as i32,
        lifecycle_offset: lifecycle_offset_with_in_time as i32,
        z_spacing: nested_z_spacing,
        nesting_depth: config.nesting_depth + 1,
        speed_multiplier: effective_speed,
        ..config.clone()
    };

    spawn_scene(
        commands,
        shaders,
        &embed.scene,
        images,
        fonts,
        font_metrics,
        white_pixel,
        sdf_shaders,
        entity,
        &nested_config,
    );

    entity
}

/// Spawn an image layer (lazy - visual components spawned later by lifecycle system).
#[allow(dead_code)]
pub(crate) fn calculate_embed_position_compensation(
    pivot: (f32, f32),
    scale: (f32, f32),
    rotation_deg: f32,
    has_parent: bool,
) -> (f32, f32) {
    // Convert pivot to Bevy coordinates (X same, Y flipped if root)
    let pivot_x = pivot.0;
    let pivot_y = if has_parent { pivot.1 } else { -pivot.1 };

    // Object offset from rotation center is -pivot (in Bevy coords)
    // After scaling
    let scaled_offset_x = -pivot_x * scale.0;
    let scaled_offset_y = -pivot_y * scale.1;

    // After rotation (Bevy uses opposite rotation direction)
    let rotation_rad = (-rotation_deg).to_radians();
    let rotated_offset_x =
        scaled_offset_x * rotation_rad.cos() - scaled_offset_y * rotation_rad.sin();
    let rotated_offset_y =
        scaled_offset_x * rotation_rad.sin() + scaled_offset_y * rotation_rad.cos();

    // The compensation is: rotated_offset - original_offset
    // original_offset is -pivot, so: rotated_offset + pivot
    let comp_x = rotated_offset_x + pivot_x;
    let comp_y = rotated_offset_y + pivot_y;

    (comp_x, comp_y)
}

pub(crate) fn calculate_pivot_compensation(
    pivot: (f32, f32),
    scale: (f32, f32),
    _rotation_deg: f32, // Kept for API compatibility, but not used
    has_parent: bool,
) -> (f32, f32) {
    let pivot_x = pivot.0;
    let pivot_y = pivot.1;

    // Compensation formula: pivot * (1 - scale)
    // This moves the entity position to keep the visual center correct after scaling
    let comp_x = pivot_x * (1.0 - scale.0);
    let comp_y = if has_parent {
        pivot_y * (1.0 - scale.1) // Y already flipped in parent coordinate system
    } else {
        -pivot_y * (1.0 - scale.1) // Flip Y for Bevy (AM Y-down, Bevy Y-up)
    };

    (comp_x, comp_y)
}

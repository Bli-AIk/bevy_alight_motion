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
use crate::schema::{AmAnimatedFloat, AmAnimatedVec2, AmLayer, AmScene};
use crate::sdf::AmSdfShaders;

use super::components::*;
use super::effects::*;
use super::helpers::*;
use super::spawn_shape::spawn_shape;
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
    // Also track previous layer for path-repeat linking
    let mut prev_layer_info: Option<(Entity, u64, String)> = None; // (entity, layer_id, shape_type)
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
                // Link path-repeat to previous layer
                let path_repeat_effect = extract_path_repeat_effect(&shape.effects);
                eprintln!(
                    "[Spawn] shape {}: effects_count={}, has_path_repeat={}, prev_layer={:?}",
                    shape.id,
                    shape.effects.len(),
                    path_repeat_effect.has_effect(),
                    prev_layer_info.as_ref().map(|(e, id, _)| (*e, *id))
                );
                if path_repeat_effect.has_effect()
                    && let Some((prev_entity, prev_id, ref prev_shape)) = prev_layer_info
                {
                    bevy::log::warn!(
                        "[Spawn] Inserting AmPathRepeat on entity {:?}, source={:?}",
                        entity,
                        prev_entity
                    );
                    commands
                        .entity(entity)
                        .insert(crate::animation::AmPathRepeat {
                            source_entity: prev_entity,
                            copy_entities: Vec::new(),
                            source_shape_type: prev_shape.clone(),
                            source_layer_id: prev_id,
                            source_animated: Default::default(),
                        });
                }
                prev_layer_info = Some((entity, shape.id, shape.shape_type.clone()));
            }
            AmLayer::Nullobj(null) => {
                let entity = spawn_null(commands, null, config, z);
                entity_map.insert(null.id, entity);
                if null.parent != 0 {
                    parent_relations.push((entity, null.parent));
                } else {
                    commands.entity(parent).add_child(entity);
                }
                prev_layer_info = Some((entity, null.id, String::new()));
            }
            AmLayer::EmbedScene(embed) => {
                // Check for echokf effect on this embed
                let echokf = extract_echokf_effect(&embed.effects);

                let max_count = echokf.max_count();
                if echokf.enabled && max_count > 0 {
                    let seconds = echokf.static_seconds();
                    eprintln!(
                        "[ECHOKF] embed '{}' id={}: max_count={}, seconds={:.3}, mode={}, alpha_kf={}",
                        embed.label,
                        embed.id,
                        max_count,
                        seconds,
                        echokf.mode,
                        echokf.alpha.keyframes.len()
                    );

                    // Build base echo alpha config with parent timing
                    let base_echo_alpha = crate::animation::EchoAlphaConfig {
                        alpha_keyframes: echokf.alpha.clone(),
                        fraction: 0.0,
                        parent_start: embed.start_time,
                        parent_end: embed.end_time,
                        parent_time_offset: config.time_offset,
                        parent_speed: config.speed_multiplier,
                    };

                    if echokf.mode == 0 {
                        // Mode 0 (atop): render original first (bottom), echoes on top
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

                        for i in 0..max_count {
                            let echo_index = (max_count - 1 - i) as f32;
                            let fraction = echo_index / max_count as f32;
                            let time_shift_ms = (1.0 - fraction) * seconds * 1000.0;
                            let echo_z = z + (i as f32 + 1.0) * config.z_spacing * 0.001;

                            let mut echo_config = config.clone();
                            echo_config.echo_time_shift_ms += time_shift_ms;
                            echo_config.echo_alpha_config =
                                Some(crate::animation::EchoAlphaConfig {
                                    fraction,
                                    ..base_echo_alpha.clone()
                                });

                            let echo_entity = spawn_embed_scene(
                                commands,
                                shaders,
                                embed,
                                images,
                                fonts,
                                font_metrics,
                                white_pixel,
                                sdf_shaders,
                                &echo_config,
                                echo_z,
                            );
                            if embed.parent != 0 {
                                parent_relations.push((echo_entity, embed.parent));
                            } else {
                                commands.entity(parent).add_child(echo_entity);
                            }
                        }
                    } else {
                        // Mode 1 (behind): echoes first, then original on top
                        for i in 0..max_count {
                            let echo_index = i as f32;
                            let fraction = echo_index / max_count as f32;
                            let time_shift_ms = (1.0 - fraction) * seconds * 1000.0;
                            let echo_z = z - (max_count - i) as f32 * config.z_spacing * 0.001;

                            let mut echo_config = config.clone();
                            echo_config.echo_time_shift_ms += time_shift_ms;
                            echo_config.echo_alpha_config =
                                Some(crate::animation::EchoAlphaConfig {
                                    fraction,
                                    ..base_echo_alpha.clone()
                                });

                            let echo_entity = spawn_embed_scene(
                                commands,
                                shaders,
                                embed,
                                images,
                                fonts,
                                font_metrics,
                                white_pixel,
                                sdf_shaders,
                                &echo_config,
                                echo_z,
                            );
                            if embed.parent != 0 {
                                parent_relations.push((echo_entity, embed.parent));
                            } else {
                                commands.entity(parent).add_child(echo_entity);
                            }
                        }

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
                } else {
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
                prev_layer_info = Some((
                    entity_map
                        .get(&embed.id)
                        .copied()
                        .unwrap_or(Entity::PLACEHOLDER),
                    embed.id,
                    String::new(),
                ));
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
                prev_layer_info = Some((entity, text.id, String::new()));
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
                prev_layer_info = Some((entity, image.id, String::new()));
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
    let mut all_transform2 = extract_all_transform2_effects(&null.effects);
    let transform2 = if all_transform2.is_empty() {
        Transform2Params::default()
    } else {
        all_transform2.remove(0)
    };
    let extra_transform2 = all_transform2;
    let wipe_effect = extract_wipe_effect(&null.effects);
    let stretch_segment = extract_stretch_segment_effect(&null.effects);
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
    let spin_rpm = extract_spin_rpm(&null.effects);
    let threshold_effect = extract_threshold_effect(&null.effects);
    let grid_effect = extract_grid_effect(&null.effects);
    let pixelate_effect = extract_pixelate_effect(&null.effects);
    let solid_color_effect = extract_solid_color_effect(&null.effects);

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
                // Swing effect
                swing_freq: swing_effect.freq,
                swing_a1: swing_effect.a1,
                swing_a2: swing_effect.a2,
                swing_phase: swing_effect.phase,
                swing_type: swing_effect.swing_type,
                // Oscillate effect
                oscillate_direction: oscillate_effect.direction,
                oscillate_angle: oscillate_effect.angle,
                oscillate_freq: oscillate_effect.freq,
                oscillate_mag: oscillate_effect.mag,
                oscillate_wave_type: oscillate_effect.wave_type,
                oscillate_phase: oscillate_effect.phase,
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

    // Extract transform2 effects from embed
    let mut all_embed_transform2 = extract_all_transform2_effects(&embed.effects);
    bevy::log::info!(
        "[EMBED_T2] '{}' (id={}): {} effects parsed, {} transform2 extracted, primary posz kf={}",
        embed.label,
        embed.id,
        embed.effects.len(),
        all_embed_transform2.len(),
        all_embed_transform2
            .first()
            .map(|t| t.pos_z.keyframes.len())
            .unwrap_or(0)
    );
    let embed_transform2 = if all_embed_transform2.is_empty() {
        Transform2Params::default()
    } else {
        all_embed_transform2.remove(0)
    };
    let embed_extra_transform2 = all_embed_transform2;

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
                linear_repeat_random_order: false,
                linear_repeat_seed: AmAnimatedFloat::default(),
                linear_repeat2: None,
                // Radial repeat effect (defaults for embed scene)
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
                // Swing effect (defaults for embed scene)
                swing_freq: AmAnimatedFloat::default(),
                swing_a1: AmAnimatedFloat::default(),
                swing_a2: AmAnimatedFloat::default(),
                swing_phase: AmAnimatedFloat::default(),
                swing_type: 0,
                // Oscillate effect (defaults for embed scene)
                oscillate_direction: 0,
                oscillate_angle: AmAnimatedFloat::default(),
                oscillate_freq: AmAnimatedFloat::default(),
                oscillate_mag: AmAnimatedFloat::default(),
                oscillate_wave_type: 0,
                oscillate_phase: AmAnimatedFloat::default(),
                // Spin effect (defaults for embed scene)
                spin_rpm: AmAnimatedFloat::default(),
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
                // Pixelate effect (defaults for embed scene)
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

    // Insert group fill component if this embed has a fill type
    {
        use crate::effects::{AmGroupFill, GroupFillType};
        match embed.fill_type.as_str() {
            "none" => {
                commands.entity(entity).insert(AmGroupFill {
                    fill_type: GroupFillType::None,
                    fill_color: Vec4::ZERO,
                });
            }
            "color" => {
                let color = if let Some(ref fc) = embed.fill_color {
                    if let Ok(c) = crate::schema::parse_color(&fc.value) {
                        let srgb = bevy::color::Color::srgba(c[0], c[1], c[2], c[3]);
                        let linear = srgb.to_linear();
                        Vec4::new(linear.red, linear.green, linear.blue, linear.alpha)
                    } else {
                        Vec4::ONE
                    }
                } else {
                    Vec4::ONE
                };
                commands.entity(entity).insert(AmGroupFill {
                    fill_type: GroupFillType::Color,
                    fill_color: color,
                });
            }
            "gradient" => {
                if let Some(ref g) = embed.gradient {
                    let gradient_type = match g.gradient_type.as_str() {
                        "linear" => 1u8,
                        "radial" => 2u8,
                        "sweep" => 3u8,
                        _ => 1u8,
                    };
                    let start_color = if let Ok(c) = crate::schema::parse_color(&g.start_color) {
                        let srgb = bevy::color::Color::srgba(c[0], c[1], c[2], c[3]);
                        let linear = srgb.to_linear();
                        Vec4::new(linear.red, linear.green, linear.blue, linear.alpha)
                    } else {
                        Vec4::ZERO
                    };
                    let end_color = if let Ok(c) = crate::schema::parse_color(&g.end_color) {
                        let srgb = bevy::color::Color::srgba(c[0], c[1], c[2], c[3]);
                        let linear = srgb.to_linear();
                        Vec4::new(linear.red, linear.green, linear.blue, linear.alpha)
                    } else {
                        Vec4::ONE
                    };
                    let start_pt = g.start.unwrap_or([0.5, 0.0]);
                    let end_pt = g.end.unwrap_or([0.5, 1.0]);
                    commands.entity(entity).insert(AmGroupFill {
                        fill_type: GroupFillType::Gradient {
                            gradient_type,
                            start_color,
                            end_color,
                            points: Vec4::new(start_pt[0], start_pt[1], end_pt[0], end_pt[1]),
                        },
                        fill_color: Vec4::ONE,
                    });
                }
            }
            _ => {}
        }
    }

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

    // Parse retime mode (same as collect path)
    let retime_mode = crate::animation::RetimeMode::parse(&embed.scene.retime);
    let retime_info = if retime_mode != crate::animation::RetimeMode::Off {
        let container_duration = (embed.end_time - embed.start_time) as f32;
        let nested_total = embed.scene.total_time as f32;
        Some(crate::animation::AmRetimeInfo {
            mode: retime_mode,
            embed_global_start: global_start,
            container_duration_ms: container_duration,
            nested_total_time_ms: nested_total,
            embed_speed: effective_speed,
        })
    } else {
        config.retime.clone()
    };

    let nested_config = AmSceneConfig {
        canvas_width: embed.scene.width as f32,
        canvas_height: embed.scene.height as f32,
        time_offset: time_offset_with_in_time as i32,
        lifecycle_offset: lifecycle_offset_with_in_time as i32,
        z_spacing: nested_z_spacing,
        nesting_depth: config.nesting_depth + 1,
        speed_multiplier: effective_speed,
        scene_fps: embed.scene.fps as f32,
        scene_total_time: embed.scene.total_time as f32,
        retime: retime_info,
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

/// Extract gradient data from an AmGradient into uniform-ready values.
/// Returns (gradient_type, start_color, end_color, points).
pub(crate) fn extract_gradient_data(
    gradient: &Option<crate::schema::AmGradient>,
) -> (u8, bevy::math::Vec4, bevy::math::Vec4, bevy::math::Vec4) {
    use bevy::math::Vec4;
    if let Some(g) = gradient {
        let grad_type = match g.gradient_type.as_str() {
            "linear" => 1u8,
            "radial" => 2u8,
            "sweep" => 3u8,
            _ => 0u8,
        };
        if grad_type == 0 {
            return (0, Vec4::ZERO, Vec4::ZERO, Vec4::ZERO);
        }
        let start_color = crate::schema::parse_color(&g.start_color)
            .map(|c| {
                // Store in sRGB space for sRGB-space interpolation (matching AM's NanoVG)
                Vec4::new(c[0], c[1], c[2], c[3])
            })
            .unwrap_or(Vec4::ZERO);
        let end_color = crate::schema::parse_color(&g.end_color)
            .map(|c| Vec4::new(c[0], c[1], c[2], c[3]))
            .unwrap_or(Vec4::ZERO);
        let start_pt = g.start.unwrap_or([0.0, 0.0]);
        let end_pt = g.end.unwrap_or([1.0, 1.0]);
        let points = Vec4::new(start_pt[0], start_pt[1], end_pt[0], end_pt[1]);
        (grad_type, start_color, end_color, points)
    } else {
        (0, Vec4::ZERO, Vec4::ZERO, Vec4::ZERO)
    }
}

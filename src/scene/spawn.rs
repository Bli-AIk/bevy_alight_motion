//! # spawn.rs
//!
//! # 实体生成模块
//!
//! Entity spawning functions for AM scene layers.
//! AM 场景图层的实体生成函数。

use bevy::asset::Assets;
use bevy::prelude::*;
use std::collections::HashMap;

use crate::loader::FontMetrics;
use crate::schema::{AmLayer, AmScene};
use crate::sdf::AmSdfShaders;

use super::components::*;
use super::effects::*;
use super::spawn_embed::spawn_embed_scene;
use super::spawn_null::spawn_null;
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

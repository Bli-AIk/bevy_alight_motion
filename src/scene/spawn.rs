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
use crate::schema::{AmEmbedScene, AmLayer, AmScene};
use crate::sdf::AmSdfShaders;

use super::components::*;
use super::effects::*;
use super::spawn_embed::spawn_embed_scene;
use super::spawn_null::spawn_null;
use super::spawn_shape::spawn_shape;
use super::spawn_visual::{spawn_image, spawn_text};

/// Attaches an entity to its parent layer or the scene root.
fn attach_to_parent(
    commands: &mut Commands,
    entity: Entity,
    layer_parent: u64,
    parent_relations: &mut Vec<(Entity, u64)>,
    scene_parent: Entity,
) {
    if layer_parent != 0 {
        parent_relations.push((entity, layer_parent));
    } else {
        commands.entity(scene_parent).add_child(entity);
    }
}

/// Spawns N copies of an embedScene for the repeat effect.
/// Each copy gets cumulative transform offsets (position, rotation, scale, alpha)
/// and optional time shift via the echo_time_shift_ms mechanism.
///
/// 为重复效果生成 N 个 embedScene 副本。
/// 每个副本获得累积的变换偏移（位置、旋转、缩放、透明度）
/// 以及通过 echo_time_shift_ms 机制实现的可选时间偏移。
fn spawn_repeat_copies(
    commands: &mut Commands,
    shaders: &mut Assets<Shader>,
    embed: &AmEmbedScene,
    images: &HashMap<String, Handle<Image>>,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    white_pixel: &Handle<Image>,
    sdf_shaders: &AmSdfShaders,
    config: &AmSceneConfig,
    z: f32,
    entity_map: &mut HashMap<u64, Entity>,
    parent_relations: &mut Vec<(Entity, u64)>,
    scene_parent: Entity,
    repeat: &RepeatParams,
    count: usize,
) {
    let time_val = repeat.time.value.unwrap_or(0.0);
    let offset_val = repeat.offset.value.unwrap_or([0.0, 0.0]);
    let angle_val = repeat.angle.value.unwrap_or(0.0);
    let scale_val = repeat.scale.value.unwrap_or(1.0);
    let alpha_val = repeat.alpha.value.unwrap_or(1.0);

    // AM's time parameter is in FRAMES, convert to ms per-copy
    let frame_duration_ms = 1000.0 / config.scene_fps;

    // AM accumulates transforms per-copy
    let mut acc_offset = Vec2::ZERO;
    let mut acc_angle: f32 = 0.0;
    let mut acc_scale: f32 = 1.0;
    let mut acc_alpha: f32 = 1.0;
    let mut acc_time: f32 = 0.0;

    for i in 0..count {
        if acc_alpha <= 0.0 && i > 0 {
            break;
        }

        let mut copy_config = config.clone();

        // Time offset: shift animation state via echo_time_shift_ms
        let time_shift_ms = acc_time * frame_duration_ms;
        copy_config.echo_time_shift_ms += time_shift_ms;
        copy_config.repeat_alpha_factor *= acc_alpha;
        copy_config.repeat_offset = acc_offset;
        copy_config.repeat_rotation_deg = acc_angle;
        copy_config.repeat_scale_factor = acc_scale;

        // Z ordering: copy 0 at bottom, copy N-1 on top
        let copy_z = z + i as f32 * config.z_spacing * 0.001;

        let entity = spawn_embed_scene(
            commands,
            shaders,
            embed,
            images,
            fonts,
            font_metrics,
            white_pixel,
            sdf_shaders,
            &copy_config,
            copy_z,
        );

        // Only register copy 0 in entity_map (it's the "original")
        if i == 0 {
            entity_map.insert(embed.id, entity);
        }
        attach_to_parent(
            commands,
            entity,
            embed.parent,
            parent_relations,
            scene_parent,
        );

        // Accumulate transforms for next copy (AM-style)
        acc_offset += Vec2::new(offset_val[0], -offset_val[1]);
        acc_angle += angle_val;
        acc_scale *= scale_val;
        acc_alpha -= 1.0 - alpha_val;
        acc_time += time_val;
    }
}

/// Handles spawning of an embedded scene layer, including echo (echokf) and repeat effects.
fn handle_embed_layer(
    commands: &mut Commands,
    shaders: &mut Assets<Shader>,
    embed: &AmEmbedScene,
    images: &HashMap<String, Handle<Image>>,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    white_pixel: &Handle<Image>,
    sdf_shaders: &AmSdfShaders,
    config: &AmSceneConfig,
    z: f32,
    entity_map: &mut HashMap<u64, Entity>,
    parent_relations: &mut Vec<(Entity, u64)>,
    scene_parent: Entity,
) {
    let echokf = extract_echokf_effect(&embed.effects);
    let max_count = echokf.max_count();

    if !echokf.enabled || max_count == 0 {
        // No echo — check for repeat effect on this group
        let repeat = extract_repeat_effect(&embed.effects);
        let repeat_count = repeat.count.value.unwrap_or(0.0) as i32;

        if repeat_count > 1 {
            // Repeat effect on embedScene: spawn N copies with cumulative transforms
            spawn_repeat_copies(
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
                entity_map,
                parent_relations,
                scene_parent,
                &repeat,
                repeat_count as usize,
            );
            return;
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
        attach_to_parent(
            commands,
            entity,
            embed.parent,
            parent_relations,
            scene_parent,
        );
        return;
    }

    let seconds = echokf.static_seconds();

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
        attach_to_parent(
            commands,
            entity,
            embed.parent,
            parent_relations,
            scene_parent,
        );

        for i in 0..max_count {
            let echo_index = (max_count - 1 - i) as f32;
            let fraction = echo_index / max_count as f32;
            let time_shift_ms = (1.0 - fraction) * seconds * 1000.0;
            let echo_z = z + (i as f32 + 1.0) * config.z_spacing * 0.001;

            let mut echo_config = config.clone();
            echo_config.echo_time_shift_ms += time_shift_ms;
            echo_config.echo_alpha_config = Some(crate::animation::EchoAlphaConfig {
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
            attach_to_parent(
                commands,
                echo_entity,
                embed.parent,
                parent_relations,
                scene_parent,
            );
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
            echo_config.echo_alpha_config = Some(crate::animation::EchoAlphaConfig {
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
            attach_to_parent(
                commands,
                echo_entity,
                embed.parent,
                parent_relations,
                scene_parent,
            );
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
        attach_to_parent(
            commands,
            entity,
            embed.parent,
            parent_relations,
            scene_parent,
        );
    }
}

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
                handle_embed_layer(
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
                    &mut entity_map,
                    &mut parent_relations,
                    parent,
                );
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

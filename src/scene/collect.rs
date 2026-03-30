//! Acts as the main scene-to-pending-layer collection pass.
//! It walks the parsed `AmScene`, dispatches each authored layer to the matching
//! collector, flattens nested results, and produces the `PendingLayer` list that
//! later spawning systems consume.
//!
//! 从场景 schema 到待生成图层列表的主收集入口。它会遍历解析后的
//! `AmScene`，把每个作者侧图层分发给对应的收集器，再把嵌套结果拍平，最终产出
//! 后续生成系统会消费的 `PendingLayer` 列表。

mod embed;
mod flatten;

use bevy::prelude::*;
use std::collections::HashMap;

use crate::loader::FontMetrics;
use crate::schema::{AmLayer, AmScene};

use self::embed::collect_embed_layer;
use self::flatten::flatten_pending_layers;
use super::collect_camera::*;
use super::collect_image::*;
use super::collect_mask::{apply_mask_to_children, lift_masks_to_composite_embeds};
use super::collect_shape::*;
use super::collect_types::*;
use super::components::*;

pub fn collect_pending_layers(
    scene: &AmScene,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    config: &AmSceneConfig,
) -> Vec<PendingLayer> {
    let mut pending_layers = Vec::new();

    let layer_count = scene.layers.len();

    bevy::log::trace!(
        "collect_pending_layers: layer_count={}, z_spacing={}, nesting_depth={}",
        layer_count,
        config.z_spacing,
        config.nesting_depth
    );

    let z_base = if config.nesting_depth > 0 {
        config.z_spacing * 0.1
    } else {
        0.0
    };

    for (idx, layer) in scene.layers.iter().enumerate() {
        let z = z_base + idx as f32 * config.z_spacing;
        collect_layer(&mut pending_layers, layer, fonts, font_metrics, config, z);
    }

    let mut flattened = flatten_pending_layers(pending_layers, config.nesting_depth);
    apply_mask_to_children(&mut flattened);
    lift_masks_to_composite_embeds(&mut flattened);

    bevy::log::trace!(
        "Collected {} pending layers (after flatten)",
        flattened.len()
    );
    flattened
}

pub(crate) fn collect_layer(
    pending: &mut Vec<PendingLayer>,
    layer: &AmLayer,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    config: &AmSceneConfig,
    z: f32,
) {
    match layer {
        AmLayer::Shape(shape) => {
            if let Some(pl) = collect_shape(shape, config, z) {
                bevy::log::trace!(
                    "  Collected shape '{}' (id={}, time={}..{}ms)",
                    shape.label,
                    shape.id,
                    shape.start_time,
                    shape.end_time
                );
                pending.push(pl);
            }
        }
        AmLayer::Nullobj(null) => {
            if let Some(pl) = collect_null(null, config, z) {
                bevy::log::trace!(
                    "  Collected null '{}' (id={}, time={}..{}ms)",
                    null.label,
                    null.id,
                    null.start_time,
                    null.end_time
                );
                pending.push(pl);
            }
        }
        AmLayer::EmbedScene(embed) => {
            collect_embed_layer(pending, embed, fonts, font_metrics, config, z);
        }
        AmLayer::Text(text) => {
            if let Some(pl) = collect_text(text, fonts, font_metrics, config, z) {
                bevy::log::trace!(
                    "  Collected text '{}' (id={}, time={}..{}ms)",
                    text.label,
                    text.id,
                    text.start_time,
                    text.end_time
                );
                pending.push(pl);
            }
        }
        AmLayer::Image(image) => {
            if let Some(pl) = collect_image(image, config, z) {
                bevy::log::trace!(
                    "  Collected image '{}' (id={}, time={}..{}ms)",
                    image.label,
                    image.id,
                    image.start_time,
                    image.end_time
                );
                pending.push(pl);
            }
        }
        AmLayer::Camera(camera) => {
            if let Some(pl) = collect_camera(camera, config, z) {
                bevy::log::trace!(
                    "  Collected camera '{}' (id={}, time={}..{}ms)",
                    camera.label,
                    camera.id,
                    camera.start_time,
                    camera.end_time
                );
                pending.push(pl);
            }
        }
        AmLayer::Bookmark(_) | AmLayer::Audio(_) | AmLayer::Video(_) => {}
    }
}

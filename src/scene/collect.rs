//! # collect.rs
//!
//! # 图层收集模块
//!
//! Functions for collecting pending layers from AM scenes.
//! 从 AM 场景收集待处理图层的函数。

use bevy::prelude::*;
use std::collections::HashMap;
use std::{
    collections::HashSet,
    sync::{Mutex, OnceLock},
};

use crate::loader::FontMetrics;
use crate::schema::{AmEmbedScene, AmLayer, AmScene};

use super::collect_camera::*;
use super::collect_echo::*;
use super::collect_embed::*;
use super::collect_image::*;
use super::collect_mask::apply_mask_to_children;
use super::collect_shape::*;
use super::collect_types::*;
use super::components::*;

fn trace_flatten_once(key: impl Into<String>, message: impl FnOnce() -> String) {
    if std::env::var_os("AM_FLATTEN_TRACE").is_none() {
        return;
    }

    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let key = key.into();

    let should_log = {
        let mut guard = seen.lock().expect("flatten trace mutex poisoned");
        guard.insert(key)
    };

    if should_log {
        bevy::log::warn!("{}", message());
    }
}

/// Remap IDs and references for a single flattened child during the flatten pass.
/// Handles parent, containing_embed_id, animated.layer_id, and mask_info remapping.
fn remap_flattened_child(
    child: &mut PendingLayer,
    id_mappings: &[(u64, u64)],
    layer_id: u64,
    is_embed: bool,
    embed_bevy_pos: Vec3,
    child_embed_id: u64,
) {
    let original_parent = child.parent;

    // Remap the parent reference
    if original_parent == 0 {
        child.parent = layer_id;

        if is_embed {
            child.animated.embed_offset = Vec2::new(embed_bevy_pos.x, embed_bevy_pos.y);
            bevy::log::trace!(
                "[FlattenDebug] Setting embed_offset for '{}' (id={}): offset=({:.1},{:.1})",
                child.label,
                child.id,
                embed_bevy_pos.x,
                embed_bevy_pos.y
            );
        }
    } else {
        // Find the new ID for the parent in our mapping list
        // Search for the first mapping where old_id matches original_parent
        let new_parent_id = id_mappings
            .iter()
            .find(|(old, _new)| *old == original_parent)
            .map(|(_, new)| *new);

        if let Some(new_parent_id) = new_parent_id {
            child.parent = new_parent_id;
            child.animated.parent_layer_id = new_parent_id;
        } else {
            // Parent is external to this flatten batch - keep original
            // (will be remapped by outer flatten call if needed)
            bevy::log::trace!(
                "[Flatten] Parent {} not found in mapping for '{}', keeping as-is",
                original_parent,
                child.label
            );
        }
    }

    // **Hybrid Rendering Pipeline Fix**:
    // Remap containing_embed_id to the correct embed entity ID.
    //
    // Case 1: containing_embed_id == child_embed_id (direct child of current embed)
    //         -> Set to current embed's remapped ID (layer_id for parent's children)
    // Case 2: containing_embed_id is in id_mappings (grandchild referencing a nested embed)
    //         -> Use the remapped ID from id_mappings
    // Case 3: containing_embed_id points to current layer (this layer IS the embed)
    //         -> This shouldn't happen (is_embed check in should_decouple)
    if child.containing_embed_id != 0 {
        if original_parent == 0 && child.containing_embed_id == child_embed_id && is_embed {
            // Direct content of this embed - use the embed's own ID (layer_id)
            // Since this embed layer's ID hasn't been remapped yet (it's the current layer),
            // we need to use `layer_id` which will become part of parent's flattening
            child.containing_embed_id = layer_id;
            bevy::log::trace!(
                "[Flatten] Remapped containing_embed_id for '{}': {} -> {} (direct child of embed)",
                child.label,
                child_embed_id,
                layer_id
            );
        } else {
            // Find in mappings
            let new_embed_id = id_mappings
                .iter()
                .find(|(old, _new)| *old == child.containing_embed_id)
                .map(|(_, new)| *new);

            if let Some(new_embed_id) = new_embed_id {
                // Grandchild referencing a nested embed
                child.containing_embed_id = new_embed_id;
                bevy::log::trace!(
                    "[Flatten] Remapped containing_embed_id for '{}' via id_mappings",
                    child.label
                );
            }
        }
        // If neither case matches, keep the original ID (will be remapped in outer call)
    }

    // Also update the layer_id in animated component
    child.animated.layer_id = child.id;

    // **CRITICAL**: Remap mask_layer_id and mask_parent_layer_id in mask_info to new IDs
    // This is essential for nested masks to work correctly, since the
    // mask layer's ID and its parent's ID get remapped during flattening.
    if let Some(ref mut info) = child.mask_info {
        for mask in info.masks.iter_mut() {
            // Look up the new ID for this mask layer
            let new_mask_id = id_mappings
                .iter()
                .find(|(old, _new)| *old == mask.mask_layer_id)
                .map(|(_, new)| *new);

            if let Some(new_mask_id) = new_mask_id {
                bevy::log::debug!(
                    "[Flatten] Remapped mask_layer_id for '{}': {} -> {}",
                    child.label,
                    mask.mask_layer_id,
                    new_mask_id
                );
                mask.mask_layer_id = new_mask_id;
            }

            // Also remap the mask's parent layer ID
            let new_mask_parent = id_mappings
                .iter()
                .find(|(old, _new)| *old == mask.mask_parent_layer_id)
                .map(|(_, new)| *new);

            if let Some(new_parent_id) = new_mask_parent {
                mask.mask_parent_layer_id = new_parent_id;
            } else if mask.mask_parent_layer_id == 0 && is_embed {
                // Root masks inside the flattened child scene become direct children of the
                // current embed. Preserve that new scope so runtime mask math does not keep
                // treating them as top-level masks after flattening.
                mask.mask_parent_layer_id = layer_id;
            }
        }
    }
}

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

    // For nested scenes, start at a small offset above parent to avoid z-fighting
    let z_base = if config.nesting_depth > 0 {
        config.z_spacing * 0.1
    } else {
        0.0
    };

    // Simple sequential z allocation
    for (idx, layer) in scene.layers.iter().enumerate() {
        let z = z_base + idx as f32 * config.z_spacing;
        collect_layer(&mut pending_layers, layer, fonts, font_metrics, config, z);
    }

    // Flatten all nested children into a single list
    // Pass nesting_depth so that nested embeds know their absolute depth in the hierarchy
    let mut flattened = flatten_pending_layers(pending_layers, config.nesting_depth);

    // Apply mask relationships - mask layers affect layers below them
    apply_mask_to_children(&mut flattened);

    bevy::log::trace!(
        "Collected {} pending layers (after flatten)",
        flattened.len()
    );
    flattened
}

/// Flatten a tree of PendingLayers into a single list.
/// Children of embed scenes are extracted and their parent is set to the embed's ID.
/// For spatial decoupling, only DIRECT children of top-level embeds have their `containing_embed_id` set.
/// Nested embeds become normal Bevy children so transforms propagate correctly.
/// `nesting_depth` is the absolute depth in the scene hierarchy (0 = top-level, 1 = inside one embed, etc.)
pub(crate) fn flatten_pending_layers(
    layers: Vec<PendingLayer>,
    nesting_depth: u32,
) -> Vec<PendingLayer> {
    // Reset counter at the start of each project load for deterministic behavior
    // (not strictly necessary but helps with debugging)
    flatten_pending_layers_inner(layers, 0, 0, nesting_depth, 0)
}

/// Inner recursive function with containing_embed tracking.
/// `embed_depth`: local depth within this flatten call (0 = not inside any embed in this call)
/// `base_nesting_depth`: absolute scene nesting level when flatten was called (0 = top-level scene)
/// `instance_counter`: monotonically increasing counter for generating unique IDs per embed instance
///
/// Spatial decoupling logic:
/// - Only content inside top-level embeds (base_nesting_depth == 0 && embed_depth == 1) gets spatially decoupled
/// - Content inside nested embeds (base_nesting_depth > 0 OR embed_depth > 1) becomes Bevy children
#[expect(clippy::only_used_in_recursion)] // reason: embed_depth tracks nesting level for spatial decoupling decisions
pub(crate) fn flatten_pending_layers_inner(
    layers: Vec<PendingLayer>,
    current_embed_id: u64,
    embed_depth: u32,
    base_nesting_depth: u32,
    mut instance_counter: u64,
) -> Vec<PendingLayer> {
    let mut result = Vec::new();

    for layer in layers {
        let layer_id = layer.id;
        let children = layer.children.clone();
        let is_embed = matches!(layer.spec, AmLayerSpec::EmbedScene);

        // Get embed's Bevy position for child coordinate adjustment
        let embed_bevy_pos = layer.transform.translation;

        // Add the layer itself (with children cleared)
        let mut layer_without_children = layer;
        layer_without_children.children = Vec::new();

        // **Hybrid Rendering Pipeline**:
        // Set containing_embed_id for ALL non-embed content inside ANY embed.
        // This allows propagate_render_layers_system to assign RenderLayers based on
        // the parent embed's strategy (Direct -> Layer 0, Composite -> RTT layer).
        //
        // Conditions for spatial decoupling:
        // - embed_depth >= 1: we're inside at least one embed
        // - !is_embed: not an embed layer itself (embeds remain as Bevy children for transform)
        //
        // The key insight: all content inside embeds should be marked with their
        // immediate parent embed's ID, regardless of nesting depth.
        let should_decouple = embed_depth >= 1 && !is_embed;
        let assigned_embed_id = if should_decouple { current_embed_id } else { 0 };
        layer_without_children.containing_embed_id = assigned_embed_id;

        if should_decouple && assigned_embed_id != 0 {
            bevy::log::debug!(
                "[Flatten] Content '{}' (id={}) assigned to embed {} (depth={})",
                layer_without_children.label,
                layer_without_children.id,
                assigned_embed_id,
                embed_depth
            );
        }

        result.push(layer_without_children);

        // Recursively flatten children and update their parent reference
        if !children.is_empty() {
            // Increment instance counter for this embed instance to ensure unique IDs
            instance_counter += 1;
            let current_instance = instance_counter;

            // Determine the embed ID and depth for children:
            // - If this layer IS an embed, its children are at depth+1
            // - Otherwise, inherit the current context
            let (child_embed_id, child_depth) = if is_embed {
                (layer_id, embed_depth + 1)
            } else {
                (current_embed_id, embed_depth)
            };

            let flattened_children = flatten_pending_layers_inner(
                children,
                child_embed_id,
                child_depth,
                base_nesting_depth,
                instance_counter,
            );

            // Remap IDs to be unique per instance.
            // Since children may contain layers with the same original IDs (from different embed instances),
            // we use Vec<(old_id, new_id, index)> to track each layer's mapping individually.

            // Pass 1: Generate unique IDs for each child and build mapping
            let id_mappings: Vec<(u64, u64)> = flattened_children
                .iter()
                .map(|child| {
                    let old_id = child.id;
                    let new_id = generate_unique_id(
                        current_instance
                            .wrapping_mul(1_000_000)
                            .wrapping_add(child.id),
                    );
                    (old_id, new_id)
                })
                .collect();

            // Build z_map from OLD IDs to absolute z values (before remapping).
            // AM uses absolute z-ordering within a scene, but Bevy's Transform hierarchy
            // accumulates z from parent to child. We must make child z values relative
            // to their parent so that global z matches the intended absolute z-order.
            let z_map: std::collections::HashMap<u64, f32> = flattened_children
                .iter()
                .map(|c| (c.id, c.transform.translation.z))
                .collect();
            let embed_parent_ids: std::collections::HashSet<u64> = flattened_children
                .iter()
                .filter(|c| matches!(c.spec, AmLayerSpec::EmbedScene))
                .map(|c| c.id)
                .collect();
            // Pass 2: Apply the mapping with correct parent lookup
            for (idx, mut child) in flattened_children.into_iter().enumerate() {
                let original_parent = child.parent;
                let old_z = child.transform.translation.z;
                child.id = id_mappings[idx].1;
                remap_flattened_child(
                    &mut child,
                    &id_mappings,
                    layer_id,
                    is_embed,
                    embed_bevy_pos,
                    child_embed_id,
                );

                // Fix z-accumulation: make child z relative to parent's z.
                // Without this, a child at absolute z=0.001 parented to a layer at z=0.005
                // would get global z=0.006 (parent + child), breaking AM's absolute z-order.
                let inherit_parent_z =
                    original_parent != 0 && !embed_parent_ids.contains(&original_parent);
                let parent_z = match (inherit_parent_z, z_map.get(&original_parent).copied()) {
                    (true, Some(z)) => z,
                    _ => 0.0,
                };
                child.transform.translation.z -= parent_z;

                #[expect(clippy::excessive_nesting)]
                // reason: targeted flatten debug logging stays inside the nested traversal
                if child.label.starts_with("spr_s_boneloop_0.png Copy")
                    || child.label.starts_with("Rectangle 1 Copy")
                {
                    trace_flatten_once(format!("{}:{}", child.id, child.label), || {
                        format!(
                            "[FLATTEN] label='{}' old_parent={} new_parent={} old_z={:.4} parent_z={:.4} new_z={:.4} containing_embed={} is_embed_ctx={}",
                            child.label,
                            original_parent,
                            child.parent,
                            old_z,
                            parent_z,
                            child.transform.translation.z,
                            child.containing_embed_id,
                            is_embed,
                        )
                    });
                }

                result.push(child);
            }
        }
    }

    result
}

/// Recursively extend the lifecycle end_time of all children in a PendingLayer tree.
/// Only extends PendingLayer.end_time (for spawn/despawn lifecycle), NOT animated.end_time
/// (which is used for animation duration normalization in calc_layer_time).
/// The is_active() method in AmAnimated accounts for echo_time_shift_ms separately.
fn extend_children_lifecycle(pl: &mut PendingLayer, extension_ms: f32) {
    let ext = extension_ms as i32;
    for child in &mut pl.children {
        child.end_time += ext;
        // NOTE: Do NOT extend child.animated.end_time — that would change the animation
        // duration in calc_layer_time, making the animation play slower.
        // Instead, is_active() uses echo_time_shift_ms to extend its active window.
        extend_children_lifecycle(child, extension_ms);
    }
}

/// Collect N copies of an embedScene for the repeat effect.
/// Each copy gets cumulative transform offsets and time delay.
///
/// 为重复效果收集 N 个 embedScene 副本。
fn collect_repeat_copies(
    pending: &mut Vec<PendingLayer>,
    embed: &AmEmbedScene,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    config: &AmSceneConfig,
    z: f32,
    repeat: &super::effects::RepeatParams,
    count: usize,
) {
    let time_val = repeat.time.value.unwrap_or(0.0);
    let offset_val = repeat.offset.value.unwrap_or([0.0, 0.0]);
    let angle_val = repeat.angle.value.unwrap_or(0.0);
    let scale_val = repeat.scale.value.unwrap_or(1.0);
    let alpha_val = repeat.alpha.value.unwrap_or(1.0);

    // AM's time parameter is in FRAMES of the current rendering context.
    // When inside a nested embed, AM's retimeNestedScene multiplies fps by 16x+,
    // so frame duration is much shorter. Use render_fps (not scene_fps) to match AM.
    let frame_duration_ms = 1000.0 / config.render_fps;

    // For mask embeds with repeat: merge all copies' children into a single PendingLayer.
    // Each copy renders at a different time offset, producing the circle at different
    // positions. Merging ensures a single RTT captures all circles, which the shader
    // can sample once to get the complete mask.
    let is_mask_embed = embed.blending == "mask" || embed.blending == "exclude";

    // AM accumulates transforms per-copy: offset/angle/time linear, scale exponential, alpha linear decrease
    let mut acc_offset = Vec2::ZERO;
    let mut acc_angle: f32 = 0.0;
    let mut acc_scale: f32 = 1.0;
    let mut acc_alpha: f32 = 1.0;
    let mut acc_time: f32 = 0.0; // in frames

    let mut base_pl: Option<PendingLayer> = None;

    for i in 0..count {
        // Skip rendering if alpha has gone to zero or below (matches AM behavior)
        if acc_alpha <= 0.0 && i > 0 {
            break;
        }

        let mut copy_config = config.clone();

        // Time offset: shift animation state via echo_time_shift_ms.
        // AM uses roundToInt(accTime) for frame-based rounding, plus a sub-frame
        // correction round(frac*fps) (SceneElementRenderingKt.java:1304-1307).
        // For the sub-frame part, use scene_fps (not render_fps) because
        // nested embed retiming inflates render_fps to 480+, which makes
        // round(frac*render_fps) overwhelm the frame shift.
        let rounded_frames = acc_time.round();
        let frac = acc_time - acc_time.trunc();
        let sub_frame_ms = (frac * config.scene_fps).round();
        let time_shift_ms = rounded_frames * frame_duration_ms + sub_frame_ms;
        copy_config.echo_time_shift_ms += time_shift_ms;

        // Alpha
        copy_config.repeat_alpha_factor *= acc_alpha;

        // Position offset: AM coords (Y-down) → Bevy coords (Y-up)
        copy_config.repeat_offset = acc_offset;

        // Rotation: degrees
        copy_config.repeat_rotation_deg = acc_angle;

        // Scale: exponential (scale^i)
        copy_config.repeat_scale_factor = acc_scale;

        // Z ordering: copy 0 at bottom, copy N-1 on top
        let copy_z = z + i as f32 * config.z_spacing * 0.001;

        let mut pl = collect_embed_scene(embed, fonts, font_metrics, &copy_config, copy_z);

        // Extend children's lifecycles to account for animation time shift.
        if time_shift_ms > 0.0 {
            extend_children_lifecycle(&mut pl, time_shift_ms);
        }

        if is_mask_embed {
            // Merge copies into a single embed: all children render into one RTT.
            if i == 0 {
                base_pl = Some(pl);
            } else {
                remap_echo_pl_ids(&mut pl);
                apply_echo_copy_transform(&mut pl, acc_scale, acc_offset, acc_angle);
                // Extend base copy with this copy's children
                let base = base_pl.as_mut().expect("copy 0 must set base_pl");
                base.children.extend(pl.children);
            }
        } else {
            // Remap IDs for copies > 0 to avoid conflicts
            if i > 0 {
                remap_echo_pl_ids(&mut pl);
            }
            pending.push(pl);
        }

        // Accumulate transforms for next copy (AM-style: linear for offset/angle/time,
        // multiplicative for scale, linear decrease for alpha)
        acc_offset += Vec2::new(offset_val[0], -offset_val[1]);
        acc_angle += angle_val;
        acc_scale *= scale_val;
        acc_alpha -= 1.0 - alpha_val;
        acc_time += time_val;
    }

    if let Some(pl) = base_pl {
        pending.push(pl);
    }
}

/// Collect an embed scene layer, handling echokf and repeat effects if present.
fn collect_embed_layer(
    pending: &mut Vec<PendingLayer>,
    embed: &AmEmbedScene,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    config: &AmSceneConfig,
    z: f32,
) {
    let echokf = super::effects::extract_echokf_effect(&embed.effects);
    let max_count = echokf.max_count();

    if !echokf.enabled || max_count == 0 {
        // No echo — check for repeat effect on this group
        let repeat = super::effects::extract_repeat_effect(&embed.effects);
        let repeat_count = repeat.count.value.unwrap_or(0.0) as i32;

        if repeat_count > 1 {
            collect_repeat_copies(
                pending,
                embed,
                fonts,
                font_metrics,
                config,
                z,
                &repeat,
                repeat_count as usize,
            );
            return;
        }

        let pl = collect_embed_scene(embed, fonts, font_metrics, config, z);
        bevy::log::trace!(
            "  Collected embed '{}' (id={}, time={}..{}ms, inTime={:?}, outTime={:?}, children={})",
            embed.label,
            embed.id,
            embed.start_time,
            embed.end_time,
            embed.in_time,
            embed.out_time,
            pl.children.len()
        );
        pending.push(pl);
        return;
    }

    let seconds = echokf.static_seconds();
    let is_dynamic = echokf.is_dynamic() || !echokf.alpha.keyframes.is_empty();

    let base_echo_alpha = crate::animation::EchoAlphaConfig {
        alpha_keyframes: echokf.alpha.clone(),
        fraction: 0.0,
        parent_start: embed.start_time,
        parent_end: embed.end_time,
        parent_time_offset: config.time_offset,
        parent_speed: config.speed_multiplier,
    };

    // Build echo runtime template for dynamic echoes
    let echo_rt_template = if is_dynamic {
        Some(crate::animation::AmEchoRuntime {
            echo_index: 0,
            max_count,
            mode: echokf.mode,
            count_kf: echokf.count.clone(),
            seconds_kf: echokf.seconds.clone(),
            alpha_kf: echokf.alpha.clone(),
            embed_start: embed.start_time as f32,
            embed_end: embed.end_time as f32,
            embed_time_offset: config.time_offset,
            embed_speed: config.speed_multiplier,
        })
    } else {
        None
    };

    if echokf.mode == 0 {
        // Mode 0 (atop): original first, then echoes on top
        let pl = collect_embed_scene(embed, fonts, font_metrics, config, z);
        pending.push(pl);

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

            let mut echo_pl = collect_embed_scene(embed, fonts, font_metrics, &echo_config, echo_z);
            remap_echo_pl_ids(&mut echo_pl);
            // Attach echo runtime for dynamic updates
            if let Some(ref template) = echo_rt_template {
                echo_pl.echo_runtime = Some(crate::animation::AmEchoRuntime {
                    echo_index: echo_index as u32,
                    ..template.clone()
                });
            }
            pending.push(echo_pl);
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

            let mut echo_pl = collect_embed_scene(embed, fonts, font_metrics, &echo_config, echo_z);
            remap_echo_pl_ids(&mut echo_pl);
            if let Some(ref template) = echo_rt_template {
                echo_pl.echo_runtime = Some(crate::animation::AmEchoRuntime {
                    echo_index: echo_index as u32,
                    ..template.clone()
                });
            }
            pending.push(echo_pl);
        }

        let pl = collect_embed_scene(embed, fonts, font_metrics, config, z);
        pending.push(pl);
    }
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
        // Ignore unsupported layer types
        AmLayer::Bookmark(_) | AmLayer::Audio(_) | AmLayer::Video(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::remap_flattened_child;
    use crate::animation::AmAnimated;
    use crate::scene::{AmBlendingMode, AmLayerSpec, AmMaskEntry, AmMaskInfo, PendingLayer};
    use bevy::prelude::*;

    fn make_pending_layer(
        id: u64,
        parent: u64,
        containing_embed_id: u64,
        label: &str,
    ) -> PendingLayer {
        PendingLayer {
            id,
            label: label.to_string(),
            parent,
            start_time: 0,
            end_time: 0,
            transform: Transform::default(),
            animated: AmAnimated::default(),
            spec: AmLayerSpec::Null,
            z_index: 0.0,
            children: Vec::new(),
            blending_mode: AmBlendingMode::Normal,
            mask_info: None,
            palette_params: None,
            embed_scene_size: None,
            containing_embed_id,
            from_deeply_nested_scene: false,
            echo_runtime: None,
            group_fill: None,
            embed_requires_composite: false,
            embed_dynamic_resolution: false,
            embed_inner_total_time: None,
            hidden: false,
        }
    }

    #[test]
    fn remap_preserves_nested_embed_owner_for_nested_children() {
        let current_embed_layer_id = 102_373_407;
        let nested_embed_old_id = 12_372_971;
        let nested_embed_new_id = 1_000_000_000_021;
        let mut child = make_pending_layer(
            12_368_973,
            nested_embed_old_id,
            nested_embed_old_id,
            "spr_s_boneloop_0.png Copy 5",
        );

        remap_flattened_child(
            &mut child,
            &[(nested_embed_old_id, nested_embed_new_id)],
            current_embed_layer_id,
            true,
            Vec3::ZERO,
            nested_embed_old_id,
        );

        assert_eq!(child.parent, nested_embed_new_id);
        assert_eq!(child.animated.parent_layer_id, nested_embed_new_id);
        assert_eq!(child.containing_embed_id, nested_embed_new_id);
    }

    #[test]
    fn remap_assigns_current_embed_owner_for_direct_children() {
        let current_embed_layer_id = 102_373_407;
        let mut child = make_pending_layer(
            12_372_970,
            0,
            current_embed_layer_id,
            "spr_s_boneloop_0.png Copy",
        );

        remap_flattened_child(
            &mut child,
            &[],
            current_embed_layer_id,
            true,
            Vec3::ZERO,
            current_embed_layer_id,
        );

        assert_eq!(child.parent, current_embed_layer_id);
        assert_eq!(child.containing_embed_id, current_embed_layer_id);
    }

    #[test]
    fn remap_promotes_flattened_root_mask_parent_to_current_embed() {
        let current_embed_layer_id = 102_373_489;
        let original_mask_layer_id = 12_373_002;
        let remapped_mask_layer_id = 1_000_000_000_062;
        let mut child = make_pending_layer(
            12_373_493,
            0,
            current_embed_layer_id,
            "spr_s_boneloop_0.png Copy",
        );
        child.mask_info = Some(AmMaskInfo {
            masks: vec![AmMaskEntry {
                mask_layer_id: original_mask_layer_id,
                mask_parent_layer_id: 0,
                ..Default::default()
            }],
        });

        remap_flattened_child(
            &mut child,
            &[(original_mask_layer_id, remapped_mask_layer_id)],
            current_embed_layer_id,
            true,
            Vec3::ZERO,
            current_embed_layer_id,
        );

        let masks = &child.mask_info.as_ref().expect("mask info").masks;
        assert_eq!(masks.len(), 1);
        assert_eq!(masks[0].mask_layer_id, remapped_mask_layer_id);
        assert_eq!(masks[0].mask_parent_layer_id, current_embed_layer_id);
    }
}

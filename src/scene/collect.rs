//! # collect.rs
//!
//! # 图层收集模块
//!
//! Functions for collecting pending layers from AM scenes.
//! 从 AM 场景收集待处理图层的函数。

use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::loader::FontMetrics;
use crate::schema::{AmEmbedScene, AmLayer, AmScene};

use super::collect_camera::*;
use super::collect_embed::*;
use super::collect_image::*;
use super::collect_shape::*;
use super::collect_types::*;
use super::components::*;
use super::helpers::*;

/// Global counter for generating unique IDs during flatten
static UNIQUE_ID_COUNTER: AtomicU64 = AtomicU64::new(1_000_000_000_000);

/// Generate a unique ID that won't collide with original IDs
/// Simply uses a monotonically increasing counter to guarantee uniqueness
fn generate_unique_id(_base_id: u64) -> u64 {
    // Just use the counter directly - this guarantees uniqueness
    UNIQUE_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Remap all IDs in an echo PendingLayer tree to unique IDs.
/// This prevents echo copies from colliding with the original entity IDs.
/// The root's `parent` is NOT remapped (it references an entity in the outer scope).
fn remap_echo_pl_ids(pl: &mut PendingLayer) {
    // Build old→new ID mapping for the entire tree
    let mut id_map = HashMap::new();
    collect_ids_for_remap(pl, &mut id_map);

    // Apply mapping
    apply_id_remap(pl, &id_map, true);
}

fn collect_ids_for_remap(pl: &PendingLayer, id_map: &mut HashMap<u64, u64>) {
    id_map
        .entry(pl.id)
        .or_insert_with(|| generate_unique_id(pl.id));
    for child in &pl.children {
        collect_ids_for_remap(child, id_map);
    }
}

fn apply_id_remap(pl: &mut PendingLayer, id_map: &HashMap<u64, u64>, is_root: bool) {
    if let Some(&new_id) = id_map.get(&pl.id) {
        pl.id = new_id;
        pl.animated.layer_id = new_id;
    }
    // Don't remap root's parent (it's in the outer scope)
    if !is_root {
        if let Some(&new_parent) = id_map.get(&pl.parent) {
            pl.parent = new_parent;
        }
        if let Some(&new_parent) = id_map.get(&pl.animated.parent_layer_id) {
            pl.animated.parent_layer_id = new_parent;
        }
    }
    if let Some(&new_embed) = id_map.get(&pl.containing_embed_id) {
        pl.containing_embed_id = new_embed;
    }
    // Remap mask references
    if let Some(ref mut mask_info) = pl.mask_info {
        for entry in &mut mask_info.masks {
            if let Some(&new_mask) = id_map.get(&entry.mask_layer_id) {
                entry.mask_layer_id = new_mask;
            }
        }
    }
    for child in &mut pl.children {
        apply_id_remap(child, id_map, false);
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
        if child.containing_embed_id == child_embed_id && is_embed {
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

    // **CRITICAL**: Remap mask_layer_id in mask_info to new IDs
    // This is essential for nested masks to work correctly, since the
    // mask layer's ID gets remapped during flattening.
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

            // Pass 2: Apply the mapping with correct parent lookup
            for (idx, mut child) in flattened_children.into_iter().enumerate() {
                child.id = id_mappings[idx].1;
                remap_flattened_child(
                    &mut child,
                    &id_mappings,
                    layer_id,
                    is_embed,
                    embed_bevy_pos,
                    child_embed_id,
                );
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

    // AM accumulates transforms per-copy: offset/angle/time linear, scale exponential, alpha linear decrease
    let mut acc_offset = Vec2::ZERO;
    let mut acc_angle: f32 = 0.0;
    let mut acc_scale: f32 = 1.0;
    let mut acc_alpha: f32 = 1.0;
    let mut acc_time: f32 = 0.0; // in frames

    for i in 0..count {
        // Skip rendering if alpha has gone to zero or below (matches AM behavior)
        if acc_alpha <= 0.0 && i > 0 {
            break;
        }

        let mut copy_config = config.clone();

        // Time offset: shift animation state via echo_time_shift_ms
        // AM shifts the render frame by accTime frames, converting to ms.
        let time_shift_ms = acc_time * frame_duration_ms;
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

        // Remap IDs for copies > 0 to avoid conflicts
        if i > 0 {
            remap_echo_pl_ids(&mut pl);
        }

        pending.push(pl);

        // Accumulate transforms for next copy (AM-style: linear for offset/angle/time,
        // multiplicative for scale, linear decrease for alpha)
        acc_offset += Vec2::new(offset_val[0], -offset_val[1]);
        acc_angle += angle_val;
        acc_scale *= scale_val;
        acc_alpha -= 1.0 - alpha_val;
        acc_time += time_val;
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
pub(crate) fn apply_mask_to_children(layers: &mut [PendingLayer]) {
    // Find all mask layers and their info
    // Masks are layers with blending_mode=Mask or Exclude (regardless of parent)
    let mut mask_layers: Vec<(u64, u64, f32, AmMaskEntry)> = Vec::new(); // (mask_id, mask_parent_id, z_index, mask_entry)

    // Build lookup table for parent transform info (needed to transform child mask coordinates to global)
    let parent_transform_info: std::collections::HashMap<
        u64,
        (bevy::math::Vec2, bevy::math::Vec2),
    > = layers
        .iter()
        .map(|l| {
            let scale = get_scale_at_normalized_time(&l.animated.scale, 0.0);
            (
                l.id,
                (
                    bevy::math::Vec2::new(l.transform.translation.x, l.transform.translation.y),
                    bevy::math::Vec2::new(scale.0, scale.1),
                ),
            )
        })
        .collect();

    for layer in layers.iter() {
        let is_mask = layer.blending_mode == AmBlendingMode::Mask
            || layer.blending_mode == AmBlendingMode::Exclude;
        if !is_mask {
            continue;
        }

        // Extract mask geometry from the layer's transform and spec
        let mask_entry = extract_mask_info_from_layer(layer);
        let Some(mut entry) = mask_entry else {
            continue;
        };

        // Set is_exclude based on blending mode
        entry.is_exclude = layer.blending_mode == AmBlendingMode::Exclude;

        // For child masks, transform coordinates to global space using parent's transform
        if layer.parent != 0 {
            if let Some(&(parent_translation, parent_scale)) =
                parent_transform_info.get(&layer.parent)
            {
                // Transform child mask center to global coordinates
                let global_center = parent_translation
                    + bevy::math::Vec2::new(
                        entry.center.x * parent_scale.x,
                        entry.center.y * parent_scale.y,
                    );
                // Transform half_size by parent scale
                let global_half_size = bevy::math::Vec2::new(
                    entry.half_size.x * parent_scale.x,
                    entry.half_size.y * parent_scale.y,
                );

                bevy::log::trace!(
                    "[MASK] Found child {} layer '{}' (id={}, parent={}) at z={:.4}, global_center=({:.1},{:.1}), global_half_size=({:.1},{:.1}), time={}..{}ms",
                    if entry.is_exclude { "exclude" } else { "mask" },
                    layer.label,
                    layer.id,
                    layer.parent,
                    layer.z_index,
                    global_center.x,
                    global_center.y,
                    global_half_size.x,
                    global_half_size.y,
                    entry.start_time,
                    entry.end_time
                );

                entry.center = global_center;
                entry.half_size = global_half_size;
            }
        } else {
            bevy::log::trace!(
                "[MASK] Found root {} layer '{}' (id={}) at z={:.4}, center=({:.1},{:.1}), half_size=({:.1},{:.1}), time={}..{}ms",
                if entry.is_exclude { "exclude" } else { "mask" },
                layer.label,
                layer.id,
                layer.z_index,
                entry.center.x,
                entry.center.y,
                entry.half_size.x,
                entry.half_size.y,
                entry.start_time,
                entry.end_time
            );
        }

        mask_layers.push((layer.id, layer.parent, layer.z_index, entry));
    }

    if mask_layers.is_empty() {
        return;
    }

    // Build parent_id lookup for ancestor checking
    let parent_map: std::collections::HashMap<u64, u64> =
        layers.iter().map(|l| (l.id, l.parent)).collect();

    // Helper to check if `layer_id` is an ancestor of `mask_id`
    // An ancestor is in the parent chain: mask -> parent -> grandparent -> ...
    fn is_ancestor_of_mask(
        layer_id: u64,
        mask_parent_id: u64,
        parent_map: &std::collections::HashMap<u64, u64>,
    ) -> bool {
        if mask_parent_id == 0 {
            return false; // Root mask has no ancestors
        }
        if layer_id == mask_parent_id {
            return true; // Direct parent
        }
        // Check grandparents
        if let Some(&grandparent_id) = parent_map.get(&mask_parent_id)
            && grandparent_id != 0
        {
            return is_ancestor_of_mask(layer_id, grandparent_id, parent_map);
        }
        false
    }

    // For each non-mask root layer, collect ALL masks that are above it (higher z-index)
    // This allows the runtime system to choose the correct mask based on current time
    for layer in layers.iter_mut() {
        if layer.blending_mode == AmBlendingMode::Mask
            || layer.blending_mode == AmBlendingMode::Exclude
        {
            continue; // Don't apply mask to mask layer itself
        }

        if layer.parent != 0 {
            continue; // Only consider root-level layers for initial mask assignment
        }

        // Collect all masks that are above this layer (higher z-index)
        // Skip masks where this layer is an ancestor of the mask
        let mut applicable_masks: Vec<AmMaskEntry> = Vec::new();
        for (mask_id, mask_parent_id, mask_z, mask_entry) in &mask_layers {
            if *mask_z > layer.z_index
                && *mask_id != layer.id
                && !is_ancestor_of_mask(layer.id, *mask_parent_id, &parent_map)
            {
                applicable_masks.push(mask_entry.clone());
            }
        }

        if !applicable_masks.is_empty() {
            bevy::log::trace!(
                "[MASK] Root layer '{}' (id={}, z={:.4}) will be clipped by {} mask(s)",
                layer.label,
                layer.id,
                layer.z_index,
                applicable_masks.len()
            );

            // Create or update mask_info with all applicable masks
            if layer.mask_info.is_none() {
                layer.mask_info = Some(AmMaskInfo {
                    masks: applicable_masks,
                });
            } else if let Some(ref mut info) = layer.mask_info {
                info.masks.extend(applicable_masks);
            }
        }
    }

    // NOTE: We propagate masks to ALL children for shader-based clipping.
    // According to user feedback, masks should treat the entire group as a whole,
    // clipping pixels at the shader level rather than hiding entire elements.
    // This means all child elements will have mask info for the shader to use,
    // but we do NOT use visibility-based hiding (apply_mask_clipping_system is disabled
    // for child layers).

    // Build map of layer_id -> masks
    let mut layer_masks: std::collections::HashMap<u64, Vec<AmMaskEntry>> =
        std::collections::HashMap::new();
    for layer in layers.iter() {
        if let Some(ref info) = layer.mask_info {
            layer_masks.insert(layer.id, info.masks.clone());
        }
    }

    // Propagate to children
    loop {
        let mut changes = false;
        for layer in layers.iter_mut() {
            if layer.blending_mode == AmBlendingMode::Mask
                || layer.blending_mode == AmBlendingMode::Exclude
            {
                continue;
            }
            if layer.mask_info.is_some() {
                continue; // Already has masks
            }
            // Check if this layer's parent or containing embed has masks
            let source_masks = if layer.parent != 0 {
                layer_masks.get(&layer.parent).cloned()
            } else if layer.containing_embed_id != 0 {
                layer_masks.get(&layer.containing_embed_id).cloned()
            } else {
                None
            };
            if let Some(masks) = source_masks {
                layer.mask_info = Some(AmMaskInfo {
                    masks: masks.clone(),
                });
                bevy::log::debug!(
                    "[MASK] Propagated {} mask(s) to child layer '{}' (id={})",
                    masks.len(),
                    layer.label,
                    layer.id
                );
                changes = true;
            }
        }
        if !changes {
            break;
        }
        // Update the map for next iteration
        for layer in layers.iter() {
            if let Some(ref info) = layer.mask_info {
                layer_masks.insert(layer.id, info.masks.clone());
            }
        }
    }
}

/// Extract mask geometry info from a layer's transform and spec.
/// For animated scales (like SDF shapes), we need to get the scale at t=0 from the animation data.
pub(crate) fn extract_mask_info_from_layer(layer: &PendingLayer) -> Option<AmMaskEntry> {
    let (width, height, pivot_x, pivot_y, is_circle, stroke_extension) = match &layer.spec {
        AmLayerSpec::SdfShape {
            width,
            height,
            pivot_x,
            pivot_y,
            shape_type,
            stroke_width,
            stroke_direction,
            ..
        } => {
            let is_circle = shape_type == ".circle";
            // Mask visible area includes stroke that extends beyond fill
            let ext = match stroke_direction.as_str() {
                "inside" => 0.0,
                "outside" => *stroke_width,
                _ => *stroke_width * 0.5, // "centered"
            };
            (*width, *height, *pivot_x, *pivot_y, is_circle, ext)
        }
        AmLayerSpec::SpriteShape { width, height, .. } => (*width, *height, 0.0, 0.0, false, 0.0),
        _ => return None,
    };

    // Get scale from animation data at t=0, since transform.scale might be (1,1) for SDF shapes
    let (scale_x, scale_y) = get_scale_at_normalized_time(&layer.animated.scale, 0.0);

    // For SDF shapes, transform.translation is the pivot point position.
    // The shape is scaled around this pivot point.
    // The geometric center of the scaled shape relative to the pivot is:
    // Center = Pivot - Scale * PivotOffset
    // Where PivotOffset (in Bevy coords) is (pivot_x, -pivot_y).
    // So:
    // Center.x = Pivot.x - scale_x * pivot_x
    // Center.y = Pivot.y - scale_y * (-pivot_y) = Pivot.y + scale_y * pivot_y
    let center_x = layer.transform.translation.x - pivot_x * scale_x;
    let center_y = layer.transform.translation.y + pivot_y * scale_y;

    bevy::log::debug!(
        "[MASK] Extracting mask info: width={}, height={}, pivot=({:.1},{:.1}), scale=({:.3},{:.3}), translation=({:.1},{:.1}), center=({:.1},{:.1}), half_size=({:.1},{:.1}), stroke_ext={:.1}, is_circle={}, time={}..{}, lifecycle_offset={}",
        width,
        height,
        pivot_x,
        pivot_y,
        scale_x,
        scale_y,
        layer.transform.translation.x,
        layer.transform.translation.y,
        center_x,
        center_y,
        width / 2.0 * scale_x + stroke_extension,
        height / 2.0 * scale_y + stroke_extension,
        stroke_extension,
        is_circle,
        layer.start_time,
        layer.end_time,
        layer.animated.lifecycle_offset
    );

    // Convert local time to global time using lifecycle_offset
    // For nested embeds, lifecycle_offset accounts for parent time offset
    // Global time = local_time + lifecycle_offset
    let global_start = layer.start_time + layer.animated.lifecycle_offset;
    let global_end = layer.end_time + layer.animated.lifecycle_offset;

    bevy::log::debug!(
        "[MASK] Converted to global time: {}..{}ms (local {}..{}, offset={})",
        global_start,
        global_end,
        layer.start_time,
        layer.end_time,
        layer.animated.lifecycle_offset
    );

    Some(AmMaskEntry {
        center: Vec2::new(center_x, center_y),
        half_size: Vec2::new(
            width / 2.0 * scale_x + stroke_extension,
            height / 2.0 * scale_y + stroke_extension,
        ),
        rotation: layer
            .transform
            .rotation
            .to_euler(bevy::math::EulerRot::ZYX)
            .0,
        scale: Vec2::new(scale_x, scale_y),
        is_circle,
        start_time: global_start,
        end_time: global_end,
        mask_layer_id: layer.id,
        is_exclude: layer.blending_mode == AmBlendingMode::Exclude,
    })
}

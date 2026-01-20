//! # collect.rs
//!
//! # 图层收集模块
//!
//! Functions for collecting pending layers from AM scenes.
//! 从 AM 场景收集待处理图层的函数。

use bevy::prelude::*;
use std::collections::HashMap;

use crate::animation::AmAnimated;
use crate::loader::FontMetrics;
use crate::schema::{AmLayer, AmScene};

use super::collect_types::*;
use super::components::*;
use super::effects::*;
use super::helpers::*;

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
    flatten_pending_layers_inner(layers, 0, 0, nesting_depth)
}

/// Inner recursive function with containing_embed tracking.
/// `embed_depth`: local depth within this flatten call (0 = not inside any embed in this call)
/// `base_nesting_depth`: absolute scene nesting level when flatten was called (0 = top-level scene)
///
/// Spatial decoupling logic:
/// - Only content inside top-level embeds (base_nesting_depth == 0 && embed_depth == 1) gets spatially decoupled
/// - Content inside nested embeds (base_nesting_depth > 0 OR embed_depth > 1) becomes Bevy children
pub(crate) fn flatten_pending_layers_inner(
    layers: Vec<PendingLayer>,
    current_embed_id: u64,
    embed_depth: u32,
    base_nesting_depth: u32,
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
        // Only set containing_embed_id for direct NON-EMBED children of top-level embeds
        // that were NOT collected from a nested scene:
        // - base_nesting_depth == 0: we're flattening the top-level scene
        // - embed_depth == 1: this is a direct child of a top-level embed
        // - !is_embed: not an embed layer (embeds need to be Bevy children for transform propagation)
        // - !from_deeply_nested_scene: wasn't collected from inside another embed's scene
        let from_nested = layer_without_children.from_deeply_nested_scene;
        let should_decouple =
            base_nesting_depth == 0 && embed_depth == 1 && !is_embed && !from_nested;
        let assigned_embed_id = if should_decouple { current_embed_id } else { 0 };
        layer_without_children.containing_embed_id = assigned_embed_id;
        result.push(layer_without_children);

        // Recursively flatten children and update their parent reference
        // We need to remap IDs to make them unique per embed instance
        if !children.is_empty() {
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
            );

            // Build a map of old ID -> new ID for this embed's children
            // IMPORTANT: Use enumerated index to ensure uniqueness when multiple children
            // have the same ID (which can happen with deeply nested embeds that share
            // the same internal structure, e.g., "编组 3" and "编组 3 Copy" both containing
            // "编组 2" with id=358, leading to duplicate intermediate IDs like 358000353).
            let mut id_remap: std::collections::HashMap<u64, u64> =
                std::collections::HashMap::new();
            for (idx, child) in flattened_children.iter().enumerate() {
                // Create unique ID by combining layer_id, child_id, and index
                // The index ensures uniqueness even when child.id is duplicated
                // Format: layer_id * 1_000_000 + child.id + idx * 1_000_000_000_000
                // This gives each duplicate a distinct ID while preserving the base structure
                let base_id = layer_id.wrapping_mul(1_000_000).wrapping_add(child.id);
                let unique_id = if id_remap.contains_key(&child.id) {
                    // This ID already exists, add index offset to make it unique
                    base_id.wrapping_add((idx as u64).wrapping_mul(1_000_000_000))
                } else {
                    base_id
                };
                id_remap.insert(child.id, unique_id);
            }

            // Now remap IDs, but we need to handle duplicates specially
            // Build a fresh remap that tracks which IDs we've seen
            let mut seen_ids: std::collections::HashSet<u64> = std::collections::HashSet::new();

            for (idx, mut child) in flattened_children.into_iter().enumerate() {
                let old_id = child.id;

                // Calculate the new ID with index offset if needed
                let base_id = layer_id.wrapping_mul(1_000_000).wrapping_add(old_id);
                let new_id = if seen_ids.contains(&base_id) {
                    // Base ID already used, add index offset
                    base_id.wrapping_add((idx as u64).wrapping_mul(1_000_000_000))
                } else {
                    base_id
                };
                seen_ids.insert(new_id);
                child.id = new_id;

                // Remap the parent reference and adjust coordinates
                if child.parent == 0 {
                    child.parent = layer_id;

                    // For spatial decoupling, embed children are NOT Bevy children of embed.
                    // They render directly to the RTT camera at origin (0,0).
                    // The child's coordinates were calculated relative to inner canvas center,
                    // which is exactly where the RTT camera is positioned - no adjustment needed.
                    //
                    // We still set embed_offset and inv_fit_scale for size compensation:
                    // - inv_fit_scale: compensate for project fit scaling on sprite sizes
                    // - embed_offset: used to identify this as embed content (Vec2 != ZERO)
                    //
                    // NOTE: We no longer subtract embed_bevy_pos from coordinates since content
                    // renders directly at origin for the RTT camera.
                    if is_embed {
                        // Store embed offset to identify as embed content (triggers inv_fit_scale use)
                        // The actual value is not used for coordinate adjustment anymore
                        child.animated.embed_offset = Vec2::new(embed_bevy_pos.x, embed_bevy_pos.y);
                        bevy::log::info!(
                            "[FlattenDebug] Setting embed_offset for '{}' (id={}): offset=({:.1},{:.1})",
                            child.label,
                            child.id,
                            embed_bevy_pos.x,
                            embed_bevy_pos.y
                        );
                    }
                } else if let Some(&new_parent_id) = id_remap.get(&child.parent) {
                    child.parent = new_parent_id;
                }

                // Remap containing_embed_id if it was set to a child ID
                if child.containing_embed_id != 0
                    && let Some(&new_embed_id) = id_remap.get(&child.containing_embed_id)
                {
                    child.containing_embed_id = new_embed_id;
                }

                // Also update the layer_id in animated component
                child.animated.layer_id = child.id;

                result.push(child);
            }
        }
    }

    result
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
            let pl = collect_embed_scene(embed, fonts, font_metrics, config, z);
            bevy::log::info!(
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
        // Ignore unsupported layer types
        AmLayer::Bookmark(_) | AmLayer::Audio(_) | AmLayer::Camera(_) | AmLayer::Video(_) => {}
    }
}
pub(crate) fn apply_mask_to_children(layers: &mut [PendingLayer]) {
    // Find all mask layers and their info
    // Masks are root-level layers (parent=0) with blending_mode=Mask or Exclude
    let mut mask_layers: Vec<(u64, f32, AmMaskEntry)> = Vec::new(); // (mask_id, z_index, mask_entry)

    for layer in layers.iter() {
        let is_mask = layer.blending_mode == AmBlendingMode::Mask
            || layer.blending_mode == AmBlendingMode::Exclude;
        if is_mask && layer.parent == 0 {
            // Extract mask geometry from the layer's transform and spec
            let mask_entry = extract_mask_info_from_layer(layer);
            if let Some(mut entry) = mask_entry {
                // Set is_exclude based on blending mode
                entry.is_exclude = layer.blending_mode == AmBlendingMode::Exclude;
                bevy::log::info!(
                    "[MASK] Found {} layer '{}' (id={}) at z={:.4}, center=({:.1},{:.1}), half_size=({:.1},{:.1}), time={}..{}ms",
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
                mask_layers.push((layer.id, layer.z_index, entry));
            }
        }
    }

    if mask_layers.is_empty() {
        return;
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
        let mut applicable_masks: Vec<AmMaskEntry> = Vec::new();
        for (mask_id, mask_z, mask_entry) in &mask_layers {
            if *mask_z > layer.z_index && *mask_id != layer.id {
                applicable_masks.push(mask_entry.clone());
            }
        }

        if !applicable_masks.is_empty() {
            bevy::log::info!(
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
            // Check if this layer's parent has masks
            if layer.parent != 0 {
                if let Some(parent_masks) = layer_masks.get(&layer.parent) {
                    layer.mask_info = Some(AmMaskInfo {
                        masks: parent_masks.clone(),
                    });
                    bevy::log::debug!(
                        "[MASK] Propagated {} mask(s) to child layer '{}' (id={})",
                        parent_masks.len(),
                        layer.label,
                        layer.id
                    );
                    changes = true;
                }
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
    let (width, height, pivot_x, pivot_y, is_circle) = match &layer.spec {
        AmLayerSpec::SdfShape {
            width,
            height,
            pivot_x,
            pivot_y,
            shape_type,
            ..
        } => {
            let is_circle = shape_type == ".circle";
            (*width, *height, *pivot_x, *pivot_y, is_circle)
        }
        AmLayerSpec::SpriteShape { width, height, .. } => (*width, *height, 0.0, 0.0, false),
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
        "[MASK] Extracting mask info: width={}, height={}, pivot=({:.1},{:.1}), scale=({:.3},{:.3}), translation=({:.1},{:.1}), center=({:.1},{:.1}), half_size=({:.1},{:.1}), is_circle={}, time={}..{}",
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
        width / 2.0 * scale_x,
        height / 2.0 * scale_y,
        is_circle,
        layer.start_time,
        layer.end_time
    );

    Some(AmMaskEntry {
        center: Vec2::new(center_x, center_y),
        half_size: Vec2::new(width / 2.0 * scale_x, height / 2.0 * scale_y),
        rotation: layer
            .transform
            .rotation
            .to_euler(bevy::math::EulerRot::ZYX)
            .0,
        scale: Vec2::new(scale_x, scale_y),
        is_circle,
        start_time: layer.start_time,
        end_time: layer.end_time,
        mask_layer_id: layer.id,
        is_exclude: layer.blending_mode == AmBlendingMode::Exclude,
    })
}

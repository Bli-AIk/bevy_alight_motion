//! # collect_mask.rs
//!
//! # 蒙版收集模块
//!
//! Functions for collecting and propagating mask information on pending layers.
//! 收集和传播待处理图层上蒙版信息的函数。

use bevy::prelude::*;

use super::components::*;
use super::helpers::*;

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

        // Hidden mask layers should not clip anything
        if layer.hidden {
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

    // Propagate to children at the current level
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

    // Propagate masks recursively into nested children (embed sub-trees).
    // When a Direct-strategy embed has a mask, its nested children (collected
    // by collect_embed_scene) also need the mask so individual shapes are clipped.
    for layer in layers.iter_mut() {
        if let Some(ref info) = layer.mask_info
            && !layer.children.is_empty()
        {
            let masks = info.masks.clone();
            propagate_masks_to_nested_children(&mut layer.children, &masks);
        }
    }
}

/// Recursively propagate mask entries into nested PendingLayer children.
fn propagate_masks_to_nested_children(children: &mut [PendingLayer], masks: &[AmMaskEntry]) {
    for child in children.iter_mut() {
        if child.blending_mode == AmBlendingMode::Mask
            || child.blending_mode == AmBlendingMode::Exclude
        {
            continue;
        }
        match child.mask_info {
            Some(ref mut info) => {
                let new_masks: Vec<_> = masks
                    .iter()
                    .filter(|m| {
                        !info
                            .masks
                            .iter()
                            .any(|e| e.mask_layer_id == m.mask_layer_id)
                    })
                    .cloned()
                    .collect();
                info.masks.extend(new_masks);
            }
            None => {
                child.mask_info = Some(AmMaskInfo {
                    masks: masks.to_vec(),
                });
            }
        }
        if !child.children.is_empty() {
            propagate_masks_to_nested_children(&mut child.children, masks);
        }
    }
}

/// Extract mask geometry info from a layer's transform and spec.
/// For animated scales (like SDF shapes), we need to get the scale at t=0 from the animation data.
pub(crate) fn extract_mask_info_from_layer(layer: &PendingLayer) -> Option<AmMaskEntry> {
    // Handle EmbedScene (group) masks - these use RTT texture instead of SDF
    if matches!(layer.spec, AmLayerSpec::EmbedScene) {
        let (scene_w, scene_h) = layer.embed_scene_size.unwrap_or((1280.0, 960.0));

        // Convert local time to global time using lifecycle_offset
        let global_start = layer.start_time + layer.animated.lifecycle_offset;
        let global_end = layer.end_time + layer.animated.lifecycle_offset;

        let center_x = layer.transform.translation.x;
        let center_y = layer.transform.translation.y;

        bevy::log::debug!(
            "[MASK] Extracting EMBED mask info: id={}, label='{}', scene_size=({},{}), center=({:.1},{:.1}), time={}..{}ms",
            layer.id,
            layer.label,
            scene_w,
            scene_h,
            center_x,
            center_y,
            global_start,
            global_end
        );

        return Some(AmMaskEntry {
            center: Vec2::new(center_x, center_y),
            half_size: Vec2::new(scene_w / 2.0, scene_h / 2.0),
            rotation: layer
                .transform
                .rotation
                .to_euler(bevy::math::EulerRot::ZYX)
                .0,
            scale: Vec2::ONE,
            is_circle: false,
            start_time: global_start,
            end_time: global_end,
            mask_layer_id: layer.id,
            is_exclude: layer.blending_mode == AmBlendingMode::Exclude,
            mask_parent_layer_id: layer.parent,
            is_embed_mask: true,
            embed_scene_size: Some((scene_w, scene_h)),
        });
    }

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
        mask_parent_layer_id: layer.parent,
        is_embed_mask: false,
        embed_scene_size: None,
    })
}

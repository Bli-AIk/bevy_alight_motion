//! # collect_mask.rs
//!
//! # 蒙版收集模块
//!
//! Functions for collecting and propagating mask information on pending layers.
//! 收集和传播待处理图层上蒙版信息的函数。

use bevy::prelude::*;

use super::components::*;
use super::helpers::*;

#[derive(Clone, Copy, Debug, Default)]
struct LayerWorldTransform2d {
    translation: Vec2,
    scale: Vec2,
    rotation: f32,
}

fn rotate_vec2(v: Vec2, angle: f32) -> Vec2 {
    let (sin, cos) = angle.sin_cos();
    Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos)
}

fn compose_world_transform(
    parent: LayerWorldTransform2d,
    local_translation: Vec2,
    local_scale: Vec2,
    local_rotation: f32,
) -> LayerWorldTransform2d {
    let scaled_local = Vec2::new(
        local_translation.x * parent.scale.x,
        local_translation.y * parent.scale.y,
    );
    let rotated_local = rotate_vec2(scaled_local, parent.rotation);
    LayerWorldTransform2d {
        translation: parent.translation + rotated_local,
        scale: Vec2::new(
            parent.scale.x * local_scale.x,
            parent.scale.y * local_scale.y,
        ),
        rotation: parent.rotation + local_rotation,
    }
}

pub(crate) fn apply_mask_to_children(layers: &mut [PendingLayer]) {
    // Find all mask layers and their info
    // Masks are layers with blending_mode=Mask or Exclude (regardless of parent)
    let mut mask_layers: Vec<(u64, u64, f32, AmMaskEntry)> = Vec::new(); // (mask_id, mask_parent_id, z_index, mask_entry)

    // Build lookup tables for world transform reconstruction at t=0.
    let parent_map: std::collections::HashMap<u64, u64> =
        layers.iter().map(|l| (l.id, l.parent)).collect();
    let local_transform_info: std::collections::HashMap<u64, LayerWorldTransform2d> = layers
        .iter()
        .map(|l| {
            let scale = get_scale_at_normalized_time(&l.animated.scale, 0.0);
            let rotation = l.transform.rotation.to_euler(bevy::math::EulerRot::ZYX).0;
            (
                l.id,
                LayerWorldTransform2d {
                    translation: Vec2::new(l.transform.translation.x, l.transform.translation.y),
                    scale: Vec2::new(scale.0, scale.1),
                    rotation,
                },
            )
        })
        .collect();
    let mut world_transform_cache: std::collections::HashMap<u64, LayerWorldTransform2d> =
        std::collections::HashMap::new();

    fn world_transform_for(
        layer_id: u64,
        parent_map: &std::collections::HashMap<u64, u64>,
        local_transform_info: &std::collections::HashMap<u64, LayerWorldTransform2d>,
        cache: &mut std::collections::HashMap<u64, LayerWorldTransform2d>,
    ) -> Option<LayerWorldTransform2d> {
        if let Some(cached) = cache.get(&layer_id).copied() {
            return Some(cached);
        }

        let local = local_transform_info.get(&layer_id).copied()?;
        let world = match parent_map.get(&layer_id).copied().unwrap_or(0) {
            0 => local,
            parent_id => {
                let parent_world =
                    world_transform_for(parent_id, parent_map, local_transform_info, cache)?;
                compose_world_transform(
                    parent_world,
                    local.translation,
                    local.scale,
                    local.rotation,
                )
            }
        };
        cache.insert(layer_id, world);
        Some(world)
    }

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
            if let Some(parent_world) = world_transform_for(
                layer.parent,
                &parent_map,
                &local_transform_info,
                &mut world_transform_cache,
            ) {
                let scaled_center = Vec2::new(
                    entry.center.x * parent_world.scale.x,
                    entry.center.y * parent_world.scale.y,
                );
                let global_center =
                    parent_world.translation + rotate_vec2(scaled_center, parent_world.rotation);
                let global_half_size = bevy::math::Vec2::new(
                    entry.half_size.x * parent_world.scale.x.abs(),
                    entry.half_size.y * parent_world.scale.y.abs(),
                );

                bevy::log::trace!(
                    "[MASK] Found child {} layer '{}' (id={}, parent={}) at z={:.4}, global_center=({:.1},{:.1}), global_half_size=({:.1},{:.1}), parent_rot={:.3}, time={}..{}ms",
                    if entry.is_exclude { "exclude" } else { "mask" },
                    layer.label,
                    layer.id,
                    layer.parent,
                    layer.z_index,
                    global_center.x,
                    global_center.y,
                    global_half_size.x,
                    global_half_size.y,
                    parent_world.rotation,
                    entry.start_time,
                    entry.end_time
                );

                entry.center = global_center;
                entry.half_size = global_half_size;
                entry.rotation += parent_world.rotation;
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

    // For each non-mask layer, collect all masks that are above it (higher z-index)
    // within the same masking scope.
    //
    // Root masks affect root layers below them and then propagate to descendants.
    // Child masks affect earlier siblings under the same parent and then propagate
    // through that sibling subtree.
    for layer in layers.iter_mut() {
        if layer.blending_mode == AmBlendingMode::Mask
            || layer.blending_mode == AmBlendingMode::Exclude
        {
            continue; // Don't apply mask to mask layer itself
        }

        let mut applicable_masks: Vec<AmMaskEntry> = Vec::new();
        for (mask_id, mask_parent_id, mask_z, mask_entry) in &mask_layers {
            if *mask_id == layer.id || *mask_z <= layer.z_index {
                continue;
            }

            let applies = if layer.parent == 0 {
                // Root-scope layers can be clipped by masks above them in the layer stack.
                // Child masks are allowed here as well: AM uses these to crop lower root-level
                // siblings into a framed region. Exclude only the ancestor chain that owns the mask.
                !is_ancestor_of_mask(layer.id, *mask_parent_id, &parent_map)
            } else {
                // Child masks clip earlier siblings in the same parent scope.
                *mask_parent_id == layer.parent
            };

            if applies
                && !applicable_masks
                    .iter()
                    .any(|existing| existing.mask_layer_id == mask_entry.mask_layer_id)
            {
                applicable_masks.push(mask_entry.clone());
            }
        }

        if !applicable_masks.is_empty() {
            bevy::log::trace!(
                "[MASK] Layer '{}' (id={}, parent={}, z={:.4}) will be clipped by {} mask(s)",
                layer.label,
                layer.id,
                layer.parent,
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

    // NOTE: We propagate masks to children for shader-based clipping.
    // Masks treat the entire group as a whole, clipping pixels at the shader
    // level rather than hiding entire elements.
    //
    // IMPORTANT: Masks must NOT propagate through Composite (RTT) embed
    // boundaries. In Alight Motion, a mask clips the embed's composite
    // output (display quad) in the parent's coordinate space, NOT the
    // individual layers rendered inside the embed's RTT. Propagating
    // masks into RTT content would evaluate them in the wrong coordinate
    // space (global vs RTT-local), causing incorrect clipping.

    // Build set of layer IDs that are Composite-strategy embeds.
    // These embeds render to their own texture; masks should clip
    // the display quad, not penetrate into the RTT content.
    let composite_embed_ids: std::collections::HashSet<u64> = layers
        .iter()
        .filter(|l| {
            l.embed_render_plan
                .as_ref()
                .is_some_and(|p| p.requires_composite)
        })
        .map(|l| l.id)
        .collect();

    // Build map of layer_id -> masks
    let mut layer_masks: std::collections::HashMap<u64, Vec<AmMaskEntry>> =
        std::collections::HashMap::new();
    for layer in layers.iter() {
        if let Some(ref info) = layer.mask_info {
            layer_masks.insert(layer.id, info.masks.clone());
        }
    }

    // Propagate to children at the current level.
    // Stop propagation at Composite embed boundaries.
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
            // Check if this layer's parent or containing embed has masks.
            // Do NOT inherit masks from a Composite embed parent — the
            // mask clips the embed's display quad, not its RTT content.
            let source_id = if layer.parent != 0 {
                Some(layer.parent)
            } else if layer.containing_embed_id != 0 {
                Some(layer.containing_embed_id)
            } else {
                None
            };
            let source_masks = source_id
                .filter(|id| !composite_embed_ids.contains(id))
                .and_then(|id| layer_masks.get(&id).cloned());
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
    // Only propagate for non-Composite embeds (Direct strategy) where
    // children render in the parent's camera and need shader-based clipping.
    for layer in layers.iter_mut() {
        if let Some(ref info) = layer.mask_info
            && !layer.children.is_empty()
        {
            let is_composite = layer
                .embed_render_plan
                .as_ref()
                .is_some_and(|p| p.requires_composite);
            if !is_composite {
                let masks = info.masks.clone();
                propagate_masks_to_nested_children(&mut layer.children, &masks);
            }
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

/// Lift masks from shapes inside Composite embeds to the embed entity.
///
/// In Alight Motion, masks clip the embed's composite output (the RTT
/// texture displayed by the embed's quad), not individual layers inside
/// the RTT. After inner-scene mask collection assigns masks to shapes,
/// this pass moves those masks up to the Composite embed so the shader
/// evaluates the mask on the display quad instead.
///
/// AM 中蒙版裁剪 embed 的合成输出（RTT 纹理），而非 RTT 内部的
/// 单独图层。此函数将内部场景分配给形状的蒙版提升到 Composite embed
/// 实体上，使 shader 在显示四边形上评估蒙版。
pub(crate) fn lift_masks_to_composite_embeds(layers: &mut [PendingLayer]) {
    let composite_embed_ids: std::collections::HashSet<u64> = layers
        .iter()
        .filter(|l| {
            l.embed_render_plan
                .as_ref()
                .is_some_and(|p| p.requires_composite)
        })
        .map(|l| l.id)
        .collect();

    if composite_embed_ids.is_empty() {
        return;
    }

    // Collect masks from children of Composite embeds to lift to the embed.
    let mut masks_to_lift: std::collections::HashMap<u64, Vec<AmMaskEntry>> =
        std::collections::HashMap::new();

    for layer in layers.iter() {
        if layer.mask_info.is_none() {
            continue;
        }
        let embed_id = if layer.parent != 0 && composite_embed_ids.contains(&layer.parent) {
            Some(layer.parent)
        } else if layer.containing_embed_id != 0
            && composite_embed_ids.contains(&layer.containing_embed_id)
        {
            Some(layer.containing_embed_id)
        } else {
            None
        };
        if let Some(eid) = embed_id {
            let masks = &layer.mask_info.as_ref().unwrap().masks;
            masks_to_lift.entry(eid).or_default().extend(masks.clone());
        }
    }

    // Strip masks from children of Composite embeds.
    for layer in layers.iter_mut() {
        let in_composite = (layer.parent != 0 && composite_embed_ids.contains(&layer.parent))
            || (layer.containing_embed_id != 0
                && composite_embed_ids.contains(&layer.containing_embed_id));
        if in_composite && layer.mask_info.is_some() {
            layer.mask_info = None;
        }
    }

    // Assign the lifted masks to the Composite embed entities.
    for layer in layers.iter_mut() {
        let Some(mut masks) = masks_to_lift.remove(&layer.id) else {
            continue;
        };
        masks.dedup_by_key(|m| m.mask_layer_id);
        let info = layer
            .mask_info
            .get_or_insert_with(|| AmMaskInfo { masks: Vec::new() });
        for m in masks {
            if !info
                .masks
                .iter()
                .any(|e| e.mask_layer_id == m.mask_layer_id)
            {
                info.masks.push(m);
            }
        }
        bevy::log::debug!(
            "[MASK] Lifted {} mask(s) to Composite embed '{}' (id={})",
            info.masks.len(),
            layer.label,
            layer.id
        );
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

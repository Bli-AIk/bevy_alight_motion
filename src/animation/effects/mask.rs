//! Mask-related effect systems: mesh blur helper and unified mask system.

use bevy::prelude::*;

use crate::animation::components::{AmAnimated, AmPlayback};
use crate::animation::interpolation::{interpolate_float, interpolate_vec2};
use crate::scene::{AmLayerMarker, AmMaskInfo};

/// Helper function to update mesh vertices and UVs for dynamic blur expansion.
/// This allows the blur glow/halo effect to extend beyond original image boundaries.
/// Note: This assumes CENTER anchor since anchor info is not stored in AmAnimated.
#[allow(dead_code)]
fn update_mesh_for_blur(
    mesh: &mut Mesh,
    width: f32,
    height: f32,
    _anchor: &bevy::sprite::Anchor, // Reserved for future use
    blur_expansion: f32,
) {
    // For center anchor, offset is 0
    let offset_x = 0.0;
    let offset_y = 0.0;

    // Original half-sizes
    let half_w = width / 2.0;
    let half_h = height / 2.0;

    // Vertices expand outward from original rectangle by blur_expansion
    let vertices: Vec<[f32; 3]> = vec![
        [
            offset_x - half_w - blur_expansion,
            offset_y - half_h - blur_expansion,
            0.0,
        ],
        [
            offset_x + half_w + blur_expansion,
            offset_y - half_h - blur_expansion,
            0.0,
        ],
        [
            offset_x + half_w + blur_expansion,
            offset_y + half_h + blur_expansion,
            0.0,
        ],
        [
            offset_x - half_w - blur_expansion,
            offset_y + half_h + blur_expansion,
            0.0,
        ],
    ];

    // UV coordinates that map the expanded mesh to extended texture sampling
    let uv_expand_x = if width > 0.0 {
        blur_expansion / width
    } else {
        0.0
    };
    let uv_expand_y = if height > 0.0 {
        blur_expansion / height
    } else {
        0.0
    };
    let uvs: Vec<[f32; 2]> = vec![
        [-uv_expand_x, 1.0 + uv_expand_y],      // bottom-left
        [1.0 + uv_expand_x, 1.0 + uv_expand_y], // bottom-right
        [1.0 + uv_expand_x, -uv_expand_y],      // top-right
        [-uv_expand_x, -uv_expand_y],           // top-left
    ];

    // Update mesh attributes
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
}

/// System to dynamically update mask state on entities with UnifiedEffectMaterial.
/// This system enables/disables mask clipping based on whether the mask layer is currently active.
/// Supports up to 2 simultaneous masks for dual-mask, dual-exclude, and mixed effects.
///
/// **Dynamic Transform Support**: This system reads the mask layer's current animated transform
/// to support animated masks (rotation, scale, position changes over time).
pub fn update_unified_mask_system(
    playback: Res<AmPlayback>,
    query: Query<(
        &AmMaskInfo,
        &MeshMaterial2d<crate::masked_sprite::UnifiedEffectMaterial>,
        &AmLayerMarker,
        &GlobalTransform,
    )>,
    pending_query: Query<&crate::scene::AmPendingLayers>,
    // Query for mask layer data - we look these up by mask_layer_id
    // Use GlobalTransform instead of Transform to get world position for nested embed masks
    mask_layer_query: Query<(&GlobalTransform, &AmAnimated, &crate::scene::AmLayerSpec)>,
    mut materials: ResMut<Assets<crate::masked_sprite::UnifiedEffectMaterial>>,
) {
    if playback.force_stopped {
        return;
    }

    // Get pending layers to access spawned_entities mapping
    let Some(pending) = pending_query.iter().next() else {
        return;
    };
    let fit_scale = 1.0 / pending.inv_fit_scale;

    let global_time = playback.current_time_ms as u64;
    let global_time_sec = playback.current_time_ms / 1000.0;

    for (mask_info, material_handle, marker, entity_global_transform) in query.iter() {
        // Get all active masks for current time (supports up to 2)
        let active_masks = mask_info.get_active_masks(global_time);

        // Get entity's global scale - this is the coordinate system we need to match
        let entity_global_scale = entity_global_transform.to_scale_rotation_translation().0;

        if let Some(material) = materials.get_mut(&material_handle.0) {
            if active_masks.is_empty() {
                // No active masks - disable masking
                material.uniform_data.effect_flags.x = 0.0;
                material.uniform_data.mask2_flags.x = 0.0;
                material.uniform_data.mask2_flags.y = 0.0; // mask1 rotation
                material.uniform_data.mask2_flags.z = 0.0; // mask2 rotation
                // Debug log when mask is disabled for unified effect
                static UNIFIED_MASK_LOG: std::sync::atomic::AtomicU32 =
                    std::sync::atomic::AtomicU32::new(0);
                let count = UNIFIED_MASK_LOG.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                // Log at frame 32 time (1066ms) and after
                if (1060..=1100).contains(&global_time) && count < 50 {
                    bevy::log::info!(
                        "[UNIFIED_MASK_DISABLED] '{}' at time {}ms: effect_flags set to 0, mask_info.masks.len={}, mask_times={:?}",
                        marker.label,
                        global_time,
                        mask_info.masks.len(),
                        mask_info
                            .masks
                            .iter()
                            .map(|m| (m.start_time, m.end_time))
                            .collect::<Vec<_>>()
                    );
                }
            } else {
                // Helper function to compute mask parameters from layer transform
                // IMPORTANT: The returned coordinates must be in the same space as the entity's world_position.
                // We use entity_global_scale to match the entity's coordinate space.
                let compute_mask_params = |mask: &crate::scene::AmMaskEntry| -> (Vec2, Vec2, f32) {
                    // Try to get the mask layer's current transform and animation data
                    if let Some(&mask_entity) = pending.spawned_entities.get(&mask.mask_layer_id) {
                        if let Ok((mask_global_transform, animated, spec)) =
                            mask_layer_query.get(mask_entity)
                        {
                            // Get base shape dimensions from spec
                            let (base_width, base_height, pivot_x, pivot_y) = match spec {
                                crate::scene::AmLayerSpec::SdfShape {
                                    width,
                                    height,
                                    pivot_x,
                                    pivot_y,
                                    ..
                                } => (*width, *height, *pivot_x, *pivot_y),
                                crate::scene::AmLayerSpec::SpriteShape {
                                    width, height, ..
                                } => (*width, *height, 0.0, 0.0),
                                _ => (
                                    mask.half_size.x * 2.0 / mask.scale.x,
                                    mask.half_size.y * 2.0 / mask.scale.y,
                                    0.0,
                                    0.0,
                                ),
                            };

                            // Calculate layer-local time for interpolation
                            let layer_time =
                                (global_time_sec - animated.start_time as f32 / 1000.0).max(0.0);

                            // Get animated values using interpolation
                            // Rotation
                            let rotation_deg =
                                interpolate_float(&animated.rotation, layer_time).unwrap_or(0.0);
                            let rotation_rad = (-rotation_deg).to_radians(); // Bevy uses opposite rotation direction

                            // Scale (local animated scale)
                            let [scale_x, scale_y] =
                                interpolate_vec2(&animated.scale, layer_time).unwrap_or([1.0, 1.0]);

                            // Size - get animated size (AM stores full dimensions, we need half-extents)
                            let [_anim_size_x, _anim_size_y] =
                                interpolate_vec2(&animated.size, layer_time)
                                    .unwrap_or([base_width, base_height]);

                            // Use the mask layer's GlobalTransform to get world position
                            // This already includes AM project root offset and all parent transforms
                            let mask_translation = mask_global_transform.translation();

                            // Calculate center: accounting for pivot offset with rotation
                            // Pivot offset is scaled by local animated scale, then by entity's global scale
                            let scaled_offset_x = -pivot_x * scale_x * entity_global_scale.x;
                            let scaled_offset_y = pivot_y * scale_y * entity_global_scale.y;

                            let rotated_offset_x = scaled_offset_x * rotation_rad.cos()
                                - scaled_offset_y * rotation_rad.sin();
                            let rotated_offset_y = scaled_offset_x * rotation_rad.sin()
                                + scaled_offset_y * rotation_rad.cos();

                            // Use mask's world translation and add pivot offset
                            let center_x = mask_translation.x + rotated_offset_x;
                            let center_y = mask_translation.y + rotated_offset_y;

                            // Half-size: Use precomputed mask.half_size from collect stage
                            // This already includes initial scale and parent scale for child masks
                            //
                            // IMPORTANT: AM's mask clipping region does NOT animate with scale!
                            // The mask's scale animation only affects the visual stroke/border,
                            // not the actual clipping rectangle. This matches reference behavior
                            // where the mask boundary stays constant while bones animate.
                            //
                            // Previously we calculated scale_ratio = current_scale / initial_scale
                            // and applied it to half_size. This caused the mask to expand/shrink
                            // with the scale animation, which is incorrect.
                            let half_width = mask.half_size.x * fit_scale;
                            let half_height = mask.half_size.y * fit_scale;

                            bevy::log::trace!(
                                "[MASK-UE] mask_trans=({:.1},{:.1}), mask_half_size=({:.1},{:.1}), pivot=({:.1},{:.1}) => center=({:.1},{:.1}), half_size=({:.1},{:.1}), current_scale=({:.3},{:.3}), initial_scale=({:.3},{:.3}), layer_time={:.4}",
                                mask_translation.x,
                                mask_translation.y,
                                mask.half_size.x,
                                mask.half_size.y,
                                pivot_x,
                                pivot_y,
                                center_x,
                                center_y,
                                half_width,
                                half_height,
                                scale_x,
                                scale_y,
                                mask.scale.x,
                                mask.scale.y,
                                layer_time
                            );

                            // Return world coordinates in entity's coordinate space
                            return (
                                Vec2::new(center_x, center_y),
                                Vec2::new(half_width, half_height),
                                rotation_rad,
                            );
                        } else {
                            bevy::log::warn!(
                                "[MASK] Mask entity found but query failed for id={}",
                                mask.mask_layer_id
                            );
                        }
                    } else {
                        bevy::log::warn!(
                            "[MASK] Mask entity NOT found for id={} (spawned_entities has {} entries)",
                            mask.mask_layer_id,
                            pending.spawned_entities.len()
                        );
                    }
                    // Fallback to stored values if transform lookup fails
                    bevy::log::debug!(
                        "[MASK] Using fallback center=({:.1},{:.1}) for mask_layer_id={}",
                        mask.center.x,
                        mask.center.y,
                        mask.mask_layer_id
                    );
                    (
                        mask.center * fit_scale,
                        mask.half_size * fit_scale * mask.scale,
                        mask.rotation,
                    )
                };

                // First mask
                let mask1 = active_masks[0];
                let (mask1_center, mask1_half_size, mask1_rotation) = compute_mask_params(mask1);

                let base_type1 = if mask1.is_circle { 2.0 } else { 1.0 };
                material.uniform_data.effect_flags.x = if mask1.is_exclude {
                    base_type1 + 2.0
                } else {
                    base_type1
                };
                material.uniform_data.mask_params = bevy::math::Vec4::new(
                    mask1_center.x,
                    mask1_center.y,
                    mask1_half_size.x,
                    mask1_half_size.y,
                );
                // Store mask1 rotation in mask2_flags.y (radians)
                material.uniform_data.mask2_flags.y = mask1_rotation;

                // Second mask (if present)
                if active_masks.len() >= 2 {
                    let mask2 = active_masks[1];
                    let (mask2_center, mask2_half_size, mask2_rotation) =
                        compute_mask_params(mask2);

                    let base_type2 = if mask2.is_circle { 2.0 } else { 1.0 };
                    material.uniform_data.mask2_flags.x = if mask2.is_exclude {
                        base_type2 + 2.0
                    } else {
                        base_type2
                    };
                    material.uniform_data.mask2_params = bevy::math::Vec4::new(
                        mask2_center.x,
                        mask2_center.y,
                        mask2_half_size.x,
                        mask2_half_size.y,
                    );
                    // Store mask2 rotation in mask2_flags.z (radians)
                    material.uniform_data.mask2_flags.z = mask2_rotation;

                    bevy::log::debug!(
                        "[UnifiedMask] '{}' time={}, DUAL mask: mask1_type={:.0} center=({:.1},{:.1}) rot={:.2}°, mask2_type={:.0} center=({:.1},{:.1}) rot={:.2}°",
                        marker.label,
                        global_time,
                        material.uniform_data.effect_flags.x,
                        mask1_center.x,
                        mask1_center.y,
                        mask1_rotation.to_degrees(),
                        material.uniform_data.mask2_flags.x,
                        mask2_center.x,
                        mask2_center.y,
                        mask2_rotation.to_degrees()
                    );
                } else {
                    // Only one mask - disable second mask
                    material.uniform_data.mask2_flags.x = 0.0;
                    material.uniform_data.mask2_flags.z = 0.0;

                    // Log entity world position for debugging child layer mask issues
                    let entity_world_pos = entity_global_transform.translation();
                    bevy::log::debug!(
                        "[UnifiedMask] '{}' time={}, mask_type={:.0}, center=({:.1},{:.1}), half_size=({:.1},{:.1}), rot={:.2}°, entity_world_pos=({:.1},{:.1})",
                        marker.label,
                        global_time,
                        material.uniform_data.effect_flags.x,
                        mask1_center.x,
                        mask1_center.y,
                        mask1_half_size.x,
                        mask1_half_size.y,
                        mask1_rotation.to_degrees(),
                        entity_world_pos.x,
                        entity_world_pos.y
                    );
                }
            }
        }
    }
}

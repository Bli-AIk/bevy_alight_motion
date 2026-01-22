//! # sdf.rs
//!
//! # SDF 模块
//!
//! SDF (Signed Distance Field) shape animation systems and related functionality.
//! Contains animate_sdf_opacity_system, animate_sdf_scale_system, update_sdf_mask_system, etc.
//!
//! SDF（有符号距离场）形状动画系统及相关功能。
//! 包含 animate_sdf_opacity_system、animate_sdf_scale_system、update_sdf_mask_system 等。

use bevy::prelude::*;

use crate::scene::{AmLayerMarker, AmMaskInfo};
use crate::sdf_material::{SdfMaterial, repack_with_alpha};

use super::components::{AmAnimated, AmPlayback, AmSdfParams, AmSdfShapeParent};
use super::interpolation::{interpolate_float, interpolate_vec2, interpolate_vec3_with_extrapolation};

/// System to dynamically update mask state on SDF shapes based on mask layer timing.
/// This system enables/disables mask clipping based on whether the mask layer is currently active.
/// Now supports animated masks by reading GlobalTransform and AmAnimated from mask layer entities.
pub fn update_sdf_mask_system(
    playback: Res<AmPlayback>,
    parent_query: Query<
        (&AmAnimated, &Children, &AmMaskInfo, &AmLayerMarker, &GlobalTransform),
        With<AmSdfShapeParent>,
    >,
    pending_query: Query<&crate::scene::AmPendingLayers>,
    // Use GlobalTransform for correct world position of nested embed masks
    mask_layer_query: Query<(&GlobalTransform, &AmAnimated, &crate::scene::AmLayerSpec)>,
    mut sdf_query: Query<(&MeshMaterial2d<SdfMaterial>, &GlobalTransform)>,
    mut materials: ResMut<Assets<SdfMaterial>>,
) {
    if playback.force_stopped {
        return;
    }

    // Get fit_scale and spawned_entities from AmPendingLayers
    let pending = match pending_query.iter().next() {
        Some(p) => p,
        None => return,
    };
    let fit_scale = 1.0 / pending.inv_fit_scale;

    let global_time = playback.current_time_ms;
    let global_time_sec = global_time as f32 / 1000.0;

    for (_animated, children, mask_info, marker, parent_global_transform) in parent_query.iter() {
        // Log parent entity's GlobalTransform for debugging
        let parent_scale = parent_global_transform.to_scale_rotation_translation().0;
        bevy::log::debug!(
            "[MaskParent] '{}' parent_global_scale=({:.2},{:.2})",
            marker.label,
            parent_scale.x, parent_scale.y,
        );
        
        // Get all active masks for current time (supports up to 2)
        let active_masks = mask_info.get_active_masks(global_time as u64);

        // Get parent's global scale - this is the coordinate system we need to match
        let parent_global_scale = parent_global_transform.to_scale_rotation_translation().0;

        // Helper closure to compute mask parameters from mask layer transform
        // IMPORTANT: The returned coordinates must be in the same space as the SDF child entity's world_position.
        // The SDF child entity's GlobalTransform inherits from parent_global_transform (scale = fit_scale).
        // So mask coordinates must also be scaled by fit_scale, NOT by mask's own GlobalTransform.
        let compute_mask_params = |mask: &crate::scene::AmMaskEntry| -> (Vec2, Vec2, f32) {
            // Try to get the mask layer's current transform and animation data
            if let Some(&mask_entity) = pending.spawned_entities.get(&mask.mask_layer_id) {
                if let Ok((_global_transform, mask_animated, spec)) =
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
                        crate::scene::AmLayerSpec::SpriteShape { width, height, .. } => {
                            (*width, *height, 0.0, 0.0)
                        }
                        _ => (
                            mask.half_size.x * 2.0 / mask.scale.x,
                            mask.half_size.y * 2.0 / mask.scale.y,
                            0.0,
                            0.0,
                        ),
                    };

                    // Calculate layer-local time for interpolation
                    let layer_time =
                        (global_time_sec - mask_animated.start_time as f32 / 1000.0).max(0.0);

                    // Get animated values using interpolation
                    // Rotation
                    let rotation_deg =
                        interpolate_float(&mask_animated.rotation, layer_time).unwrap_or(0.0);
                    let rotation_rad = (-rotation_deg).to_radians(); // Bevy uses opposite rotation direction

                    // Scale (local animated scale)
                    let [scale_x, scale_y] =
                        interpolate_vec2(&mask_animated.scale, layer_time).unwrap_or([1.0, 1.0]);

                    // Size - get animated size (AM stores full dimensions, we need half-extents)
                    let [anim_size_x, anim_size_y] =
                        interpolate_vec2(&mask_animated.size, layer_time)
                            .unwrap_or([base_width, base_height]);

                    // Use the mask layer's GlobalTransform to get world position
                    // This already includes AM project root offset and all parent transforms
                    let mask_translation = _global_transform.translation();
                    let mask_global_scale = _global_transform.to_scale_rotation_translation().0;

                    // The mask layer's GlobalTransform includes its own animated scale (1.75)
                    // But the SDF child entity's GlobalTransform only includes parent_global_scale (0.5)
                    // We need to use parent_global_scale for size calculations to match SDF's coordinate space
                    
                    // Calculate the ratio between mask's position scale and SDF's scale
                    // Mask's translation is already in world coords (scaled by mask's global scale)
                    // We need to convert it to SDF's coordinate space
                    let scale_ratio_x = parent_global_scale.x / mask_global_scale.x;
                    let scale_ratio_y = parent_global_scale.y / mask_global_scale.y;
                    
                    // Adjust mask translation to SDF coordinate space
                    // The mask's translation is scaled by mask_global_scale, but SDF uses parent_global_scale
                    // Actually, both mask and SDF are children of the same AM project root
                    // So they share the same base translation from the root
                    // The difference is only in their local scales
                    
                    // For center calculation:
                    // - mask_translation is the world position of mask's pivot point
                    // - We need to add the pivot offset to get geometric center
                    // - The pivot offset should be scaled by mask's own scale (scale_x, scale_y)
                    //   and then by how much the SDF's scale differs from mask's scale
                    
                    // Since mask and SDF share the same root transform, their world positions should align
                    // The issue is that mask's local scale affects its position calculation differently
                    
                    // Actually, let's just use the mask's world position directly
                    // The pivot offset needs to be calculated in world coords
                    let scaled_offset_x = -pivot_x * scale_x * parent_global_scale.x;
                    let scaled_offset_y = pivot_y * scale_y * parent_global_scale.y;

                    let rotated_offset_x =
                        scaled_offset_x * rotation_rad.cos() - scaled_offset_y * rotation_rad.sin();
                    let rotated_offset_y =
                        scaled_offset_x * rotation_rad.sin() + scaled_offset_y * rotation_rad.cos();

                    // Use mask's world translation and add pivot offset
                    let center_x = mask_translation.x + rotated_offset_x;
                    let center_y = mask_translation.y + rotated_offset_y;

                    // Half-size uses animated size and local scale, then scaled by parent's global scale
                    let half_width = anim_size_x * 0.5 * scale_x.abs() * parent_global_scale.x.abs();
                    let half_height = anim_size_y * 0.5 * scale_y.abs() * parent_global_scale.y.abs();

                    bevy::log::debug!(
                        "[MaskDebug] mask_layer_id={}, mask_trans=({:.1},{:.1}), mask_scale=({:.2},{:.2}), parent_scale=({:.2},{:.2}), scale=({:.2},{:.2}), pivot=({:.1},{:.1}) => center=({:.1},{:.1}), half_size=({:.1},{:.1})",
                        mask.mask_layer_id,
                        mask_translation.x, mask_translation.y,
                        mask_global_scale.x, mask_global_scale.y,
                        parent_global_scale.x, parent_global_scale.y,
                        scale_x, scale_y,
                        pivot_x, pivot_y,
                        center_x, center_y,
                        half_width, half_height
                    );

                    // Return world coordinates in parent's coordinate space
                    return (
                        Vec2::new(center_x, center_y),
                        Vec2::new(half_width, half_height),
                        rotation_rad,
                    );
                }
            }
            // Fallback to stored values if transform lookup fails
            (
                mask.center * fit_scale,
                mask.half_size * fit_scale * mask.scale,
                mask.rotation,
            )
        };

        for child in children.iter() {
            if let Ok((material_handle, child_global_transform)) = sdf_query.get_mut(child)
                && let Some(material) = materials.get_mut(&material_handle.0)
            {
                // Log child SDF entity's world position for debugging
                let child_translation = child_global_transform.translation();
                let child_scale = child_global_transform.to_scale_rotation_translation().0;
                let frame_half = material.uniform_data.frame_half;
                
                if active_masks.is_empty() {
                    // No active masks
                    material.uniform_data.mask_type = 0.0;
                    material.uniform_data.mask2_type = 0.0;
                } else {
                    // First mask
                    let mask1 = active_masks[0];
                    let (mask1_center, mask1_half_size, mask1_rotation) =
                        compute_mask_params(mask1);

                    // Log comparison between mask and child SDF entity
                    bevy::log::debug!(
                        "[MaskVsSdf] mask_center=({:.1},{:.1}), mask_half=({:.1},{:.1}) | sdf_pos=({:.1},{:.1}), sdf_scale=({:.2},{:.2}), frame_half={:.1}, sdf_world_range=[{:.1}..{:.1}, {:.1}..{:.1}]",
                        mask1_center.x, mask1_center.y,
                        mask1_half_size.x, mask1_half_size.y,
                        child_translation.x, child_translation.y,
                        child_scale.x, child_scale.y,
                        frame_half,
                        child_translation.x - frame_half * child_scale.x, child_translation.x + frame_half * child_scale.x,
                        child_translation.y - frame_half * child_scale.y, child_translation.y + frame_half * child_scale.y,
                    );

                    material.uniform_data.mask_params = bevy::math::Vec4::new(
                        mask1_center.x,
                        mask1_center.y,
                        mask1_half_size.x,
                        mask1_half_size.y,
                    );
                    let base_type1 = if mask1.is_circle { 2.0 } else { 1.0 };
                    material.uniform_data.mask_type = if mask1.is_exclude {
                        base_type1 + 2.0
                    } else {
                        base_type1
                    };
                    // Store rotation in mask_rotation field (will add to SdfUniformData)
                    material.uniform_data.mask_rotation = mask1_rotation;

                    // Second mask (if present)
                    if active_masks.len() >= 2 {
                        let mask2 = active_masks[1];
                        let (mask2_center, mask2_half_size, mask2_rotation) =
                            compute_mask_params(mask2);

                        material.uniform_data.mask2_params = bevy::math::Vec4::new(
                            mask2_center.x,
                            mask2_center.y,
                            mask2_half_size.x,
                            mask2_half_size.y,
                        );
                        let base_type2 = if mask2.is_circle { 2.0 } else { 1.0 };
                        material.uniform_data.mask2_type = if mask2.is_exclude {
                            base_type2 + 2.0
                        } else {
                            base_type2
                        };
                        material.uniform_data.mask2_rotation = mask2_rotation;

                        bevy::log::debug!(
                            "[SdfMask] '{}' time={}, DUAL mask: mask1_type={:.0} rot={:.2}°, mask2_type={:.0} rot={:.2}°",
                            marker.label,
                            global_time,
                            material.uniform_data.mask_type,
                            mask1_rotation.to_degrees(),
                            material.uniform_data.mask2_type,
                            mask2_rotation.to_degrees()
                        );
                    } else {
                        // Only one mask
                        material.uniform_data.mask2_type = 0.0;
                        material.uniform_data.mask2_rotation = 0.0;

                        bevy::log::debug!(
                            "[SdfMask] '{}' time={}, mask_type={:.0}, center=({:.1},{:.1}), half_size=({:.1},{:.1}), rot={:.2}°",
                            marker.label,
                            global_time,
                            material.uniform_data.mask_type,
                            mask1_center.x,
                            mask1_center.y,
                            mask1_half_size.x,
                            mask1_half_size.y,
                            mask1_rotation.to_degrees()
                        );
                    }
                }
            }
        }
    }
}

/// System to animate SDF shape opacity (handles SdfMaterial entities).
/// Uses Visibility component for proper show/hide behavior and material alpha for opacity animation.
/// Only skips updates when force_stopped is true (for inspector editing).
/// Respects AmForceHidden component - if present on parent, keeps visibility Hidden.
pub fn animate_sdf_opacity_system(
    playback: Res<AmPlayback>,
    parent_query: Query<
        (
            &AmAnimated,
            &Children,
            &AmLayerMarker,
            Option<&crate::scene::AmForceHidden>,
        ),
        With<AmSdfShapeParent>,
    >,
    mut sdf_query: Query<(&MeshMaterial2d<SdfMaterial>, &AmSdfParams, &mut Visibility)>,
    mut materials: ResMut<Assets<SdfMaterial>>,
) {
    // Skip animation only when force stopped (for inspector editing)
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (animated, children, _marker, force_hidden) in parent_query.iter() {
        // Use local time for visibility check (affected by speed)
        let local_time = animated.calc_local_time(global_time);
        let layer_time = animated.calc_layer_time(local_time);
        let opacity = interpolate_float(&animated.opacity, layer_time).unwrap_or(1.0);

        // Check if this layer is forced hidden (external control)
        let is_force_hidden = force_hidden.is_some();

        // Update all SDF children
        for child in children.iter() {
            if let Ok((material_handle, sdf_params, mut visibility)) = sdf_query.get_mut(child) {
                // Check if layer is active
                if !animated.is_active(local_time) {
                    // Hide shape when outside its time range
                    *visibility = Visibility::Hidden;
                    if let Some(material) = materials.get_mut(&material_handle.0) {
                        material.uniform_data.color.w = 0.0;
                        material.uniform_data.params.w =
                            repack_with_alpha(sdf_params.packed_stroke, 0.0);
                    }
                    continue;
                }

                // If force hidden, keep visibility Hidden but still update material for proper timing
                if is_force_hidden {
                    *visibility = Visibility::Hidden;
                } else {
                    // Show shape when within its time range and not force hidden
                    *visibility = Visibility::Inherited;
                }

                if let Some(material) = materials.get_mut(&material_handle.0) {
                    // Multiply by base_alpha to preserve original fill color transparency
                    let final_alpha = opacity * animated.base_alpha;
                    material.uniform_data.color.w = final_alpha.clamp(0.0, 1.0);

                    // Also update stroke alpha: base_stroke_alpha * opacity
                    let final_stroke_alpha = sdf_params.base_stroke_alpha * opacity;
                    material.uniform_data.params.w =
                        repack_with_alpha(sdf_params.packed_stroke, final_stroke_alpha);
                }
            }
        }
    }
}

/// System to update SDF shape dimensions based on parent scale animation.
///
/// ## New Approach (parametric SDF)
/// Instead of using Transform.scale, we update SdfMaterial.params to change the SDF dimensions:
/// - params.x = base_half_width * animation_scale_x
/// - params.y = base_half_height * animation_scale_y
/// - params.z = stroke_width (constant)
/// - params.w = packed_stroke_color (constant)
///
/// This allows non-uniform scaling while keeping stroke width constant.
///
/// Also updates the child transform translation to account for pivot scaling.
/// Since the parent (Pivot) is not scaled, we must move the child (Center)
/// to simulate scaling around the pivot.
pub fn animate_sdf_scale_system(
    playback: Res<AmPlayback>,
    parent_query: Query<(&AmAnimated, &Children), With<AmSdfShapeParent>>,
    mut sdf_query: Query<(&MeshMaterial2d<SdfMaterial>, &AmSdfParams, &mut Transform)>,
    mut materials: ResMut<Assets<SdfMaterial>>,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (animated, children) in parent_query.iter() {
        // Use local time for visibility check (affected by speed)
        let local_time = animated.calc_local_time(global_time);

        // Skip if outside active time range
        if !animated.is_active(local_time) {
            continue;
        }

        // Use animation local time for interpolation
        let layer_time = animated.calc_layer_time(local_time);

        // Get animation scale from keyframes
        let anim_scale = interpolate_vec2(&animated.scale, layer_time).unwrap_or([1.0, 1.0]);

        // Get animated stroke width (or use base value from sdf_params if no animation)
        let stroke_width_animated = if !animated.stroke_width.keyframes.is_empty() {
            interpolate_float(&animated.stroke_width, layer_time).unwrap_or(0.0)
        } else {
            // No animation, will use sdf_params.stroke_width below
            -1.0 // Sentinel value to indicate no animation
        };

        // Update SDF child's params to reflect scaled dimensions
        for child in children.iter() {
            if let Ok((material_handle, sdf_params, mut transform)) = sdf_query.get_mut(child) {
                // Calculate scaled dimensions
                let scaled_half_width = sdf_params.base_half_width * anim_scale[0];
                let scaled_half_height = sdf_params.base_half_height * anim_scale[1];

                // Use animated stroke width if available, otherwise use base value
                let final_stroke_width = if stroke_width_animated >= 0.0 {
                    stroke_width_animated
                } else {
                    sdf_params.stroke_width
                };

                // Update material params: (half_width, half_height, stroke_width, packed_stroke)
                if let Some(material) = materials.get_mut(&material_handle.0) {
                    material.uniform_data.params = Vec4::new(
                        scaled_half_width,
                        scaled_half_height,
                        final_stroke_width,
                        sdf_params.packed_stroke,
                    );
                }

                // Update translation to simulate scaling around pivot
                // Center position = -Pivot * Scale
                // Account for Y-flip: AM pivot_y is down (+), Bevy Y is up
                // So translation.y = pivot_y * scale_y (positive pivot_y moves center UP relative to pivot)
                let new_x = -sdf_params.base_pivot_x * anim_scale[0];
                let new_y = sdf_params.base_pivot_y * anim_scale[1];

                transform.translation.x = new_x;
                transform.translation.y = new_y;
            }
        }
    }
}

/// System to apply mask clipping to layers that have an AmMaskInfo component.
///
/// NOTE: This system is DISABLED because it uses center-based visibility control,
/// which doesn't treat groups as a whole. Instead, we use shader-based pixel clipping
/// via update_effect_mask_system, which properly clips at the pixel level.
///
/// The original implementation checked if the sprite center is within the mask bounds
/// and hid the entire sprite if outside. This caused issues with long sprites that
/// extend beyond the mask - they would be completely hidden even if partially inside.
pub fn apply_mask_clipping_system(
    _playback: Res<AmPlayback>,
    _query: Query<(
        &GlobalTransform,
        &ChildOf,
        &AmMaskInfo,
        &mut Visibility,
        &AmLayerMarker,
    )>,
    _parent_query: Query<&GlobalTransform>,
) {
    // Disabled: using shader-based mask clipping instead
    // Masks should clip at pixel level, not hide entire sprites based on center position
}

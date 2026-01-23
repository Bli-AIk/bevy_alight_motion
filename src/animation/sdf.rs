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
use super::interpolation::{
    interpolate_float, interpolate_vec2, interpolate_vec3_with_extrapolation,
};

/// System to dynamically update mask state on SDF shapes based on mask layer timing.
/// This system enables/disables mask clipping based on whether the mask layer is currently active.
/// Now supports animated masks by reading GlobalTransform and AmAnimated from mask layer entities.
pub fn update_sdf_mask_system(
    playback: Res<AmPlayback>,
    parent_query: Query<
        (
            &AmAnimated,
            &Children,
            &AmMaskInfo,
            &AmLayerMarker,
            &GlobalTransform,
        ),
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

    // Log fit_scale once per frame
    static mut LOGGED_SCALE: bool = false;
    unsafe {
        if !LOGGED_SCALE {
            bevy::log::info!(
                "[MASK_SYSTEM] fit_scale={}, inv_fit_scale={}",
                fit_scale,
                pending.inv_fit_scale
            );
            LOGGED_SCALE = true;
        }
    }

    let global_time = playback.current_time_ms;
    let global_time_sec = global_time as f32 / 1000.0;

    for (_animated, children, mask_info, marker, parent_global_transform) in parent_query.iter() {
        // Log parent entity's GlobalTransform for debugging
        let parent_scale = parent_global_transform.to_scale_rotation_translation().0;
        bevy::log::debug!(
            "[MaskParent] '{}' parent_global_scale=({:.2},{:.2})",
            marker.label,
            parent_scale.x,
            parent_scale.y,
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

                    // Half-size: Use precomputed mask.half_size as base (already includes parent scale for child masks)
                    // Then apply fit_scale and animated scale ratio
                    // mask.half_size = base_half_size * initial_scale * parent_scale (from collect stage)
                    // So we need to apply: fit_scale * (current_scale / initial_scale) for animation
                    // Since initial_scale = mask.scale, the ratio is (scale_x/mask.scale.x, scale_y/mask.scale.y)
                    let scale_ratio_x = if mask.scale.x.abs() > 0.001 {
                        scale_x / mask.scale.x
                    } else {
                        1.0
                    };
                    let scale_ratio_y = if mask.scale.y.abs() > 0.001 {
                        scale_y / mask.scale.y
                    } else {
                        1.0
                    };
                    let half_width = mask.half_size.x * fit_scale * scale_ratio_x.abs();
                    let half_height = mask.half_size.y * fit_scale * scale_ratio_y.abs();

                    bevy::log::debug!(
                        "[MaskDebug] mask_layer_id={}, mask_trans=({:.1},{:.1}), mask_scale=({:.2},{:.2}), parent_scale=({:.2},{:.2}), scale=({:.2},{:.2}), pivot=({:.1},{:.1}) => center=({:.1},{:.1}), half_size=({:.1},{:.1})",
                        mask.mask_layer_id,
                        mask_translation.x,
                        mask_translation.y,
                        mask_global_scale.x,
                        mask_global_scale.y,
                        parent_global_scale.x,
                        parent_global_scale.y,
                        scale_x,
                        scale_y,
                        pivot_x,
                        pivot_y,
                        center_x,
                        center_y,
                        half_width,
                        half_height
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
                    // No active masks - disable masking
                    material.uniform_data.mask_type = 0.0;
                    material.uniform_data.mask2_type = 0.0;
                    // Debug log when mask is disabled
                    static MASK_DISABLE_LOG: std::sync::atomic::AtomicU32 =
                        std::sync::atomic::AtomicU32::new(0);
                    let count = MASK_DISABLE_LOG.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if count < 20 {
                        bevy::log::info!(
                            "[MASK_DISABLED] '{}' at time {}ms: mask_type set to 0 (no active masks)",
                            marker.label,
                            global_time
                        );
                    }
                } else {
                    // First mask
                    let mask1 = active_masks[0];
                    let (mask1_center, mask1_half_size, mask1_rotation) =
                        compute_mask_params(mask1);

                    // Log comparison between mask and child SDF entity
                    bevy::log::debug!(
                        "[MaskVsSdf] mask_center=({:.1},{:.1}), mask_half=({:.1},{:.1}) | sdf_pos=({:.1},{:.1}), sdf_scale=({:.2},{:.2}), frame_half={:.1}, sdf_world_range=[{:.1}..{:.1}, {:.1}..{:.1}]",
                        mask1_center.x,
                        mask1_center.y,
                        mask1_half_size.x,
                        mask1_half_size.y,
                        child_translation.x,
                        child_translation.y,
                        child_scale.x,
                        child_scale.y,
                        frame_half,
                        child_translation.x - frame_half * child_scale.x,
                        child_translation.x + frame_half * child_scale.x,
                        child_translation.y - frame_half * child_scale.y,
                        child_translation.y + frame_half * child_scale.y,
                    );

                    // Use calculated mask params
                    material.uniform_data.mask_params = bevy::math::Vec4::new(
                        mask1_center.x,
                        mask1_center.y,
                        mask1_half_size.x,
                        mask1_half_size.y,
                    );

                    // Debug log actual mask params being sent to shader
                    bevy::log::debug!(
                        "[SdfMaskParams] '{}': shader_center=({:.1},{:.1}), shader_half=({:.1},{:.1})",
                        marker.label,
                        mask1_center.x,
                        mask1_center.y,
                        mask1_half_size.x,
                        mask1_half_size.y
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

    // Debug: count SDF parents
    static SDF_PARENT_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let parent_count = parent_query.iter().count();
    let cnt = SDF_PARENT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if cnt < 5 {
        bevy::log::info!(
            "[SDF_SYSTEM] animate_sdf_scale_system: {} SDF parents found at time {:.1}ms",
            parent_count,
            global_time
        );
    }

    for (animated, children) in parent_query.iter() {
        // Debug: Log scale_assist status for first few occurrences
        if animated.scale_assist_axis != 0 {
            static SDF_DEBUG_COUNTER: std::sync::atomic::AtomicU32 =
                std::sync::atomic::AtomicU32::new(0);
            let count = SDF_DEBUG_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if count < 20 {
                bevy::log::info!(
                    "[SDF_DEBUG] Found SDF parent with scale_assist: layer={}, axis={}, kf_count={}, time={:.1}ms",
                    animated.layer_id,
                    animated.scale_assist_axis,
                    animated.scale_assist.keyframes.len(),
                    global_time
                );
            }
        }

        // Use local time for visibility check (affected by speed)
        let local_time = animated.calc_local_time(global_time);

        // Skip if outside active time range
        if !animated.is_active(local_time) {
            continue;
        }

        // Use animation local time for interpolation
        let layer_time = animated.calc_layer_time(local_time);

        // Get animation scale from keyframes
        let mut anim_scale = interpolate_vec2(&animated.scale, layer_time).unwrap_or([1.0, 1.0]);

        // Apply scale_assist effect (multiplies scale based on axis)
        // Formula derived from reference video analysis:
        //   axis=1 (Y only): scale_y *= scale_param
        //   axis=2 (X only): scale_x *= scale_param
        //   axis=3 (Both):   scale_x *= scale_param
        //                    scale_y /= (scale_param^SCALE_POWER * damp_factor)
        //                    where damp_factor = damp^(1 + DAMP_COEFF*(damp-1)^DAMP_POWER)
        if animated.scale_assist_axis != 0 {
            if let Some(scale_param) = interpolate_float(&animated.scale_assist, layer_time) {
                // Get damp value (defaults to 1.0)
                let damp_param =
                    interpolate_float(&animated.scale_assist_damp, layer_time).unwrap_or(1.0);

                // Constants derived from empirical analysis of AM reference videos
                // scale divisor = scale_param^SCALE_POWER
                // damp factor = damp^(1 + DAMP_COEFF*(damp-1)^DAMP_POWER)
                const SCALE_POWER: f32 = 1.7067; // = ln(2) / ln(1.501), makes scale_y=0.5 when scale_param=1.501
                const DAMP_COEFF: f32 = 2.75;
                const DAMP_POWER: f32 = 1.93;

                let scale_before = anim_scale;

                match animated.scale_assist_axis {
                    1 => {
                        // Y only (vertical stretch)
                        anim_scale[1] *= scale_param;
                    }
                    2 => {
                        // X only (horizontal stretch)
                        anim_scale[0] *= scale_param;
                    }
                    3 => {
                        // Both axes - X stretches, Y compresses
                        // This creates the characteristic "line stretch" effect
                        let damp_exp = 1.0 + DAMP_COEFF * (damp_param - 1.0).powf(DAMP_POWER);
                        let damp_factor = damp_param.powf(damp_exp);
                        let scale_divisor = scale_param.powf(SCALE_POWER) * damp_factor;
                        anim_scale[0] *= scale_param;
                        anim_scale[1] /= scale_divisor;
                    }
                    _ => {}
                }

                // Debug log
                static SDF_SCALE_DEBUG: std::sync::atomic::AtomicU32 =
                    std::sync::atomic::AtomicU32::new(0);
                let cnt = SDF_SCALE_DEBUG.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if cnt < 50 {
                    bevy::log::info!(
                        "[SDF_SCALE_ASSIST] layer={}, time={:.1}ms, scale_param={:.4}, damp={:.4}, scale: ({:.4},{:.4}) -> ({:.4},{:.4})",
                        animated.layer_id,
                        global_time,
                        scale_param,
                        damp_param,
                        scale_before[0],
                        scale_before[1],
                        anim_scale[0],
                        anim_scale[1]
                    );
                }
            }
        }

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

                    // Dynamically update frame_half to accommodate scaled dimensions
                    // This is critical for scale_assist effect which can create extreme stretching
                    // The shader uses frame_half to define the coordinate system range
                    let new_frame_half =
                        scaled_half_width.max(scaled_half_height) + final_stroke_width * 2.0;
                    // Only update if larger than current (avoid shrinking mesh bounds)
                    if new_frame_half > material.uniform_data.frame_half {
                        material.uniform_data.frame_half = new_frame_half;
                    }
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

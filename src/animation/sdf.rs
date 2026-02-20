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
use super::interpolation::{interpolate_float, interpolate_vec2};

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

    // Log fit_scale once per frame (DEBUG level)
    static mut LOGGED_SCALE: bool = false;
    unsafe {
        if !LOGGED_SCALE {
            bevy::log::debug!(
                "[MASK_SYSTEM] fit_scale={}, inv_fit_scale={}",
                fit_scale,
                pending.inv_fit_scale
            );
            LOGGED_SCALE = true;
        }
    }

    let global_time = playback.current_time_ms;

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
        // Returns (center, half_size, rotation, blend_params)
        // blend_params = Vec3(fill_alpha, opacity, stroke_width_world)
        let compute_mask_params =
            |mask: &crate::scene::AmMaskEntry| -> (Vec2, Vec2, f32, Vec3) {
                // Try to get the mask layer's current transform and animation data
                if let Some(&mask_entity) = pending.spawned_entities.get(&mask.mask_layer_id)
                    && let Ok((_global_transform, mask_animated, spec)) =
                        mask_layer_query.get(mask_entity)
                {
                    // Get base shape dimensions and fill alpha from spec
                    let (base_width, base_height, pivot_x, pivot_y, fill_alpha, initial_sw, stroke_dir) =
                        match spec {
                            crate::scene::AmLayerSpec::SdfShape {
                                width,
                                height,
                                pivot_x,
                                pivot_y,
                                fill_color,
                                no_fill,
                                stroke_width,
                                stroke_direction,
                                ..
                            } => {
                                let fa = if *no_fill {
                                    0.0
                                } else if let Some(fc) = fill_color {
                                    // Parse alpha from #AARRGGBB format
                                    if fc.value.len() >= 3 && fc.value.starts_with('#') {
                                        let alpha_hex = &fc.value[1..3];
                                        u8::from_str_radix(alpha_hex, 16).unwrap_or(255) as f32
                                            / 255.0
                                    } else {
                                        1.0
                                    }
                                } else {
                                    1.0
                                };
                                (
                                    *width,
                                    *height,
                                    *pivot_x,
                                    *pivot_y,
                                    fa,
                                    *stroke_width,
                                    stroke_direction.as_str(),
                                )
                            }
                            crate::scene::AmLayerSpec::SpriteShape { width, height, .. } => {
                                (*width, *height, 0.0, 0.0, 1.0, 0.0, "centered")
                            }
                            _ => (
                                mask.half_size.x * 2.0 / mask.scale.x,
                                mask.half_size.y * 2.0 / mask.scale.y,
                                0.0,
                                0.0,
                                1.0,
                                0.0,
                                "centered",
                            ),
                        };

                    // Calculate normalized layer time for interpolation
                    let local_time = mask_animated.calc_local_time(playback.current_time_ms);
                    let layer_time = mask_animated.calc_layer_time(local_time);

                    // Animated opacity and stroke width
                    let mask_opacity =
                        interpolate_float(&mask_animated.opacity, layer_time).unwrap_or(1.0);
                    let current_sw =
                        interpolate_float(&mask_animated.stroke_width, layer_time)
                            .unwrap_or(initial_sw);

                    // Get animated values using interpolation
                    // Rotation
                    let rotation_deg =
                        interpolate_float(&mask_animated.rotation, layer_time).unwrap_or(0.0);
                    let rotation_rad = (-rotation_deg).to_radians();

                    // Scale (local animated scale)
                    let [scale_x, scale_y] =
                        interpolate_vec2(&mask_animated.scale, layer_time).unwrap_or([1.0, 1.0]);

                    // Size - get animated size (AM stores full dimensions, we need half-extents)
                    let [anim_size_x, anim_size_y] =
                        interpolate_vec2(&mask_animated.size, layer_time)
                            .unwrap_or([base_width, base_height]);

                    let mask_translation = _global_transform.translation();
                    let mask_global_scale = _global_transform.to_scale_rotation_translation().0;

                    let _scale_ratio_x = parent_global_scale.x / mask_global_scale.x;
                    let _scale_ratio_y = parent_global_scale.y / mask_global_scale.y;

                    let scaled_offset_x = -pivot_x * scale_x * parent_global_scale.x;
                    let scaled_offset_y = pivot_y * scale_y * parent_global_scale.y;

                    let rotated_offset_x =
                        scaled_offset_x * rotation_rad.cos() - scaled_offset_y * rotation_rad.sin();
                    let rotated_offset_y =
                        scaled_offset_x * rotation_rad.sin() + scaled_offset_y * rotation_rad.cos();

                    let center_x = mask_translation.x + rotated_offset_x;
                    let center_y = mask_translation.y + rotated_offset_y;

                    // Half-size: Compute from current animated size and scale.
                    let initial_stroke_ext_x =
                        mask.half_size.x - base_width / 2.0 * mask.scale.x;
                    let initial_stroke_ext_y =
                        mask.half_size.y - base_height / 2.0 * mask.scale.y;
                    let ext = |sw: f32| match stroke_dir {
                        "inside" => 0.0,
                        "outside" => sw,
                        _ => sw * 0.5,
                    };
                    let stroke_delta = ext(current_sw) - ext(initial_sw);
                    let half_width =
                        (anim_size_x / 2.0 * scale_x + initial_stroke_ext_x + stroke_delta)
                            * fit_scale;
                    let half_height =
                        (anim_size_y / 2.0 * scale_y + initial_stroke_ext_y + stroke_delta)
                            * fit_scale;

                    // Stroke width in world units (same scale as half_size)
                    let sw_world = current_sw * fit_scale;

                    bevy::log::debug!(
                        "[MaskDebug] mask_layer_id={}, center=({:.1},{:.1}), half=({:.1},{:.1}), fill_alpha={:.2}, opacity={:.2}, sw={:.1}",
                        mask.mask_layer_id,
                        center_x, center_y,
                        half_width, half_height,
                        fill_alpha, mask_opacity, sw_world,
                    );

                    return (
                        Vec2::new(center_x, center_y),
                        Vec2::new(half_width, half_height),
                        rotation_rad,
                        Vec3::new(fill_alpha, mask_opacity, sw_world),
                    );
                }
                // Fallback
                (
                    mask.center * fit_scale,
                    mask.half_size * fit_scale * mask.scale,
                    mask.rotation,
                    Vec3::new(1.0, 1.0, 0.0),
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
                    // No active masks - disable masking (content visible without clipping)
                    material.uniform_data.mask_type = 0.0;
                    material.uniform_data.mask2_type = 0.0;
                } else {
                    // First mask
                    let mask1 = active_masks[0];
                    let (mask1_center, mask1_half_size, mask1_rotation, mask1_blend) =
                        compute_mask_params(mask1);

                    // Use calculated mask params
                    material.uniform_data.mask_params = bevy::math::Vec4::new(
                        mask1_center.x,
                        mask1_center.y,
                        mask1_half_size.x,
                        mask1_half_size.y,
                    );
                    material.uniform_data.mask_blend = bevy::math::Vec4::new(
                        mask1_blend.x,
                        mask1_blend.y,
                        mask1_blend.z,
                        0.0,
                    );

                    let base_type1 = if mask1.is_circle { 2.0 } else { 1.0 };
                    material.uniform_data.mask_type = if mask1.is_exclude {
                        base_type1 + 2.0
                    } else {
                        base_type1
                    };
                    material.uniform_data.mask_rotation = mask1_rotation;

                    // Second mask (if present)
                    if active_masks.len() >= 2 {
                        let mask2 = active_masks[1];
                        let (mask2_center, mask2_half_size, mask2_rotation, mask2_blend) =
                            compute_mask_params(mask2);

                        material.uniform_data.mask2_params = bevy::math::Vec4::new(
                            mask2_center.x,
                            mask2_center.y,
                            mask2_half_size.x,
                            mask2_half_size.y,
                        );
                        material.uniform_data.mask2_blend = bevy::math::Vec4::new(
                            mask2_blend.x,
                            mask2_blend.y,
                            mask2_blend.z,
                            0.0,
                        );
                        let base_type2 = if mask2.is_circle { 2.0 } else { 1.0 };
                        material.uniform_data.mask2_type = if mask2.is_exclude {
                            base_type2 + 2.0
                        } else {
                            base_type2
                        };
                        material.uniform_data.mask2_rotation = mask2_rotation;
                    } else {
                        material.uniform_data.mask2_type = 0.0;
                        material.uniform_data.mask2_rotation = 0.0;
                        material.uniform_data.mask2_blend = bevy::math::Vec4::ZERO;
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
        bevy::log::debug!(
            "[SDF_SYSTEM] animate_sdf_scale_system: {} SDF parents found at time {:.1}ms",
            parent_count,
            global_time
        );
    }

    for (animated, children) in parent_query.iter() {
        // Debug: Log parent transform for SDF rendering issues
        bevy::log::debug!(
            "[SDF_PARENT] layer={}: children_count={}",
            animated.layer_id,
            children.len()
        );

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
        if animated.scale_assist_axis != 0
            && let Some(scale_param) = interpolate_float(&animated.scale_assist, layer_time)
        {
            // Get damp value (defaults to 1.0)
            let damp_param =
                interpolate_float(&animated.scale_assist_damp, layer_time).unwrap_or(1.0);

            // Constants derived from empirical analysis of AM reference videos
            // scale divisor = scale_param^SCALE_POWER
            // damp factor = damp^(1 + DAMP_COEFF*(damp-1)^DAMP_POWER)
            const SCALE_POWER: f32 = 1.71; // = ln(2) / ln(1.501), makes scale_y=0.5 when scale_param=1.501
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

        // Get animated stroke width (or use base value from sdf_params if no animation)
        let stroke_width_animated = if !animated.stroke_width.keyframes.is_empty() {
            interpolate_float(&animated.stroke_width, layer_time).unwrap_or(0.0)
        } else {
            // No animation, will use sdf_params.stroke_width below
            -1.0 // Sentinel value to indicate no animation
        };

        // Interpolate animated shape-specific properties (if any have keyframes)
        let mut shape_extra_anim = [0.0f32; 4];
        let mut has_shape_anim = false;
        for (i, prop) in animated.shape_props.iter().enumerate() {
            if !prop.keyframes.is_empty() {
                has_shape_anim = true;
                shape_extra_anim[i] = interpolate_float(prop, layer_time).unwrap_or(0.0);
            } else if let Some(v) = prop.value {
                shape_extra_anim[i] = v;
            }
        }
        // Interpolate animated shape points (for vertex-based shapes)
        let mut shape_pts_anim = [[0.0f32; 2]; 5];
        let mut has_pts_anim = false;
        for (i, pt) in animated.shape_points.iter().enumerate() {
            if !pt.keyframes.is_empty() {
                has_pts_anim = true;
                shape_pts_anim[i] = interpolate_vec2(pt, layer_time).unwrap_or([0.0, 0.0]);
            } else if let Some(v) = pt.value {
                shape_pts_anim[i] = v;
            }
        }

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
                    bevy::log::debug!(
                        "[SDF_SCALE] layer={}: scaled_half=({:.1},{:.1}), stroke={:.1}, frame_half={:.1}, anim_scale=({:.2},{:.2})",
                        animated.layer_id,
                        scaled_half_width,
                        scaled_half_height,
                        final_stroke_width,
                        material.uniform_data.frame_half,
                        anim_scale[0],
                        anim_scale[1]
                    );
                    material.uniform_data.params = Vec4::new(
                        scaled_half_width,
                        scaled_half_height,
                        final_stroke_width,
                        sdf_params.packed_stroke,
                    );

                    // Update shape_extra from animated properties
                    if has_shape_anim {
                        material.uniform_data.shape_extra = Vec4::new(
                            shape_extra_anim[0], shape_extra_anim[1],
                            shape_extra_anim[2], shape_extra_anim[3],
                        );
                    }
                    if has_pts_anim {
                        material.uniform_data.shape_extra = Vec4::new(
                            shape_pts_anim[0][0], shape_pts_anim[0][1],
                            shape_pts_anim[1][0], shape_pts_anim[1][1],
                        );
                        material.uniform_data.shape_extra2 = Vec4::new(
                            shape_pts_anim[2][0], shape_pts_anim[2][1],
                            shape_pts_anim[3][0], shape_pts_anim[3][1],
                        );
                        material.uniform_data.shape_extra3 = Vec4::new(
                            shape_pts_anim[4][0], shape_pts_anim[4][1],
                            0.0, 0.0,
                        );
                    }

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

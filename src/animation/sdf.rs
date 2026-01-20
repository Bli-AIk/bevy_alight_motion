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
use crate::sdf_material::{repack_with_alpha, SdfMaterial};

use super::components::{AmAnimated, AmPlayback, AmSdfParams, AmSdfShapeParent};
use super::interpolation::{interpolate_float, interpolate_vec2};

/// System to dynamically update mask state on SDF shapes based on mask layer timing.
/// This system enables/disables mask clipping based on whether the mask layer is currently active.
pub fn update_sdf_mask_system(
    playback: Res<AmPlayback>,
    parent_query: Query<
        (&AmAnimated, &Children, &AmMaskInfo, &AmLayerMarker),
        With<AmSdfShapeParent>,
    >,
    pending_query: Query<&crate::scene::AmPendingLayers>,
    mut sdf_query: Query<&MeshMaterial2d<SdfMaterial>>,
    mut materials: ResMut<Assets<SdfMaterial>>,
) {
    if playback.force_stopped {
        return;
    }

    // Get fit_scale from AmPendingLayers (stored as inverse)
    let fit_scale = pending_query
        .iter()
        .next()
        .map(|p| 1.0 / p.inv_fit_scale)
        .unwrap_or(1.0);

    let global_time = playback.current_time_ms;

    for (_animated, children, mask_info, marker) in parent_query.iter() {
        // Get all active masks for current time (supports up to 2)
        let active_masks = mask_info.get_active_masks(global_time as u64);

        for child in children.iter() {
            if let Ok(material_handle) = sdf_query.get_mut(child) {
                if let Some(material) = materials.get_mut(&material_handle.0) {
                    if active_masks.is_empty() {
                        // No active masks
                        material.uniform_data.mask_type = 0.0;
                        material.uniform_data.mask2_type = 0.0;
                    } else {
                        // First mask
                        let mask1 = active_masks[0];
                        material.uniform_data.mask_params = bevy::math::Vec4::new(
                            mask1.center.x * fit_scale,
                            mask1.center.y * fit_scale,
                            mask1.half_size.x * fit_scale,
                            mask1.half_size.y * fit_scale,
                        );
                        let base_type1 = if mask1.is_circle { 2.0 } else { 1.0 };
                        material.uniform_data.mask_type =
                            if mask1.is_exclude { base_type1 + 2.0 } else { base_type1 };

                        // Second mask (if present)
                        if active_masks.len() >= 2 {
                            let mask2 = active_masks[1];
                            material.uniform_data.mask2_params = bevy::math::Vec4::new(
                                mask2.center.x * fit_scale,
                                mask2.center.y * fit_scale,
                                mask2.half_size.x * fit_scale,
                                mask2.half_size.y * fit_scale,
                            );
                            let base_type2 = if mask2.is_circle { 2.0 } else { 1.0 };
                            material.uniform_data.mask2_type =
                                if mask2.is_exclude { base_type2 + 2.0 } else { base_type2 };

                            bevy::log::debug!(
                                "[SdfMask] '{}' time={}, DUAL mask: mask1_type={:.0}, mask2_type={:.0}",
                                marker.label, global_time, material.uniform_data.mask_type, material.uniform_data.mask2_type
                            );
                        } else {
                            // Only one mask
                            material.uniform_data.mask2_type = 0.0;

                            bevy::log::debug!(
                                "[SdfMask] '{}' time={}, mask_type={:.0}, center=({:.1},{:.1}), half_size=({:.1},{:.1})",
                                marker.label, global_time, material.uniform_data.mask_type,
                                mask1.center.x * fit_scale, mask1.center.y * fit_scale,
                                mask1.half_size.x * fit_scale, mask1.half_size.y * fit_scale
                            );
                        }
                    }
                }
            }
        }
    }
}

/// System to animate SDF shape opacity (handles SdfMaterial entities).
/// Uses Visibility component for proper show/hide behavior and material alpha for opacity animation.
/// Only skips updates when force_stopped is true (for inspector editing).
pub fn animate_sdf_opacity_system(
    playback: Res<AmPlayback>,
    parent_query: Query<(&AmAnimated, &Children, &AmLayerMarker), With<AmSdfShapeParent>>,
    mut sdf_query: Query<(&MeshMaterial2d<SdfMaterial>, &AmSdfParams, &mut Visibility)>,
    mut materials: ResMut<Assets<SdfMaterial>>,
) {
    // Skip animation only when force stopped (for inspector editing)
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (animated, children, _marker) in parent_query.iter() {
        // Use local time for visibility check (affected by speed)
        let local_time = animated.calc_local_time(global_time);
        let layer_time = animated.calc_layer_time(local_time);
        let opacity = interpolate_float(&animated.opacity, layer_time).unwrap_or(1.0);

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

                // Show shape when within its time range
                *visibility = Visibility::Inherited;

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

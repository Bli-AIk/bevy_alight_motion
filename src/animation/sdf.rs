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
use super::interpolation::{interpolate_color, interpolate_float, interpolate_vec2};
use super::sdf_geometry::compute_sdf_shape_half_extent;
use super::sdf_helpers::{
    accumulate_parent_scale, apply_solidcolor_blend, compute_sdf_own_scale, trace_sdf_once,
    update_sdf_child_material,
};
use super::sdf_mask::{
    apply_sdf_mask_linear_repeat, apply_sdf_mask_radial_repeat, compute_sdf_mask_params,
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

        for child in children.iter() {
            let Ok((material_handle, child_global_transform)) = sdf_query.get_mut(child) else {
                continue;
            };
            let Some(material) = materials.get_mut(&material_handle.0) else {
                continue;
            };
            let _child_translation = child_global_transform.translation();
            let _child_scale = child_global_transform.to_scale_rotation_translation().0;
            let _frame_half = material.uniform_data.frame_half;

            if active_masks.is_empty() {
                material.uniform_data.mask_type = 0.0;
                material.uniform_data.mask2_type = 0.0;
                material.uniform_data.mask1_rr_params1 = Vec4::ZERO;
                continue;
            }

            let mask1 = active_masks[0];
            let (mask1_center, mask1_half_size, mask1_rotation, mask1_blend) =
                compute_sdf_mask_params(
                    mask1,
                    pending,
                    &mask_layer_query,
                    playback.current_time_ms,
                    fit_scale,
                );

            material.uniform_data.mask_params = bevy::math::Vec4::new(
                mask1_center.x,
                mask1_center.y,
                mask1_half_size.x,
                mask1_half_size.y,
            );
            material.uniform_data.mask_blend =
                bevy::math::Vec4::new(mask1_blend.x, mask1_blend.y, mask1_blend.z, 0.0);

            let base_type1 = if mask1.is_circle { 2.0 } else { 1.0 };
            material.uniform_data.mask_type = if mask1.is_exclude {
                base_type1 + 2.0
            } else {
                base_type1
            };
            material.uniform_data.mask_rotation = mask1_rotation;

            // Process radial repeat effect on mask1
            apply_sdf_mask_radial_repeat(
                mask1,
                pending,
                &mask_layer_query,
                playback.current_time_ms,
                fit_scale,
                material,
            );

            // Process linear repeat effect on mask1
            apply_sdf_mask_linear_repeat(
                mask1,
                pending,
                &mask_layer_query,
                playback.current_time_ms,
                fit_scale,
                material,
            );

            if active_masks.len() >= 2 {
                let mask2 = active_masks[1];
                let (mask2_center, mask2_half_size, mask2_rotation, mask2_blend) =
                    compute_sdf_mask_params(
                        mask2,
                        pending,
                        &mask_layer_query,
                        playback.current_time_ms,
                        fit_scale,
                    );

                material.uniform_data.mask2_params = bevy::math::Vec4::new(
                    mask2_center.x,
                    mask2_center.y,
                    mask2_half_size.x,
                    mask2_half_size.y,
                );
                material.uniform_data.mask2_blend =
                    bevy::math::Vec4::new(mask2_blend.x, mask2_blend.y, mask2_blend.z, 0.0);
                let base_type2 = 1.0 + mask2.is_circle as u8 as f32;
                material.uniform_data.mask2_type = base_type2 + mask2.is_exclude as u8 as f32 * 2.0;
                material.uniform_data.mask2_rotation = mask2_rotation;
            } else {
                material.uniform_data.mask2_type = 0.0;
                material.uniform_data.mask2_rotation = 0.0;
                material.uniform_data.mask2_blend = bevy::math::Vec4::ZERO;
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
    mut sdf_query: Query<(
        &MeshMaterial2d<SdfMaterial>,
        &AmSdfParams,
        &mut Visibility,
        &GlobalTransform,
        Option<&ChildOf>,
    )>,
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
            let Ok((material_handle, sdf_params, mut visibility, child_gt, child_of)) =
                sdf_query.get_mut(child)
            else {
                continue;
            };
            // Check if layer is active
            if !animated.is_active(local_time)
                && let Some(material) = materials.get_mut(&material_handle.0)
            {
                *visibility = Visibility::Hidden;
                material.uniform_data.color.w = 0.0;
                material.uniform_data.params.w = repack_with_alpha(sdf_params.packed_stroke, 0.0);
                continue;
            }
            if !animated.is_active(local_time) {
                *visibility = Visibility::Hidden;
                continue;
            }

            // If force hidden, keep visibility Hidden but still update material for proper timing
            if is_force_hidden {
                *visibility = Visibility::Hidden;
            } else {
                *visibility = Visibility::Inherited;
            }

            let Some(material) = materials.get_mut(&material_handle.0) else {
                continue;
            };

            // Animate fill color RGB (interpolate keyframes, convert sRGB→linear)
            if let Some(fc_srgb) = interpolate_color(&animated.fill_color, layer_time) {
                material.uniform_data.color.x = fc_srgb.x.powf(2.2);
                material.uniform_data.color.y = fc_srgb.y.powf(2.2);
                material.uniform_data.color.z = fc_srgb.z.powf(2.2);
            }

            // Multiply by base_alpha to preserve original fill color transparency
            let mut final_alpha = opacity * animated.base_alpha;
            // Apply fade effect (fade in/out)
            final_alpha *= animated.calc_fade_alpha(layer_time);
            // Apply echo alpha (for echokf effect) to both fill and stroke
            let echo_mult = if let Some(ref echo_cfg) = animated.echo_alpha_config {
                echo_cfg.evaluate(global_time)
            } else {
                1.0
            };
            final_alpha *= echo_mult;
            material.uniform_data.color.w = final_alpha.clamp(0.0, 1.0);

            // Also update stroke alpha: base_stroke_alpha * opacity * echo_alpha
            let final_stroke_alpha =
                (sdf_params.base_stroke_alpha * opacity * echo_mult).clamp(0.0, 1.0);
            material.uniform_data.params.w =
                repack_with_alpha(sdf_params.packed_stroke, final_stroke_alpha);

            if _marker.label.starts_with("Rectangle 1 Copy") {
                let parent = child_of.map(|c| c.parent());
                #[expect(clippy::excessive_nesting)]
                // reason: keep the targeted Rectangle 1 Copy trace beside the opacity update
                trace_sdf_once(format!("{}:{}", _marker.id, _marker.label), || {
                    format!(
                        "[SDF] layer_id={} label='{}' parent={:?} vis={:?} fill_alpha={:.3} global_z={:.4} stroke_width={:.3} frame_half={:.3}",
                        _marker.id,
                        _marker.label,
                        parent,
                        *visibility,
                        material.uniform_data.color.w,
                        child_gt.translation().z,
                        material.uniform_data.params.z,
                        material.uniform_data.frame_half,
                    )
                });
            }

            // Apply solidcolor effect: mix base fill color with solid color
            let sc_alpha =
                interpolate_float(&animated.solid_color_alpha, layer_time).unwrap_or(0.0);
            if sc_alpha > 0.0 {
                let sc_color = interpolate_color(&animated.solid_color, layer_time)
                    .unwrap_or(bevy::math::Vec4::ZERO);
                apply_solidcolor_blend(
                    &mut material.uniform_data.color,
                    &animated.base_fill_color,
                    sc_color,
                    sc_alpha,
                    animated.solid_color_blend_mode,
                );
            }

            // Pass pixelate2 threshold to shader via gradient_config.y
            let pix_thresh =
                interpolate_float(&animated.pixelate_threshold, layer_time).unwrap_or(0.0);
            material.uniform_data.gradient_config.y = pix_thresh;
        }
    }
}

/// System to update stretch segment parameters on SDF shapes.
/// Reads stretch animation properties and passes them to the SDF shader uniform.
pub fn animate_sdf_stretch_system(
    playback: Res<AmPlayback>,
    parent_query: Query<(&AmAnimated, &Children, &GlobalTransform), With<AmSdfShapeParent>>,
    sdf_query: Query<&MeshMaterial2d<SdfMaterial>>,
    mut materials: ResMut<Assets<SdfMaterial>>,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (animated, children, global_transform) in parent_query.iter() {
        let local_time = animated.calc_local_time(global_time);
        if !animated.is_active(local_time) {
            continue;
        }
        let layer_time = animated.calc_layer_time(local_time);

        // Check if this layer has stretch effect
        let has_stretch = animated.stretch_amount.value.is_some()
            || !animated.stretch_amount.keyframes.is_empty();
        if !has_stretch {
            continue;
        }

        let angle_deg = interpolate_float(&animated.stretch_angle, layer_time).unwrap_or(0.0);
        let angle_rad = angle_deg.to_radians();
        let stretch_raw = interpolate_float(&animated.stretch_amount, layer_time).unwrap_or(0.0);
        let offset_raw = interpolate_float(&animated.stretch_offset, layer_time).unwrap_or(0.0);
        let smooth_raw = interpolate_float(&animated.stretch_smooth, layer_time).unwrap_or(0.0);

        let adj_stretch = stretch_raw / 500.0;
        let offset_norm = offset_raw / 1000.0;

        let scene_width = animated.canvas_width;
        let scene_height = animated.canvas_height;

        // Extract the entity's Z-rotation from its GlobalTransform
        let (_, quat, _) = global_transform.to_scale_rotation_translation();
        let transform_rot = quat.to_euler(bevy::math::EulerRot::ZYX).0;

        let stretch_params = Vec4::new(angle_rad, adj_stretch, offset_norm, smooth_raw);
        let stretch_meta = Vec4::new(transform_rot, 0.0, scene_width, scene_height);

        // Compute mesh expansion needed for stretch displacement
        let cos_a = angle_rad.cos().abs();
        let sin_a = angle_rad.sin().abs();
        let dx_screen = cos_a * adj_stretch * scene_width;
        let dy_screen = sin_a * adj_stretch * scene_height;
        // Rotate screen-space displacement back to local space
        let rot_cos = transform_rot.cos().abs();
        let rot_sin = transform_rot.sin().abs();
        let extra = (rot_cos * dx_screen + rot_sin * dy_screen)
            .max(rot_sin * dx_screen + rot_cos * dy_screen);

        for child in children.iter() {
            let Ok(material_handle) = sdf_query.get(child) else {
                continue;
            };
            let Some(material) = materials.get_mut(material_handle) else {
                continue;
            };
            material.uniform_data.stretch_params = stretch_params;
            material.uniform_data.stretch_meta = stretch_meta;

            // Expand frame_half to accommodate stretch displacement
            let base_half = compute_sdf_shape_half_extent(&material.uniform_data)
                + material.uniform_data.params.z.abs() * 2.0;
            let needed = base_half + extra;
            if needed > material.uniform_data.frame_half {
                material.uniform_data.frame_half = needed;
            }
        }
    }
}

/// System to update linear repeat uniforms on SDF shapes.
/// The SDF shader renders repeat.line copies in local shape space, so the CPU no longer
/// offsets the parent transform for count>0. Count=0 is still treated as hidden.
pub fn animate_sdf_repeat_system(
    playback: Res<AmPlayback>,
    parent_query: Query<(&AmAnimated, &Children), With<AmSdfShapeParent>>,
    sdf_query: Query<&MeshMaterial2d<SdfMaterial>>,
    mut materials: ResMut<Assets<SdfMaterial>>,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (animated, children) in parent_query.iter() {
        let local_time = animated.calc_local_time(global_time);
        let default_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
        let default_params2 = Vec4::new(0.0, 0.0, 1.0, 1.0);
        let default_params3 = Vec4::new(0.0, 1.0, 0.0, 0.0);
        let default_params4 = Vec4::ZERO;
        let default_params5 = Vec4::ZERO;

        let (params1, params2, params3, params4, params5) = if animated.is_active(local_time)
            && (animated.linear_repeat_count.value.is_some()
                || !animated.linear_repeat_count.keyframes.is_empty())
        {
            let layer_time = animated.calc_layer_time(local_time);
            let count = interpolate_float(&animated.linear_repeat_count, layer_time)
                .unwrap_or(0.0)
                .round();
            let position = interpolate_vec2(&animated.linear_repeat_position, layer_time)
                .unwrap_or([0.0, 0.0]);
            let offset =
                interpolate_vec2(&animated.linear_repeat_offset, layer_time).unwrap_or([0.0, 0.0]);
            let angle = interpolate_float(&animated.linear_repeat_angle, layer_time).unwrap_or(0.0);
            let scale = interpolate_float(&animated.linear_repeat_scale, layer_time).unwrap_or(1.0);
            let alpha = interpolate_float(&animated.linear_repeat_alpha, layer_time).unwrap_or(1.0);
            let start = interpolate_float(&animated.linear_repeat_start, layer_time).unwrap_or(0.0);
            let end = interpolate_float(&animated.linear_repeat_end, layer_time).unwrap_or(1.0);
            let phase = interpolate_float(&animated.linear_repeat_phase, layer_time).unwrap_or(0.0);
            let overlap =
                interpolate_float(&animated.linear_repeat_overlap, layer_time).unwrap_or(0.0);
            let ease_in =
                interpolate_float(&animated.linear_repeat_ease_in, layer_time).unwrap_or(0.0);
            let ease_out =
                interpolate_float(&animated.linear_repeat_ease_out, layer_time).unwrap_or(0.0);
            let shape_invert_alt = animated.linear_repeat_shape * 100
                + if animated.linear_repeat_invert { 10 } else { 0 }
                + if animated.linear_repeat_color_alt_copies {
                    1
                } else {
                    0
                };
            let params5 = if animated.linear_repeat_random_order {
                let seed =
                    interpolate_float(&animated.linear_repeat_seed, layer_time).unwrap_or(0.0);
                let (lo, hi) =
                    crate::animation::effects::repeat::compute_java_random_state_packed(seed);
                Vec4::new(1.0, lo, hi, 0.0)
            } else {
                Vec4::ZERO
            };
            (
                Vec4::new(count, position[0], position[1], angle),
                Vec4::new(offset[0], offset[1], scale, alpha),
                Vec4::new(start, end, phase, overlap),
                Vec4::new(ease_in, ease_out, 0.0, shape_invert_alt as f32),
                params5,
            )
        } else {
            (
                default_params1,
                default_params2,
                default_params3,
                default_params4,
                default_params5,
            )
        };

        for child in children.iter() {
            let Ok(material_handle) = sdf_query.get(child) else {
                continue;
            };
            let Some(material) = materials.get_mut(&material_handle.0) else {
                continue;
            };
            material.uniform_data.linear_repeat_params1 = params1;
            material.uniform_data.linear_repeat_params2 = params2;
            material.uniform_data.linear_repeat_params3 = params3;
            material.uniform_data.linear_repeat_params4 = params4;
            material.uniform_data.linear_repeat_params5 = params5;
        }
    }
}

/// System to update SDF shape dimensions based on parent scale animation.
///
/// ## New Approach (parametric SDF)
/// Instead of using Transform.scale, we update SdfMaterial.params to change the SDF dimensions:
/// - params.x = base_half_width * animation_scale_x
/// - params.y = base_half_height * animation_scale_y
/// - params.z = stroke_width (constant — AM scales path coordinates, not stroke)
/// - params.w = packed_stroke_color (constant)
///
/// This allows non-uniform scaling while keeping stroke width constant.
/// Note: Stroke width is NOT scaled with shape animation because AM applies
/// scale to path vertices directly, not through NanoVG's transform matrix.
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

    // --- First pass: compute own anim_scale for all active SDF shapes ---
    // Store in HashMap so child shapes can look up parent scale.
    // 第一遍：计算所有活跃 SDF 形状的自身动画缩放，存入 HashMap 供子形状查询父级缩放。
    let mut scale_map: std::collections::HashMap<u64, [f32; 2]> = std::collections::HashMap::new();
    let mut parent_map: std::collections::HashMap<u64, u64> = std::collections::HashMap::new();

    for (animated, _children) in parent_query.iter() {
        let local_time = animated.calc_local_time(global_time);
        if !animated.is_active(local_time) {
            continue;
        }
        let layer_time = animated.calc_layer_time(local_time);
        let anim_scale = compute_sdf_own_scale(animated, layer_time, global_time);
        scale_map.insert(animated.layer_id, anim_scale);
        if animated.has_parent && animated.parent_layer_id != 0 {
            parent_map.insert(animated.layer_id, animated.parent_layer_id);
        }
    }

    // --- Second pass: update SDF children with combined (own × parent) scale ---
    // 第二遍：用合并缩放（自身 × 父级）更新 SDF 子实体。
    for (animated, children) in parent_query.iter() {
        let local_time = animated.calc_local_time(global_time);
        if !animated.is_active(local_time) {
            continue;
        }
        let layer_time = animated.calc_layer_time(local_time);

        let own_scale = scale_map
            .get(&animated.layer_id)
            .copied()
            .unwrap_or([1.0, 1.0]);

        // Accumulate parent scale through the hierarchy chain
        // 沿父级链累积缩放
        let parent_scale = accumulate_parent_scale(animated.layer_id, &parent_map, &scale_map);
        let combined_scale = [
            own_scale[0] * parent_scale[0],
            own_scale[1] * parent_scale[1],
        ];

        // Get animated stroke width (or use base value from sdf_params if no animation)
        let stroke_width_animated = if !animated.stroke_width.keyframes.is_empty() {
            interpolate_float(&animated.stroke_width, layer_time).unwrap_or(0.0)
        } else {
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
            let Ok((material_handle, sdf_params, mut transform)) = sdf_query.get_mut(child) else {
                continue;
            };
            let Some(material) = materials.get_mut(&material_handle.0) else {
                continue;
            };

            update_sdf_child_material(
                &mut material.uniform_data,
                sdf_params,
                &mut transform,
                own_scale,
                combined_scale,
                stroke_width_animated,
                has_shape_anim,
                &shape_extra_anim,
                has_pts_anim,
                &shape_pts_anim,
            );
        }
    }
}

/// Compensate for SDF parent shapes having Transform.scale=(1,1,1).
/// SDF shapes don't use Transform.scale for visual scaling (they use shader uniforms),
/// so Bevy's hierarchy doesn't propagate the parent's visual scale to children's positions.
/// This system multiplies each SDF child's local position by the accumulated parent scale.
///
/// SDF 父级形状的 Transform.scale 恒为 (1,1,1)，视觉缩放在着色器中处理。
/// 因此 Bevy 层级不会将父级缩放传播到子级位置。本系统对此进行补偿。
pub fn compensate_sdf_parent_scale_system(
    playback: Res<AmPlayback>,
    mut query: Query<
        (Entity, &AmAnimated, &mut Transform, Option<&ChildOf>),
        With<AmSdfShapeParent>,
    >,
) {
    if playback.force_stopped {
        return;
    }
    let global_time = playback.current_time_ms;

    // First pass: collect visual scale and parent relationships for all active SDF shapes
    let mut scale_map: std::collections::HashMap<u64, [f32; 2]> = std::collections::HashMap::new();
    let mut parent_map: std::collections::HashMap<u64, u64> = std::collections::HashMap::new();

    for (_, animated, _, _) in query.iter() {
        let local_time = animated.calc_local_time(global_time);
        if !animated.is_active(local_time) {
            continue;
        }
        let layer_time = animated.calc_layer_time(local_time);
        scale_map.insert(
            animated.layer_id,
            compute_sdf_own_scale(animated, layer_time, global_time),
        );
        if animated.has_parent && animated.parent_layer_id != 0 {
            parent_map.insert(animated.layer_id, animated.parent_layer_id);
        }
    }

    // Second pass: apply accumulated parent scale to child positions
    // Only scale the LOCATION component, not the pivot offset
    for (_, animated, mut transform, parent) in query.iter_mut() {
        if !animated.has_parent || animated.parent_layer_id == 0 {
            continue;
        }
        if parent.is_none() {
            continue;
        }
        let acc = accumulate_parent_scale(animated.layer_id, &parent_map, &scale_map);
        if (acc[0] - 1.0).abs() > 1e-5 || (acc[1] - 1.0).abs() > 1e-5 {
            let layer_time = {
                let lt = animated.calc_local_time(global_time);
                animated.calc_layer_time(lt)
            };
            let has_explicit_location =
                animated.location.value.is_some() || !animated.location.keyframes.is_empty();
            if has_explicit_location {
                let pivot = interpolate_vec2(&animated.pivot, layer_time).unwrap_or([0.0, 0.0]);
                // Decompose position into location + pivot components
                // SDF: bx = loc_x + pivot_x, by = -loc_y - pivot_y
                let loc_x = transform.translation.x - pivot[0];
                let loc_y = transform.translation.y + pivot[1];
                // Only scale the location part by parent's visual scale
                transform.translation.x = loc_x * acc[0] + pivot[0];
                transform.translation.y = loc_y * acc[1] - pivot[1];
            } else {
                // Child SDF helpers/mask targets often omit `location` and rely on the
                // pivot-only local offset as their authored position. In that case the
                // whole local translation should inherit the parent's visual scale, or the
                // child drifts away from sibling mask layers that were authored in the
                // same local space.
                transform.translation.x *= acc[0];
                transform.translation.y *= acc[1];
            }
        }
    }
}

/// Propagate SDF visual scale to non-SDF descendants.
///
/// SDF parents keep `Transform.scale = 1` and apply their visual scale in shader uniforms,
/// so Bevy's hierarchy does not pass that scale to regular sprite/text/effect children.
/// This system reapplies the accumulated SDF-ancestor scale to those descendants'
/// local translation and scale after `animate_transform_system` has reset them.
pub fn compensate_sdf_ancestor_scale_for_children_system(
    playback: Res<AmPlayback>,
    animated_query: Query<&AmAnimated>,
    sdf_query: Query<&AmAnimated, With<AmSdfShapeParent>>,
    mut child_query: Query<
        (&AmAnimated, &mut Transform, Option<&ChildOf>),
        Without<AmSdfShapeParent>,
    >,
) {
    if playback.force_stopped {
        return;
    }
    let global_time = playback.current_time_ms;

    let mut scale_map: std::collections::HashMap<u64, [f32; 2]> = std::collections::HashMap::new();
    let mut parent_map: std::collections::HashMap<u64, u64> = std::collections::HashMap::new();

    for animated in animated_query.iter() {
        if animated.has_parent && animated.parent_layer_id != 0 {
            parent_map.insert(animated.layer_id, animated.parent_layer_id);
        }
    }

    for animated in sdf_query.iter() {
        let local_time = animated.calc_local_time(global_time);
        if !animated.is_active(local_time) {
            continue;
        }
        let layer_time = animated.calc_layer_time(local_time);
        scale_map.insert(
            animated.layer_id,
            compute_sdf_own_scale(animated, layer_time, global_time),
        );
    }

    for (animated, mut transform, parent) in child_query.iter_mut() {
        if !animated.has_parent || animated.parent_layer_id == 0 || parent.is_none() {
            continue;
        }

        let acc = accumulate_parent_scale(animated.layer_id, &parent_map, &scale_map);
        if (acc[0] - 1.0).abs() <= 1e-5 && (acc[1] - 1.0).abs() <= 1e-5 {
            continue;
        }

        transform.translation.x *= acc[0];
        transform.translation.y *= acc[1];
        transform.scale.x *= acc[0];
        transform.scale.y *= acc[1];
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

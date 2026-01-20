//! # effects.rs
//!
//! # 效果模块
//!
//! Unified effect animation systems for wipe, stretch, blur, and palette effects.
//! Contains animate_unified_effect_system, update_unified_mask_system, animate_rtt_blur_system, etc.
//!
//! 统一效果动画系统，用于擦除、拉伸、模糊和调色板效果。
//! 包含 animate_unified_effect_system、update_unified_mask_system、animate_rtt_blur_system 等。

use bevy::prelude::*;

use crate::scene::{AmLayerMarker, AmMaskInfo};

use super::components::{AmAnimated, AmPlayback, DEBUG_NEGATIVE_HEIGHT_SCALE};
use super::interpolation::{interpolate_float, interpolate_vec2};

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
    )>,
    pending_query: Query<&crate::scene::AmPendingLayers>,
    // Query for mask layer data - we look these up by mask_layer_id
    mask_layer_query: Query<(&Transform, &super::components::AmAnimated, &crate::scene::AmLayerSpec)>,
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

    for (mask_info, material_handle, marker) in query.iter() {
        // Get all active masks for current time (supports up to 2)
        let active_masks = mask_info.get_active_masks(global_time);

        if let Some(material) = materials.get_mut(&material_handle.0) {
            if active_masks.is_empty() {
                // No active masks - disable masking
                material.effect_flags.x = 0.0;
                material.mask2_flags.x = 0.0;
                material.mask2_flags.y = 0.0; // mask1 rotation
                material.mask2_flags.z = 0.0; // mask2 rotation
            } else {
                // Helper function to compute mask parameters from layer transform
                let compute_mask_params = |mask: &crate::scene::AmMaskEntry| -> (Vec2, Vec2, f32) {
                    // Try to get the mask layer's current transform and animation data
                    if let Some(&mask_entity) = pending.spawned_entities.get(&mask.mask_layer_id) {
                        if let Ok((transform, animated, spec)) = mask_layer_query.get(mask_entity) {
                            // Get base shape dimensions from spec
                            let (base_width, base_height, pivot_x, pivot_y) = match spec {
                                crate::scene::AmLayerSpec::SdfShape { width, height, pivot_x, pivot_y, .. } => {
                                    (*width, *height, *pivot_x, *pivot_y)
                                }
                                crate::scene::AmLayerSpec::SpriteShape { width, height, .. } => {
                                    (*width, *height, 0.0, 0.0)
                                }
                                _ => (mask.half_size.x * 2.0 / mask.scale.x, mask.half_size.y * 2.0 / mask.scale.y, 0.0, 0.0)
                            };
                            
                            // Calculate layer-local time for interpolation
                            let layer_time = (global_time_sec - animated.start_time as f32 / 1000.0).max(0.0);
                            
                            // Get animated values using interpolation
                            // Rotation
                            let rotation_deg = interpolate_float(&animated.rotation, layer_time).unwrap_or(0.0);
                            let rotation_rad = (-rotation_deg).to_radians(); // Bevy uses opposite rotation direction
                            
                            // Scale
                            let [scale_x, scale_y] = interpolate_vec2(&animated.scale, layer_time)
                                .unwrap_or([1.0, 1.0]);
                            
                            // Size - get animated size (AM stores full dimensions, we need half-extents)
                            let [anim_size_x, anim_size_y] = interpolate_vec2(&animated.size, layer_time)
                                .unwrap_or([base_width, base_height]);
                            
                            // Location (use transform.translation which is already converted)
                            // Note: For SDF shapes, transform.translation is the pivot position
                            let translation = transform.translation;
                            
                            // Calculate center: accounting for pivot offset with rotation
                            // For SDF shapes with pivot, the visual center rotates around the pivot
                            let scaled_offset_x = -pivot_x * scale_x;
                            let scaled_offset_y = pivot_y * scale_y; // Y negated for Bevy coords
                            
                            let rotated_offset_x = scaled_offset_x * rotation_rad.cos() - scaled_offset_y * rotation_rad.sin();
                            let rotated_offset_y = scaled_offset_x * rotation_rad.sin() + scaled_offset_y * rotation_rad.cos();
                            
                            let center_x = translation.x + rotated_offset_x;
                            let center_y = translation.y + rotated_offset_y;
                            
                            // Half-size uses animated size and scaled by transform scale
                            let half_width = anim_size_x * 0.5 * scale_x.abs();
                            let half_height = anim_size_y * 0.5 * scale_y.abs();
                            
                            return (
                                Vec2::new(center_x * fit_scale, center_y * fit_scale),
                                Vec2::new(half_width * fit_scale, half_height * fit_scale),
                                rotation_rad // Already negated above for Bevy coords
                            );
                        }
                    }
                    // Fallback to stored values if transform lookup fails
                    (
                        mask.center * fit_scale,
                        mask.half_size * fit_scale * mask.scale,
                        mask.rotation
                    )
                };
                
                // First mask
                let mask1 = active_masks[0];
                let (mask1_center, mask1_half_size, mask1_rotation) = compute_mask_params(mask1);
                
                let base_type1 = if mask1.is_circle { 2.0 } else { 1.0 };
                material.effect_flags.x = if mask1.is_exclude {
                    base_type1 + 2.0
                } else {
                    base_type1
                };
                material.mask_params = bevy::math::Vec4::new(
                    mask1_center.x,
                    mask1_center.y,
                    mask1_half_size.x,
                    mask1_half_size.y,
                );
                // Store mask1 rotation in mask2_flags.y (radians)
                material.mask2_flags.y = mask1_rotation;

                // Second mask (if present)
                if active_masks.len() >= 2 {
                    let mask2 = active_masks[1];
                    let (mask2_center, mask2_half_size, mask2_rotation) = compute_mask_params(mask2);
                    
                    let base_type2 = if mask2.is_circle { 2.0 } else { 1.0 };
                    material.mask2_flags.x = if mask2.is_exclude {
                        base_type2 + 2.0
                    } else {
                        base_type2
                    };
                    material.mask2_params = bevy::math::Vec4::new(
                        mask2_center.x,
                        mask2_center.y,
                        mask2_half_size.x,
                        mask2_half_size.y,
                    );
                    // Store mask2 rotation in mask2_flags.z (radians)
                    material.mask2_flags.z = mask2_rotation;

                    bevy::log::debug!(
                        "[UnifiedMask] '{}' time={}, DUAL mask: mask1_type={:.0} center=({:.1},{:.1}) rot={:.2}°, mask2_type={:.0} center=({:.1},{:.1}) rot={:.2}°",
                        marker.label,
                        global_time,
                        material.effect_flags.x,
                        mask1_center.x,
                        mask1_center.y,
                        mask1_rotation.to_degrees(),
                        material.mask2_flags.x,
                        mask2_center.x,
                        mask2_center.y,
                        mask2_rotation.to_degrees()
                    );
                } else {
                    // Only one mask - disable second mask
                    material.mask2_flags.x = 0.0;
                    material.mask2_flags.z = 0.0;

                    bevy::log::debug!(
                        "[UnifiedMask] '{}' time={}, mask_type={:.0}, center=({:.1},{:.1}), half_size=({:.1},{:.1}), rot={:.2}°",
                        marker.label,
                        global_time,
                        material.effect_flags.x,
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

/// System to animate effects on sprites using UnifiedEffectMaterial.
/// This system handles all effect types (wipe, stretch segment, mask, blur) in a single pass.
/// It is designed for the RTT architecture where effects are stackable.
#[allow(clippy::type_complexity)]
pub fn animate_unified_effect_system(
    playback: Res<AmPlayback>,
    mut commands: Commands,
    query: Query<(
        Entity,
        &AmAnimated,
        &MeshMaterial2d<crate::masked_sprite::UnifiedEffectMaterial>,
        &Transform,
        &bevy::mesh::Mesh2d,
        Option<&crate::scene::AmEmbedContentMarker>,
    )>,
    mut materials: ResMut<Assets<crate::masked_sprite::UnifiedEffectMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (entity, animated, material_handle, transform, _mesh2d, embed_marker) in query.iter() {
        // Use local time for visibility check (affected by speed)
        let local_time = animated.calc_local_time(global_time);

        // Get material to update alpha
        if let Some(material) = materials.get_mut(&material_handle.0) {
            if !animated.is_active(local_time) {
                // Hide layer by setting alpha to 0
                material.color.alpha = 0.0;
                continue;
            }

            // Layer is active - restore alpha (will be updated by opacity below)
            let layer_time = animated.calc_layer_time(local_time);
            let opacity = interpolate_float(&animated.opacity, layer_time).unwrap_or(1.0);
            material.color.alpha = opacity * animated.base_alpha;
        } else if !animated.is_active(local_time) {
            continue;
        }

        // Use animation local time for interpolation (affected by speed)
        let layer_time = animated.calc_layer_time(local_time);

        // Get sprite base size and scale
        let sprite_size = interpolate_vec2(&animated.size, layer_time).unwrap_or([100.0, 100.0]);
        let scale = interpolate_vec2(&animated.scale, layer_time).unwrap_or([1.0, 1.0]);
        // Actual rendered size = base size * scale
        // Use abs() because negative size in AM behaves same as positive (no flip)
        let orig_width = (sprite_size[0] * scale[0]).abs().max(1.0);
        let orig_height = (sprite_size[1] * scale[1]).abs().max(1.0);

        // NOTE: inv_fit_scale is NOT applied to RTT content dimensions
        // RTT content renders at scene's internal resolution, and the final
        // display size is determined by embed's transform scale and main scene's fit_scale.
        // Applying inv_fit_scale here would incorrectly enlarge the content.

        // Get transform rotation angle for effect compensation
        // In Bevy, rotation is stored as Quat, extract Z rotation
        let (_, _, transform_rotation_rad) = transform.rotation.to_euler(bevy::math::EulerRot::XYZ);

        // Calculate "world-space" dimensions for stretch calculations
        // When element is rotated, its local width/height swap in world space
        let rot_cos = transform_rotation_rad.cos().abs();
        let rot_sin = transform_rotation_rad.sin().abs();
        let _world_width = orig_width * rot_cos + orig_height * rot_sin;
        let world_height = orig_width * rot_sin + orig_height * rot_cos;
        let _ = world_height; // Reserved for future use

        // Check which effects are active
        let has_wipe = animated.wipe_end.value != Some(1.0)
            || !animated.wipe_end.keyframes.is_empty()
            || animated.wipe_start.value.is_some()
            || !animated.wipe_start.keyframes.is_empty();

        let has_stretch = animated.stretch_amount.value.is_some()
            || !animated.stretch_amount.keyframes.is_empty()
            || animated.stretch_angle.value.is_some()
            || !animated.stretch_angle.keyframes.is_empty()
            || animated.stretch_offset.value.is_some()
            || !animated.stretch_offset.keyframes.is_empty()
            || animated.stretch_smooth.value.is_some()
            || !animated.stretch_smooth.keyframes.is_empty();

        if let Some(material) = materials.get_mut(&material_handle.0) {
            // Update wipe parameters if needed
            if has_wipe {
                material.set_wipe_enabled(true);
                let wipe_start = interpolate_float(&animated.wipe_start, layer_time).unwrap_or(0.0);
                let wipe_end = interpolate_float(&animated.wipe_end, layer_time).unwrap_or(1.0);
                let wipe_angle = interpolate_float(&animated.wipe_angle, layer_time).unwrap_or(0.0);
                let wipe_feather =
                    interpolate_float(&animated.wipe_feather, layer_time).unwrap_or(0.0);
                material.wipe_params = Vec4::new(wipe_start, wipe_end, wipe_angle, wipe_feather);
            } else {
                material.set_wipe_enabled(false);
            }

            // Update blur parameters if needed
            let has_blur = animated.blur_strength.value.is_some()
                || !animated.blur_strength.keyframes.is_empty();
            if has_blur {
                let blur_strength =
                    interpolate_float(&animated.blur_strength, layer_time).unwrap_or(0.0);
                if blur_strength > 0.001 {
                    material.set_blur_enabled(true);
                    // AM strength 2.0 produces very strong blur
                    // Testing shows AM blur is much stronger than expected
                    // Use strength * 80 for closer match to AM's blur intensity
                    let blur_radius_px = blur_strength * 80.0;

                    // Expand mesh to allow blur overflow (circular glow effect)
                    // The blur samples beyond the texture boundary, so mesh needs to be larger
                    // AM's blur glow extends significantly - use 2x radius for full coverage
                    let blur_expansion = blur_radius_px * 2.0;

                    // Pass blur parameters to shader
                    // blur_params.x = blur radius in pixels
                    // blur_params.y = original width (for UV calculations)
                    // blur_params.z = original height (for UV calculations)
                    // blur_params.w = blur expansion in pixels
                    material.blur_params =
                        Vec4::new(blur_radius_px, orig_width, orig_height, blur_expansion);

                    // Update mesh bounds for blur overflow
                    // Create new mesh with expanded bounds (similar to stretch segment approach)
                    let half_w = orig_width / 2.0;
                    let half_h = orig_height / 2.0;

                    // Vertices expand outward by blur_expansion
                    let min_x = -half_w - blur_expansion;
                    let max_x = half_w + blur_expansion;
                    let min_y = -half_h - blur_expansion;
                    let max_y = half_h + blur_expansion;

                    // Calculate UV coordinates that extend beyond 0-1 for blur sampling
                    // The shader will treat out-of-bounds samples as transparent
                    let uv_expand_x = blur_expansion / orig_width;
                    let uv_expand_y = blur_expansion / orig_height;

                    let vertices = vec![
                        [min_x, min_y, 0.0],
                        [max_x, min_y, 0.0],
                        [max_x, max_y, 0.0],
                        [min_x, max_y, 0.0],
                    ];
                    let normals = vec![
                        [0.0, 0.0, 1.0],
                        [0.0, 0.0, 1.0],
                        [0.0, 0.0, 1.0],
                        [0.0, 0.0, 1.0],
                    ];
                    // UV coords extend beyond [0,1] to sample the expanded blur area
                    let uvs = vec![
                        [-uv_expand_x, 1.0 + uv_expand_y],      // bottom-left
                        [1.0 + uv_expand_x, 1.0 + uv_expand_y], // bottom-right
                        [1.0 + uv_expand_x, -uv_expand_y],      // top-right
                        [-uv_expand_x, -uv_expand_y],           // top-left
                    ];
                    let indices = vec![0u32, 1, 2, 0, 2, 3];

                    let mut new_mesh = Mesh::new(
                        bevy::mesh::PrimitiveTopology::TriangleList,
                        bevy::asset::RenderAssetUsages::RENDER_WORLD
                            | bevy::asset::RenderAssetUsages::MAIN_WORLD,
                    );
                    new_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
                    new_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                    new_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                    new_mesh.insert_indices(bevy::mesh::Indices::U32(indices));

                    let new_mesh_handle = meshes.add(new_mesh);
                    commands
                        .entity(entity)
                        .insert(bevy::mesh::Mesh2d(new_mesh_handle));
                } else {
                    material.set_blur_enabled(false);
                    // Reset mesh to original bounds when blur is disabled
                    // This ensures no leftover expansion from previous frames
                }
            } else {
                material.set_blur_enabled(false);
            }

            // Update stretch segment parameters if needed
            if has_stretch {
                material.set_stretch_enabled(true);

                let angle_deg =
                    interpolate_float(&animated.stretch_angle, layer_time).unwrap_or(0.0);
                // Compensate for transform rotation: subtract transform rotation from effect angle
                // This ensures the stretch effect is applied in world space, not local space
                // Note: transform rotation is already negated in animate_transform_system (for Bevy's coord system)
                // So we add it back here to get the original AM rotation value
                let angle_rad = angle_deg.to_radians() + transform_rotation_rad;
                let stretch_px =
                    interpolate_float(&animated.stretch_amount, layer_time).unwrap_or(0.0);
                let offset_px =
                    interpolate_float(&animated.stretch_offset, layer_time).unwrap_or(0.0);
                let smooth = interpolate_float(&animated.stretch_smooth, layer_time).unwrap_or(0.0);
                let smooth_width = smooth * 0.3;

                // Calculate mesh expansion for stretch segment effect
                //
                // The base_size determines how much stretch_px translates to actual pixel stretch.
                // Through black-box testing, we found that the formula depends on the aspect ratio:
                //
                // - For wide shapes (width >= height): use orig_width directly
                // - For tall shapes (width < height): use weighted formula with rotation
                //
                // Special case: when size.y is negative (AM uses this for certain flip/transform
                // operations), the stretch calculation needs to use the diagonal length instead.
                let has_negative_size_y = sprite_size[1] < 0.0;

                // Debug: log raw values for negative height embed content
                if has_negative_size_y && embed_marker.is_some() {
                    info!(
                        "[StretchDebug] layer_id={} sprite_size=({:.2},{:.2}) scale=({:.2},{:.2}) orig=({:.2},{:.2})",
                        animated.layer_id,
                        sprite_size[0],
                        sprite_size[1],
                        scale[0],
                        scale[1],
                        orig_width,
                        orig_height
                    );
                }

                let base_size = if has_negative_size_y {
                    // For negative height, use diagonal length as base, with optional scale factor
                    (orig_width * orig_width + orig_height * orig_height).sqrt()
                        * DEBUG_NEGATIVE_HEIGHT_SCALE
                } else if orig_width >= orig_height {
                    // Wide shape: use original width
                    orig_width
                } else {
                    // Tall shape: use weighted formula with rotation
                    let rot_cos = transform_rotation_rad.cos().abs();
                    let rot_sin = transform_rotation_rad.sin().abs();
                    let world_w = orig_width * rot_cos + orig_height * rot_sin;
                    0.8 * world_w + 0.2 * orig_width
                };
                let base_divisor = base_size / 4.27; // Best match for reference
                let stretch_factor = 1.0 + stretch_px / base_divisor;

                let mut actual_stretch_px = orig_width * stretch_factor - orig_width;

                // Hack: Compensate for RTT stretch issue in groups
                // The issue causes grouped elements to appear shorter/less stretched than expected
                // This seems related to the ratio between RTT canvas height and the standard 960.0 height
                if embed_marker.is_some() {
                    let ratio = animated.canvas_height / 960.0;
                    actual_stretch_px *= ratio;
                }

                let angle_factor = 1.0 - 0.1 * angle_rad.sin().abs();
                let half_gap = actual_stretch_px * 0.5 * angle_factor;

                let rotate = |x: f32, y: f32, angle: f32| -> (f32, f32) {
                    let c = angle.cos();
                    let s = angle.sin();
                    (x * c - y * s, x * s + y * c)
                };

                let transform_vertex = |vx: f32, vy: f32| -> (f32, f32) {
                    let (rx, ry) = rotate(vx, vy, angle_rad);
                    let shifted_x = rx + offset_px;
                    let pushed_x = rx + shifted_x.signum() * half_gap;
                    rotate(pushed_x, ry, -angle_rad)
                };

                let hw = orig_width / 2.0;
                let hh = orig_height / 2.0;
                let corners = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)];

                let mut min_x = f32::MAX;
                let mut max_x = f32::MIN;
                let mut min_y = f32::MAX;
                let mut max_y = f32::MIN;

                for (cx, cy) in corners {
                    let (tx, ty) = transform_vertex(cx, cy);
                    min_x = min_x.min(tx);
                    max_x = max_x.max(tx);
                    min_y = min_y.min(ty);
                    max_y = max_y.max(ty);
                }

                // No padding - the calculated bounds should be exact for stretch effect
                // Padding would cause sample_uv to go outside [0,1] range

                let new_width = max_x - min_x;
                let new_height = max_y - min_y;
                let center_offset_x = (min_x + max_x) / 2.0;
                let center_offset_y = (min_y + max_y) / 2.0;

                // Debug: log stretch calculation details (trace level)
                if stretch_px > 0.1 {
                    let is_embed_content = animated.embed_offset != Vec2::ZERO;
                    trace!(
                        "[Stretch] layer_id={} is_embed={} canvas=({:.0},{:.0}) stretch_px={:.1} actual={:.1} new_h={:.1} neg_h={} base_size={:.1}",
                        animated.layer_id,
                        is_embed_content,
                        animated.canvas_width,
                        animated.canvas_height,
                        stretch_px,
                        actual_stretch_px,
                        new_height,
                        has_negative_size_y,
                        base_size
                    );
                }

                // Update material parameters
                material.stretch_params =
                    Vec4::new(angle_rad, actual_stretch_px, offset_px, smooth_width);
                material.original_size = Vec4::new(orig_width, orig_height, new_width, new_height);
                material.mesh_offset = Vec4::new(center_offset_x, center_offset_y, 0.0, 0.0);

                // Create new mesh with expanded bounds
                let vertices = vec![
                    [min_x, min_y, 0.0],
                    [max_x, min_y, 0.0],
                    [max_x, max_y, 0.0],
                    [min_x, max_y, 0.0],
                ];
                let normals = vec![
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0],
                ];
                let uvs = vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
                let indices = vec![0u32, 1, 2, 0, 2, 3];

                let mut new_mesh = Mesh::new(
                    bevy::mesh::PrimitiveTopology::TriangleList,
                    bevy::asset::RenderAssetUsages::RENDER_WORLD
                        | bevy::asset::RenderAssetUsages::MAIN_WORLD,
                );
                new_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
                new_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                new_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                new_mesh.insert_indices(bevy::mesh::Indices::U32(indices));

                let new_mesh_handle = meshes.add(new_mesh);
                commands
                    .entity(entity)
                    .insert(bevy::mesh::Mesh2d(new_mesh_handle));
            } else {
                material.set_stretch_enabled(false);
                
                // For effect sprites without blur/stretch, still need to update mesh size
                // when scale/size animation changes. This ensures content scales correctly.
                // This applies to BOTH regular content AND embed content.
                // Bounds clipping (if needed) is handled separately by apply_embed_bounds_clipping_system.
                if !has_blur {
                    let half_w = orig_width / 2.0;
                    let half_h = orig_height / 2.0;
                    
                    let vertices = vec![
                        [-half_w, -half_h, 0.0],
                        [half_w, -half_h, 0.0],
                        [half_w, half_h, 0.0],
                        [-half_w, half_h, 0.0],
                    ];
                    let normals = vec![
                        [0.0, 0.0, 1.0],
                        [0.0, 0.0, 1.0],
                        [0.0, 0.0, 1.0],
                        [0.0, 0.0, 1.0],
                    ];
                    let uvs = vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
                    let indices = vec![0u32, 1, 2, 0, 2, 3];

                    let mut new_mesh = Mesh::new(
                        bevy::mesh::PrimitiveTopology::TriangleList,
                        bevy::asset::RenderAssetUsages::RENDER_WORLD
                            | bevy::asset::RenderAssetUsages::MAIN_WORLD,
                    );
                    new_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
                    new_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                    new_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                    new_mesh.insert_indices(bevy::mesh::Indices::U32(indices));

                    let new_mesh_handle = meshes.add(new_mesh);
                    commands
                        .entity(entity)
                        .insert(bevy::mesh::Mesh2d(new_mesh_handle));
                }
            }

            // Update palette map alpha if present
            let has_palette = animated.palette_alpha.value.is_some()
                || !animated.palette_alpha.keyframes.is_empty();
            let palette_enabled = material.is_palette_enabled();
            if has_palette && palette_enabled {
                let palette_alpha =
                    interpolate_float(&animated.palette_alpha, layer_time).unwrap_or(1.0);
                material.set_palette_alpha(palette_alpha);
            }
        }
    }
}

/// System to animate RTT-based Gaussian blur effect.
/// This updates the GaussianBlurEffect component's radius based on animation keyframes.
pub fn animate_rtt_blur_system(
    playback: Res<AmPlayback>,
    mut query: Query<(&AmAnimated, &mut crate::gaussian_blur::GaussianBlurEffect)>,
) {
    // Skip animation only when force stopped
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (animated, mut blur_effect) in query.iter_mut() {
        // Use local time for visibility check (affected by speed)
        let local_time = animated.calc_local_time(global_time);

        // Check if layer is active at current local time
        if !animated.is_active(local_time) {
            continue;
        }

        // Use animation local time for interpolation
        let layer_time = animated.calc_layer_time(local_time);

        // Check if this layer has blur animation
        let has_blur =
            animated.blur_strength.value.is_some() || !animated.blur_strength.keyframes.is_empty();

        if has_blur {
            let blur_strength =
                interpolate_float(&animated.blur_strength, layer_time).unwrap_or(0.0);
            // AM strength 2.0 produces very strong blur
            // Use strength * 80 for closer match to AM's blur intensity
            let blur_radius_px = blur_strength * 80.0;

            if (blur_effect.radius - blur_radius_px).abs() > 0.1 {
                bevy::log::debug!(
                    "[BlurAnim] Updating blur radius: {:.1} -> {:.1} (strength={:.3})",
                    blur_effect.radius,
                    blur_radius_px,
                    blur_strength
                );
                blur_effect.radius = blur_radius_px;
            }
        }
    }
}

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
    for (mask_info, material_handle, _marker, entity_global_transform) in query.iter() {
        // Get all active masks for current time (supports up to 2)
        let active_masks = mask_info.get_active_masks(global_time);

        // Get entity's global scale - this is the coordinate system we need to match
        let entity_global_scale = entity_global_transform.to_scale_rotation_translation().0;

        if let Some(material) = materials.get_mut(&material_handle.0) {
            if active_masks.is_empty() {
                // No active masks - disable masking (content visible without clipping)
                material.uniform_data.effect_flags.x = 0.0;
                material.uniform_data.mask2_flags.x = 0.0;
                material.uniform_data.mask2_flags.y = 0.0; // mask1 rotation
                material.uniform_data.mask2_flags.z = 0.0; // mask2 rotation
            } else {
                // Returns (center, half_size, rotation, blend_params)
                // blend_params = Vec3(fill_alpha, opacity, stroke_width_world)
                let compute_mask_params =
                    |mask: &crate::scene::AmMaskEntry| -> (Vec2, Vec2, f32, Vec3) {
                        if let Some(&mask_entity) =
                            pending.spawned_entities.get(&mask.mask_layer_id)
                            && let Ok((mask_global_transform, animated, spec)) =
                                mask_layer_query.get(mask_entity)
                        {
                            let (
                                base_width,
                                base_height,
                                pivot_x,
                                pivot_y,
                                fill_alpha,
                                initial_sw,
                                stroke_dir,
                            ) = match spec {
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
                                crate::scene::AmLayerSpec::SpriteShape {
                                    width, height, ..
                                } => (*width, *height, 0.0, 0.0, 1.0, 0.0, "centered"),
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

                            let local_time = animated.calc_local_time(playback.current_time_ms);
                            let layer_time = animated.calc_layer_time(local_time);

                            let mask_opacity =
                                interpolate_float(&animated.opacity, layer_time).unwrap_or(1.0);
                            let current_sw = interpolate_float(&animated.stroke_width, layer_time)
                                .unwrap_or(initial_sw);

                            let rotation_deg =
                                interpolate_float(&animated.rotation, layer_time).unwrap_or(0.0);
                            let rotation_rad = (-rotation_deg).to_radians();

                            let [scale_x, scale_y] =
                                interpolate_vec2(&animated.scale, layer_time).unwrap_or([1.0, 1.0]);

                            let mask_translation = mask_global_transform.translation();

                            let scaled_offset_x = -pivot_x * scale_x * entity_global_scale.x;
                            let scaled_offset_y = pivot_y * scale_y * entity_global_scale.y;

                            let rotated_offset_x = scaled_offset_x * rotation_rad.cos()
                                - scaled_offset_y * rotation_rad.sin();
                            let rotated_offset_y = scaled_offset_x * rotation_rad.sin()
                                + scaled_offset_y * rotation_rad.cos();

                            let center_x = mask_translation.x + rotated_offset_x;
                            let center_y = mask_translation.y + rotated_offset_y;

                            let [anim_size_x, anim_size_y] =
                                interpolate_vec2(&animated.size, layer_time)
                                    .unwrap_or([base_width, base_height]);

                            let ext = |sw: f32| match stroke_dir {
                                "inside" => 0.0,
                                "outside" => sw,
                                _ => sw * 0.5,
                            };
                            let stroke_delta = ext(current_sw) - ext(initial_sw);
                            let initial_stroke_ext_x =
                                mask.half_size.x - base_width / 2.0 * mask.scale.x;
                            let initial_stroke_ext_y =
                                mask.half_size.y - base_height / 2.0 * mask.scale.y;
                            let half_width =
                                (anim_size_x / 2.0 * scale_x + initial_stroke_ext_x + stroke_delta)
                                    * fit_scale;
                            let half_height =
                                (anim_size_y / 2.0 * scale_y + initial_stroke_ext_y + stroke_delta)
                                    * fit_scale;

                            let sw_world = current_sw * fit_scale;

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

                // First mask
                let mask1 = active_masks[0];
                let (mask1_center, mask1_half_size, mask1_rotation, mask1_blend) =
                    compute_mask_params(mask1);

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
                material.uniform_data.mask_blend =
                    bevy::math::Vec4::new(mask1_blend.x, mask1_blend.y, mask1_blend.z, 0.0);
                material.uniform_data.mask2_flags.y = mask1_rotation;

                // Second mask (if present)
                if active_masks.len() >= 2 {
                    let mask2 = active_masks[1];
                    let (mask2_center, mask2_half_size, mask2_rotation, mask2_blend) =
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
                    material.uniform_data.mask2_blend =
                        bevy::math::Vec4::new(mask2_blend.x, mask2_blend.y, mask2_blend.z, 0.0);
                    material.uniform_data.mask2_flags.z = mask2_rotation;
                } else {
                    material.uniform_data.mask2_flags.x = 0.0;
                    material.uniform_data.mask2_flags.z = 0.0;
                    material.uniform_data.mask2_blend = bevy::math::Vec4::ZERO;
                }
            }
        }
    }
}

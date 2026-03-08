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

/// All computed mask parameters for one mask entry.
struct MaskResult {
    center: Vec2,
    half_size: Vec2,
    rotation: f32,
    blend: Vec3,
    /// Stretch-segment params for the mask layer (angle_rad, adj_stretch, offset, smooth).
    stretch1: Vec4,
    stretch2: Vec4,
    /// (aspect_w, aspect_h, orig_half_w, orig_half_h) for shader stretch evaluation.
    stretch_info: Vec4,
}

/// Compute mask parameters for a mask entry used by UnifiedEffectMaterial.
fn compute_mask_params(
    mask: &crate::scene::AmMaskEntry,
    pending: &crate::scene::AmPendingLayers,
    mask_layer_query: &Query<(&GlobalTransform, &AmAnimated, &crate::scene::AmLayerSpec)>,
    playback_time: f32,
    entity_global_scale: Vec3,
    fit_scale: f32,
) -> MaskResult {
    let fallback = MaskResult {
        center: mask.center * fit_scale,
        half_size: mask.half_size * fit_scale * mask.scale,
        rotation: mask.rotation,
        blend: Vec3::new(1.0, 1.0, 0.0),
        stretch1: Vec4::ZERO,
        stretch2: Vec4::ZERO,
        stretch_info: Vec4::ZERO,
    };

    let Some(&mask_entity) = pending.spawned_entities.get(&mask.mask_layer_id) else {
        return fallback;
    };
    let Ok((mask_global_transform, animated, spec)) = mask_layer_query.get(mask_entity) else {
        return fallback;
    };

    let (base_width, base_height, pivot_x, pivot_y, fill_alpha, initial_sw, stroke_dir) = match spec
    {
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
                    u8::from_str_radix(alpha_hex, 16).unwrap_or(255) as f32 / 255.0
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

    let local_time = animated.calc_local_time(playback_time);
    let layer_time = animated.calc_layer_time(local_time);

    let mask_opacity = interpolate_float(&animated.opacity, layer_time).unwrap_or(1.0);
    let current_sw = interpolate_float(&animated.stroke_width, layer_time).unwrap_or(initial_sw);

    let rotation_deg = interpolate_float(&animated.rotation, layer_time).unwrap_or(0.0);
    let rotation_rad = (-rotation_deg).to_radians();

    let [scale_x, scale_y] = interpolate_vec2(&animated.scale, layer_time).unwrap_or([1.0, 1.0]);

    let mask_translation = mask_global_transform.translation();

    let scaled_offset_x = -pivot_x * scale_x * entity_global_scale.x;
    let scaled_offset_y = pivot_y * scale_y * entity_global_scale.y;

    let rotated_offset_x =
        scaled_offset_x * rotation_rad.cos() - scaled_offset_y * rotation_rad.sin();
    let rotated_offset_y =
        scaled_offset_x * rotation_rad.sin() + scaled_offset_y * rotation_rad.cos();

    let center_x = mask_translation.x + rotated_offset_x;
    let center_y = mask_translation.y + rotated_offset_y;

    let [anim_size_x, anim_size_y] =
        interpolate_vec2(&animated.size, layer_time).unwrap_or([base_width, base_height]);

    let ext = |sw: f32| match stroke_dir {
        "inside" => 0.0,
        "outside" => sw,
        _ => sw * 0.5,
    };
    let stroke_delta = ext(current_sw) - ext(initial_sw);
    let initial_stroke_ext_x = mask.half_size.x - base_width / 2.0 * mask.scale.x;
    let initial_stroke_ext_y = mask.half_size.y - base_height / 2.0 * mask.scale.y;
    let mut half_width =
        (anim_size_x / 2.0 * scale_x + initial_stroke_ext_x + stroke_delta) * fit_scale;
    let mut half_height =
        (anim_size_y / 2.0 * scale_y + initial_stroke_ext_y + stroke_delta) * fit_scale;

    // Expand mask bounds for stretch-segment effects on the mask layer.
    // Same formula as animate_unified_effect_system mesh expansion.
    let stretch_raw = interpolate_float(&animated.stretch_amount, layer_time).unwrap_or(0.0);
    if stretch_raw > 0.0 {
        let angle_deg = interpolate_float(&animated.stretch_angle, layer_time).unwrap_or(0.0);
        let angle_rad = angle_deg.to_radians();
        let adj = stretch_raw / 500.0;
        let scene_w = animated.canvas_width;
        let scene_h = animated.canvas_height;
        let dx = angle_rad.cos().abs() * adj * scene_w * fit_scale;
        let dy = angle_rad.sin().abs() * adj * scene_h * fit_scale;
        let rc = rotation_rad.cos().abs();
        let rs = rotation_rad.sin().abs();
        half_width += rc * dx + rs * dy;
        half_height += rs * dx + rc * dy;
    }
    let stretch2_raw = interpolate_float(&animated.stretch_seg2_amount, layer_time).unwrap_or(0.0);
    if stretch2_raw > 0.0 {
        let angle_deg = interpolate_float(&animated.stretch_seg2_angle, layer_time).unwrap_or(0.0);
        let angle_rad = angle_deg.to_radians();
        let adj = stretch2_raw / 500.0;
        let scene_w = animated.canvas_width;
        let scene_h = animated.canvas_height;
        let dx = angle_rad.cos().abs() * adj * scene_w * fit_scale;
        let dy = angle_rad.sin().abs() * adj * scene_h * fit_scale;
        let rc = rotation_rad.cos().abs();
        let rs = rotation_rad.sin().abs();
        half_width += rc * dx + rs * dy;
        half_height += rs * dx + rc * dy;
    }

    let sw_world = current_sw * fit_scale;

    // Compute the original (un-expanded) half_size for the shader's UV mapping.
    let orig_half_w =
        (anim_size_x / 2.0 * scale_x + initial_stroke_ext_x + stroke_delta) * fit_scale;
    let orig_half_h =
        (anim_size_y / 2.0 * scale_y + initial_stroke_ext_y + stroke_delta) * fit_scale;

    // Build stretch-segment shader params (same as animate_unified_effect_system).
    let scene_w = animated.canvas_width;
    let scene_h = animated.canvas_height;

    let stretch1 = {
        let s = interpolate_float(&animated.stretch_amount, layer_time).unwrap_or(0.0);
        if s > 0.0 {
            let a = interpolate_float(&animated.stretch_angle, layer_time)
                .unwrap_or(0.0)
                .to_radians();
            let o = interpolate_float(&animated.stretch_offset, layer_time).unwrap_or(0.0) / 1000.0;
            let sm = interpolate_float(&animated.stretch_smooth, layer_time).unwrap_or(0.0);
            Vec4::new(a, s / 500.0, o, sm)
        } else {
            Vec4::ZERO
        }
    };
    let stretch2 = {
        let s = interpolate_float(&animated.stretch_seg2_amount, layer_time).unwrap_or(0.0);
        if s > 0.0 {
            let a = interpolate_float(&animated.stretch_seg2_angle, layer_time)
                .unwrap_or(0.0)
                .to_radians();
            let o = interpolate_float(&animated.stretch_seg2_offset, layer_time).unwrap_or(0.0)
                / 1000.0;
            let sm = interpolate_float(&animated.stretch_seg2_smooth, layer_time).unwrap_or(0.0);
            Vec4::new(a, s / 500.0, o, sm)
        } else {
            Vec4::ZERO
        }
    };

    MaskResult {
        center: Vec2::new(center_x, center_y),
        half_size: Vec2::new(half_width, half_height),
        rotation: rotation_rad,
        blend: Vec3::new(fill_alpha, mask_opacity, sw_world),
        stretch1,
        stretch2,
        stretch_info: Vec4::new(
            scene_w * fit_scale,
            scene_h * fit_scale,
            orig_half_w,
            orig_half_h,
        ),
    }
}

/// Encode mask shape (rect/circle) and exclude flag as a single float flag value.
#[inline]
fn mask_type_flag(is_circle: bool, is_exclude: bool) -> f32 {
    1.0 + is_circle as u8 as f32 + 2.0 * is_exclude as u8 as f32
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
    mask_layer_query: Query<(&GlobalTransform, &AmAnimated, &crate::scene::AmLayerSpec)>,
    mut materials: ResMut<Assets<crate::masked_sprite::UnifiedEffectMaterial>>,
) {
    if playback.force_stopped {
        return;
    }

    let Some(pending) = pending_query.iter().next() else {
        return;
    };
    let fit_scale = 1.0 / pending.inv_fit_scale;

    let global_time = playback.current_time_ms as u64;
    for (mask_info, material_handle, _marker, entity_global_transform) in query.iter() {
        let active_masks = mask_info.get_active_masks(global_time);
        let entity_global_scale = entity_global_transform.to_scale_rotation_translation().0;

        let Some(material) = materials.get_mut(&material_handle.0) else {
            continue;
        };

        if active_masks.is_empty() {
            material.uniform_data.effect_flags.x = 0.0;
            material.uniform_data.mask2_flags.x = 0.0;
            material.uniform_data.mask2_flags.y = 0.0;
            material.uniform_data.mask2_flags.z = 0.0;
            continue;
        }

        // First mask
        let mask1 = active_masks[0];
        let m1 = compute_mask_params(
            mask1,
            pending,
            &mask_layer_query,
            playback.current_time_ms,
            entity_global_scale,
            fit_scale,
        );

        material.uniform_data.effect_flags.x = mask_type_flag(mask1.is_circle, mask1.is_exclude);
        material.uniform_data.mask_params =
            bevy::math::Vec4::new(m1.center.x, m1.center.y, m1.half_size.x, m1.half_size.y);
        material.uniform_data.mask_blend =
            bevy::math::Vec4::new(m1.blend.x, m1.blend.y, m1.blend.z, 0.0);
        material.uniform_data.mask2_flags.y = m1.rotation;
        material.uniform_data.mask1_stretch1_params = m1.stretch1;
        material.uniform_data.mask1_stretch2_params = m1.stretch2;
        material.uniform_data.mask1_stretch_info = m1.stretch_info;

        // Second mask (if present)
        if active_masks.len() >= 2 {
            let mask2 = active_masks[1];
            let m2 = compute_mask_params(
                mask2,
                pending,
                &mask_layer_query,
                playback.current_time_ms,
                entity_global_scale,
                fit_scale,
            );

            material.uniform_data.mask2_flags.x = mask_type_flag(mask2.is_circle, mask2.is_exclude);
            material.uniform_data.mask2_params =
                bevy::math::Vec4::new(m2.center.x, m2.center.y, m2.half_size.x, m2.half_size.y);
            material.uniform_data.mask2_blend =
                bevy::math::Vec4::new(m2.blend.x, m2.blend.y, m2.blend.z, 0.0);
            material.uniform_data.mask2_flags.z = m2.rotation;
        } else {
            material.uniform_data.mask2_flags.x = 0.0;
            material.uniform_data.mask2_flags.z = 0.0;
            material.uniform_data.mask2_blend = bevy::math::Vec4::ZERO;
        }
    }
}

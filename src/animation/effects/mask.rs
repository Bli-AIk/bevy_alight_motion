//! Mask-related effect systems: mesh blur helper and unified mask system.

use bevy::prelude::*;

use crate::animation::components::{AmAnimated, AmPlayback};
use crate::animation::effects::repeat::compute_java_random_state_packed;
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
    fit_scale: f32,
) -> MaskResult {
    let fallback = MaskResult {
        center: mask.center * fit_scale,
        half_size: Vec2::new(mask.half_size.x.abs(), mask.half_size.y.abs()) * fit_scale,
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

    let (mask_global_scale, _, mask_translation) =
        mask_global_transform.to_scale_rotation_translation();

    // The mask geometry must follow the mask layer's own world transform.
    // Using the masked entity's global scale breaks mirrored branches:
    // negative-scale content would shift the mask to the wrong side and get clipped away.
    let scaled_offset_x = -pivot_x * scale_x * mask_global_scale.x;
    let scaled_offset_y = pivot_y * scale_y * mask_global_scale.y;

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
        half_size: Vec2::new(half_width.abs(), half_height.abs()),
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

/// Extract mask layer's repeat parameters (basic repeat + linear repeat) and write them
/// into the material uniforms so the shader can compute per-copy mask displacement.
fn set_mask_repeat_uniforms(
    mask_entry: &crate::scene::AmMaskEntry,
    pending: &crate::scene::AmPendingLayers,
    mask_layer_query: &Query<(&GlobalTransform, &AmAnimated, &crate::scene::AmLayerSpec)>,
    playback_time: f32,
    fit_scale: f32,
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
) {
    // Find the mask layer entity
    let Some(&mask_entity) = pending.spawned_entities.get(&mask_entry.mask_layer_id) else {
        bevy::log::warn!(
            "[MASK-RPT] mask entity NOT in spawned_entities for layer_id={}",
            mask_entry.mask_layer_id
        );
        material.uniform_data.mask1_lr_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
        material.uniform_data.mask1_lr2_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
        material.uniform_data.mask1_repeat_params1 = Vec4::ZERO;
        material.uniform_data.mask1_rr_params1 = Vec4::ZERO;
        return;
    };
    let Ok((_gt, animated, _spec)) = mask_layer_query.get(mask_entity) else {
        bevy::log::warn!(
            "[MASK-RPT] mask entity {:?} missing query components (GT/Animated/Spec)",
            mask_entity
        );
        material.uniform_data.mask1_lr_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
        material.uniform_data.mask1_lr2_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
        material.uniform_data.mask1_repeat_params1 = Vec4::ZERO;
        material.uniform_data.mask1_rr_params1 = Vec4::ZERO;
        return;
    };

    let local_time = animated.calc_local_time(playback_time);
    let layer_time = animated.calc_layer_time(local_time);

    // --- Basic repeat (com.alightcreative.effects.repeat) ---
    let rp_count = interpolate_float(&animated.repeat_count, layer_time).unwrap_or(0.0);
    bevy::log::warn!(
        "[MASK-RPT] mask layer_id={} rp_count={:.1} lr_count={:.1}",
        mask_entry.mask_layer_id,
        rp_count,
        interpolate_float(&animated.linear_repeat_count, layer_time).unwrap_or(0.0)
    );
    if rp_count > 0.0 {
        let rp_offset = interpolate_vec2(&animated.repeat_offset, layer_time).unwrap_or([0.0, 0.0]);
        let rp_angle = interpolate_float(&animated.repeat_angle, layer_time).unwrap_or(0.0);
        let rp_scale = interpolate_float(&animated.repeat_scale, layer_time).unwrap_or(1.0);
        let rp_alpha = interpolate_float(&animated.repeat_alpha, layer_time).unwrap_or(1.0);

        // Convert offset from pixel_coord space to world units (fit_scale only, no mask_scale)
        let off_world_x = rp_offset[0] * fit_scale;
        let off_world_y = -rp_offset[1] * fit_scale;

        material.uniform_data.mask1_repeat_params1 =
            Vec4::new(rp_count.floor(), off_world_x, off_world_y, rp_angle);
        material.uniform_data.mask1_repeat_params2 = Vec4::new(rp_scale, rp_alpha, 0.0, 0.0);
    } else {
        material.uniform_data.mask1_repeat_params1 = Vec4::ZERO;
        material.uniform_data.mask1_repeat_params2 = Vec4::new(1.0, 1.0, 0.0, 0.0);
    }

    // Process first linear repeat (flat fields on AmAnimated)
    let count = interpolate_float(&animated.linear_repeat_count, layer_time)
        .unwrap_or(0.0)
        .round();
    if count > 0.0 {
        let pos =
            interpolate_vec2(&animated.linear_repeat_position, layer_time).unwrap_or([0.0, 0.0]);
        let off =
            interpolate_vec2(&animated.linear_repeat_offset, layer_time).unwrap_or([0.0, 0.0]);
        let angle = interpolate_float(&animated.linear_repeat_angle, layer_time).unwrap_or(0.0);
        let lr_scale = interpolate_float(&animated.linear_repeat_scale, layer_time).unwrap_or(1.0);
        let alpha = interpolate_float(&animated.linear_repeat_alpha, layer_time).unwrap_or(1.0);
        let start = interpolate_float(&animated.linear_repeat_start, layer_time).unwrap_or(0.0);
        let end = interpolate_float(&animated.linear_repeat_end, layer_time).unwrap_or(1.0);
        let phase = interpolate_float(&animated.linear_repeat_phase, layer_time).unwrap_or(0.0);
        let overlap = interpolate_float(&animated.linear_repeat_overlap, layer_time).unwrap_or(0.0);
        let ease_in = interpolate_float(&animated.linear_repeat_ease_in, layer_time).unwrap_or(0.0);
        let ease_out =
            interpolate_float(&animated.linear_repeat_ease_out, layer_time).unwrap_or(0.0);
        let sia = animated.linear_repeat_shape * 100
            + if animated.linear_repeat_invert { 10 } else { 0 }
            + if animated.linear_repeat_color_alt_copies {
                1
            } else {
                0
            };

        // Convert position/offset from pixel_coord space to world units.
        // pixel_coord already accounts for element scale (orig_width = sprite_size * scale).
        // pixel_coord to world: multiply by fit_scale only (no mask_scale multiplication).
        // Y flip: AM Y-down → Bevy Y-up.
        let pos_world_x = pos[0] * fit_scale;
        let pos_world_y = -pos[1] * fit_scale;
        let off_world_x = off[0] * fit_scale;
        let off_world_y = -off[1] * fit_scale;

        material.uniform_data.mask1_lr_params1 = Vec4::new(count, pos_world_x, pos_world_y, angle);
        material.uniform_data.mask1_lr_params2 =
            Vec4::new(off_world_x, off_world_y, lr_scale, alpha);
        material.uniform_data.mask1_lr_params3 = Vec4::new(start, end, phase, overlap);
        material.uniform_data.mask1_lr_params4 = Vec4::new(ease_in, ease_out, 0.0, sia as f32);
        material.uniform_data.mask1_lr_params5 = if animated.linear_repeat_random_order {
            let seed = interpolate_float(&animated.linear_repeat_seed, layer_time).unwrap_or(0.0);
            let (lo, hi) = compute_java_random_state_packed(seed);
            Vec4::new(1.0, lo, hi, 0.0)
        } else {
            Vec4::ZERO
        };
    } else {
        material.uniform_data.mask1_lr_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
    }

    // Process second linear repeat
    if let Some(ref lr2) = animated.linear_repeat2 {
        let count2 = interpolate_float(&lr2.count, layer_time)
            .unwrap_or(0.0)
            .round();
        if count2 > 0.0 {
            let pos2 = interpolate_vec2(&lr2.position, layer_time).unwrap_or([0.0, 0.0]);
            let off2 = interpolate_vec2(&lr2.offset, layer_time).unwrap_or([0.0, 0.0]);
            let angle2 = interpolate_float(&lr2.angle, layer_time).unwrap_or(0.0);
            let scale2 = interpolate_float(&lr2.scale, layer_time).unwrap_or(1.0);
            let alpha2 = interpolate_float(&lr2.alpha, layer_time).unwrap_or(1.0);
            let start2 = interpolate_float(&lr2.start, layer_time).unwrap_or(0.0);
            let end2 = interpolate_float(&lr2.end, layer_time).unwrap_or(1.0);
            let phase2 = interpolate_float(&lr2.phase, layer_time).unwrap_or(0.0);
            let overlap2 = interpolate_float(&lr2.overlap, layer_time).unwrap_or(0.0);
            let ease_in2 = interpolate_float(&lr2.ease_in, layer_time).unwrap_or(0.0);
            let ease_out2 = interpolate_float(&lr2.ease_out, layer_time).unwrap_or(0.0);
            let sia2 = lr2.shape * 100
                + if lr2.invert { 10 } else { 0 }
                + if lr2.color_alt_copies { 1 } else { 0 };

            let pos2_world_x = pos2[0] * fit_scale;
            let pos2_world_y = -pos2[1] * fit_scale;
            let off2_world_x = off2[0] * fit_scale;
            let off2_world_y = -off2[1] * fit_scale;

            material.uniform_data.mask1_lr2_params1 =
                Vec4::new(count2, pos2_world_x, pos2_world_y, angle2);
            material.uniform_data.mask1_lr2_params2 =
                Vec4::new(off2_world_x, off2_world_y, scale2, alpha2);
            material.uniform_data.mask1_lr2_params3 = Vec4::new(start2, end2, phase2, overlap2);
            material.uniform_data.mask1_lr2_params4 =
                Vec4::new(ease_in2, ease_out2, 0.0, sia2 as f32);
            material.uniform_data.mask1_lr2_params5 = if lr2.random_order {
                let seed2 = interpolate_float(&lr2.seed, layer_time).unwrap_or(0.0);
                let (lo2, hi2) = compute_java_random_state_packed(seed2);
                Vec4::new(1.0, lo2, hi2, 0.0)
            } else {
                Vec4::ZERO
            };
        } else {
            material.uniform_data.mask1_lr2_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
        }
    } else {
        material.uniform_data.mask1_lr2_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
    }

    // --- Radial repeat (com.alightcreative.effects.radialrepeat) on mask ---
    let rr_count = interpolate_float(&animated.radial_repeat_count, layer_time)
        .unwrap_or(0.0)
        .round();
    if rr_count > 0.0 {
        let radius = interpolate_float(&animated.radial_repeat_radius, layer_time).unwrap_or(0.0);
        let orientation =
            interpolate_float(&animated.radial_repeat_orientation, layer_time).unwrap_or(0.0);
        let start_angle =
            interpolate_float(&animated.radial_repeat_start_angle, layer_time).unwrap_or(0.0);
        let sweep = interpolate_float(&animated.radial_repeat_sweep, layer_time).unwrap_or(360.0);
        let base_scale =
            interpolate_float(&animated.radial_repeat_base_scale, layer_time).unwrap_or(1.0);
        let offset =
            interpolate_vec2(&animated.radial_repeat_offset, layer_time).unwrap_or([0.0, 0.0]);
        let angle = interpolate_float(&animated.radial_repeat_angle, layer_time).unwrap_or(0.0);
        let rr_scale = interpolate_float(&animated.radial_repeat_scale, layer_time).unwrap_or(1.0);
        let alpha = interpolate_float(&animated.radial_repeat_alpha, layer_time).unwrap_or(1.0);
        let start = interpolate_float(&animated.radial_repeat_start, layer_time).unwrap_or(0.0);
        let end = interpolate_float(&animated.radial_repeat_end, layer_time).unwrap_or(1.0);
        let phase = interpolate_float(&animated.radial_repeat_phase, layer_time).unwrap_or(0.0);
        let overlap = interpolate_float(&animated.radial_repeat_overlap, layer_time).unwrap_or(0.0);
        let ease_in = interpolate_float(&animated.radial_repeat_ease_in, layer_time).unwrap_or(0.0);
        let ease_out =
            interpolate_float(&animated.radial_repeat_ease_out, layer_time).unwrap_or(0.0);

        let sia = animated.radial_repeat_shape * 100
            + if animated.radial_repeat_invert { 10 } else { 0 }
            + if animated.radial_repeat_color_alt_copies {
                1
            } else {
                0
            };

        // Convert offset & radius from pixel_coord space to world units
        let off_world_x = offset[0] * fit_scale;
        let off_world_y = -offset[1] * fit_scale;
        let radius_world = radius * fit_scale;

        material.uniform_data.mask1_rr_params1 =
            Vec4::new(rr_count, radius_world, orientation, start_angle);
        material.uniform_data.mask1_rr_params2 = Vec4::new(sweep, base_scale, angle, rr_scale);
        material.uniform_data.mask1_rr_params3 = Vec4::new(alpha, off_world_x, off_world_y, 0.0);
        material.uniform_data.mask1_rr_params4 = Vec4::new(start, end, phase, overlap);
        material.uniform_data.mask1_rr_params5 = Vec4::new(
            ease_in,
            ease_out,
            sia as f32,
            if animated.radial_repeat_random_order {
                animated.radial_repeat_seed + 0.5
            } else {
                animated.radial_repeat_seed
            },
        );
    } else {
        material.uniform_data.mask1_rr_params1 = Vec4::ZERO;
    }
}

/// Apply embed mask UV mapping from the mask entity's transform.
fn apply_embed_mask_uv(
    mask_entity: Entity,
    mask1: &crate::scene::AmMaskEntry,
    mask_layer_query: &Query<(
        &GlobalTransform,
        &crate::animation::AmAnimated,
        &crate::scene::AmLayerSpec,
    )>,
    fit_scale: f32,
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
) {
    let Ok((mask_gt, _, _)) = mask_layer_query.get(mask_entity) else {
        return;
    };
    let (mask_scale, mask_rot, mask_pos) = mask_gt.to_scale_rotation_translation();
    let mask_rotation = mask_rot.to_euler(bevy::math::EulerRot::ZYX).0;
    let (scene_w, scene_h) = mask1.embed_scene_size.unwrap_or((1280.0, 960.0));
    let half_w = scene_w / 2.0 * fit_scale * mask_scale.x;
    let half_h = scene_h / 2.0 * fit_scale * mask_scale.y;

    bevy::log::warn!(
        "[MASK-DBG] mask pos=({:.1},{:.1}), half=({:.1},{:.1}), rot={:.3}",
        mask_pos.x,
        mask_pos.y,
        half_w,
        half_h,
        mask_rotation
    );

    material.uniform_data.mask_params = Vec4::new(mask_pos.x, mask_pos.y, half_w, half_h);
    material.uniform_data.mask2_flags.y = mask_rotation;
}

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
    embed_rtt_marker_query: Query<(Entity, &AmLayerMarker, &crate::effects::EmbedSceneRtt)>,
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
        let Some(material) = materials.get_mut(&material_handle.0) else {
            continue;
        };

        if active_masks.is_empty() {
            material.uniform_data.effect_flags.x = 0.0;
            material.uniform_data.mask2_flags.x = 0.0;
            material.uniform_data.mask2_flags.y = 0.0;
            material.uniform_data.mask2_flags.z = 0.0;
            material.uniform_data.mask1_lr_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
            material.uniform_data.mask1_lr2_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
            material.uniform_data.mask1_repeat_params1 = Vec4::ZERO;
            material.uniform_data.mask1_rr_params1 = Vec4::ZERO;
            continue;
        }

        bevy::log::warn!(
            "[MASK-DBG] entity has {} active masks, first is_embed={}",
            active_masks.len(),
            active_masks[0].is_embed_mask
        );

        // First mask
        let mask1 = active_masks[0];

        if mask1.is_embed_mask {
            // Texture-based mask (embedScene/group): set type 5.0/6.0 and find RTT texture
            let mask_type = if mask1.is_exclude { 6.0 } else { 5.0 };
            material.uniform_data.effect_flags.x = mask_type;

            // Find the mask embed container entity by layer_id - it must have EmbedSceneRtt
            let rtt_match = embed_rtt_marker_query
                .iter()
                .find(|(_, m, _)| m.id == mask1.mask_layer_id);

            bevy::log::warn!(
                "[MASK-DBG] embed mask: layer_id={}, rtt_match={}",
                mask1.mask_layer_id,
                rtt_match.is_some()
            );

            if let Some((mask_entity, _, rtt)) = rtt_match {
                material.mask_texture = Some(rtt.render_texture.clone());
                bevy::log::warn!("[MASK-DBG] RTT found for mask entity {:?}", mask_entity);

                // Get mask transform for UV mapping
                apply_embed_mask_uv(mask_entity, mask1, &mask_layer_query, fit_scale, material);
            } else {
                // RTT not ready yet - disable mask
                bevy::log::warn!(
                    "[MASK-DBG] RTT NOT found for mask layer_id={}",
                    mask1.mask_layer_id
                );
                material.uniform_data.effect_flags.x = 0.0;
                material.mask_texture = None;
            }
            // Apply repeat effects to texture mask (basic repeat + linear repeat)
            // The shader will sample the RTT texture at multiple offset positions.
            set_mask_repeat_uniforms(
                mask1,
                pending,
                &mask_layer_query,
                playback.current_time_ms,
                fit_scale,
                material,
            );
        } else {
            // SDF-based mask (shape)
            let m1 = compute_mask_params(
                mask1,
                pending,
                &mask_layer_query,
                playback.current_time_ms,
                fit_scale,
            );

            material.uniform_data.effect_flags.x =
                mask_type_flag(mask1.is_circle, mask1.is_exclude);
            material.uniform_data.mask_params =
                bevy::math::Vec4::new(m1.center.x, m1.center.y, m1.half_size.x, m1.half_size.y);
            material.uniform_data.mask_blend =
                bevy::math::Vec4::new(m1.blend.x, m1.blend.y, m1.blend.z, 0.0);
            material.uniform_data.mask2_flags.y = m1.rotation;
            material.uniform_data.mask1_stretch1_params = m1.stretch1;
            material.uniform_data.mask1_stretch2_params = m1.stretch2;
            material.uniform_data.mask1_stretch_info = m1.stretch_info;

            // Extract mask1's repeat data (basic repeat + linear repeat)
            set_mask_repeat_uniforms(
                mask1,
                pending,
                &mask_layer_query,
                playback.current_time_ms,
                fit_scale,
                material,
            );
        } // close SDF mask else block

        // Second mask (if present)
        if active_masks.len() >= 2 {
            let mask2 = active_masks[1];
            let m2 = compute_mask_params(
                mask2,
                pending,
                &mask_layer_query,
                playback.current_time_ms,
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

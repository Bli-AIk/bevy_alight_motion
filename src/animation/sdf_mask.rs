//! # sdf_mask.rs
//!
//! # SDF 蒙版计算模块
//!
//! Functions for computing mask parameters and repeat effects on SDF shapes.
//! 用于计算 SDF 形状蒙版参数和重复效果的函数。

use bevy::prelude::*;

use crate::sdf_material::SdfMaterial;

use super::components::AmAnimated;
use super::interpolation::{interpolate_float, interpolate_vec2};

/// Compute mask parameters from a mask entry using the mask layer's current animated state.
/// Returns (center, half_size, rotation_rad, blend_params).
/// blend_params = Vec3(fill_alpha, opacity, stroke_width_world).
///
/// For child masks (mask_parent_layer_id != 0), the parent's animated scale is looked up
/// at runtime because SDF parents use Transform.scale=(1,1) and don't propagate scale
/// through the Bevy hierarchy.
pub(crate) fn compute_sdf_mask_params(
    mask: &crate::scene::AmMaskEntry,
    pending: &crate::scene::AmPendingLayers,
    mask_layer_query: &Query<(&GlobalTransform, &AmAnimated, &crate::scene::AmLayerSpec)>,
    playback_time: f32,
    fit_scale: f32,
) -> (Vec2, Vec2, f32, Vec3) {
    let Some(&mask_entity) = pending.spawned_entities.get(&mask.mask_layer_id) else {
        return (
            mask.center * fit_scale,
            Vec2::new(mask.half_size.x.abs(), mask.half_size.y.abs()) * fit_scale,
            mask.rotation,
            Vec3::new(1.0, 1.0, 0.0),
        );
    };
    let Ok((_global_transform, mask_animated, spec)) = mask_layer_query.get(mask_entity) else {
        return (
            mask.center * fit_scale,
            Vec2::new(mask.half_size.x.abs(), mask.half_size.y.abs()) * fit_scale,
            mask.rotation,
            Vec3::new(1.0, 1.0, 0.0),
        );
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

    let local_time = mask_animated.calc_local_time(playback_time);
    let layer_time = mask_animated.calc_layer_time(local_time);
    let mask_opacity = interpolate_float(&mask_animated.opacity, layer_time).unwrap_or(1.0);
    let current_sw =
        interpolate_float(&mask_animated.stroke_width, layer_time).unwrap_or(initial_sw);
    let rotation_deg = interpolate_float(&mask_animated.rotation, layer_time).unwrap_or(0.0);
    let rotation_rad = (-rotation_deg).to_radians();
    let [scale_x, scale_y] =
        interpolate_vec2(&mask_animated.scale, layer_time).unwrap_or([1.0, 1.0]);
    let [anim_size_x, anim_size_y] =
        interpolate_vec2(&mask_animated.size, layer_time).unwrap_or([base_width, base_height]);
    let mask_parent_scale = if mask.mask_parent_layer_id != 0 {
        pending
            .spawned_entities
            .get(&mask.mask_parent_layer_id)
            .and_then(|&pe| mask_layer_query.get(pe).ok())
            .map(|(_, pa, _)| {
                let plt = pa.calc_local_time(playback_time);
                let pltime = pa.calc_layer_time(plt);
                let [psx, psy] = interpolate_vec2(&pa.scale, pltime).unwrap_or([1.0, 1.0]);
                Vec2::new(psx, psy)
            })
            .unwrap_or(Vec2::ONE)
    } else {
        Vec2::ONE
    };

    let (center_x, center_y) = if mask.mask_parent_layer_id != 0 {
        let mask_pos = _global_transform.translation().truncate();

        // Child SDF mask entities already bake the AM parent scale into their animated
        // world-space pivot position. Re-scaling the parent-relative offset here shifts the
        // mask a second time and can move the clip window away from the intended content.
        let corrected_pos = mask_pos;

        let scaled_offset_x = -pivot_x * scale_x * mask_parent_scale.x * fit_scale;
        let scaled_offset_y = pivot_y * scale_y * mask_parent_scale.y * fit_scale;
        let rotated_offset_x =
            scaled_offset_x * rotation_rad.cos() - scaled_offset_y * rotation_rad.sin();
        let rotated_offset_y =
            scaled_offset_x * rotation_rad.sin() + scaled_offset_y * rotation_rad.cos();

        (
            corrected_pos.x + rotated_offset_x,
            corrected_pos.y + rotated_offset_y,
        )
    } else {
        let mask_translation = _global_transform.translation();
        let mask_global_scale = _global_transform.to_scale_rotation_translation().0;
        let scaled_offset_x = -pivot_x * scale_x * mask_global_scale.x;
        let scaled_offset_y = pivot_y * scale_y * mask_global_scale.y;
        let rotated_offset_x =
            scaled_offset_x * rotation_rad.cos() - scaled_offset_y * rotation_rad.sin();
        let rotated_offset_y =
            scaled_offset_x * rotation_rad.sin() + scaled_offset_y * rotation_rad.cos();
        (
            mask_translation.x + rotated_offset_x,
            mask_translation.y + rotated_offset_y,
        )
    };

    // Compute mask half_size
    let ext = |sw: f32| match stroke_dir {
        "inside" => 0.0,
        "outside" => sw,
        _ => sw * 0.5,
    };
    let current_stroke_ext = ext(current_sw);
    let parent_abs_scale = Vec2::new(mask_parent_scale.x.abs(), mask_parent_scale.y.abs());
    let geom_half_w = if mask.mask_parent_layer_id != 0 {
        anim_size_x / 2.0 * scale_x * parent_abs_scale.x
    } else {
        anim_size_x / 2.0 * scale_x
    };
    let geom_half_h = if mask.mask_parent_layer_id != 0 {
        anim_size_y / 2.0 * scale_y * parent_abs_scale.y
    } else {
        anim_size_y / 2.0 * scale_y
    };
    let half_width = ((geom_half_w + current_stroke_ext) * fit_scale).abs();
    let half_height = ((geom_half_h + current_stroke_ext) * fit_scale).abs();
    let sw_world = current_sw * fit_scale;

    bevy::log::debug!(
        "[MaskDebug] mask_layer_id={}, center=({:.1},{:.1}), half=({:.1},{:.1}), fill_alpha={:.2}, opacity={:.2}, sw={:.1}",
        mask.mask_layer_id,
        center_x,
        center_y,
        half_width,
        half_height,
        fill_alpha,
        mask_opacity,
        sw_world,
    );

    (
        Vec2::new(center_x, center_y),
        Vec2::new(half_width, half_height),
        rotation_rad,
        Vec3::new(fill_alpha, mask_opacity, sw_world),
    )
}

/// Apply radial repeat effect params from a mask layer to an SDF material.
pub(crate) fn apply_sdf_mask_radial_repeat(
    mask: &crate::scene::AmMaskEntry,
    pending: &crate::scene::AmPendingLayers,
    mask_layer_query: &Query<(&GlobalTransform, &AmAnimated, &crate::scene::AmLayerSpec)>,
    playback_time: f32,
    fit_scale: f32,
    material: &mut SdfMaterial,
) {
    let Some(&mask_entity) = pending.spawned_entities.get(&mask.mask_layer_id) else {
        material.uniform_data.mask1_rr_params1 = Vec4::ZERO;
        return;
    };
    let Ok((_, animated, _)) = mask_layer_query.get(mask_entity) else {
        material.uniform_data.mask1_rr_params1 = Vec4::ZERO;
        return;
    };

    let local_time = animated.calc_local_time(playback_time);
    let layer_time = animated.calc_layer_time(local_time);

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

/// Apply linear repeat effect params from a mask layer to an SDF material.
pub(crate) fn apply_sdf_mask_linear_repeat(
    mask: &crate::scene::AmMaskEntry,
    pending: &crate::scene::AmPendingLayers,
    mask_layer_query: &Query<(&GlobalTransform, &AmAnimated, &crate::scene::AmLayerSpec)>,
    playback_time: f32,
    fit_scale: f32,
    material: &mut SdfMaterial,
) {
    let Some(&mask_entity) = pending.spawned_entities.get(&mask.mask_layer_id) else {
        material.uniform_data.mask1_lr_params1 = Vec4::ZERO;
        return;
    };
    let Ok((_, animated, _)) = mask_layer_query.get(mask_entity) else {
        material.uniform_data.mask1_lr_params1 = Vec4::ZERO;
        return;
    };

    let local_time = animated.calc_local_time(playback_time);
    let layer_time = animated.calc_layer_time(local_time);
    let parent_repeat_scale = if mask.mask_parent_layer_id != 0 {
        pending
            .spawned_entities
            .get(&mask.mask_parent_layer_id)
            .and_then(|&parent_entity| mask_layer_query.get(parent_entity).ok())
            .map(|(_, parent_animated, _)| {
                let parent_local_time = parent_animated.calc_local_time(playback_time);
                let parent_layer_time = parent_animated.calc_layer_time(parent_local_time);
                interpolate_vec2(&parent_animated.scale, parent_layer_time).unwrap_or([1.0, 1.0])
            })
            .unwrap_or([1.0, 1.0])
    } else {
        [1.0, 1.0]
    };
    let repeat_scale_x = parent_repeat_scale[0].abs();
    let repeat_scale_y = parent_repeat_scale[1].abs();
    let lr_count = interpolate_float(&animated.linear_repeat_count, layer_time)
        .unwrap_or(0.0)
        .round();
    if lr_count > 0.0 {
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

        let pos_world_x = pos[0] * fit_scale * repeat_scale_x;
        let pos_world_y = -pos[1] * fit_scale * repeat_scale_y;
        let off_world_x = off[0] * fit_scale * repeat_scale_x;
        let off_world_y = -off[1] * fit_scale * repeat_scale_y;

        material.uniform_data.mask1_lr_params1 =
            Vec4::new(lr_count, pos_world_x, pos_world_y, angle);
        material.uniform_data.mask1_lr_params2 =
            Vec4::new(off_world_x, off_world_y, lr_scale, alpha);
        material.uniform_data.mask1_lr_params3 = Vec4::new(start, end, phase, overlap);
        material.uniform_data.mask1_lr_params4 = Vec4::new(ease_in, ease_out, 0.0, sia as f32);

        material.uniform_data.mask1_lr_params5 = if animated.linear_repeat_random_order {
            let seed = interpolate_float(&animated.linear_repeat_seed, layer_time).unwrap_or(0.0);
            let (lo, hi) =
                crate::animation::effects::repeat::compute_java_random_state_packed(seed);
            Vec4::new(1.0, lo, hi, 0.0)
        } else {
            Vec4::ZERO
        };
    } else {
        material.uniform_data.mask1_lr_params1 = Vec4::ZERO;
    }
}

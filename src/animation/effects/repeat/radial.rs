//! Evaluates the radial-repeat effect for unified visuals.
//! It computes circular copy placement, scale, alpha, color, and timing values
//! from the animated parameters and writes the result into shader uniforms.
//!
//! 负责为统一材质视觉对象求值 radial repeat 效果。它根据动画参数计算环形
//! 副本的排布、缩放、透明度、颜色和时序，并把结果写入 shader uniform。

use bevy::prelude::*;

use crate::animation::components::AmAnimated;
use crate::animation::interpolation::{interpolate_color, interpolate_float, interpolate_vec2};

pub(crate) fn process_radial_repeat_effect(
    animated: &AmAnimated,
    layer_time: f32,
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    orig_width: f32,
    orig_height: f32,
    mesh2d: &bevy::mesh::Mesh2d,
    meshes: &mut Assets<Mesh>,
) {
    let has_radial_repeat = animated.radial_repeat_count.value.is_some_and(|v| v > 0.0)
        || animated
            .radial_repeat_count
            .keyframes
            .iter()
            .any(|kf| kf.value.parse::<f32>().unwrap_or(0.0) > 0.0);
    if has_radial_repeat {
        let count = interpolate_float(&animated.radial_repeat_count, layer_time).unwrap_or(0.0);
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
        let scale = interpolate_float(&animated.radial_repeat_scale, layer_time).unwrap_or(1.0);
        let alpha = interpolate_float(&animated.radial_repeat_alpha, layer_time).unwrap_or(1.0);
        let fill_color_srgb = interpolate_color(&animated.radial_repeat_fill_color, layer_time)
            .unwrap_or(Vec4::new(1.0, 1.0, 1.0, 1.0));
        let fill_color = Vec4::new(
            fill_color_srgb.x.powf(2.2),
            fill_color_srgb.y.powf(2.2),
            fill_color_srgb.z.powf(2.2),
            fill_color_srgb.w,
        );
        let blend = interpolate_float(&animated.radial_repeat_blend, layer_time).unwrap_or(0.0);
        let start = interpolate_float(&animated.radial_repeat_start, layer_time).unwrap_or(0.0);
        let end = interpolate_float(&animated.radial_repeat_end, layer_time).unwrap_or(1.0);
        let phase = interpolate_float(&animated.radial_repeat_phase, layer_time).unwrap_or(0.0);
        let ease_in = interpolate_float(&animated.radial_repeat_ease_in, layer_time).unwrap_or(0.0);
        let ease_out =
            interpolate_float(&animated.radial_repeat_ease_out, layer_time).unwrap_or(0.0);
        let overlap = interpolate_float(&animated.radial_repeat_overlap, layer_time).unwrap_or(0.0);

        let shape_invert_alt = animated.radial_repeat_shape * 100
            + if animated.radial_repeat_invert { 10 } else { 0 }
            + if animated.radial_repeat_color_alt_copies {
                1
            } else {
                0
            };

        let count_for_shader = if count.round() <= 0.0 { -1.0 } else { count };

        material.uniform_data.radial_repeat_params1 =
            Vec4::new(count_for_shader, radius, orientation, start_angle);
        material.uniform_data.radial_repeat_params2 = Vec4::new(sweep, base_scale, angle, scale);
        material.uniform_data.radial_repeat_params3 = Vec4::new(alpha, offset[0], offset[1], blend);
        material.uniform_data.radial_repeat_params4 = Vec4::new(start, end, phase, overlap);
        material.uniform_data.radial_repeat_params5 = Vec4::new(
            ease_in,
            ease_out,
            shape_invert_alt as f32,
            if animated.radial_repeat_random_order {
                animated.radial_repeat_seed + 0.5
            } else {
                animated.radial_repeat_seed
            },
        );
        material.uniform_data.radial_repeat_fill_color = fill_color;

        // Element pivot for rotation correction: AM's `rotatedBy` applies
        // the spread rotation around the element's pivot, so the inverse
        // transform must account for the pivot offset.
        let pivot = interpolate_vec2(&animated.pivot, layer_time).unwrap_or([0.0, 0.0]);
        material.uniform_data.radial_repeat_params6 = Vec4::new(pivot[0], pivot[1], 0.0, 0.0);

        let pivot_mag = (pivot[0].powi(2) + pivot[1].powi(2)).sqrt();
        let max_mix = scale.abs().max(1.0);
        let visual_scale = (max_mix * base_scale).abs().max(1.0);
        let max_extent = radius.abs() * max_mix
            + orig_width.max(orig_height) / 2.0 * visual_scale
            + offset[0].abs()
            + offset[1].abs()
            + pivot_mag;
        let padding = 20.0;
        let min_x = -max_extent - padding;
        let max_x = max_extent + padding;
        let min_y = -max_extent - padding;
        let max_y = max_extent + padding;

        let new_width = max_x - min_x;
        let new_height = max_y - min_y;
        material.uniform_data.original_size =
            Vec4::new(orig_width, orig_height, new_width, new_height);

        let uv_min_x = min_x / orig_width + 0.5;
        let uv_max_x = max_x / orig_width + 0.5;
        let uv_at_bottom = 0.5 - min_y / orig_height;
        let uv_at_top = 0.5 - max_y / orig_height;

        let vertices = vec![
            [min_x, min_y, 0.0],
            [max_x, min_y, 0.0],
            [max_x, max_y, 0.0],
            [min_x, max_y, 0.0],
        ];
        let uvs = vec![
            [uv_min_x, uv_at_bottom],
            [uv_max_x, uv_at_bottom],
            [uv_max_x, uv_at_top],
            [uv_min_x, uv_at_top],
        ];
        let indices = vec![0u32, 1, 2, 0, 2, 3];

        super::overwrite_repeat_mesh(meshes, mesh2d, vertices, uvs, indices);
    } else {
        material.uniform_data.radial_repeat_params1 = Vec4::ZERO;
        material.uniform_data.radial_repeat_params2 = Vec4::new(360.0, 1.0, 0.0, 1.0);
        material.uniform_data.radial_repeat_params3 = Vec4::new(1.0, 0.0, 0.0, 0.0);
        material.uniform_data.radial_repeat_params4 = Vec4::new(0.0, 1.0, 0.0, 0.0);
        material.uniform_data.radial_repeat_params5 = Vec4::ZERO;
        material.uniform_data.radial_repeat_params6 = Vec4::ZERO;
        material.uniform_data.radial_repeat_fill_color = Vec4::new(1.0, 1.0, 1.0, 1.0);
    }
}

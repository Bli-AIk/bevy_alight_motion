//! Implements the basic repeat effect for unified visuals.
//! Compared with the linear and radial variants, this path focuses on the simple
//! accumulated offset/rotation/scale repeat that Alight Motion applies to many
//! layers by default.
//!
//! 实现统一材质视觉对象的基础 repeat 效果。和 linear、radial 变体相比，
//! 这里主要处理 Alight Motion 默认那种简单的累计位移、旋转和缩放重复逻辑。

use bevy::prelude::*;

use crate::animation::components::AmAnimated;
use crate::animation::interpolation::{interpolate_float, interpolate_vec2};

pub(crate) fn process_repeat_effect(
    animated: &AmAnimated,
    layer_time: f32,
    uniform: &mut crate::masked_sprite::UnifiedEffectUniform,
    orig_width: f32,
    orig_height: f32,
    mesh2d: &bevy::mesh::Mesh2d,
    meshes: &mut Assets<Mesh>,
) {
    let has_repeat = animated.repeat_count.value.is_some_and(|v| v > 0.0)
        || animated
            .repeat_count
            .keyframes
            .iter()
            .any(|kf| kf.value.parse::<f32>().unwrap_or(0.0) > 0.0);
    if has_repeat {
        let count = interpolate_float(&animated.repeat_count, layer_time).unwrap_or(0.0);
        let offset = interpolate_vec2(&animated.repeat_offset, layer_time).unwrap_or([0.0, 0.0]);
        let angle = interpolate_float(&animated.repeat_angle, layer_time).unwrap_or(0.0);
        let repeat_scale = interpolate_float(&animated.repeat_scale, layer_time).unwrap_or(1.0);
        let alpha = interpolate_float(&animated.repeat_alpha, layer_time).unwrap_or(1.0);

        bevy::log::debug!(
            "[RepeatEffect] layer={} time={:.2} count={:.1} offset=({:.1},{:.1}) angle={:.1} scale={:.2} alpha={:.2}",
            animated.layer_id,
            layer_time,
            count,
            offset[0],
            offset[1],
            angle,
            repeat_scale,
            alpha
        );

        uniform.repeat_params1 = Vec4::new(count, offset[0], offset[1], angle);
        uniform.repeat_params2 = Vec4::new(repeat_scale, alpha, 0.0, 0.0);

        let n = (count.floor() as i32 - 1).max(0);
        let angle_rad = angle.to_radians();
        let mut min_x = -orig_width / 2.0;
        let mut max_x = orig_width / 2.0;
        let mut min_y = -orig_height / 2.0;
        let mut max_y = orig_height / 2.0;

        for i in 0..=n {
            let fi = i as f32;
            let cum_alpha = 1.0 - fi * (1.0 - alpha);
            if cum_alpha <= 0.0 {
                break;
            }
            let cum_offset_x = offset[0] * fi;
            let cum_offset_y = -offset[1] * fi;
            let cum_scale = repeat_scale.powf(fi);
            let cum_angle = angle_rad * fi;
            let half_w = orig_width / 2.0 * cum_scale;
            let half_h = orig_height / 2.0 * cum_scale;
            let corners = [
                (-half_w, -half_h),
                (half_w, -half_h),
                (half_w, half_h),
                (-half_w, half_h),
            ];

            let cos_a = cum_angle.cos();
            let sin_a = cum_angle.sin();
            for (cx, cy) in corners {
                let rx = cx * cos_a - cy * sin_a + cum_offset_x;
                let ry = cx * sin_a + cy * cos_a + cum_offset_y;
                min_x = min_x.min(rx);
                max_x = max_x.max(rx);
                min_y = min_y.min(ry);
                max_y = max_y.max(ry);
            }
        }

        let padding = 10.0;
        min_x -= padding;
        max_x += padding;
        min_y -= padding;
        max_y += padding;

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
        uniform.repeat_params1 = Vec4::ZERO;
        uniform.repeat_params2 = Vec4::new(1.0, 1.0, 0.0, 0.0);
    }
}

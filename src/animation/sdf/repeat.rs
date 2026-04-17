//! Applies repeat-effect uniforms to SDF materials at runtime.
//!
//! 在运行时把重复效果参数写入 SDF 材质。
//!
//! Linear repeat and radial repeat are represented as animated fields on `AmAnimated`, but the GPU
//! shaders expect flattened parameter vectors. The module reads the active repeat keyframes for each
//! parent SDF layer and encodes them into the material uniform blocks consumed by the fragment
//! shader.
//!
//! 线性重复和径向重复在 `AmAnimated` 里以动画字段表示，但 GPU shader 需要的是展平后的参数向量。
//! 该模块会读取每个父级 SDF 图层当前生效的重复关键帧，并把它们编码进片元 shader 消费的
//! 材质 uniform 块。

use bevy::prelude::*;

use crate::sdf_material::SdfMaterial;

use super::super::components::{AmAnimated, AmPlayback, AmSdfShapeParent};
use super::super::interpolation::{interpolate_float, interpolate_vec2};

pub fn animate_sdf_repeat_system(
    playback: Res<AmPlayback>,
    parent_query: Query<
        (&AmAnimated, &Children),
        (With<AmSdfShapeParent>, Without<crate::scene::AmHibernated>),
    >,
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

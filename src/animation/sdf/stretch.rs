//! Computes stretch-effect uniforms for SDF parents.
//!
//! 计算 SDF 父级图层的拉伸效果 uniform。
//!
//! Stretch on SDF-backed layers depends on the authored angle, smoothness, offsets, and the live
//! world-space size of the parent shape. The module translates those animated properties into the
//! normalized parameters expected by the SDF material so stretch is rendered in shader space rather
//! than by deforming geometry on the CPU.
//!
//! 基于 SDF 的图层拉伸效果既依赖作者设置的角度、平滑度、偏移，也依赖父级形状当前的世界空间尺寸。
//! 该模块会把这些动画属性换算成 SDF 材质需要的归一化参数，让拉伸在 shader 空间里完成，而不是
//! 通过 CPU 直接改几何体。

use bevy::prelude::*;

use crate::sdf_material::SdfMaterial;

use super::super::components::{AmAnimated, AmPlayback, AmSdfShapeParent};
use super::super::interpolation::interpolate_float;
use super::super::sdf_geometry::compute_sdf_shape_half_extent;

pub fn animate_sdf_stretch_system(
    playback: Res<AmPlayback>,
    parent_query: Query<
        (&AmAnimated, &Children, &GlobalTransform),
        (With<AmSdfShapeParent>, Without<crate::scene::AmHibernated>),
    >,
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

        let scene_width = animated.canvas_width;
        let scene_height = animated.canvas_height;

        let adj_stretch = stretch_raw / 500.0;
        let offset_norm = offset_raw / 1000.0;

        let (_, quat, _) = global_transform.to_scale_rotation_translation();
        let transform_rot = quat.to_euler(bevy::math::EulerRot::ZYX).0;

        let stretch_params = Vec4::new(angle_rad, adj_stretch, offset_norm, smooth_raw);
        let stretch_meta = Vec4::new(transform_rot, 0.0, scene_width, scene_height);

        let cos_a = angle_rad.cos().abs();
        let sin_a = angle_rad.sin().abs();
        let dx_screen = cos_a * adj_stretch * scene_width;
        let dy_screen = sin_a * adj_stretch * scene_height;
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

            let base_half = compute_sdf_shape_half_extent(&material.uniform_data)
                + material.uniform_data.params.z.abs() * 2.0;
            let needed = base_half + extra;
            if needed > material.uniform_data.frame_half {
                material.uniform_data.frame_half = needed;
            }
        }
    }
}

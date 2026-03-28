//! Builds the initial transform payload for spawned runtime layers.
//! 为运行时新生成的图层构建初始变换载荷。
//!
//! Scene collection produces `PendingLayer` records, but spawning still has to resolve anchor
//! offsets, fit-scale compensation, initial animated values, and effect-derived flags into a single
//! `Transform + AmAnimated` bundle. This file performs that translation so later spawn steps can
//! attach visuals without having to recompute the authored transform state.
//! scene 收集阶段只会生成 `PendingLayer`，真正生成实体时仍要把锚点偏移、fit-scale 补偿、初始动画值
//! 和效果派生标记统一折算成一组 `Transform + AmAnimated`。这个文件负责这一步转换，后续的 spawn
//! 阶段就可以直接附加可视对象，而不用重复计算作者定义的初始变换状态。

use bevy::prelude::*;

use crate::scene::PendingLayer;

use super::super::interpolation::{
    interpolate_float, interpolate_vec2, interpolate_vec3_with_extrapolation,
};

fn embed_like_pivot_compensation(
    pivot_x: f32,
    pivot_y: f32,
    scale: [f32; 2],
    rotation_deg_bevy: f32,
    has_parent: bool,
) -> (f32, f32) {
    let pivot_bevy_y = if has_parent { pivot_y } else { -pivot_y };
    let scaled_offset_x = -pivot_x * scale[0];
    let scaled_offset_y = -pivot_bevy_y * scale[1];
    let rotation_rad = (-rotation_deg_bevy).to_radians();
    let rotated_offset_x =
        scaled_offset_x * rotation_rad.cos() - scaled_offset_y * rotation_rad.sin();
    let rotated_offset_y =
        scaled_offset_x * rotation_rad.sin() + scaled_offset_y * rotation_rad.cos();

    (pivot_x + rotated_offset_x, pivot_bevy_y + rotated_offset_y)
}

pub(super) struct SpawnSetup {
    pub(super) layer_time: f32,
    pub(super) transform: Transform,
    pub(super) animated: crate::animation::components::AmAnimated,
}

pub(super) fn build_spawn_setup(
    layer: &PendingLayer,
    global_time: f32,
    inv_fit_scale: f32,
    embed_owner_id: u64,
) -> SpawnSetup {
    let has_wipe = layer.animated.wipe_end.value != Some(1.0)
        || !layer.animated.wipe_end.keyframes.is_empty()
        || layer.animated.wipe_start.value.is_some()
        || !layer.animated.wipe_start.keyframes.is_empty();

    let has_stretch = layer.animated.stretch_amount.value.is_some()
        || !layer.animated.stretch_amount.keyframes.is_empty()
        || layer.animated.stretch_angle.value.is_some()
        || !layer.animated.stretch_angle.keyframes.is_empty()
        || layer.animated.stretch_offset.value.is_some()
        || !layer.animated.stretch_offset.keyframes.is_empty()
        || layer.animated.stretch_smooth.value.is_some()
        || !layer.animated.stretch_smooth.keyframes.is_empty()
        || layer.animated.stretch_seg2_amount.value.is_some()
        || !layer.animated.stretch_seg2_amount.keyframes.is_empty()
        || layer.animated.stretch_seg2_angle.value.is_some()
        || !layer.animated.stretch_seg2_angle.keyframes.is_empty();

    let has_blur = layer.animated.blur_strength.value.is_some()
        || !layer.animated.blur_strength.keyframes.is_empty();

    let has_mask = layer.mask_info.is_some();
    let has_stretch2 = layer.animated.stretch2_scale.value.is_some()
        || !layer.animated.stretch2_scale.keyframes.is_empty();
    let needs_effect = has_wipe || has_stretch || has_mask || has_blur || has_stretch2;

    let animated = &layer.animated;
    let local_time = animated.calc_local_time(global_time);

    bevy::log::trace!(
        "[SpawnTime] '{}' global_time={:.1}, local_time={:.1}, start_time={}, end_time={}, time_offset={:.1}, speed={:.2}",
        layer.label,
        global_time,
        local_time,
        layer.start_time,
        layer.end_time,
        animated.time_offset,
        animated.speed_multiplier
    );

    let layer_time = animated.calc_layer_time(local_time);

    let actual_scale = interpolate_vec2(&animated.scale, layer_time).unwrap_or([1.0, 1.0]);
    let current_scale =
        if matches!(layer.spec, crate::scene::AmLayerSpec::SdfShape { .. }) || needs_effect {
            [1.0_f32, 1.0_f32]
        } else {
            actual_scale
        };

    let initial_position =
        if let Some(loc) = interpolate_vec3_with_extrapolation(&animated.location, layer_time) {
            let (mut bx, mut by) = if animated.has_parent {
                (loc[0], -loc[1])
            } else {
                (
                    loc[0] - animated.canvas_width / 2.0,
                    animated.canvas_height / 2.0 - loc[1],
                )
            };

            if let Some(pivot) = interpolate_vec2(&animated.pivot, layer_time) {
                let pivot_x = pivot[0];
                let pivot_y = pivot[1];

                if matches!(layer.spec, crate::scene::AmLayerSpec::SdfShape { .. }) {
                    bx += pivot_x;
                    by -= pivot_y;
                } else if matches!(
                    layer.spec,
                    crate::scene::AmLayerSpec::EmbedScene | crate::scene::AmLayerSpec::Null
                ) {
                    let authored_rotation_deg =
                        interpolate_float(&animated.rotation, layer_time).unwrap_or(0.0);
                    let bevy_rotation_deg =
                        -authored_rotation_deg + animated.repeat_rotation_offset_deg;
                    let (comp_x, comp_y) = embed_like_pivot_compensation(
                        pivot_x,
                        pivot_y,
                        current_scale,
                        bevy_rotation_deg,
                        animated.has_parent,
                    );
                    bx += comp_x;
                    by += comp_y;
                }
            }

            if let Some(mut effect_x) = interpolate_float(&animated.effect_pos_x, layer_time) {
                if animated.effect_xinv {
                    effect_x = -effect_x;
                }
                bx += effect_x;
            }
            if let Some(mut effect_y) = interpolate_float(&animated.effect_pos_y, layer_time) {
                if animated.effect_yinv {
                    effect_y = -effect_y;
                }
                by -= effect_y;
            }
            for extra in &animated.extra_transform2 {
                let ex = interpolate_float(&extra.pos_x, layer_time).unwrap_or(0.0);
                bx += if extra.xinv { -ex } else { ex };
                let ey = interpolate_float(&extra.pos_y, layer_time).unwrap_or(0.0);
                by -= if extra.yinv { -ey } else { ey };
            }

            if !animated.has_parent {
                by -= animated.font_y_offset;
            }

            if !matches!(layer.spec, crate::scene::AmLayerSpec::SdfShape { .. }) {
                bx += animated.anchor_offset.x;
                by += animated.anchor_offset.y;
            }

            bx += animated.repeat_position_offset.x;
            by += animated.repeat_position_offset.y;

            Vec3::new(bx, by, layer.transform.translation.z)
        } else {
            layer.transform.translation
        };

    let initial_rotation = if let Some(rot_deg) = interpolate_float(&animated.rotation, layer_time)
    {
        let total_deg = -rot_deg + animated.repeat_rotation_offset_deg;
        Quat::from_rotation_z(total_deg.to_radians())
    } else {
        layer.transform.rotation
    };

    let rsf = animated.repeat_scale_factor;
    let initial_scale =
        if needs_effect || matches!(layer.spec, crate::scene::AmLayerSpec::SdfShape { .. }) {
            Vec3::new(actual_scale[0].signum(), actual_scale[1].signum(), 1.0)
        } else {
            Vec3::new(current_scale[0] * rsf, current_scale[1] * rsf, 1.0)
        };

    bevy::log::debug!(
        "[SpawnInit] '{}' layer_time={:.4}, pos=({:.1},{:.1},{:.4}), rot={:.2}°, scale=({:.3},{:.3})",
        layer.label,
        layer_time,
        initial_position.x,
        initial_position.y,
        initial_position.z,
        initial_rotation
            .to_euler(bevy::math::EulerRot::ZYX)
            .0
            .to_degrees(),
        initial_scale.x,
        initial_scale.y
    );

    let mut animated = layer.animated.clone();
    animated.blend_mode = layer.blending_mode;
    if animated.scale_assist_axis != 0 {
        bevy::log::info!(
            "[SPAWN] Layer '{}' has scale_assist_axis={}, keyframes={}",
            layer.label,
            animated.scale_assist_axis,
            animated.scale_assist.keyframes.len()
        );
    }
    if embed_owner_id != 0 {
        animated.inv_fit_scale = inv_fit_scale;
    }

    SpawnSetup {
        layer_time,
        transform: Transform {
            translation: initial_position,
            rotation: initial_rotation,
            scale: initial_scale,
        },
        animated,
    }
}

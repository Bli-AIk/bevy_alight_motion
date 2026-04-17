//! Resolves animated scale for SDF parents and propagates it to their children.
//! 解析 SDF 父级图层的缩放动画，并把结果传播给子对象。
//!
//! SDF visuals split their transform between the entity transform and material-backed shape data.
//! This module computes the effective scale after parenting, repeat, and fit-scale adjustments, then
//! updates both transforms and shader uniforms so the rendered shape size matches Alight Motion's
//! authored result.
//! SDF 可视对象会把变换拆分到实体 `Transform` 和材质中的形状数据上。这个模块负责在父子继承、
//! 重复效果和 fit-scale 补偿之后求出最终缩放，并同步更新变换与 shader uniform，确保渲染出的
//! 形状尺寸与 Alight Motion 原工程一致。

use bevy::prelude::*;
use std::collections::HashMap;

use crate::sdf_material::SdfMaterial;

use super::super::components::{AmAnimated, AmPlayback, AmSdfParams, AmSdfShapeParent};
use super::super::interpolation::{interpolate_float, interpolate_vec2};
use super::super::sdf_helpers::{
    accumulate_parent_scale, compute_sdf_own_scale, update_sdf_child_material,
};

pub fn animate_sdf_scale_system(
    playback: Res<AmPlayback>,
    parent_query: Query<
        (&AmAnimated, &Children),
        (With<AmSdfShapeParent>, Without<crate::scene::AmHibernated>),
    >,
    mut sdf_query: Query<(&MeshMaterial2d<SdfMaterial>, &AmSdfParams, &mut Transform)>,
    mut materials: ResMut<Assets<SdfMaterial>>,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;
    let mut scale_map: HashMap<u64, [f32; 2]> = HashMap::new();
    let mut parent_map: HashMap<u64, u64> = HashMap::new();

    // Cache env var once per frame instead of per SDF child
    let disable_ancestor_pivot_comp =
        std::env::var_os("AM_DISABLE_SDF_ANCESTOR_PIVOT_COMP").is_some();

    for (animated, _children) in parent_query.iter() {
        let local_time = animated.calc_local_time(global_time);
        if !animated.is_active(local_time) {
            continue;
        }
        let layer_time = animated.calc_layer_time(local_time);
        let anim_scale = compute_sdf_own_scale(animated, layer_time, global_time);
        scale_map.insert(animated.layer_id, anim_scale);
        if animated.has_parent && animated.parent_layer_id != 0 {
            parent_map.insert(animated.layer_id, animated.parent_layer_id);
        }
    }

    for (animated, children) in parent_query.iter() {
        let local_time = animated.calc_local_time(global_time);
        if !animated.is_active(local_time) {
            continue;
        }
        let layer_time = animated.calc_layer_time(local_time);

        let own_scale = scale_map
            .get(&animated.layer_id)
            .copied()
            .unwrap_or([1.0, 1.0]);
        let parent_scale = accumulate_parent_scale(animated.layer_id, &parent_map, &scale_map);
        let combined_scale = [
            own_scale[0] * parent_scale[0],
            own_scale[1] * parent_scale[1],
        ];

        let stroke_width_animated = if !animated.stroke_width.keyframes.is_empty() {
            interpolate_float(&animated.stroke_width, layer_time).unwrap_or(0.0)
        } else {
            -1.0
        };

        let mut shape_extra_anim = [0.0f32; 4];
        let mut has_shape_anim = false;
        for (i, prop) in animated.shape_props.iter().enumerate() {
            if !prop.keyframes.is_empty() {
                has_shape_anim = true;
                shape_extra_anim[i] = interpolate_float(prop, layer_time).unwrap_or(0.0);
            } else if let Some(v) = prop.value {
                shape_extra_anim[i] = v;
            }
        }

        let mut shape_pts_anim = [[0.0f32; 2]; 5];
        let mut has_pts_anim = false;
        for (i, pt) in animated.shape_points.iter().enumerate() {
            if !pt.keyframes.is_empty() {
                has_pts_anim = true;
                shape_pts_anim[i] = interpolate_vec2(pt, layer_time).unwrap_or([0.0, 0.0]);
            } else if let Some(v) = pt.value {
                shape_pts_anim[i] = v;
            }
        }

        for child in children.iter() {
            let Ok((material_handle, sdf_params, mut transform)) = sdf_query.get_mut(child) else {
                continue;
            };
            let Some(material) = materials.get_mut(&material_handle.0) else {
                continue;
            };

            update_sdf_child_material(
                &mut material.uniform_data,
                sdf_params,
                &mut transform,
                own_scale,
                combined_scale,
                stroke_width_animated,
                has_shape_anim,
                &shape_extra_anim,
                has_pts_anim,
                &shape_pts_anim,
                disable_ancestor_pivot_comp,
            );
        }
    }
}

pub fn compensate_sdf_parent_scale_system(
    playback: Res<AmPlayback>,
    mut query: Query<
        (Entity, &AmAnimated, &mut Transform, Option<&ChildOf>),
        With<AmSdfShapeParent>,
    >,
) {
    if playback.force_stopped {
        return;
    }
    let global_time = playback.current_time_ms;

    let mut scale_map: HashMap<u64, [f32; 2]> = HashMap::new();
    let mut parent_map: HashMap<u64, u64> = HashMap::new();

    for (_, animated, _, _) in query.iter() {
        let local_time = animated.calc_local_time(global_time);
        if !animated.is_active(local_time) {
            continue;
        }
        let layer_time = animated.calc_layer_time(local_time);
        scale_map.insert(
            animated.layer_id,
            compute_sdf_own_scale(animated, layer_time, global_time),
        );
        if animated.has_parent && animated.parent_layer_id != 0 {
            parent_map.insert(animated.layer_id, animated.parent_layer_id);
        }
    }

    for (_, animated, mut transform, parent) in query.iter_mut() {
        if !animated.has_parent || animated.parent_layer_id == 0 || parent.is_none() {
            continue;
        }
        let acc = accumulate_parent_scale(animated.layer_id, &parent_map, &scale_map);
        if (acc[0] - 1.0).abs() > 1e-5 || (acc[1] - 1.0).abs() > 1e-5 {
            let layer_time = {
                let lt = animated.calc_local_time(global_time);
                animated.calc_layer_time(lt)
            };
            let has_explicit_location =
                animated.location.value.is_some() || !animated.location.keyframes.is_empty();
            if has_explicit_location {
                let pivot = interpolate_vec2(&animated.pivot, layer_time).unwrap_or([0.0, 0.0]);
                let loc_x = transform.translation.x - pivot[0];
                let loc_y = transform.translation.y + pivot[1];
                transform.translation.x = loc_x * acc[0] + pivot[0];
                transform.translation.y = loc_y * acc[1] - pivot[1];
            } else {
                transform.translation.x *= acc[0];
                transform.translation.y *= acc[1];
            }
        }
    }
}

pub fn compensate_sdf_ancestor_scale_for_children_system(
    playback: Res<AmPlayback>,
    animated_query: Query<&AmAnimated>,
    sdf_query: Query<&AmAnimated, With<AmSdfShapeParent>>,
    mut child_query: Query<
        (&AmAnimated, &mut Transform, Option<&ChildOf>),
        Without<AmSdfShapeParent>,
    >,
) {
    if playback.force_stopped {
        return;
    }
    let global_time = playback.current_time_ms;

    let mut scale_map: HashMap<u64, [f32; 2]> = HashMap::new();
    let mut parent_map: HashMap<u64, u64> = HashMap::new();

    for animated in animated_query.iter() {
        if animated.has_parent && animated.parent_layer_id != 0 {
            parent_map.insert(animated.layer_id, animated.parent_layer_id);
        }
    }

    for animated in sdf_query.iter() {
        let local_time = animated.calc_local_time(global_time);
        if !animated.is_active(local_time) {
            continue;
        }
        let layer_time = animated.calc_layer_time(local_time);
        scale_map.insert(
            animated.layer_id,
            compute_sdf_own_scale(animated, layer_time, global_time),
        );
    }

    for (animated, mut transform, parent) in child_query.iter_mut() {
        if !animated.has_parent || animated.parent_layer_id == 0 || parent.is_none() {
            continue;
        }

        let acc = accumulate_parent_scale(animated.layer_id, &parent_map, &scale_map);
        if (acc[0] - 1.0).abs() <= 1e-5 && (acc[1] - 1.0).abs() <= 1e-5 {
            continue;
        }

        transform.translation.x *= acc[0];
        transform.translation.y *= acc[1];
        transform.scale.x *= acc[0];
        transform.scale.y *= acc[1];
    }
}

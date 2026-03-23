//! Shared helpers for SDF animation systems.
//! 提供 SDF 动画系统共享的辅助逻辑。
//!
//! The SDF runtime spreads work across opacity, scale, mask, stretch, and repeat systems, but they
//! all need the same low-level operations: color blending, parent-scale accumulation, uniform patch
//! helpers, and one-shot tracing. This file gathers those cross-cutting helpers so the individual
//! systems can stay focused on their specific animation responsibility.
//! SDF 运行时被拆成透明度、缩放、遮罩、拉伸、重复等多个系统，但它们都依赖同一批底层操作：
//! 颜色混合、父级缩放累积、uniform 更新辅助以及一次性 trace。这个文件把这些横切逻辑集中起来，
//! 让每个系统只关注自己的那一段动画职责。

use bevy::prelude::*;
use std::{
    collections::HashSet,
    sync::{Mutex, OnceLock},
};

use crate::animation::components::{AmAnimated, AmSdfParams};
use crate::animation::interpolation::{interpolate_float, interpolate_vec2};
use crate::animation::sdf_geometry::compute_sdf_shape_half_extent;
use crate::sdf_material::{SdfMaterialUniform, SdfShapeType};

pub(super) fn apply_solidcolor_blend(
    color: &mut Vec4,
    base: &[f32; 4],
    sc_color: Vec4,
    sc_alpha: f32,
    blend_mode: i32,
) {
    match blend_mode {
        0 => {
            color.x = base[0] + (sc_color.x - base[0]) * sc_alpha;
            color.y = base[1] + (sc_color.y - base[1]) * sc_alpha;
            color.z = base[2] + (sc_color.z - base[2]) * sc_alpha;
        }
        1 => {
            let mr = base[0] * sc_color.x;
            let mg = base[1] * sc_color.y;
            let mb = base[2] * sc_color.z;
            color.x = base[0] + (mr - base[0]) * sc_alpha;
            color.y = base[1] + (mg - base[1]) * sc_alpha;
            color.z = base[2] + (mb - base[2]) * sc_alpha;
        }
        2 => {
            let sr = 1.0 - (1.0 - base[0]) * (1.0 - sc_color.x);
            let sg = 1.0 - (1.0 - base[1]) * (1.0 - sc_color.y);
            let sb = 1.0 - (1.0 - base[2]) * (1.0 - sc_color.z);
            color.x = base[0] + (sr - base[0]) * sc_alpha;
            color.y = base[1] + (sg - base[1]) * sc_alpha;
            color.z = base[2] + (sb - base[2]) * sc_alpha;
        }
        _ => {}
    }
}

pub(super) fn apply_shape_animation_updates(
    uniform: &mut SdfMaterialUniform,
    has_shape_anim: bool,
    shape_extra_anim: &[f32; 4],
    has_pts_anim: bool,
    shape_pts_anim: &[[f32; 2]; 5],
) {
    let is_arrow = (uniform.shape_type.round() as i32) == (SdfShapeType::Arrow.to_f32() as i32);

    if is_arrow {
        if has_pts_anim {
            uniform.shape_extra = Vec4::new(
                shape_pts_anim[0][0],
                shape_pts_anim[0][1],
                shape_pts_anim[1][0],
                shape_pts_anim[1][1],
            );
        }
        if has_shape_anim {
            uniform.shape_extra2 = Vec4::new(
                shape_extra_anim[0],
                shape_extra_anim[1],
                shape_extra_anim[2],
                0.0,
            );
        }
        return;
    }

    if has_shape_anim {
        uniform.shape_extra = Vec4::new(
            shape_extra_anim[0],
            shape_extra_anim[1],
            shape_extra_anim[2],
            shape_extra_anim[3],
        );
    }
    if has_pts_anim {
        uniform.shape_extra = Vec4::new(
            shape_pts_anim[0][0],
            shape_pts_anim[0][1],
            shape_pts_anim[1][0],
            shape_pts_anim[1][1],
        );
        uniform.shape_extra2 = Vec4::new(
            shape_pts_anim[2][0],
            shape_pts_anim[2][1],
            shape_pts_anim[3][0],
            shape_pts_anim[3][1],
        );
        uniform.shape_extra3 = Vec4::new(shape_pts_anim[4][0], shape_pts_anim[4][1], 0.0, 0.0);
    }
}

pub(super) fn trace_sdf_once(key: impl Into<String>, message: impl FnOnce() -> String) {
    if std::env::var_os("AM_SDF_TRACE").is_none() {
        return;
    }

    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let key = key.into();

    let should_log = {
        let mut guard = seen.lock().expect("sdf trace mutex poisoned");
        guard.insert(key)
    };

    if should_log {
        bevy::log::warn!("{}", message());
    }
}

pub(super) fn update_sdf_child_material(
    uniform: &mut SdfMaterialUniform,
    sdf_params: &AmSdfParams,
    transform: &mut Transform,
    own_scale: [f32; 2],
    combined_scale: [f32; 2],
    stroke_width_animated: f32,
    has_shape_anim: bool,
    shape_extra_anim: &[f32; 4],
    has_pts_anim: bool,
    shape_pts_anim: &[[f32; 2]; 5],
) {
    let disable_ancestor_pivot_comp =
        std::env::var_os("AM_DISABLE_SDF_ANCESTOR_PIVOT_COMP").is_some();

    let scaled_half_width = sdf_params.base_half_width * combined_scale[0];
    let scaled_half_height = sdf_params.base_half_height * combined_scale[1];

    let mut final_stroke_width = if stroke_width_animated >= 0.0 {
        stroke_width_animated
    } else {
        sdf_params.stroke_width
    };
    if scaled_half_width.abs() < 0.1 && scaled_half_height.abs() < 0.1 {
        final_stroke_width = 0.0;
    }

    let pivot_scale = if disable_ancestor_pivot_comp {
        own_scale
    } else {
        combined_scale
    };
    transform.translation.x = -sdf_params.base_pivot_x * pivot_scale[0];
    transform.translation.y = sdf_params.base_pivot_y * pivot_scale[1];

    uniform.params = Vec4::new(
        scaled_half_width,
        scaled_half_height,
        final_stroke_width,
        sdf_params.packed_stroke,
    );

    apply_shape_animation_updates(
        uniform,
        has_shape_anim,
        shape_extra_anim,
        has_pts_anim,
        shape_pts_anim,
    );

    let new_frame_half = compute_sdf_shape_half_extent(uniform) + final_stroke_width.abs() * 2.0;
    if new_frame_half > uniform.frame_half {
        uniform.frame_half = new_frame_half;
    }

    if sdf_params.spawn_frame_half > 0.0 {
        let mesh_scale = uniform.frame_half / sdf_params.spawn_frame_half;
        if mesh_scale > 1.001 {
            transform.scale = Vec3::new(mesh_scale, mesh_scale, 1.0);
        }
    }
}

pub(super) fn compute_sdf_own_scale(
    animated: &AmAnimated,
    layer_time: f32,
    _global_time: f32,
) -> [f32; 2] {
    let mut anim_scale = interpolate_vec2(&animated.scale, layer_time).unwrap_or([1.0, 1.0]);

    if animated.scale_assist_axis != 0
        && let Some(scale_param) = interpolate_float(&animated.scale_assist, layer_time)
    {
        let damp_param = interpolate_float(&animated.scale_assist_damp, layer_time).unwrap_or(1.0);

        const SCALE_POWER: f32 = 1.71;
        const DAMP_COEFF: f32 = 2.75;
        const DAMP_POWER: f32 = 1.93;

        match animated.scale_assist_axis {
            1 => anim_scale[1] *= scale_param,
            2 => anim_scale[0] *= scale_param,
            3 => {
                let damp_exp = 1.0 + DAMP_COEFF * (damp_param - 1.0).powf(DAMP_POWER);
                let damp_factor = damp_param.powf(damp_exp);
                let scale_divisor = scale_param.powf(SCALE_POWER) * damp_factor;
                anim_scale[0] *= scale_param;
                anim_scale[1] /= scale_divisor;
            }
            _ => {}
        }
    }

    let mut posz_offset = 0.0_f32;
    if let Some(mut posz) = interpolate_float(&animated.effect_posz, layer_time) {
        if animated.effect_zinv {
            posz = 2.0 - posz;
        }
        posz_offset += posz - 1.0;
    }
    for extra in &animated.extra_transform2 {
        let Some(mut posz) = interpolate_float(&extra.pos_z, layer_time) else {
            continue;
        };
        if extra.zinv {
            posz = 2.0 - posz;
        }
        posz_offset += posz - 1.0;
    }
    let combined_posz = 1.0 + posz_offset;
    anim_scale[0] *= combined_posz;
    anim_scale[1] *= combined_posz;

    anim_scale
}

pub(super) fn accumulate_parent_scale(
    layer_id: u64,
    parent_map: &std::collections::HashMap<u64, u64>,
    scale_map: &std::collections::HashMap<u64, [f32; 2]>,
) -> [f32; 2] {
    let Some(&parent_id) = parent_map.get(&layer_id) else {
        return [1.0, 1.0];
    };
    let parent_scale = scale_map.get(&parent_id).copied().unwrap_or([1.0, 1.0]);
    let grandparent_scale = accumulate_parent_scale(parent_id, parent_map, scale_map);
    [
        parent_scale[0] * grandparent_scale[0],
        parent_scale[1] * grandparent_scale[1],
    ]
}

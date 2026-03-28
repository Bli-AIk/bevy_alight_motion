//! Applies position, rotation, scale, pivot, and related spatial
//! effects to animated entities each frame. It is the main transform execution
//! path for the runtime, including repeat offsets, noise-driven motion, embed
//! adjustments, and the special handling needed by SDF and unified visuals.
//!
//! 负责在每一帧把位置、旋转、缩放、pivot 以及相关空间效果应用到动画实体上。
//! 它是运行时最主要的变换执行路径，同时处理 repeat 偏移、噪声驱动位移、嵌套场景修正，
//! 以及 SDF 与统一材质视觉对象所需的特殊逻辑。

use bevy::prelude::*;
use std::collections::HashMap;

use crate::animation::components::{
    AmAnimated, AmPlayback, AmSdfShapeParent, AmUnifiedUsesTransformScale,
};
use crate::animation::interpolation::{
    interpolate_float, interpolate_vec2, interpolate_vec2_reverse, interpolate_vec3_reverse,
};
use crate::animation::noise_effects::{compute_jitter, compute_simplex_displace};
use crate::scene::{AmLayerMarker, AmLayerSpec};

use super::shared::{
    apply_oscillate, compute_normalized_frame_delta, invert_transform_component,
    resolve_unwrapped_rotation_deg,
};

#[derive(Clone, Copy, Debug)]
struct PerspectiveParentState {
    base_location: Vec2,
    pivot: Vec2,
    rotation_deg: f32,
    scale: Vec2,
    z: f32,
}

#[derive(Clone, Copy, Debug)]
struct AnimatedSpatialState {
    translation: Vec2,
    rotation_deg: f32,
    pivot_x: f32,
    pivot_y: f32,
    pivot_comp_scale: Vec2,
    effective_scale: Vec2,
    z: f32,
    has_parent: bool,
}

#[derive(Clone, Copy, Debug)]
struct PendingPerspectiveNullState {
    entity: Entity,
    parent_entity: Option<Entity>,
    child_state: AnimatedSpatialState,
}

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

fn trace_position_enabled(layer_id: u64, label: &str) -> bool {
    let trace_id_match = std::env::var_os("AM_TRACE_POS_IDS")
        .and_then(|value| value.into_string().ok())
        .is_some_and(|ids| {
            ids.split(',')
                .filter_map(|value| value.trim().parse::<u64>().ok())
                .any(|id| id == layer_id)
        });
    let trace_label_match = std::env::var_os("AM_TRACE_POS_LABELS")
        .and_then(|value| value.into_string().ok())
        .is_some_and(|labels| {
            labels
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .any(|value| value == label)
        });
    trace_id_match || trace_label_match
}

fn world_space_pivot(pivot_x: f32, pivot_y: f32) -> Vec2 {
    Vec2::new(pivot_x, -pivot_y)
}

fn rotate_vec2(vec: Vec2, rotation_deg: f32) -> Vec2 {
    let rotation_rad = rotation_deg.to_radians();
    let (sin, cos) = rotation_rad.sin_cos();
    Vec2::new(vec.x * cos - vec.y * sin, vec.x * sin + vec.y * cos)
}

fn translation_without_pivot_compensation(
    translation: Vec2,
    pivot_x: f32,
    pivot_y: f32,
    scale: Vec2,
    rotation_deg: f32,
    has_parent: bool,
) -> Vec2 {
    let (comp_x, comp_y) = embed_like_pivot_compensation(
        pivot_x,
        pivot_y,
        [scale.x, scale.y],
        rotation_deg,
        has_parent,
    );
    translation - Vec2::new(comp_x, comp_y)
}

fn apply_perspective_parenting(
    parent_state: PerspectiveParentState,
    child_state: AnimatedSpatialState,
) -> (Vec2, f32, Vec2, f32) {
    let child_base_location = translation_without_pivot_compensation(
        child_state.translation,
        child_state.pivot_x,
        child_state.pivot_y,
        child_state.pivot_comp_scale,
        child_state.rotation_deg,
        child_state.has_parent,
    );
    let child_world_pivot = world_space_pivot(child_state.pivot_x, child_state.pivot_y);
    let parent_scaled_delta = Vec2::new(
        parent_state.scale.x * ((child_base_location + child_world_pivot) - parent_state.pivot).x,
        parent_state.scale.y * ((child_base_location + child_world_pivot) - parent_state.pivot).y,
    );
    let combined_base_location = parent_state.base_location
        + rotate_vec2(parent_scaled_delta, parent_state.rotation_deg)
        - child_world_pivot
        + parent_state.pivot;
    let combined_rotation_deg = parent_state.rotation_deg + child_state.rotation_deg;
    let combined_scale = Vec2::new(
        parent_state.scale.x * child_state.effective_scale.x,
        parent_state.scale.y * child_state.effective_scale.y,
    );
    let (comp_x, comp_y) = embed_like_pivot_compensation(
        child_state.pivot_x,
        child_state.pivot_y,
        [combined_scale.x, combined_scale.y],
        combined_rotation_deg,
        false,
    );
    (
        combined_base_location + Vec2::new(comp_x, comp_y),
        combined_rotation_deg,
        combined_scale,
        parent_state.z + child_state.z,
    )
}

fn perspective_parent_state_from_world_transform(
    world_translation: Vec2,
    pivot_x: f32,
    pivot_y: f32,
    pivot_comp_scale: Vec2,
    rotation_deg: f32,
    scale: Vec2,
    z: f32,
) -> PerspectiveParentState {
    let base_location = translation_without_pivot_compensation(
        world_translation,
        pivot_x,
        pivot_y,
        pivot_comp_scale,
        rotation_deg,
        false,
    );

    PerspectiveParentState {
        base_location,
        pivot: world_space_pivot(pivot_x, pivot_y),
        rotation_deg,
        scale,
        z,
    }
}

fn apply_sdf_linear_repeat(
    sdf_parent: Option<&AmSdfShapeParent>,
    animated: &AmAnimated,
    layer_time: f32,
    bx: &mut f32,
    by: &mut f32,
) {
    if sdf_parent.is_none() {
        return;
    }
    let Some(d) = crate::animation::effects::repeat::compute_sdf_linear_repeat_displacement(
        animated, layer_time,
    ) else {
        return;
    };
    if d[0].is_nan() {
        *bx = -99999.0;
        *by = -99999.0;
    } else {
        *bx += d[0];
        *by -= d[1];
    }
}

fn apply_pivot_offset(
    animated: &AmAnimated,
    layer_time: f32,
    layer_spec: &AmLayerSpec,
    sdf_parent: Option<&AmSdfShapeParent>,
    current_scale: [f32; 2],
    bx: &mut f32,
    by: &mut f32,
) {
    let Some(pivot) = interpolate_vec2(&animated.pivot, layer_time) else {
        return;
    };
    let pivot_x = pivot[0];
    let pivot_y = pivot[1];

    let is_sdf_shape = sdf_parent.is_some() || matches!(layer_spec, AmLayerSpec::SdfShape { .. });

    if is_sdf_shape {
        *bx += pivot_x;
        *by -= pivot_y;
    } else if matches!(layer_spec, AmLayerSpec::EmbedScene | AmLayerSpec::Null) {
        let authored_rotation_deg =
            interpolate_float(&animated.rotation, layer_time).unwrap_or(0.0);
        let bevy_rotation_deg = -authored_rotation_deg + animated.repeat_rotation_offset_deg;
        let (comp_x, comp_y) = embed_like_pivot_compensation(
            pivot_x,
            pivot_y,
            current_scale,
            bevy_rotation_deg,
            animated.has_parent,
        );
        *bx += comp_x;
        *by += comp_y;
    }
}

pub fn animate_transform_system(
    playback: Res<AmPlayback>,
    mut query: Query<(
        Entity,
        &AmAnimated,
        &mut Transform,
        &AmLayerMarker,
        &AmLayerSpec,
        Option<&AmSdfShapeParent>,
        Option<&crate::masked_sprite::UnifiedEffectMarker>,
        Option<&AmUnifiedUsesTransformScale>,
        Option<&crate::scene::AmEmbedContentMarker>,
        Option<&crate::scene::AmPerspectiveParent>,
        Option<&crate::scene::AmPerspectiveNull>,
        Option<&ChildOf>,
    )>,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;
    let mut perspective_parents = HashMap::new();
    let mut pending_perspective_nulls = Vec::new();
    let mut spatial_states = HashMap::new();

    for (
        entity,
        animated,
        mut transform,
        marker,
        layer_spec,
        sdf_parent,
        effect_marker,
        unified_transform_scale,
        embed_content_marker,
        perspective_parent,
        perspective_null,
        child_of,
    ) in query.iter_mut()
    {
        let local_time = animated.calc_local_time(global_time);
        if !animated.is_active(local_time) {
            continue;
        }

        let layer_time = animated.calc_layer_time(local_time);
        let frame_delta = compute_normalized_frame_delta(animated);

        let mut actual_scale = interpolate_vec2_reverse(&animated.scale, layer_time, frame_delta)
            .unwrap_or([1.0, 1.0]);

        if animated.scale_assist_axis != 0
            && let Some(scale_param) = crate::animation::interpolation::interpolate_float(
                &animated.scale_assist,
                layer_time,
            )
        {
            let damp_param = crate::animation::interpolation::interpolate_float(
                &animated.scale_assist_damp,
                layer_time,
            )
            .unwrap_or(1.0);

            const SCALE_POWER: f32 = 1.71;
            const DAMP_COEFF: f32 = 2.75;
            const DAMP_POWER: f32 = 1.93;

            match animated.scale_assist_axis {
                1 => actual_scale[1] *= scale_param,
                2 => actual_scale[0] *= scale_param,
                3 => {
                    let damp_exp = 1.0 + DAMP_COEFF * (damp_param - 1.0).powf(DAMP_POWER);
                    let damp_factor = damp_param.powf(damp_exp);
                    let scale_divisor = scale_param.powf(SCALE_POWER) * damp_factor;
                    actual_scale[0] *= scale_param;
                    actual_scale[1] /= scale_divisor;
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
        actual_scale[0] *= combined_posz;
        actual_scale[1] *= combined_posz;

        let unified_scale_baked = effect_marker.is_some() && unified_transform_scale.is_none();
        let current_scale = if sdf_parent.is_some() || unified_scale_baked {
            [1.0_f32, 1.0_f32]
        } else {
            actual_scale
        };

        let loc =
            interpolate_vec3_reverse(&animated.location, layer_time, frame_delta).or_else(|| {
                if animated.has_parent
                    && sdf_parent.is_none()
                    && matches!(layer_spec, AmLayerSpec::SdfShape { .. })
                {
                    Some([0.0, 0.0, 0.0])
                } else {
                    None
                }
            });

        let mut oscillate_z_zoom = 1.0_f32;
        if let Some(loc) = loc {
            let (mut bx, mut by) = if animated.has_parent {
                (loc[0], -loc[1])
            } else {
                (
                    loc[0] - animated.canvas_width / 2.0,
                    animated.canvas_height / 2.0 - loc[1],
                )
            };

            if trace_position_enabled(animated.layer_id, &marker.label) {
                trace!(
                    "[PosCalc] layer={} label='{}' is_embed_content={} speed_mul={:.2} time_offset={} | global_time={:.1} local_time={:.1} layer_time={:.4} | AM_loc=({:.2},{:.2}) canvas=({:.0},{:.0}) has_parent={} | bevy=({:.2},{:.2}) scale=({:.3},{:.3})",
                    animated.layer_id,
                    marker.label,
                    embed_content_marker.is_some(),
                    animated.speed_multiplier,
                    animated.time_offset,
                    global_time,
                    local_time,
                    layer_time,
                    loc[0],
                    loc[1],
                    animated.canvas_width,
                    animated.canvas_height,
                    animated.has_parent,
                    bx,
                    by,
                    actual_scale[0],
                    actual_scale[1],
                );
            }

            apply_pivot_offset(
                animated,
                layer_time,
                &layer_spec,
                sdf_parent,
                current_scale,
                &mut bx,
                &mut by,
            );

            if let Some(effect_x) = interpolate_float(&animated.effect_pos_x, layer_time) {
                bx += invert_transform_component(effect_x, animated.effect_xinv);
            }
            if let Some(effect_y) = interpolate_float(&animated.effect_pos_y, layer_time) {
                by -= invert_transform_component(effect_y, animated.effect_yinv);
            }
            for extra in &animated.extra_transform2 {
                bx += interpolate_float(&extra.pos_x, layer_time)
                    .map(|x| invert_transform_component(x, extra.xinv))
                    .unwrap_or(0.0);
                by -= interpolate_float(&extra.pos_y, layer_time)
                    .map(|y| invert_transform_component(y, extra.yinv))
                    .unwrap_or(0.0);
            }

            if !animated.has_parent {
                by -= animated.font_y_offset;
            }

            if matches!(layer_spec, AmLayerSpec::Text { .. }) {
                bx -= animated.inv_fit_scale;
            }

            if !matches!(layer_spec, AmLayerSpec::SdfShape { .. }) && sdf_parent.is_none() {
                bx += animated.anchor_offset.x;
                by += animated.anchor_offset.y;
            }

            oscillate_z_zoom = apply_oscillate(animated, layer_time, &mut bx, &mut by);

            if animated.jitter_enabled {
                let (jdx, jdy, jz) = compute_jitter(animated, local_time);
                bx = (bx + jdx) * jz;
                by = (by + jdy) * jz;
                oscillate_z_zoom *= jz;
            }

            if animated.sd_enabled {
                let (sdx, sdy) = compute_simplex_displace(animated, layer_time, bx, by);
                bx += sdx;
                by += sdy;
            }

            bx += animated.repeat_position_offset.x;
            by += animated.repeat_position_offset.y;

            apply_sdf_linear_repeat(sdf_parent, animated, layer_time, &mut bx, &mut by);

            transform.translation = Vec3::new(bx, by, transform.translation.z);
        }

        let final_rotation = resolve_unwrapped_rotation_deg(animated, layer_time, frame_delta);
        transform.rotation = Quat::from_rotation_z(final_rotation.to_radians());

        if sdf_parent.is_none() && effect_marker.is_none() {
            transform.scale = Vec3::new(
                current_scale[0] * oscillate_z_zoom * animated.repeat_scale_factor,
                current_scale[1] * oscillate_z_zoom * animated.repeat_scale_factor,
                1.0,
            );
        } else if unified_scale_baked {
            let sign_x = actual_scale[0].signum();
            let sign_y = actual_scale[1].signum();
            transform.scale = Vec3::new(
                sign_x * combined_posz * oscillate_z_zoom,
                sign_y * combined_posz * oscillate_z_zoom,
                1.0,
            );
        }

        let pivot = interpolate_vec2(&animated.pivot, layer_time).unwrap_or([0.0, 0.0]);
        let pivot_comp_scale = Vec2::new(current_scale[0], current_scale[1]);
        let effective_scale = Vec2::new(
            actual_scale[0] * oscillate_z_zoom * animated.repeat_scale_factor,
            actual_scale[1] * oscillate_z_zoom * animated.repeat_scale_factor,
        );
        let child_state = AnimatedSpatialState {
            translation: transform.translation.truncate(),
            rotation_deg: final_rotation,
            pivot_x: pivot[0],
            pivot_y: pivot[1],
            pivot_comp_scale,
            effective_scale,
            z: transform.translation.z,
            has_parent: animated.has_parent,
        };
        spatial_states.insert(entity, child_state);

        if perspective_null.is_some() {
            pending_perspective_nulls.push(PendingPerspectiveNullState {
                entity,
                parent_entity: perspective_parent
                    .map(|parent| parent.entity)
                    .or_else(|| child_of.map(|parent| parent.parent())),
                child_state,
            });
        }
    }

    while !pending_perspective_nulls.is_empty() {
        let mut unresolved = Vec::new();
        let mut resolved_this_round = 0_usize;

        for pending in pending_perspective_nulls.drain(..) {
            let resolved_state = match pending.parent_entity {
                Some(parent_entity) => {
                    perspective_parents
                        .get(&parent_entity)
                        .copied()
                        .map(|parent_state| {
                            let (
                                combined_translation,
                                combined_rotation_deg,
                                combined_scale,
                                combined_z,
                            ) = apply_perspective_parenting(parent_state, pending.child_state);
                            perspective_parent_state_from_world_transform(
                                combined_translation,
                                pending.child_state.pivot_x,
                                pending.child_state.pivot_y,
                                combined_scale,
                                combined_rotation_deg,
                                combined_scale,
                                combined_z,
                            )
                        })
                }
                None => Some(perspective_parent_state_from_world_transform(
                    pending.child_state.translation,
                    pending.child_state.pivot_x,
                    pending.child_state.pivot_y,
                    pending.child_state.pivot_comp_scale,
                    pending.child_state.rotation_deg,
                    pending.child_state.effective_scale,
                    pending.child_state.z,
                )),
            };

            if let Some(state) = resolved_state {
                perspective_parents.insert(pending.entity, state);
                resolved_this_round += 1;
            } else {
                unresolved.push(pending);
            }
        }

        if resolved_this_round == 0 {
            for pending in &unresolved {
                bevy::log::warn!(
                    "[PerspectiveNull] unresolved perspective parent chain at entity {:?}",
                    pending.entity
                );
            }
            break;
        }

        pending_perspective_nulls = unresolved;
    }

    for (
        _entity,
        animated,
        mut transform,
        marker,
        layer_spec,
        sdf_parent,
        _effect_marker,
        _unified_transform_scale,
        _embed_content_marker,
        perspective_parent,
        _perspective_null,
        _child_of,
    ) in query.iter_mut()
    {
        let Some(parent_entity) = perspective_parent.map(|parent| parent.entity) else {
            continue;
        };
        let Some(parent_state) = perspective_parents.get(&parent_entity).copied() else {
            continue;
        };
        if animated.parenthelper_has_effect
            || sdf_parent.is_some()
            || matches!(
                layer_spec,
                AmLayerSpec::Camera { .. } | AmLayerSpec::SdfShape { .. }
            )
        {
            continue;
        }
        let Some(child_state) = spatial_states.get(&_entity).copied() else {
            continue;
        };
        let (combined_translation, combined_rotation_deg, combined_scale, combined_z) =
            apply_perspective_parenting(parent_state, child_state);

        if trace_position_enabled(animated.layer_id, &marker.label) {
            trace!(
                "[PerspectiveNull] layer={} label='{}' parent_z={:.3} child_local=({:.2},{:.2},{:.3}) -> combined=({:.2},{:.2},{:.3}) rot={:.2} scale=({:.3},{:.3})",
                animated.layer_id,
                marker.label,
                parent_state.z,
                child_state.translation.x,
                child_state.translation.y,
                child_state.z,
                combined_translation.x,
                combined_translation.y,
                combined_z,
                combined_rotation_deg,
                combined_scale.x,
                combined_scale.y,
            );
        }

        transform.translation =
            Vec3::new(combined_translation.x, combined_translation.y, combined_z);
        transform.rotation = Quat::from_rotation_z(combined_rotation_deg.to_radians());
        transform.scale = Vec3::new(combined_scale.x, combined_scale.y, 1.0);
    }

    for (
        _entity,
        _animated,
        mut transform,
        _marker,
        _layer_spec,
        _sdf_parent,
        _effect_marker,
        _unified_transform_scale,
        _embed_content_marker,
        perspective_parent,
        perspective_null,
        _child_of,
    ) in query.iter_mut()
    {
        if perspective_null.is_some() && perspective_parent.is_some() {
            transform.translation = Vec3::ZERO;
            transform.rotation = Quat::IDENTITY;
            transform.scale = Vec3::ONE;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perspective_parenting_matches_am_root_space_conversion() {
        let parent_state = PerspectiveParentState {
            base_location: Vec2::new(48.407_166, 31.243_408),
            pivot: Vec2::ZERO,
            rotation_deg: 180.0,
            scale: Vec2::splat(1.04),
            z: 0.0,
        };
        let child_state = AnimatedSpatialState {
            translation: Vec2::new(-5.734_375, 569.846_9),
            rotation_deg: 0.0,
            pivot_x: -0.361_938,
            pivot_y: 176.000_98,
            pivot_comp_scale: Vec2::ONE,
            effective_scale: Vec2::ONE,
            z: 0.0,
            has_parent: true,
        };
        let (combined_translation, combined_rotation_deg, combined_scale, _) =
            apply_perspective_parenting(parent_state, child_state);

        assert!((combined_translation.x - 54.370_916).abs() < 0.001);
        assert!((combined_translation.y + 561.397_4).abs() < 0.001);
        assert!((combined_rotation_deg - 180.0).abs() < 0.001);
        assert!((combined_scale.x - 1.04).abs() < 0.001);
        assert!((combined_scale.y - 1.04).abs() < 0.001);
    }

    #[test]
    fn nested_perspective_parent_state_reconstructs_combined_world_transform() {
        let parent_state = PerspectiveParentState {
            base_location: Vec2::new(48.407_166, 31.243_408),
            pivot: Vec2::ZERO,
            rotation_deg: 180.0,
            scale: Vec2::splat(1.04),
            z: 12.0,
        };
        let child_state = AnimatedSpatialState {
            translation: Vec2::new(-5.734_375, 569.846_9),
            rotation_deg: 0.0,
            pivot_x: -0.361_938,
            pivot_y: 176.000_98,
            pivot_comp_scale: Vec2::ONE,
            effective_scale: Vec2::ONE,
            z: 3.0,
            has_parent: true,
        };
        let (combined_translation, combined_rotation_deg, combined_scale, combined_z) =
            apply_perspective_parenting(parent_state, child_state);
        let nested_parent_state = perspective_parent_state_from_world_transform(
            combined_translation,
            child_state.pivot_x,
            child_state.pivot_y,
            combined_scale,
            combined_rotation_deg,
            combined_scale,
            combined_z,
        );
        let (comp_x, comp_y) = embed_like_pivot_compensation(
            child_state.pivot_x,
            child_state.pivot_y,
            [combined_scale.x, combined_scale.y],
            combined_rotation_deg,
            false,
        );
        let reconstructed_translation =
            nested_parent_state.base_location + Vec2::new(comp_x, comp_y);

        assert!((nested_parent_state.rotation_deg - combined_rotation_deg).abs() < 0.001);
        assert!((nested_parent_state.scale.x - combined_scale.x).abs() < 0.001);
        assert!((nested_parent_state.scale.y - combined_scale.y).abs() < 0.001);
        assert!((nested_parent_state.z - combined_z).abs() < 0.001);
        assert!((reconstructed_translation.x - combined_translation.x).abs() < 0.001);
        assert!((reconstructed_translation.y - combined_translation.y).abs() < 0.001);
    }
}

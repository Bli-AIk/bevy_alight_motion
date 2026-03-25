//! Repairs parent-helper inheritance for runtime layer transforms.
//!
//! 修正运行时图层在 Parent Helper 效果下的父子继承结果。
//!
//! Reconstructs world-space translation, rotation, and scale for layers that opt into
//! Alight Motion's parent-helper behavior. It is the runtime half of the parent-helper effect:
//! scene collection records the effect parameters, and this module applies them every frame so
//! child layers inherit only the weighted subset of parent motion that the effect requests.
//!
//! 负责在运行时重建启用了 Parent Helper 效果的图层世界空间平移、旋转和缩放。
//! 它是 Parent Helper 效果的运行时一半：scene 收集阶段先记录效果参数，这里再按帧应用，
//! 让子图层只继承效果要求的那部分父级运动。

use bevy::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    sync::{Mutex, OnceLock},
};

use crate::scene::{AmLayerMarker, AmLayerSpec};

use super::components::{AmAnimated, AmPlayback, AmUnifiedUsesTransformScale};
use super::interpolation::{interpolate_float, interpolate_vec2};
use super::systems::{compute_normalized_frame_delta, resolve_unwrapped_rotation_deg};

fn trace_parenthelper_once(key: impl Into<String>, message: impl FnOnce() -> String) {
    if std::env::var_os("AM_PARENTHELPER_TRACE").is_none() {
        return;
    }

    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let key = key.into();

    let should_log = {
        let mut guard = seen.lock().expect("parenthelper trace mutex poisoned");
        guard.insert(key)
    };

    if should_log {
        bevy::log::warn!("{}", message());
    }
}

#[derive(Clone, Copy, Debug)]
struct ParentHelperLocalState {
    translation: Vec2,
    rotation_deg: f32,
    applied_scale: Vec2,
    visual_scale: Vec2,
}

#[derive(Clone, Copy, Debug)]
struct ParentHelperSnapshot {
    local: ParentHelperLocalState,
    parent: Option<Entity>,
    has_effect: bool,
    scale_baked_in_mesh: bool,
    scale_factor: f32,
    rotate_factor: f32,
    auto_rotate: i32,
    radius_adjust: f32,
    base_size: Vec2,
}

#[derive(Clone, Copy, Debug)]
struct ParentHelperWorldState {
    translation: Vec2,
    rotation_deg: f32,
    applied_scale: Vec2,
    visual_scale: Vec2,
}

fn rotate_vec2(v: Vec2, rotation_deg: f32) -> Vec2 {
    let angle = rotation_deg.to_radians();
    let (sin, cos) = angle.sin_cos();
    Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos)
}

fn safe_div_vec2(numer: Vec2, denom: Vec2) -> Vec2 {
    Vec2::new(
        if denom.x.abs() > 1e-4 {
            numer.x / denom.x
        } else {
            numer.x
        },
        if denom.y.abs() > 1e-4 {
            numer.y / denom.y
        } else {
            numer.y
        },
    )
}

fn resolve_parenthelper_base_size(
    layer_spec: &AmLayerSpec,
    animated: &AmAnimated,
    layer_time: f32,
) -> Vec2 {
    if let Some(size) = interpolate_vec2(&animated.size, layer_time) {
        let size = Vec2::new(size[0].abs(), size[1].abs());
        if size.x > 0.0 && size.y > 0.0 {
            return size;
        }
    }

    match layer_spec {
        AmLayerSpec::SpriteShape { width, height, .. }
        | AmLayerSpec::SdfShape { width, height, .. }
        | AmLayerSpec::Image { width, height, .. } => Vec2::new(width.abs(), height.abs()),
        AmLayerSpec::Text {
            wrap_width,
            font_size,
            ..
        } => Vec2::new((*wrap_width).max(*font_size), *font_size),
        _ => Vec2::splat(100.0),
    }
}

fn resolve_parenthelper_visual_scale(
    animated: &AmAnimated,
    layer_time: f32,
    transform_scale: Vec2,
) -> Vec2 {
    let mut scale = interpolate_vec2(&animated.scale, layer_time)
        .map(|s| Vec2::new(s[0], s[1]))
        .unwrap_or_else(|| Vec2::new(transform_scale.x.abs(), transform_scale.y.abs()));

    if animated.scale_assist_axis != 0
        && let Some(scale_param) = interpolate_float(&animated.scale_assist, layer_time)
    {
        let damp_param = interpolate_float(&animated.scale_assist_damp, layer_time).unwrap_or(1.0);
        const SCALE_POWER: f32 = 1.71;
        const DAMP_COEFF: f32 = 2.75;
        const DAMP_POWER: f32 = 1.93;

        match animated.scale_assist_axis {
            1 => scale.y *= scale_param,
            2 => scale.x *= scale_param,
            3 => {
                let damp_exp = 1.0 + DAMP_COEFF * (damp_param - 1.0).powf(DAMP_POWER);
                let damp_factor = damp_param.powf(damp_exp);
                let scale_divisor = scale_param.powf(SCALE_POWER) * damp_factor;
                scale.x *= scale_param;
                scale.y /= scale_divisor;
            }
            _ => {}
        }
    }

    scale *= animated.repeat_scale_factor;

    let sign_x = if transform_scale.x < 0.0 { -1.0 } else { 1.0 };
    let sign_y = if transform_scale.y < 0.0 { -1.0 } else { 1.0 };
    Vec2::new(scale.x.abs() * sign_x, scale.y.abs() * sign_y)
}

fn compute_parenthelper_local(
    snapshot: ParentHelperSnapshot,
    parent_world: ParentHelperWorldState,
) -> ParentHelperLocalState {
    if !snapshot.has_effect {
        return snapshot.local;
    }

    let inherited_scale = Vec2::new(
        1.0 + (parent_world.visual_scale.x - 1.0) * snapshot.scale_factor,
        1.0 + (parent_world.visual_scale.y - 1.0) * snapshot.scale_factor,
    );
    let target_world_scale = inherited_scale * snapshot.local.visual_scale;
    let local_translation = snapshot.local.translation;
    let local_scale = if snapshot.scale_baked_in_mesh {
        snapshot.local.applied_scale
    } else {
        safe_div_vec2(target_world_scale, parent_world.applied_scale)
    };

    let world_position = parent_world.translation
        + rotate_vec2(
            parent_world.applied_scale * snapshot.local.translation,
            parent_world.rotation_deg,
        );

    let mut auto_rotate_deg = 0.0_f32;
    if snapshot.auto_rotate != 0 {
        let axis_coord = match snapshot.auto_rotate {
            1 => world_position.x,
            2 => -world_position.y,
            _ => 0.0,
        };
        let min_extent = (snapshot.base_size.x * target_world_scale.x.abs())
            .min(snapshot.base_size.y * target_world_scale.y.abs());
        let min_scale = target_world_scale.x.abs().min(target_world_scale.y.abs());
        let denom =
            ((min_extent / 2.0) + (snapshot.radius_adjust * min_scale)) * std::f32::consts::TAU;
        if denom.abs() > 1e-4 {
            auto_rotate_deg = axis_coord / denom * 360.0;
        }
    }

    ParentHelperLocalState {
        translation: local_translation,
        rotation_deg: snapshot.local.rotation_deg
            + parent_world.rotation_deg * (snapshot.rotate_factor - 1.0)
            - auto_rotate_deg,
        applied_scale: local_scale,
        visual_scale: snapshot.local.visual_scale,
    }
}

fn resolve_parenthelper_world(
    entity: Entity,
    snapshots: &HashMap<Entity, ParentHelperSnapshot>,
    cache: &mut HashMap<Entity, ParentHelperWorldState>,
) -> Option<ParentHelperWorldState> {
    if let Some(world) = cache.get(&entity).copied() {
        return Some(world);
    }

    let snapshot = snapshots.get(&entity).copied()?;
    let parent_world = snapshot
        .parent
        .filter(|parent_entity| snapshots.contains_key(parent_entity))
        .and_then(|parent_entity| resolve_parenthelper_world(parent_entity, snapshots, cache));

    let local = if let Some(parent_world) = parent_world {
        compute_parenthelper_local(snapshot, parent_world)
    } else {
        snapshot.local
    };

    let world = if let Some(parent_world) = parent_world {
        let inherited_scale = if snapshot.has_effect {
            Vec2::new(
                1.0 + (parent_world.visual_scale.x - 1.0) * snapshot.scale_factor,
                1.0 + (parent_world.visual_scale.y - 1.0) * snapshot.scale_factor,
            )
        } else {
            parent_world.visual_scale
        };
        let visual_scale = if snapshot.has_effect {
            inherited_scale * snapshot.local.visual_scale
        } else {
            parent_world.visual_scale * snapshot.local.visual_scale
        };
        ParentHelperWorldState {
            translation: parent_world.translation
                + rotate_vec2(
                    parent_world.applied_scale * local.translation,
                    parent_world.rotation_deg,
                ),
            rotation_deg: parent_world.rotation_deg + local.rotation_deg,
            applied_scale: parent_world.applied_scale * local.applied_scale,
            visual_scale,
        }
    } else {
        ParentHelperWorldState {
            translation: local.translation,
            rotation_deg: local.rotation_deg,
            applied_scale: local.applied_scale,
            visual_scale: local.visual_scale,
        }
    };

    cache.insert(entity, world);
    Some(world)
}

/// Apply parenthelper effect by compensating local transforms against the current parent world
/// transform. This runs after base animation has produced the normal local Transform.
pub fn apply_parenthelper_system(
    playback: Res<AmPlayback>,
    mut queries: ParamSet<(
        Query<(
            Entity,
            &AmAnimated,
            &AmLayerMarker,
            &Transform,
            &AmLayerSpec,
            Option<&crate::masked_sprite::UnifiedEffectMarker>,
            Option<&AmUnifiedUsesTransformScale>,
            Option<&ChildOf>,
        )>,
        Query<(Entity, &AmAnimated, &AmLayerMarker, &mut Transform)>,
    )>,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;
    let mut snapshots = HashMap::new();

    for (
        entity,
        animated,
        marker,
        transform,
        layer_spec,
        effect_marker,
        unified_transform_scale,
        child_of,
    ) in queries.p0().iter()
    {
        let local_time = animated.calc_local_time(global_time);
        let layer_time = animated.calc_layer_time(local_time);
        let frame_delta = compute_normalized_frame_delta(animated);
        let base_size = resolve_parenthelper_base_size(layer_spec, animated, layer_time);
        let rotation_deg = resolve_unwrapped_rotation_deg(animated, layer_time, frame_delta);
        let applied_scale = transform.scale.truncate();
        let visual_scale = resolve_parenthelper_visual_scale(animated, layer_time, applied_scale);
        let scale_factor = match animated.parenthelper_scale_mode {
            1 => 0.0,
            2 => interpolate_float(&animated.parenthelper_scale_weight, layer_time).unwrap_or(1.0),
            _ => 1.0,
        };
        let rotate_factor = match animated.parenthelper_rotate_mode {
            1 => 0.0,
            2 => interpolate_float(&animated.parenthelper_rotate_weight, layer_time).unwrap_or(1.0),
            _ => 1.0,
        };
        let radius_adjust =
            interpolate_float(&animated.parenthelper_radius_adjust, layer_time).unwrap_or(0.0);

        if animated.parenthelper_has_effect {
            trace_parenthelper_once(format!("snapshot:{}", marker.id), || {
                format!(
                    "[ParentHelper] entity={} layer_id={} label='{}' parent={:?} scale_mode={} rotate_mode={} auto_rotate={} scale_factor={:.3} rotate_factor={:.3} radius_adjust={:.3} local_visual=({:.3},{:.3}) local_applied=({:.3},{:.3})",
                    entity,
                    marker.id,
                    marker.label,
                    child_of.map(|p| p.parent()),
                    animated.parenthelper_scale_mode,
                    animated.parenthelper_rotate_mode,
                    animated.parenthelper_auto_rotate,
                    scale_factor,
                    rotate_factor,
                    radius_adjust,
                    visual_scale.x,
                    visual_scale.y,
                    applied_scale.x,
                    applied_scale.y,
                )
            });
        }

        snapshots.insert(
            entity,
            ParentHelperSnapshot {
                local: ParentHelperLocalState {
                    translation: transform.translation.truncate(),
                    rotation_deg,
                    applied_scale,
                    visual_scale,
                },
                parent: child_of.map(|p| p.parent()),
                has_effect: animated.parenthelper_has_effect && animated.has_parent,
                scale_baked_in_mesh: (effect_marker.is_some() && unified_transform_scale.is_none())
                    || matches!(layer_spec, AmLayerSpec::SdfShape { .. }),
                scale_factor,
                rotate_factor,
                auto_rotate: animated.parenthelper_auto_rotate,
                radius_adjust,
                base_size,
            },
        );
    }

    let mut world_cache = HashMap::new();
    for (entity, animated, marker, mut transform) in queries.p1().iter_mut() {
        if !animated.parenthelper_has_effect || !animated.has_parent {
            continue;
        }
        let Some(snapshot) = snapshots.get(&entity).copied() else {
            continue;
        };
        let Some(parent_entity) = snapshot.parent else {
            continue;
        };
        let Some(parent_world) =
            resolve_parenthelper_world(parent_entity, &snapshots, &mut world_cache)
        else {
            continue;
        };

        let corrected = compute_parenthelper_local(snapshot, parent_world);
        trace_parenthelper_once(format!("corrected:{}", marker.id), || {
            format!(
                "[ParentHelper:Corrected] entity={} layer_id={} label='{}' local_before=pos({:.3},{:.3}) rot={:.3} applied=({:.3},{:.3}) visual=({:.3},{:.3}) | parent_world=pos({:.3},{:.3}) rot={:.3} applied=({:.3},{:.3}) visual=({:.3},{:.3}) | local_after=pos({:.3},{:.3}) rot={:.3} applied=({:.3},{:.3}) visual=({:.3},{:.3})",
                entity,
                marker.id,
                marker.label,
                snapshot.local.translation.x,
                snapshot.local.translation.y,
                snapshot.local.rotation_deg,
                snapshot.local.applied_scale.x,
                snapshot.local.applied_scale.y,
                snapshot.local.visual_scale.x,
                snapshot.local.visual_scale.y,
                parent_world.translation.x,
                parent_world.translation.y,
                parent_world.rotation_deg,
                parent_world.applied_scale.x,
                parent_world.applied_scale.y,
                parent_world.visual_scale.x,
                parent_world.visual_scale.y,
                corrected.translation.x,
                corrected.translation.y,
                corrected.rotation_deg,
                corrected.applied_scale.x,
                corrected.applied_scale.y,
                corrected.visual_scale.x,
                corrected.visual_scale.y,
            )
        });
        transform.translation.x = corrected.translation.x;
        transform.translation.y = corrected.translation.y;
        transform.rotation = Quat::from_rotation_z(corrected.rotation_deg.to_radians());
        transform.scale.x = corrected.applied_scale.x;
        transform.scale.y = corrected.applied_scale.y;
    }
}

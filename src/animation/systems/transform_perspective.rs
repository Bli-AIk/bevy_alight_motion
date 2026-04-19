use bevy::prelude::*;
use std::collections::HashMap;

use crate::animation::components::{AmAnimated, AmSdfShapeParent};
use crate::animation::interpolation::{interpolate_float, interpolate_vec2};
use crate::scene::AmLayerSpec;

#[derive(Clone, Copy, Debug)]
pub(crate) struct PerspectiveParentState {
    pub(crate) base_location: Vec2,
    pub(crate) pivot: Vec2,
    pub(crate) rotation_deg: f32,
    pub(crate) scale: Vec2,
    pub(crate) z: f32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AnimatedSpatialState {
    pub(crate) translation: Vec2,
    pub(crate) rotation_deg: f32,
    pub(crate) pivot_x: f32,
    pub(crate) pivot_y: f32,
    pub(crate) pivot_comp_scale: Vec2,
    pub(crate) effective_scale: Vec2,
    pub(crate) z: f32,
    pub(crate) has_parent: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PendingPerspectiveNullState {
    pub(crate) entity: Entity,
    pub(crate) parent_entity: Option<Entity>,
    pub(crate) child_state: AnimatedSpatialState,
}

pub(super) fn embed_like_pivot_compensation(
    pivot_x: f32,
    pivot_y: f32,
    scale: [f32; 2],
    rotation_deg_bevy: f32,
    has_parent: bool,
) -> (f32, f32) {
    let pivot_bevy_y = if has_parent { pivot_y } else { -pivot_y };
    let scaled_offset_x = -pivot_x * scale[0];
    let scaled_offset_y = -pivot_bevy_y * scale[1];
    // Non-parented embeds use Bevy Y-up coords (pivot_y negated above), so the
    // rotation must stay in Bevy convention (rotation_deg_bevy used directly).
    // Parented embeds keep AM's Y direction for the pivot, requiring the original
    // AM angle (-rotation_deg_bevy).
    let rotation_rad = if has_parent {
        (-rotation_deg_bevy).to_radians()
    } else {
        rotation_deg_bevy.to_radians()
    };
    let rotated_offset_x =
        scaled_offset_x * rotation_rad.cos() - scaled_offset_y * rotation_rad.sin();
    let rotated_offset_y =
        scaled_offset_x * rotation_rad.sin() + scaled_offset_y * rotation_rad.cos();

    (pivot_x + rotated_offset_x, pivot_bevy_y + rotated_offset_y)
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

pub(super) fn apply_perspective_parenting(
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

pub(super) fn perspective_parent_state_from_world_transform(
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

pub(super) fn trace_position_enabled(layer_id: u64, label: &str) -> bool {
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

pub(super) fn apply_sdf_linear_repeat(
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

pub(super) fn apply_pivot_offset(
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

pub(super) fn resolve_pending_perspective_null_state(
    pending: &PendingPerspectiveNullState,
    perspective_parents: &HashMap<Entity, PerspectiveParentState>,
) -> Option<PerspectiveParentState> {
    if let Some(parent_entity) = pending.parent_entity {
        let parent_state = perspective_parents.get(&parent_entity).copied()?;
        let (combined_translation, combined_rotation_deg, combined_scale, combined_z) =
            apply_perspective_parenting(parent_state, pending.child_state);
        Some(perspective_parent_state_from_world_transform(
            combined_translation,
            pending.child_state.pivot_x,
            pending.child_state.pivot_y,
            combined_scale,
            combined_rotation_deg,
            combined_scale,
            combined_z,
        ))
    } else {
        Some(perspective_parent_state_from_world_transform(
            pending.child_state.translation,
            pending.child_state.pivot_x,
            pending.child_state.pivot_y,
            pending.child_state.pivot_comp_scale,
            pending.child_state.rotation_deg,
            pending.child_state.effective_scale,
            pending.child_state.z,
        ))
    }
}

use bevy::prelude::*;

use crate::animation::interpolation::interpolate_vec2;
use crate::animation::{AmPlayback, AmSdfShapeParent};

pub fn animate_size_system(
    playback: Res<AmPlayback>,
    parent_query: Query<(&crate::animation::AmAnimated, &Children), With<AmSdfShapeParent>>,
    mut sdf_query: Query<(
        &bevy::prelude::MeshMaterial2d<crate::sdf_material::SdfMaterial>,
        &mut crate::animation::AmSdfParams,
    )>,
    mut materials: ResMut<Assets<crate::sdf_material::SdfMaterial>>,
    mut sprite_query: Query<
        (&crate::animation::AmAnimated, &mut Sprite),
        Without<AmSdfShapeParent>,
    >,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (animated, children) in parent_query.iter() {
        if animated.size.keyframes.is_empty() && animated.size.value.is_none() {
            continue;
        }

        let local_time = animated.calc_local_time(global_time);
        if !animated.is_active(local_time) {
            continue;
        }

        let layer_time = animated.calc_layer_time(local_time);
        let Some(size) = interpolate_vec2(&animated.size, layer_time) else {
            continue;
        };
        let half_width = size[0].abs() / 2.0;
        let half_height = size[1].abs() / 2.0;

        for child in children.iter() {
            let Ok((material_handle, mut sdf_params)) = sdf_query.get_mut(child) else {
                continue;
            };
            sdf_params.base_half_width = half_width;
            sdf_params.base_half_height = half_height;

            let Some(material) = materials.get_mut(&material_handle.0) else {
                continue;
            };
            material.uniform_data.params.x = half_width;
            material.uniform_data.params.y = half_height;
        }
    }

    for (animated, mut sprite) in sprite_query.iter_mut() {
        if animated.size.keyframes.is_empty() && animated.size.value.is_none() {
            continue;
        }

        let local_time = animated.calc_local_time(global_time);
        if !animated.is_active(local_time) {
            continue;
        }

        let layer_time = animated.calc_layer_time(local_time);
        if let Some(size) = interpolate_vec2(&animated.size, layer_time) {
            sprite.custom_size = Some(Vec2::new(size[0].abs(), size[1].abs()));
        }
    }
}

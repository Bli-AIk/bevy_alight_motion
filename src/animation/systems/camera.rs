use bevy::prelude::*;

use crate::animation::interpolation::{interpolate_float, interpolate_vec3};
use crate::animation::{AmAnimated, AmCameraLayer, AmPlayback};

pub fn animate_am_camera_system(
    playback: Res<AmPlayback>,
    camera_query: Query<(&AmAnimated, &AmCameraLayer)>,
    pending_query: Query<&crate::scene::AmPendingLayers>,
    mut bevy_camera_query: Query<
        (&mut Transform, &mut Projection),
        (
            With<Camera2d>,
            Without<crate::effects::EmbedSceneRttCamera>,
            Without<crate::effects::LiftCompositeCameraMarker>,
        ),
    >,
) {
    if playback.force_stopped {
        return;
    }
    let global_time = playback.current_time_ms;

    for (animated, cam) in camera_query.iter() {
        let local_time = animated.calc_local_time(global_time);
        if !animated.is_active(local_time) {
            continue;
        }
        let layer_time = animated.calc_layer_time(local_time);

        let default_loc = [cam.scene_width / 2.0, cam.scene_height / 2.0, cam.base_z];
        let loc = interpolate_vec3(&animated.location, layer_time).unwrap_or(default_loc);
        let rotation_deg = interpolate_float(&animated.rotation, layer_time).unwrap_or(0.0);
        let fov_deg = interpolate_float(&cam.fov, layer_time).unwrap_or(60.0);

        let pan_x = loc[0] - cam.scene_width / 2.0;
        let pan_y = cam.scene_height / 2.0 - loc[1];

        let base_fov_rad = 60.0_f32.to_radians();
        let current_fov_rad = fov_deg.to_radians();
        let z_abs = loc[2].abs();
        let base_z_abs = cam.base_z.abs();
        let zoom =
            (z_abs * (current_fov_rad / 2.0).tan()) / (base_z_abs * (base_fov_rad / 2.0).tan());

        let fit_scale = pending_query
            .iter()
            .next()
            .map(|p| 1.0 / p.inv_fit_scale)
            .unwrap_or(1.0);

        for (mut transform, mut projection) in bevy_camera_query.iter_mut() {
            transform.translation.x = pan_x * fit_scale;
            transform.translation.y = pan_y * fit_scale;
            transform.rotation = Quat::from_rotation_z(-rotation_deg.to_radians());

            if let Projection::Orthographic(ref mut ortho) = *projection {
                ortho.scale = zoom;
            }
        }
    }
}

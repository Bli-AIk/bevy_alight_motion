//! Camera and visibility management for hibernated layer entities.
//!
//! When a layer goes out of its time range, it is "hibernated" — hidden and its
//! cameras disabled — instead of being fully despawned. This avoids the expensive
//! RTT texture recreation and shader pipeline recompilation at loop transitions.
//!
//! 休眠图层的相机与可见性管理。当图层超出时间范围时，将其"休眠"（隐藏并禁用相机）
//! 而不是完全销毁，从而避免循环转场时昂贵的 RTT 纹理重建和着色器管线重编译。

use bevy::prelude::*;

use crate::effects::EmbedSceneRtt;
use crate::effects::LiftCompositeCameraMarker;
use crate::gaussian_blur::BlurPassCamera;
use crate::scene::AmHibernated;

/// Disables cameras owned by hibernated entities and re-enables cameras for
/// woken entities. Runs after lifecycle `ApplyDeferred` so `AmHibernated`
/// insertions / removals are visible.
pub fn sync_hibernation_cameras_system(
    rtt_query: Query<(&EmbedSceneRtt, Has<AmHibernated>)>,
    blur_cam_query: Query<(Entity, &BlurPassCamera)>,
    composite_cam_query: Query<(Entity, &LiftCompositeCameraMarker)>,
    hibernated_query: Query<(), With<AmHibernated>>,
    mut camera_query: Query<&mut Camera>,
) {
    // Embed RTT cameras (EmbedSceneRtt on layer entity → camera_entity)
    for (rtt, hibernated) in rtt_query.iter() {
        let should_active = !hibernated;
        if let Ok(mut cam) = camera_query.get_mut(rtt.camera_entity)
            && cam.is_active != should_active
        {
            cam.is_active = should_active;
        }
    }

    // Blur pass cameras (BlurPassCamera on camera entity → parent_entity is layer)
    for (cam_entity, blur_cam) in blur_cam_query.iter() {
        let should_active = hibernated_query.get(blur_cam.parent_entity).is_err();
        if let Ok(mut cam) = camera_query.get_mut(cam_entity)
            && cam.is_active != should_active
        {
            cam.is_active = should_active;
        }
    }

    // Lift composite cameras (marker on camera entity → owner_entity is layer)
    for (cam_entity, marker) in composite_cam_query.iter() {
        let should_active = hibernated_query.get(marker.owner_entity).is_err();
        if let Ok(mut cam) = camera_query.get_mut(cam_entity)
            && cam.is_active != should_active
        {
            cam.is_active = should_active;
        }
    }
}

/// Forces `Visibility::Hidden` on hibernated entities after animation systems
/// may have overridden it. Runs at the end of the animation phase.
pub fn enforce_hibernation_visibility_system(
    mut query: Query<&mut Visibility, With<AmHibernated>>,
) {
    for mut vis in query.iter_mut() {
        if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
    }
}

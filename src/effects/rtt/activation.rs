use bevy::prelude::*;

use super::components::PendingCameraActivation;

/// Per-frame budget for activating deferred cameras (embed, blur, composite).
/// All RTT cameras are created with `is_active: false` and activated here in
/// batches to spread shader compilation and `prepare_view_targets` costs across
/// multiple frames.
///
/// 每帧激活的延迟相机预算（embed、blur、composite 全部类型）。
/// 所有 RTT 相机均以 `is_active: false` 创建，此系统分批启用。
const CAMERA_ACTIVATION_BUDGET: usize = 16;

/// Activates pending cameras in budget-controlled batches.
///
/// Runs in the lifecycle chain so that cameras created in frame N are first
/// eligible for activation in frame N+1. Covers embed scene cameras, blur
/// pass cameras, and lift composite cameras — all marked with
/// `PendingCameraActivation`.
///
/// 按预算分批激活待处理的相机。在生命周期链中运行，覆盖所有 RTT 相机类型。
pub fn activate_rtt_cameras_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Camera), With<PendingCameraActivation>>,
) {
    for (idx, (entity, mut camera)) in query.iter_mut().enumerate() {
        if idx >= CAMERA_ACTIVATION_BUDGET {
            break;
        }
        camera.is_active = true;
        commands.entity(entity).remove::<PendingCameraActivation>();
    }
}

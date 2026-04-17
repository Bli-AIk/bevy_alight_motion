use bevy::prelude::*;

use super::components::{EmbedSceneRtt, PendingRttCameraActivation};

/// Per-frame budget for activating deferred RTT cameras.
/// Cameras are created inactive by `setup_embed_scene_rtt_system` and enabled
/// here in batches so shader compilation and `prepare_view_targets` spikes
/// don't overlap with texture creation.
///
/// 每帧激活的延迟 RTT 相机预算。
/// 相机由 setup 系统以 `is_active: false` 创建，此系统分批启用，
/// 避免着色器编译尖峰与纹理创建重叠。
const RTT_CAMERA_ACTIVATION_BUDGET: usize = 4;

/// Activates pending RTT cameras in budget-controlled batches.
///
/// Runs *before* the setup system in the lifecycle chain so that cameras
/// created in frame N are first eligible for activation in frame N+1.
/// This 1-frame stagger separates texture preparation costs from rendering
/// costs, cutting the worst-frame spike roughly in half.
///
/// 按预算分批激活待处理的 RTT 相机。在 setup 系统之前运行，
/// 使帧 N 创建的相机在帧 N+1 才会被激活，将纹理准备和渲染成本分离。
pub fn activate_rtt_cameras_system(
    mut commands: Commands,
    query: Query<(Entity, &EmbedSceneRtt), With<PendingRttCameraActivation>>,
    mut camera_query: Query<&mut Camera>,
) {
    for (idx, (entity, rtt)) in query.iter().enumerate() {
        if idx >= RTT_CAMERA_ACTIVATION_BUDGET {
            break;
        }
        if let Ok(mut camera) = camera_query.get_mut(rtt.camera_entity) {
            camera.is_active = true;
        }
        commands
            .entity(entity)
            .remove::<PendingRttCameraActivation>();
    }
}

//! Defines the components that mark and configure RTT-backed embed scenes.
//! It captures the render texture, camera association, layer allocation, scene
//! bounds, and the transient “needs setup/evaluation” markers that drive the RTT
//! lifecycle systems.
//!
//! 定义了 RTT 驱动的嵌套场景所使用的组件。它描述渲染纹理、关联相机、
//! layer 分配、场景边界，以及驱动 RTT 生命周期系统的那些“等待设置/等待评估”
//! 过渡标记。

use bevy::prelude::*;

#[derive(Component)]
pub struct EmbedSceneRtt {
    pub render_texture: Handle<Image>,
    pub camera_entity: Entity,
    pub capture_root: Option<Entity>,
    pub render_layer: usize,
    pub scene_width: f32,
    pub scene_height: f32,
    pub dynamic_resolution: bool,
}

#[derive(Component)]
pub struct EmbedSceneRttCamera {
    pub embed_entity: Entity,
    pub render_layer: usize,
}

#[derive(Component, Debug, Clone)]
pub struct EmbedSceneRttCaptureRoot {
    pub embed_entity: Entity,
}

#[derive(Component, Debug, Clone)]
pub struct EmbedSceneBounds {
    pub width: f32,
    pub height: f32,
}

#[derive(Component)]
pub struct NeedsEmbedSceneRtt {
    pub scene_width: f32,
    pub scene_height: f32,
    pub render_plan: crate::effects::EmbedSceneRenderPlan,
}

#[derive(Component)]
pub struct NeedsStrategyEvaluation {
    pub scene_width: f32,
    pub scene_height: f32,
    pub has_scale_animation: bool,
    pub render_plan: crate::effects::EmbedSceneRenderPlan,
}

/// Marker placed **on camera entities** that were created with `is_active: false`.
/// The activation system enables cameras gradually to avoid shader-compilation +
/// rendering spikes during loading / loop transitions.
/// Applied to embed scene cameras, blur pass cameras, and lift composite cameras.
///
/// 放置在**相机实体**上的标记。激活系统按预算逐帧启用相机，
/// 避免加载/循环切换时的着色器编译和渲染尖峰。
/// 用于 embed 场景相机、模糊 pass 相机和 lift composite 相机。
#[derive(Component)]
pub struct PendingCameraActivation;

#[derive(Component, Debug, Clone)]
pub struct AmEmbedMask;

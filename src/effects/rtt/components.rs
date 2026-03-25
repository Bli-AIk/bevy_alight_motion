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
    pub render_layer: u8,
    pub scene_width: f32,
    pub scene_height: f32,
    pub dynamic_resolution: bool,
}

#[derive(Component)]
pub struct EmbedSceneRttCamera {
    pub embed_entity: Entity,
    pub render_layer: u8,
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
    pub dynamic_resolution: bool,
}

#[derive(Component)]
pub struct NeedsStrategyEvaluation {
    pub scene_width: f32,
    pub scene_height: f32,
    pub has_scale_animation: bool,
    pub requires_composite: bool,
    pub dynamic_resolution: bool,
}

#[derive(Component, Debug, Clone)]
pub struct AmEmbedMask;

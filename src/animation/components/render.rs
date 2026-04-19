//! Defines render-adjacent runtime components created during spawning.
//! It covers cached SDF geometry parameters, camera-layer animation data, and
//! the bookkeeping used by path-repeat copies, all of which sit close to the
//! actual rendering path but are still updated from animation state.
//!
//! 定义了生成阶段创建的、紧贴渲染路径的运行时组件。它包含缓存后的 SDF
//! 几何参数、相机图层动画数据，以及 path repeat 副本使用的状态记录；这些数据
//! 虽然靠近最终渲染，但仍然由动画状态持续驱动更新。

use bevy::prelude::*;

use super::animated::AmAnimated;
use crate::schema::AmAnimatedFloat;

/// Component to store SDF shape parameters for animation.
#[derive(Component, Debug, Clone)]
pub struct AmSdfParams {
    pub base_half_width: f32,
    pub base_half_height: f32,
    pub stroke_width: f32,
    pub packed_stroke: f32,
    pub base_stroke_alpha: f32,
    pub base_pivot_x: f32,
    pub base_pivot_y: f32,
    pub border2_width: f32,
    pub border2_packed_color: f32,
    pub border2_mode: f32,
    pub spawn_frame_half: f32,
}

#[derive(Component, Debug, Clone)]
pub struct AmSdfFillParams {
    pub base_half_width: f32,
    pub base_half_height: f32,
    pub stroke_half_width: f32,
}

#[derive(Component, Debug, Clone)]
pub struct AmSdfStrokeParams {
    pub base_half_width: f32,
    pub base_half_height: f32,
    pub stroke_half_width: f32,
}

#[derive(Component, Debug, Clone, Default)]
pub struct AmSdfShapeParent;

#[derive(Component, Debug, Clone)]
pub struct AmCameraLayer {
    pub fov: AmAnimatedFloat,
    pub base_z: f32,
    pub scene_width: f32,
    pub scene_height: f32,
}

#[derive(Component, Debug)]
pub struct AmPathRepeat {
    pub source_entity: Entity,
    pub copy_entities: Vec<Entity>,
    pub source_shape_type: String,
    pub source_layer_id: u64,
    pub source_animated: AmAnimated,
}

//! Defines project-root components and spawn-time project state.
//! 定义工程根实体组件以及生成阶段的工程状态。
//!
//! These components sit at the boundary between asset loading and scene spawning. They identify the
//! root entity for an imported project, hold pending collected layers, and track the helper
//! containers used for spawned layers, embed contents, and RTT cameras.
//! 这些组件位于资源加载与场景生成的边界上。它们用于标识导入工程的根实体、保存尚未生成的图层集合，
//! 以及记录图层容器、嵌入内容容器和 RTT 相机容器等辅助实体状态。

use bevy::prelude::*;
use std::collections::HashMap;

use crate::loader::AmProject;

#[derive(Component, Debug, Clone, Default)]
pub struct AmEmbedContent {
    pub content_entities: Vec<Entity>,
}

#[derive(Component, Debug, Clone)]
pub struct AmEmbedContentMarker {
    pub embed_entity: Entity,
    pub embed_id: u64,
}

#[derive(Bundle)]
pub struct AmProjectBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub marker: AmProjectRoot,
}

#[derive(Component, Debug, Clone)]
pub struct AmProjectRoot {
    pub handle: Handle<AmProject>,
    pub spawned: bool,
}

#[derive(Component, Debug, Clone, Default)]
pub struct AmPendingLayers {
    pub layers: Vec<super::spawn::PendingLayer>,
    pub spawned_entities: HashMap<u64, Entity>,
    /// Entities hidden but kept alive with all RTT resources intact.
    /// Avoids costly destroy/recreate at loop transitions.
    pub hibernated_entities: HashMap<u64, Entity>,
    pub inv_fit_scale: f32,
    pub layers_container: Option<Entity>,
    pub embed_contents_container: Option<Entity>,
    pub rtt_cameras_container: Option<Entity>,
}

/// Marker on layer entities that are hibernated (hidden, cameras disabled).
/// Animation systems should not override visibility while this is present.
#[derive(Component, Debug, Clone)]
pub struct AmHibernated;

#[derive(Component, Debug, Clone, Default)]
pub struct AmLayersContainer;

#[derive(Component, Debug, Clone, Default)]
pub struct AmEmbedContentsContainer;

#[derive(Component, Debug, Clone, Default)]
pub struct AmRttCamerasContainer;

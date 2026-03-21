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
    pub inv_fit_scale: f32,
    pub layers_container: Option<Entity>,
    pub embed_contents_container: Option<Entity>,
    pub rtt_cameras_container: Option<Entity>,
}

#[derive(Component, Debug, Clone, Default)]
pub struct AmLayersContainer;

#[derive(Component, Debug, Clone, Default)]
pub struct AmEmbedContentsContainer;

#[derive(Component, Debug, Clone, Default)]
pub struct AmRttCamerasContainer;

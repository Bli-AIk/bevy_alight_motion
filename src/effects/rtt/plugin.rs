use bevy::prelude::*;

use crate::effects::{
    mark_dirty_on_change_system, setup_effect_buffers_system, update_effect_buffers_system,
};

use super::{
    EmbedSceneRenderLayerPool, apply_embed_bounds_clipping_system, cleanup_embed_content_system,
    cleanup_embed_scene_rtt_system,
};

pub struct EffectRenderPlugin;

impl Plugin for EffectRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EmbedSceneRenderLayerPool>()
            .add_systems(
                Update,
                (
                    setup_effect_buffers_system,
                    update_effect_buffers_system,
                    mark_dirty_on_change_system,
                    apply_embed_bounds_clipping_system,
                    cleanup_embed_scene_rtt_system,
                    cleanup_embed_content_system,
                ),
            );
    }
}

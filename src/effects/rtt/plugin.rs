//! This file defines the internal plugin that owns effect-buffer and RTT support.
//! It bundles setup, update, clipping, and cleanup systems related to composite
//! rendering so the main crate plugin can treat render-to-texture support as a
//! single dependency.
//!
//! 这个文件定义了负责特效缓冲区和 RTT 支持的内部插件。它把相关的 setup、update、
//! clipping 和 cleanup 系统打包起来，让主插件可以把 render-to-texture 支持当成
//! 一个整体依赖来接入。

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

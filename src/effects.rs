//! # effects.rs
//!
//! # 效果模块
//!
//! RTT (Render-to-Texture) Effect System for bevy_alight_motion.
//!
//! This module implements the Ping-Pong double buffering architecture for
//! stacking arbitrary effects on layers and groups.
//!
//! RTT 效果系统。使用乒乓双缓冲架构叠加效果。

mod rtt;
mod types;

// Re-export all public types
pub use types::{
    EffectLayer, EffectOutputTexture, EffectSourceTexture, EffectType, MaskParams, PingPongBuffer,
    StretchSegmentParams, WipeParams, mark_dirty_on_change_system, mask_params_to_vec4,
    setup_effect_buffers_system, stretch_params_to_vec4, update_effect_buffers_system,
    vec4_to_mask_params, vec4_to_stretch_params, vec4_to_wipe_params, wipe_params_to_vec4,
};

pub use rtt::{
    EffectRenderPlugin, EmbedSceneRenderLayerPool, EmbedSceneRtt, EmbedSceneRttCamera,
    NeedsEmbedSceneRtt, cleanup_embed_content_system, cleanup_embed_scene_rtt_system,
    debug_rtt_camera_projection_system, fix_nested_embed_render_layers_system,
    propagate_render_layers_system, propagate_render_layers_to_children_system,
    setup_embed_scene_rtt_system, sync_rtt_camera_position_system,
};

//! # rtt.rs
//!
//! # RTT 渲染模块 - 混合渲染管线架构
//!
//! Hybrid Rendering Pipeline for embed scenes and effects.
//! 嵌入场景和效果的混合渲染管线。
//!
//! ## Architecture Philosophy
//!
//! **Default Flat, Isolate on Demand** (默认扁平，按需隔离):
//! - By default, all content renders to Layer 0 (the main camera's layer)
//! - Only allocate separate RenderLayers when mathematically necessary
//! - Use Z-index sorting within shared layers for proper depth ordering
//!
//! ## Render Strategies
//!
//! 1. **Direct**: No isolation needed. Content inherits parent's layer.
//! 2. **Stencil**: Clipping via GPU stencil/scissor test, still on parent's layer.
//! 3. **Composite**: Full RTT isolation with dedicated RenderLayer.

mod cleanup;
mod clipping;
mod components;
mod layers;
mod plugin;
mod pool;
mod setup;
mod setup_helpers;
mod strategy;
mod sync;

pub(super) const EMBED_RTT_CAMERA_Z: f32 = 1000.0;
pub(super) const EMBED_RTT_CAMERA_NEAR: f32 = -1000.0;
pub(super) const EMBED_RTT_CAMERA_FAR: f32 = 2000.0;

pub use cleanup::{cleanup_embed_content_system, cleanup_embed_scene_rtt_system};
pub use clipping::apply_embed_bounds_clipping_system;
pub use components::{
    AmEmbedMask, EmbedSceneBounds, EmbedSceneRtt, EmbedSceneRttCamera, EmbedSceneRttCaptureRoot,
    NeedsEmbedSceneRtt, NeedsStrategyEvaluation,
};
pub use layers::{
    propagate_render_layers_system, propagate_render_layers_to_children_system,
    sync_new_sdf_child_render_layers_system,
};
pub use plugin::EffectRenderPlugin;
pub use pool::{EmbedSceneRenderLayerPool, RttSetupBudget};
pub(crate) use setup::refresh_group_fill_material_texture_system;
pub use setup::{fix_nested_embed_render_layers_system, setup_embed_scene_rtt_system};
pub use strategy::evaluate_render_strategy_system;
pub use sync::{sync_rtt_camera_position_system, sync_rtt_capture_root_system};

pub(super) use super::rtt_helpers::{
    EmbedVisibleRect, compute_embed_visible_rect, propagate_to_descendants, resize_render_texture,
    scene_local_rect, sync_dynamic_resolution_mesh, sync_dynamic_resolution_sprite,
    transformed_rect_edge_lengths,
};
pub(super) use super::types::*;

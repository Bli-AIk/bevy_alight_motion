//! # effects.rs
//!
//! # 效果模块 - 混合渲染管线
//!
//! Hybrid Rendering Pipeline for bevy_alight_motion.
//!
//! This module implements:
//! - Ping-Pong double buffering for stacking effects
//! - Hybrid Rendering Pipeline with Direct/Stencil/Composite strategies
//! - Dynamic RenderLayer pool management
//!
//! 混合渲染管线。支持无限层级嵌套。

pub(crate) mod lift_composite;
mod rtt;
mod types;

// Re-export all public types
#[allow(deprecated)]
pub use types::{
    AmGroupFill,
    EffectLayer,
    EffectOutputTexture,
    EffectSourceTexture,
    EffectType,
    GroupFillType,
    MaskParams,
    NeedsRenderStrategyEvaluation,
    PingPongBuffer,
    RenderHierarchyInfo,
    // Render strategy types for hybrid rendering pipeline
    RenderStrategy,
    StretchSegmentParams,
    WipeParams,
    mark_dirty_on_change_system,
    mask_params_to_vec4,
    setup_effect_buffers_system,
    stretch_params_to_vec4,
    update_effect_buffers_system,
    vec4_to_mask_params,
    vec4_to_stretch_params,
    vec4_to_wipe_params,
    wipe_params_to_vec4,
};

pub use rtt::{
    AmEmbedMask, EffectRenderPlugin, EmbedSceneBounds, EmbedSceneRenderLayerPool, EmbedSceneRtt,
    EmbedSceneRttCamera, NeedsEmbedSceneRtt, NeedsStrategyEvaluation,
    apply_embed_bounds_clipping_system, cleanup_embed_content_system,
    cleanup_embed_scene_rtt_system, evaluate_render_strategy_system,
    fix_nested_embed_render_layers_system, propagate_render_layers_system,
    propagate_render_layers_to_children_system, setup_embed_scene_rtt_system,
    sync_rtt_camera_position_system,
};

pub use lift_composite::{
    LiftCompositeCameraMarker, LiftCompositeState, propagate_lift_render_layers_system,
    setup_lift_composite_system, update_lift_comp_material_system,
};

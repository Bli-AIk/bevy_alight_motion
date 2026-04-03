//! Defines runtime-only helper types that do not come directly from the
//! authored project schema. It includes embed-scene retime metadata, live echo
//! bookkeeping, and marker/config types that later systems use to specialize how
//! visuals are updated.
//!
//! 定义了一批不直接来自作者侧 project schema 的运行时辅助类型。它包括
//! 嵌套场景的 retime 元数据、实时 echo 状态，以及后续系统用来细化视觉更新方式的
//! 标记与配置类型。

use bevy::prelude::*;

/// Retime mode for embedded scenes.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum RetimeMode {
    #[default]
    Off,
    Stretch,
    Freeze,
    Loop,
    LoopStretch,
    Blank,
}

impl RetimeMode {
    pub fn parse(s: &str) -> Self {
        match s {
            "stretch" => Self::Stretch,
            "freeze" => Self::Freeze,
            "loop" => Self::Loop,
            "loop-stretch" => Self::LoopStretch,
            "blank" => Self::Blank,
            _ => Self::Off,
        }
    }
}

/// Retime parameters for children of a retimed embed scene.
#[derive(Debug, Clone)]
pub struct AmRetimeInfo {
    pub mode: RetimeMode,
    pub embed_global_start: f32,
    pub container_duration_ms: f32,
    pub nested_total_time_ms: f32,
    pub embed_speed: f32,
    pub comparison_frame_center_bias_ms: f32,
    /// The embed's `inTime` offset in milliseconds.  For Freeze / Loop / Blank
    /// this shifts the starting position inside the nested timeline so that
    /// playback begins at `inTime` rather than 0.
    pub in_time_ms: f32,
}

/// Runtime echokf data for dynamically updating echo entities each frame.
#[derive(Component, Debug, Clone)]
pub struct AmEchoRuntime {
    pub echo_index: u32,
    pub max_count: u32,
    pub mode: i32,
    pub count_kf: crate::schema::AmAnimatedFloat,
    pub seconds_kf: crate::schema::AmAnimatedFloat,
    pub alpha_kf: crate::schema::AmAnimatedFloat,
    pub embed_start: f32,
    pub embed_end: f32,
    pub embed_time_offset: f32,
    pub embed_speed: f32,
}

/// Echo alpha config for entities in an echokf echo subtree.
#[derive(Debug, Clone)]
pub struct EchoAlphaConfig {
    pub alpha_keyframes: crate::schema::AmAnimatedFloat,
    pub fraction: f32,
    pub parent_start: i32,
    pub parent_end: i32,
    pub parent_time_offset: f32,
    pub parent_speed: f32,
}

/// Marker for unified-material visuals whose size should come from Transform.scale.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct AmUnifiedUsesTransformScale;

/// Last mesh bounds written by unified-effect mesh maintenance.
///
/// Unified effect layers often recompute their quad every frame, but many of
/// them are visually static for long stretches. Caching the last bounds/UV rect
/// lets us skip redundant mesh writes without changing rendering behavior.
#[derive(Component, Debug, Clone, Copy)]
pub struct AmUnifiedMeshState {
    pub bounds: [f32; 4],
    pub uv_rect: [f32; 4],
    pub initialized: bool,
}

impl Default for AmUnifiedMeshState {
    fn default() -> Self {
        Self {
            bounds: [0.0; 4],
            uv_rect: [0.0; 4],
            initialized: false,
        }
    }
}

impl AmUnifiedMeshState {
    const EPSILON: f32 = 0.0005;

    pub fn matches(&self, bounds: [f32; 4], uv_rect: [f32; 4]) -> bool {
        self.initialized
            && approx_rect_eq(self.bounds, bounds)
            && approx_rect_eq(self.uv_rect, uv_rect)
    }

    pub fn store(&mut self, bounds: [f32; 4], uv_rect: [f32; 4]) {
        self.bounds = bounds;
        self.uv_rect = uv_rect;
        self.initialized = true;
    }
}

fn approx_rect_eq(lhs: [f32; 4], rhs: [f32; 4]) -> bool {
    lhs.into_iter()
        .zip(rhs)
        .all(|(left, right)| (left - right).abs() <= AmUnifiedMeshState::EPSILON)
}

impl EchoAlphaConfig {
    pub fn evaluate(&self, global_time: f32) -> f32 {
        let parent_local = (global_time - self.parent_time_offset) * self.parent_speed;
        let parent_duration = (self.parent_end - self.parent_start) as f32;
        let parent_layer_time = if parent_duration > 0.0 {
            (parent_local - self.parent_start as f32) / parent_duration
        } else {
            0.0
        };
        let alpha_at_time = super::super::interpolation::interpolate_float(
            &self.alpha_keyframes,
            parent_layer_time,
        )
        .unwrap_or(1.0);
        alpha_at_time * (1.0 - self.fraction) + self.fraction
    }
}

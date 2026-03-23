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

//! Unified Effect Material for the RTT effect system.
//!
//! This material combines basic effects (mask, wipe, stretch) in a single shader pass.
//! It is part of the RTT architecture and optimized for common single-layer effect chains.
//!
//! For complex multi-pass scenarios (e.g., group effects), the RTT ping-pong buffer
//! system in `effects.rs` handles chaining multiple effect passes.

use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d},
};

/// Unified material supporting mask, wipe, and stretch segment effects.
///
/// Effect flags control which effects are active:
/// - `effect_flags.x > 0.5`: Mask enabled
/// - `effect_flags.y > 0.5`: Wipe enabled
/// - `effect_flags.z > 0.5`: Stretch segment enabled
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct UnifiedEffectMaterial {
    #[uniform(0)]
    pub color: LinearRgba,

    /// Effect flags: (mask, wipe, stretch, reserved)
    #[uniform(1)]
    pub effect_flags: Vec4,

    /// Mask: (center_x, center_y, half_width, half_height)
    #[uniform(2)]
    pub mask_params: Vec4,

    /// Wipe: (start, end, angle, feather)
    #[uniform(3)]
    pub wipe_params: Vec4,

    /// Stretch: (angle_rad, stretch_px, offset_px, smooth)
    #[uniform(4)]
    pub stretch_params: Vec4,

    /// Size: (orig_w, orig_h, mesh_w, mesh_h)
    #[uniform(5)]
    pub original_size: Vec4,

    /// Offset: (center_off_x, center_off_y, 0, 0)
    #[uniform(6)]
    pub mesh_offset: Vec4,

    #[texture(7)]
    #[sampler(8)]
    pub texture: Option<Handle<Image>>,
}

impl Default for UnifiedEffectMaterial {
    fn default() -> Self {
        Self {
            color: LinearRgba::WHITE,
            effect_flags: Vec4::ZERO,
            mask_params: Vec4::new(0.0, 0.0, 10000.0, 10000.0), // No clip
            wipe_params: Vec4::new(0.0, 1.0, 0.0, 0.0),         // Full visible
            stretch_params: Vec4::ZERO,
            original_size: Vec4::new(100.0, 100.0, 100.0, 100.0),
            mesh_offset: Vec4::ZERO,
            texture: None,
        }
    }
}

impl UnifiedEffectMaterial {
    pub fn new(texture: Handle<Image>, width: f32, height: f32) -> Self {
        Self {
            texture: Some(texture),
            original_size: Vec4::new(width, height, width, height),
            ..default()
        }
    }

    pub fn with_mask(mut self, cx: f32, cy: f32, hw: f32, hh: f32) -> Self {
        self.effect_flags.x = 1.0;
        self.mask_params = Vec4::new(cx, cy, hw, hh);
        self
    }

    pub fn with_wipe(mut self, start: f32, end: f32, angle: f32, feather: f32) -> Self {
        self.effect_flags.y = 1.0;
        self.wipe_params = Vec4::new(start, end, angle, feather);
        self
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_stretch_segment(
        mut self,
        angle: f32,
        stretch_px: f32,
        offset_px: f32,
        smooth: f32,
        mesh_w: f32,
        mesh_h: f32,
        off_x: f32,
        off_y: f32,
    ) -> Self {
        self.effect_flags.z = 1.0;
        self.stretch_params = Vec4::new(angle, stretch_px, offset_px, smooth);
        self.original_size.z = mesh_w;
        self.original_size.w = mesh_h;
        self.mesh_offset = Vec4::new(off_x, off_y, 0.0, 0.0);
        self
    }

    pub fn set_mask_enabled(&mut self, enabled: bool) {
        self.effect_flags.x = if enabled { 1.0 } else { 0.0 };
    }

    pub fn set_wipe_enabled(&mut self, enabled: bool) {
        self.effect_flags.y = if enabled { 1.0 } else { 0.0 };
    }

    pub fn set_stretch_enabled(&mut self, enabled: bool) {
        self.effect_flags.z = if enabled { 1.0 } else { 0.0 };
    }

    pub fn is_mask_enabled(&self) -> bool { self.effect_flags.x > 0.5 }
    pub fn is_wipe_enabled(&self) -> bool { self.effect_flags.y > 0.5 }
    pub fn is_stretch_enabled(&self) -> bool { self.effect_flags.z > 0.5 }
}

impl Material2d for UnifiedEffectMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/unified_effect.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// Marker for entities using the unified effect material
#[derive(Component, Default)]
pub struct UnifiedEffectMarker;

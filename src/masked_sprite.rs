//! Unified Effect Material for the RTT effect system.
//!
//! This material combines basic effects (mask, wipe, stretch, blur) in a single shader pass.
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

/// Unified material supporting mask, wipe, stretch segment, and blur effects.
///
/// Effect flags control which effects are active:
/// - `effect_flags.x > 0.5`: Mask enabled
/// - `effect_flags.y > 0.5`: Wipe enabled
/// - `effect_flags.z > 0.5`: Stretch segment enabled
/// - `effect_flags.w > 0.5`: Blur enabled
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct UnifiedEffectMaterial {
    #[uniform(0)]
    pub color: LinearRgba,

    /// Effect flags: (mask1_type, wipe, stretch, blur)
    /// mask1_type: 0=disabled, 1=rect, 2=ellipse, 3=rect exclude, 4=ellipse exclude
    #[uniform(1)]
    pub effect_flags: Vec4,

    /// Mask 1: (center_x, center_y, half_width, half_height)
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

    /// Blur: (strength, 0, 0, 0)
    #[uniform(9)]
    pub blur_params: Vec4,

    /// Palette map flags: (enabled, count, shades, alpha)
    #[uniform(10)]
    pub palette_flags: Vec4,

    /// Palette colors 1-4: each Vec4 contains (r, g, b, a) for one color
    #[uniform(11)]
    pub palette_color1: Vec4,
    #[uniform(12)]
    pub palette_color2: Vec4,
    #[uniform(13)]
    pub palette_color3: Vec4,
    #[uniform(14)]
    pub palette_color4: Vec4,
    #[uniform(15)]
    pub palette_color5: Vec4,
    #[uniform(16)]
    pub palette_color6: Vec4,
    #[uniform(17)]
    pub palette_color7: Vec4,
    #[uniform(18)]
    pub palette_color8: Vec4,
    
    /// Mask 2: (center_x, center_y, half_width, half_height)
    /// mask2_type is stored in mask2_flags.x
    #[uniform(19)]
    pub mask2_params: Vec4,
    
    /// Mask 2 flags: (mask2_type, 0, 0, 0)
    /// mask2_type: 0=disabled, 1=rect, 2=ellipse, 3=rect exclude, 4=ellipse exclude
    #[uniform(20)]
    pub mask2_flags: Vec4,

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
            blur_params: Vec4::ZERO,
            palette_flags: Vec4::ZERO,
            palette_color1: Vec4::ZERO,
            palette_color2: Vec4::ZERO,
            palette_color3: Vec4::ZERO,
            palette_color4: Vec4::ZERO,
            palette_color5: Vec4::ZERO,
            palette_color6: Vec4::ZERO,
            palette_color7: Vec4::ZERO,
            palette_color8: Vec4::ZERO,
            mask2_params: Vec4::new(0.0, 0.0, 10000.0, 10000.0), // No clip
            mask2_flags: Vec4::ZERO,
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

    pub fn with_blur(mut self, strength: f32) -> Self {
        self.effect_flags.w = 1.0;
        self.blur_params = Vec4::new(strength, 0.0, 0.0, 0.0);
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

    pub fn set_blur_enabled(&mut self, enabled: bool) {
        self.effect_flags.w = if enabled { 1.0 } else { 0.0 };
    }

    /// Set palette map effect parameters
    #[allow(clippy::too_many_arguments)]
    pub fn with_palette_map(
        mut self,
        count: u8,
        shades: bool,
        alpha: f32,
        colors: &[Vec4; 8],
    ) -> Self {
        self.palette_flags = Vec4::new(
            1.0,                            // enabled
            count as f32,                   // count (1-8)
            if shades { 1.0 } else { 0.0 }, // shades
            alpha,                          // alpha (effect strength)
        );
        self.palette_color1 = colors[0];
        self.palette_color2 = colors[1];
        self.palette_color3 = colors[2];
        self.palette_color4 = colors[3];
        self.palette_color5 = colors[4];
        self.palette_color6 = colors[5];
        self.palette_color7 = colors[6];
        self.palette_color8 = colors[7];
        self
    }

    pub fn set_palette_enabled(&mut self, enabled: bool) {
        self.palette_flags.x = if enabled { 1.0 } else { 0.0 };
    }

    pub fn set_palette_alpha(&mut self, alpha: f32) {
        self.palette_flags.w = alpha;
    }

    pub fn is_mask_enabled(&self) -> bool {
        self.effect_flags.x > 0.5
    }
    pub fn is_wipe_enabled(&self) -> bool {
        self.effect_flags.y > 0.5
    }
    pub fn is_stretch_enabled(&self) -> bool {
        self.effect_flags.z > 0.5
    }
    pub fn is_blur_enabled(&self) -> bool {
        self.effect_flags.w > 0.5
    }
    pub fn is_palette_enabled(&self) -> bool {
        self.palette_flags.x > 0.5
    }
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

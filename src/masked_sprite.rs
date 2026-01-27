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
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d},
};

/// Packed uniform data for unified effect material.
/// All effect parameters are combined into a single ShaderType struct to minimize binding count.
/// This reduces uniform buffer bindings from 23 to 1, ensuring compatibility with hardware
/// that limits uniform bindings to 15 per shader stage.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct UnifiedEffectUniform {
    /// Tint color (LinearRgba as vec4)
    pub color: Vec4,

    /// Effect flags: (mask1_type, wipe, stretch, blur)
    /// mask1_type: 0=disabled, 1=rect, 2=ellipse, 3=rect exclude, 4=ellipse exclude
    pub effect_flags: Vec4,

    /// Mask 1: (center_x, center_y, half_width, half_height)
    pub mask_params: Vec4,

    /// Wipe: (start, end, angle, feather)
    pub wipe_params: Vec4,

    /// Stretch: (angle_rad, stretch_px, offset_px, smooth)
    pub stretch_params: Vec4,

    /// Size: (orig_w, orig_h, mesh_w, mesh_h)
    pub original_size: Vec4,

    /// Offset: (center_off_x, center_off_y, 0, 0)
    pub mesh_offset: Vec4,

    /// Blur: (strength, 0, 0, 0)
    pub blur_params: Vec4,

    /// Palette map flags: (enabled, count, shades, alpha)
    pub palette_flags: Vec4,

    /// Palette colors 1-8
    pub palette_color1: Vec4,
    pub palette_color2: Vec4,
    pub palette_color3: Vec4,
    pub palette_color4: Vec4,
    pub palette_color5: Vec4,
    pub palette_color6: Vec4,
    pub palette_color7: Vec4,
    pub palette_color8: Vec4,

    /// Mask 2: (center_x, center_y, half_width, half_height)
    pub mask2_params: Vec4,

    /// Mask 2 flags: (mask2_type, mask1_rotation, mask2_rotation, 0)
    pub mask2_flags: Vec4,

    /// Replace color flags: (enabled, lock_luminance, 0, 0)
    pub replace_color_flags: Vec4,

    /// Replace color: old color to replace (r, g, b, a)
    pub replace_old_color: Vec4,

    /// Replace color: new replacement color (r, g, b, a)
    pub replace_new_color: Vec4,

    /// Replace color params: (threshold, feather, alpha, 0)
    pub replace_color_params: Vec4,
}

/// Unified material supporting mask, wipe, stretch segment, and blur effects.
///
/// Effect flags control which effects are active:
/// - `effect_flags.x > 0.5`: Mask enabled
/// - `effect_flags.y > 0.5`: Wipe enabled
/// - `effect_flags.z > 0.5`: Stretch segment enabled
/// - `effect_flags.w > 0.5`: Blur enabled
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct UnifiedEffectMaterial {
    /// All uniform data packed into a single binding
    #[uniform(0)]
    pub uniform_data: UnifiedEffectUniform,

    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,
}

// Proxy accessors for backward compatibility
impl UnifiedEffectMaterial {
    /// Get color
    pub fn color(&self) -> LinearRgba {
        LinearRgba::new(
            self.uniform_data.color.x,
            self.uniform_data.color.y,
            self.uniform_data.color.z,
            self.uniform_data.color.w,
        )
    }

    /// Set color
    pub fn set_color(&mut self, color: LinearRgba) {
        self.uniform_data.color = Vec4::new(color.red, color.green, color.blue, color.alpha);
    }

    /// Get effect_flags
    pub fn effect_flags(&self) -> Vec4 {
        self.uniform_data.effect_flags
    }

    /// Get mask_params
    pub fn mask_params(&self) -> Vec4 {
        self.uniform_data.mask_params
    }

    /// Get wipe_params
    pub fn wipe_params(&self) -> Vec4 {
        self.uniform_data.wipe_params
    }

    /// Get stretch_params
    pub fn stretch_params(&self) -> Vec4 {
        self.uniform_data.stretch_params
    }

    /// Get original_size
    pub fn original_size(&self) -> Vec4 {
        self.uniform_data.original_size
    }

    /// Get mesh_offset
    pub fn mesh_offset(&self) -> Vec4 {
        self.uniform_data.mesh_offset
    }

    /// Get blur_params
    pub fn blur_params(&self) -> Vec4 {
        self.uniform_data.blur_params
    }

    /// Get palette_flags
    pub fn palette_flags(&self) -> Vec4 {
        self.uniform_data.palette_flags
    }

    /// Get mask2_params
    pub fn mask2_params(&self) -> Vec4 {
        self.uniform_data.mask2_params
    }

    /// Get mask2_flags
    pub fn mask2_flags(&self) -> Vec4 {
        self.uniform_data.mask2_flags
    }

    /// Get replace_color_flags
    pub fn replace_color_flags(&self) -> Vec4 {
        self.uniform_data.replace_color_flags
    }

    /// Get replace_old_color
    pub fn replace_old_color(&self) -> Vec4 {
        self.uniform_data.replace_old_color
    }

    /// Get replace_new_color
    pub fn replace_new_color(&self) -> Vec4 {
        self.uniform_data.replace_new_color
    }

    /// Get replace_color_params
    pub fn replace_color_params(&self) -> Vec4 {
        self.uniform_data.replace_color_params
    }
}

impl Default for UnifiedEffectUniform {
    fn default() -> Self {
        Self {
            color: Vec4::new(1.0, 1.0, 1.0, 1.0), // WHITE
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
            replace_color_flags: Vec4::ZERO,
            replace_old_color: Vec4::ZERO,
            replace_new_color: Vec4::ZERO,
            replace_color_params: Vec4::ZERO,
        }
    }
}

impl Default for UnifiedEffectMaterial {
    fn default() -> Self {
        Self {
            uniform_data: UnifiedEffectUniform::default(),
            texture: None,
        }
    }
}

impl UnifiedEffectMaterial {
    pub fn new(texture: Handle<Image>, width: f32, height: f32) -> Self {
        Self {
            uniform_data: UnifiedEffectUniform {
                original_size: Vec4::new(width, height, width, height),
                ..default()
            },
            texture: Some(texture),
        }
    }

    pub fn with_mask(mut self, cx: f32, cy: f32, hw: f32, hh: f32) -> Self {
        self.uniform_data.effect_flags.x = 1.0;
        self.uniform_data.mask_params = Vec4::new(cx, cy, hw, hh);
        self
    }

    pub fn with_wipe(mut self, start: f32, end: f32, angle: f32, feather: f32) -> Self {
        self.uniform_data.effect_flags.y = 1.0;
        self.uniform_data.wipe_params = Vec4::new(start, end, angle, feather);
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
        self.uniform_data.effect_flags.z = 1.0;
        self.uniform_data.stretch_params = Vec4::new(angle, stretch_px, offset_px, smooth);
        self.uniform_data.original_size.z = mesh_w;
        self.uniform_data.original_size.w = mesh_h;
        self.uniform_data.mesh_offset = Vec4::new(off_x, off_y, 0.0, 0.0);
        self
    }

    pub fn with_blur(mut self, strength: f32) -> Self {
        self.uniform_data.effect_flags.w = 1.0;
        self.uniform_data.blur_params = Vec4::new(strength, 0.0, 0.0, 0.0);
        self
    }

    pub fn set_mask_enabled(&mut self, enabled: bool) {
        self.uniform_data.effect_flags.x = if enabled { 1.0 } else { 0.0 };
    }

    pub fn set_wipe_enabled(&mut self, enabled: bool) {
        self.uniform_data.effect_flags.y = if enabled { 1.0 } else { 0.0 };
    }

    pub fn set_stretch_enabled(&mut self, enabled: bool) {
        self.uniform_data.effect_flags.z = if enabled { 1.0 } else { 0.0 };
    }

    pub fn set_blur_enabled(&mut self, enabled: bool) {
        self.uniform_data.effect_flags.w = if enabled { 1.0 } else { 0.0 };
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
        self.uniform_data.palette_flags = Vec4::new(
            1.0,                            // enabled
            count as f32,                   // count (1-8)
            if shades { 1.0 } else { 0.0 }, // shades
            alpha,                          // alpha (effect strength)
        );
        self.uniform_data.palette_color1 = colors[0];
        self.uniform_data.palette_color2 = colors[1];
        self.uniform_data.palette_color3 = colors[2];
        self.uniform_data.palette_color4 = colors[3];
        self.uniform_data.palette_color5 = colors[4];
        self.uniform_data.palette_color6 = colors[5];
        self.uniform_data.palette_color7 = colors[6];
        self.uniform_data.palette_color8 = colors[7];
        self
    }

    pub fn set_palette_enabled(&mut self, enabled: bool) {
        self.uniform_data.palette_flags.x = if enabled { 1.0 } else { 0.0 };
    }

    pub fn set_palette_alpha(&mut self, alpha: f32) {
        self.uniform_data.palette_flags.w = alpha;
    }

    /// Set replace color effect parameters
    pub fn set_replace_color(
        &mut self,
        old_color: Vec4,
        new_color: Vec4,
        threshold: f32,
        feather: f32,
        alpha: f32,
        lock_luminance: bool,
    ) {
        self.uniform_data.replace_color_flags =
            Vec4::new(1.0, if lock_luminance { 1.0 } else { 0.0 }, 0.0, 0.0);
        self.uniform_data.replace_old_color = old_color;
        self.uniform_data.replace_new_color = new_color;
        self.uniform_data.replace_color_params = Vec4::new(threshold, feather, alpha, 0.0);
    }

    pub fn set_replace_color_enabled(&mut self, enabled: bool) {
        self.uniform_data.replace_color_flags.x = if enabled { 1.0 } else { 0.0 };
    }

    pub fn is_replace_color_enabled(&self) -> bool {
        self.uniform_data.replace_color_flags.x > 0.5
    }

    pub fn is_mask_enabled(&self) -> bool {
        self.uniform_data.effect_flags.x > 0.5
    }
    pub fn is_wipe_enabled(&self) -> bool {
        self.uniform_data.effect_flags.y > 0.5
    }
    pub fn is_stretch_enabled(&self) -> bool {
        self.uniform_data.effect_flags.z > 0.5
    }
    pub fn is_blur_enabled(&self) -> bool {
        self.uniform_data.effect_flags.w > 0.5
    }
    pub fn is_palette_enabled(&self) -> bool {
        self.uniform_data.palette_flags.x > 0.5
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

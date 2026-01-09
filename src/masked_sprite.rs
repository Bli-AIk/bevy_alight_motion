//! Masked sprite material for implementing AM mask layers and wipe effects.
//!
//! This module provides a custom Material2d that clips sprites to a rectangular mask region
//! and supports wipe effects for progressive reveal/hide animations.

use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d},
};

/// Custom material for sprites that need to be clipped by a mask or have wipe effects.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct MaskedSpriteMaterial {
    /// Tint color
    #[uniform(0)]
    pub color: LinearRgba,

    /// Mask parameters: (center_x, center_y, half_width, half_height)
    #[uniform(1)]
    pub mask_params: Vec4,

    /// Wipe parameters: (wipe_start, wipe_end, wipe_angle, wipe_feather)
    /// - wipe_start/end: 0.0-1.0 percentage of sprite to show
    /// - wipe_angle: angle in radians (0 = left-to-right)
    /// - wipe_feather: edge softness (0 = sharp, higher = softer)
    #[uniform(2)]
    pub wipe_params: Vec4,

    /// The sprite texture
    #[texture(3)]
    #[sampler(4)]
    pub texture: Option<Handle<Image>>,
}

impl Default for MaskedSpriteMaterial {
    fn default() -> Self {
        Self {
            color: LinearRgba::WHITE,
            mask_params: Vec4::new(0.0, 0.0, 10000.0, 10000.0), // Very large default mask (no clipping)
            wipe_params: Vec4::new(0.0, 1.0, 0.0, 0.0), // Default: no wipe (show everything)
            texture: None,
        }
    }
}

impl Material2d for MaskedSpriteMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/masked_sprite.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// Component to mark entities using masked sprite material
#[derive(Component)]
pub struct MaskedSpriteMarker;

//! Masked sprite material for implementing AM mask layers.
//!
//! This module provides a custom Material2d that clips sprites to a rectangular mask region.

use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d},
};

/// Custom material for sprites that need to be clipped by a mask.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct MaskedSpriteMaterial {
    /// Tint color
    #[uniform(0)]
    pub color: LinearRgba,

    /// Mask parameters: (center_x, center_y, half_width, half_height)
    #[uniform(1)]
    pub mask_params: Vec4,

    /// The sprite texture
    #[texture(2)]
    #[sampler(3)]
    pub texture: Option<Handle<Image>>,
}

impl Default for MaskedSpriteMaterial {
    fn default() -> Self {
        Self {
            color: LinearRgba::WHITE,
            mask_params: Vec4::new(0.0, 0.0, 10000.0, 10000.0), // Very large default mask (no clipping)
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

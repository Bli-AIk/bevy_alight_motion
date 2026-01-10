//! Masked sprite material for implementing AM mask layers and wipe effects.
//!
//! This module provides a custom Material2d that clips sprites to a rectangular mask region
//! and supports wipe effects for progressive reveal/hide animations.
//!
//! Also provides StretchSegmentMaterial for the stretch segment UV distortion effect.

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

/// Custom material for sprites with stretch segment UV distortion effect.
///
/// This implements the "拉伸片段" (Stretch Segment) effect from Alight Motion,
/// which creates a UV domain distortion along a configurable split line.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct StretchSegmentMaterial {
    /// Tint color
    #[uniform(0)]
    pub color: LinearRgba,

    /// Stretch segment parameters: (angle_radians, stretch_amount_uv, offset_uv, smooth_width)
    /// - angle_radians: rotation of the split line (0 = vertical, PI/2 = horizontal)
    /// - stretch_amount_uv: stretch amount in UV units (relative to original texture)
    /// - offset_uv: position of the split line (UV units)
    /// - smooth_width: width of the smooth transition zone (0 = hard edge)
    #[uniform(1)]
    pub stretch_params: Vec4,

    /// Original texture size: (width, height, mesh_width, mesh_height)
    /// - xy: original texture dimensions for UV conversion
    /// - zw: expanded mesh dimensions for coordinate calculation
    #[uniform(2)]
    pub original_size: Vec4,

    /// The sprite texture
    #[texture(3)]
    #[sampler(4)]
    pub texture: Option<Handle<Image>>,
}

impl Default for StretchSegmentMaterial {
    fn default() -> Self {
        Self {
            color: LinearRgba::WHITE,
            // Default: no stretch effect (stretch=0, smooth=0)
            stretch_params: Vec4::new(0.0, 0.0, 0.0, 0.0),
            // Default original size (will be set properly during spawn)
            original_size: Vec4::new(100.0, 100.0, 0.0, 0.0),
            texture: None,
        }
    }
}

impl Material2d for StretchSegmentMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/stretch_segment.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// Component to mark entities using stretch segment material
#[derive(Component)]
pub struct StretchSegmentMarker;

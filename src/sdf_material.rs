//! Custom SDF Material for Alight Motion shapes.
//!
//! This module provides a custom Material2d implementation for rendering SDF shapes
//! (rectangles, circles, ellipses) with strokes.

use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d},
};

mod color;
mod components;
mod constructors;
mod shape_type;
#[cfg(test)]
mod tests;
mod uniform;

pub use color::{pack_color, repack_with_alpha};
pub use components::{AmSdfShapeComponent, SdfShapeMarker};
pub use shape_type::SdfShapeType;
pub use uniform::SdfMaterialUniform;

/// Custom SDF Material for rendering shapes with optional strokes.
///
/// Params layout:
/// - `params.x`: half_width (for box) or radius_x (for circle/ellipse)
/// - `params.y`: half_height (for box) or radius_y (for circle/ellipse)
/// - `params.z`: stroke_width
/// - `params.w`: packed stroke color (RGBA as u32 bits stored in f32)
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct SdfMaterial {
    /// Combined uniform data
    #[uniform(0)]
    pub uniform_data: SdfMaterialUniform,
}

impl Material2d for SdfMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/sdf_shape.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

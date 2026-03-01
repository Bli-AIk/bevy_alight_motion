//! # group_fill.rs
//!
//! # 编组填充材质
//!
//! Material2d for rendering group fill (solid color / gradient) using RTT alpha mask.
//! 使用RTT alpha蒙版渲染编组填充（纯色/渐变）的Material2d。

use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d},
};

/// Uniform data for group fill material.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct GroupFillUniform {
    /// Fill color (linear RGBA) - used for solid color fill.
    pub fill_color: Vec4,
    /// Gradient config: (type, 0, 0, 0)
    /// type: 0=solid color, 1=linear, 2=radial, 3=sweep
    pub gradient_config: Vec4,
    /// Gradient start color (linear RGBA).
    pub gradient_start_color: Vec4,
    /// Gradient end color (linear RGBA).
    pub gradient_end_color: Vec4,
    /// Gradient points: (start_x, start_y, end_x, end_y) in UV [0,1] space.
    pub gradient_points: Vec4,
}

impl Default for GroupFillUniform {
    fn default() -> Self {
        Self {
            fill_color: Vec4::ONE,
            gradient_config: Vec4::ZERO,
            gradient_start_color: Vec4::ZERO,
            gradient_end_color: Vec4::ZERO,
            gradient_points: Vec4::ZERO,
        }
    }
}

/// Material for rendering group fill using RTT texture as alpha mask.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct GroupFillMaterial {
    #[uniform(0)]
    pub uniform_data: GroupFillUniform,

    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,
}

impl Material2d for GroupFillMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/group_fill.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

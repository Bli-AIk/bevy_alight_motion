//! # sdf.rs
//!
//! # sdf.rs 文件
//!
//! ## Module Overview
//!
//! ## 模块概述
//!
//! SDF (Signed Distance Field) shape rendering module.
//!
//! SDF（有向距离场）形状渲染模块。
//!
//! ## Source File Overview
//!
//! ## 源文件概述
//!
//! This file provides SDF-based rendering for AM shapes, matching AM's stroke behavior
//! where stroke width stays constant during scale animations.
//!
//! 本文件提供基于 SDF 的 AM 形状渲染，匹配 AM 的描边行为，即在缩放动画期间描边宽度保持不变。
//!
//! ## Design Philosophy (matching AM behavior)
//!
//! ## 设计理念（匹配 AM 行为）
//!
//! AM renders stroked rectangles by:
//! 1. Drawing a base shape with a stroke
//! 2. Applying scale to change dimensions (stroke width stays constant)
//!
//! AM 渲染带描边的矩形的方式：
//! 1. 绘制带描边的基础形状
//! 2. 应用缩放来改变尺寸（描边宽度保持不变）
//!
//! We achieve this by:
//! 1. Using custom SdfMaterial that passes dimensions as uniform parameters
//! 2. Using different SDF formulas for different corner styles (round/miter/bevel)
//! 3. Passing stroke_width as a parameter to keep it constant during scaling
//!
//! 我们通过以下方式实现：
//! 1. 使用自定义 SdfMaterial 将尺寸作为 uniform 参数传递
//! 2. 对不同的角落样式（圆角/尖角/斜角）使用不同的 SDF 公式
//! 3. 将 stroke_width 作为参数传递以在缩放期间保持不变

use bevy::prelude::*;

// Re-export commonly used items from sdf_material
pub use crate::sdf_material::{SdfMaterial, SdfShapeType, pack_color, repack_with_alpha};

/// Base half-extent for SDF shapes (AM uses 100x100 base square -> 50x50 half-extent)
///
/// SDF 形状的基础半尺寸（AM 使用 100x100 基础正方形 -> 50x50 半尺寸）
pub const BASE_HALF_EXTENT: f32 = 50.0;

/// Legacy resource placeholder for shader handles.
/// This is now deprecated as we use SdfMaterial directly.
/// Kept for compatibility with scene.rs during transition.
#[derive(Resource, Default)]
pub struct AmSdfShaders {
    pub stroked_fill_box: Option<()>,
    pub stroked_fill_box_miter: Option<()>,
    pub stroked_fill_box_bevel: Option<()>,
    pub stroked_fill_circle: Option<()>,
}

/// System to handle shader hot-reload (placeholder for future implementation).
#[cfg(feature = "debug")]
pub fn hot_reload_shader_system(_keyboard: Res<ButtonInput<KeyCode>>) {
    // Hot reload not implemented for custom SDF material yet
}

/// No-op hot-reload system when debug feature is disabled.
#[cfg(not(feature = "debug"))]
pub fn hot_reload_shader_system() {
    // Intentionally empty - hot-reload only available in debug builds
}

/// Component for AM SDF shapes that need special animation handling.
/// Deprecated: Use sdf_material::AmSdfShapeComponent instead.
#[derive(Component, Debug, Clone)]
pub struct AmSdfShape {
    /// Fill color of the shape.
    pub fill_color: Color,
    /// Stroke color (if any).
    pub stroke_color: Option<Color>,
    /// Stroke width in pixels.
    pub stroke_width: f32,
    /// Corner radius for rounded rectangles.
    pub corner_radius: f32,
    /// Original width of the shape (before scale).
    pub width: f32,
    /// Original height of the shape (before scale).
    pub height: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_color() {
        // Test white (allow ±1 tolerance due to floating point precision)
        let white = Color::WHITE;
        let packed = pack_color(white);
        let bits = packed.to_bits();
        assert!((bits >> 24) >= 254, "R should be ~255"); // R
        assert!(((bits >> 16) & 0xFF) >= 254, "G should be ~255"); // G
        assert!(((bits >> 8) & 0xFF) >= 254, "B should be ~255"); // B
        assert!((bits & 0xFF) >= 254, "A should be ~255"); // A

        // Test red
        let red = Color::srgba(1.0, 0.0, 0.0, 1.0);
        let packed = pack_color(red);
        let bits = packed.to_bits();
        assert!((bits >> 24) >= 254, "R should be ~255"); // R
        assert_eq!((bits >> 16) & 0xFF, 0); // G
        assert_eq!((bits >> 8) & 0xFF, 0); // B
        assert!((bits & 0xFF) >= 254, "A should be ~255"); // A
    }
}

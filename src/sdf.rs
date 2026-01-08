//! SDF (Signed Distance Field) shape rendering module.
//!
//! This module provides SDF-based rendering for AM shapes using bevy_smud.
//! 
//! ## Design Philosophy (matching AM behavior)
//! 
//! AM renders stroked rectangles by:
//! 1. Drawing a base shape with a stroke
//! 2. Applying scale to change dimensions (stroke width stays constant)
//!
//! We achieve this by:
//! 1. Using a parametric SDF box that reads dimensions from params
//! 2. Using Chebyshev distance for sharp corners (cap="square", join="miter")
//! 3. Passing stroke_width as a parameter to keep it constant
//!
//! ## Shader Files
//! The fill shader source is in: `assets/shaders/stroked_fill.wgsl`
//! Edit that file to adjust rendering behavior (requires recompilation).

use bevy::asset::Assets;
use bevy::prelude::*;
use bevy_smud::prelude::*;

/// Base half-extent for SDF shapes (AM uses 100x100 base square -> 50x50 half-extent)
pub const BASE_HALF_EXTENT: f32 = 50.0;

/// SDF expression for a parametric box that reads dimensions from params.x and params.y.
/// This allows non-uniform scaling without Transform, which bevy_smud doesn't support.
pub const PARAMETRIC_BOX_SDF: &str = "smud::sd_box(p, vec2<f32>(params.x, params.y))";

/// Resource to hold SDF shader handles.
#[derive(Resource, Default)]
pub struct AmSdfShaders {
    /// Handle to the base box SDF shader (fixed 50x50 half-extent).
    pub base_box_sdf: Option<Handle<Shader>>,
    /// Handle to the stroked fill shader (loaded from file at compile time).
    pub stroked_fill: Option<Handle<Shader>>,
}

/// Stroked fill shader source, loaded from file at compile time.
/// Edit `assets/shaders/stroked_fill.wgsl` to modify the shader.
/// Note: Changes require recompilation to take effect.
pub const STROKED_FILL_SHADER: &str = include_str!("../assets/shaders/stroked_fill.wgsl");

/// Initialize SDF shaders resource on startup.
pub fn setup_sdf_shaders(
    mut commands: Commands,
    mut shaders: ResMut<Assets<Shader>>,
) {
    // Create a fixed-size box SDF (50x50 half-extent, 100x100 total)
    let base_box_sdf = shaders.add_sdf_expr(
        format!("smud::sd_box(p, vec2<f32>({0}, {0}))", BASE_HALF_EXTENT)
    );
    
    // Create stroked fill shader from file content (embedded at compile time)
    // The shader source is in assets/shaders/stroked_fill.wgsl for easy editing
    let stroked_fill = shaders.add_fill_body(STROKED_FILL_SHADER);
    
    commands.insert_resource(AmSdfShaders {
        base_box_sdf: Some(base_box_sdf),
        stroked_fill: Some(stroked_fill),
    });
}

/// Pack RGBA color into a u32 stored as f32 bits.
/// Format: 0xRRGGBBAA
pub fn pack_color(color: Color) -> f32 {
    let rgba = color.to_srgba();
    let r = (rgba.red * 255.0) as u32;
    let g = (rgba.green * 255.0) as u32;
    let b = (rgba.blue * 255.0) as u32;
    let a = (rgba.alpha * 255.0) as u32;
    let packed = (r << 24) | (g << 16) | (b << 8) | a;
    f32::from_bits(packed)
}

/// Create a parametric box SDF that uses params for dimensions.
/// The shader reads params.x as half_width and params.y as half_height.
/// This allows dynamic resizing without recreating the shader.
pub fn create_parametric_box_sdf(
    shaders: &mut Assets<Shader>,
) -> Handle<Shader> {
    shaders.add_sdf_expr(
        "smud::sd_box(p, vec2<f32>(params.x, params.y))"
    )
}

/// Component for AM SDF shapes that need special animation handling.
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
    fn test_sdf_shaders_resource() {
        // Just verify the resource struct can be created
        let shaders = AmSdfShaders::default();
        // Resource now holds optional shader handles
        assert!(shaders.base_box_sdf.is_none());
        assert!(shaders.stroked_fill.is_none());
    }
    
    #[test]
    fn test_pack_color() {
        // Test white
        let white = Color::WHITE;
        let packed = pack_color(white);
        let bits = packed.to_bits();
        assert_eq!(bits >> 24, 255); // R
        assert_eq!((bits >> 16) & 0xFF, 255); // G
        assert_eq!((bits >> 8) & 0xFF, 255); // B
        assert_eq!(bits & 0xFF, 255); // A
        
        // Test red
        let red = Color::srgba(1.0, 0.0, 0.0, 1.0);
        let packed = pack_color(red);
        let bits = packed.to_bits();
        assert_eq!(bits >> 24, 255); // R
        assert_eq!((bits >> 16) & 0xFF, 0); // G
        assert_eq!((bits >> 8) & 0xFF, 0); // B
        assert_eq!(bits & 0xFF, 255); // A
    }
}

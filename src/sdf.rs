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
//! Edit that file and press 'R' in the player window to hot-reload the shader.

use bevy::asset::Assets;
use bevy::prelude::*;
use bevy_smud::prelude::*;
use std::path::PathBuf;

/// Base half-extent for SDF shapes (AM uses 100x100 base square -> 50x50 half-extent)
pub const BASE_HALF_EXTENT: f32 = 50.0;

/// SDF expression for a parametric box that reads dimensions from params.x and params.y.
/// This allows non-uniform scaling without Transform, which bevy_smud doesn't support.
pub const PARAMETRIC_BOX_SDF: &str = "smud::sd_box(p, vec2<f32>(params.x, params.y))";

/// SDF expression for a parametric circle that reads radius from params.x.
pub const PARAMETRIC_CIRCLE_SDF: &str = "smud::sd_circle(p, params.x)";

/// Relative path to the stroked fill shader file (from assets folder)
pub const STROKED_FILL_BOX_FILENAME: &str = "shaders/stroked_fill_box.wgsl";
pub const STROKED_FILL_BOX_MITER_FILENAME: &str = "shaders/stroked_fill_box_miter.wgsl";
pub const STROKED_FILL_BOX_BEVEL_FILENAME: &str = "shaders/stroked_fill_box_bevel.wgsl";
pub const STROKED_FILL_CIRCLE_FILENAME: &str = "shaders/stroked_fill_circle.wgsl";

/// Resource to hold SDF shader handles.
#[derive(Resource)]
pub struct AmSdfShaders {
    /// Handle to the base box SDF shader (fixed 50x50 half-extent).
    pub base_box_sdf: Option<Handle<Shader>>,
    /// Handle to the stroked fill shader for Box (Round join).
    pub stroked_fill_box: Option<Handle<Shader>>,
    /// Handle to the stroked fill shader for Box (Miter/Square join).
    pub stroked_fill_box_miter: Option<Handle<Shader>>,
    /// Handle to the stroked fill shader for Box (Bevel join).
    pub stroked_fill_box_bevel: Option<Handle<Shader>>,
    /// Handle to the stroked fill shader for Circle.
    pub stroked_fill_circle: Option<Handle<Shader>>,
    /// Path to the shader file for hot-reload
    pub shader_file_path: Option<PathBuf>,
}

impl Default for AmSdfShaders {
    fn default() -> Self {
        Self {
            base_box_sdf: None,
            stroked_fill_box: None,
            stroked_fill_box_miter: None,
            stroked_fill_box_bevel: None,
            stroked_fill_circle: None,
            shader_file_path: None,
        }
    }
}

/// Stroked fill shader source, loaded from file at compile time (fallback).
pub const STROKED_FILL_BOX_DEFAULT: &str = include_str!("../assets/shaders/stroked_fill_box.wgsl");
pub const STROKED_FILL_BOX_MITER_DEFAULT: &str =
    include_str!("../assets/shaders/stroked_fill_box_miter.wgsl");
pub const STROKED_FILL_BOX_BEVEL_DEFAULT: &str =
    include_str!("../assets/shaders/stroked_fill_box_bevel.wgsl");
pub const STROKED_FILL_CIRCLE_DEFAULT: &str =
    include_str!("../assets/shaders/stroked_fill_circle.wgsl");

/// Initialize SDF shaders resource on startup.
pub fn setup_sdf_shaders(mut commands: Commands, mut shaders: ResMut<Assets<Shader>>) {
    // Create a fixed-size box SDF (50x50 half-extent, 100x100 total)
    let base_box_sdf = shaders.add_sdf_expr(format!(
        "smud::sd_box(p, vec2<f32>({0}, {0}))",
        BASE_HALF_EXTENT
    ));

    // Try to find the shader file path for hot-reload (debug feature only)
    #[cfg(feature = "debug")]
    let shader_file_path = find_shader_file_path();
    #[cfg(not(feature = "debug"))]
    let shader_file_path: Option<PathBuf> = None;

    // Load shader content
    let box_content = STROKED_FILL_BOX_DEFAULT.to_string();
    let box_miter_content = STROKED_FILL_BOX_MITER_DEFAULT.to_string();
    let box_bevel_content = STROKED_FILL_BOX_BEVEL_DEFAULT.to_string();
    let circle_content = STROKED_FILL_CIRCLE_DEFAULT.to_string();

    let stroked_fill_box = shaders.add_fill_body(box_content);
    let stroked_fill_box_miter = shaders.add_fill_body(box_miter_content);
    let stroked_fill_box_bevel = shaders.add_fill_body(box_bevel_content);
    let stroked_fill_circle = shaders.add_fill_body(circle_content);

    commands.insert_resource(AmSdfShaders {
        base_box_sdf: Some(base_box_sdf),
        stroked_fill_box: Some(stroked_fill_box),
        stroked_fill_box_miter: Some(stroked_fill_box_miter),
        stroked_fill_box_bevel: Some(stroked_fill_box_bevel),
        stroked_fill_circle: Some(stroked_fill_circle),
        shader_file_path,
    });
}

/// Find the shader file path by searching common locations (debug feature only).
#[cfg(feature = "debug")]
fn find_shader_file_path() -> Option<PathBuf> {
    // Legacy logic, effectively unused now but kept for compilation
    None
}

/// System to handle shader hot-reload when 'F5' key is pressed (debug feature only).
#[cfg(feature = "debug")]
pub fn hot_reload_shader(
    _keyboard: Res<ButtonInput<KeyCode>>,
    _shaders: ResMut<Assets<Shader>>,
    _sdf_shaders: ResMut<AmSdfShaders>,
    _smud_shapes: Query<&mut SmudShape>,
) {
    // Hot reload temporarily disabled due to shader split
}

/// No-op hot-reload system when debug feature is disabled.
#[cfg(not(feature = "debug"))]
pub fn hot_reload_shader() {
    // Intentionally empty - hot-reload only available in debug builds
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
pub fn create_parametric_box_sdf(shaders: &mut Assets<Shader>) -> Handle<Shader> {
    shaders.add_sdf_expr("smud::sd_box(p, vec2<f32>(params.x, params.y))")
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

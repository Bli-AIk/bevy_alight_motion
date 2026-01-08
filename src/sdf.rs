//! SDF (Signed Distance Field) shape rendering module.
//!
//! This module provides SDF-based rendering for AM shapes using bevy_smud.
//! 
//! ## Design Philosophy (matching AM behavior)
//! 
//! AM renders stroked rectangles by:
//! 1. Drawing a base 100x100 square with the stroke
//! 2. Applying scale transform to stretch it into the desired shape
//! 3. The stroke width remains constant (not scaled)
//!
//! We achieve this by:
//! 1. Using a fixed 50x50 half-extent SDF box as the base
//! 2. Applying the shape's scale to the SDF entity's transform
//! 3. Using a custom fill shader that renders both fill and stroke
//!    based on SDF distance, with stroke_width passed as a parameter

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
    /// Handle to the stroked fill shader.
    pub stroked_fill: Option<Handle<Shader>>,
}

/// Initialize SDF shaders resource on startup.
pub fn setup_sdf_shaders(mut commands: Commands, mut shaders: ResMut<Assets<Shader>>) {
    // Create a fixed-size box SDF (50x50 half-extent, 100x100 total)
    // This matches AM's approach of drawing a base square then scaling it
    let base_box_sdf = shaders.add_sdf_expr(
        format!("smud::sd_box(p, vec2<f32>({0}, {0}))", BASE_HALF_EXTENT)
    );
    
    // Create the stroked fill shader that handles both fill and stroke
    // params.z = stroke_width, params.w = packed stroke color
    let stroked_fill = shaders.add_fill_body(STROKED_FILL_SHADER);
    
    commands.insert_resource(AmSdfShaders {
        base_box_sdf: Some(base_box_sdf),
        stroked_fill: Some(stroked_fill),
    });
}

/// Inline stroked fill shader source.
/// This shader renders both fill and stroke in a single pass:
/// - Fill color (input.color) when distance < 0
/// - Stroke color (from packed u32 in fill shader uniform) when 0 <= distance < stroke_width  
/// - Transparent outside
///
/// ## Params usage:
/// - params.x: half_width (used by SDF)
/// - params.y: half_height (used by SDF)
/// - params.z: stroke_width
/// - params.w: packed_stroke_color (RGBA as u32 bits stored in f32)
///
/// NOTE: bevy_smud fill shader body is a code fragment, cannot define functions.
/// All logic must be inline.
pub const STROKED_FILL_SHADER: &str = r#"
let stroke_width = input.params.z;

// Unpack stroke color from params.w (packed RGBA as u32 bits stored in f32)
let stroke_bits = bitcast<u32>(input.params.w);
let stroke_r = f32((stroke_bits >> 24u) & 0xFFu) / 255.0;
let stroke_g = f32((stroke_bits >> 16u) & 0xFFu) / 255.0;
let stroke_b = f32((stroke_bits >> 8u) & 0xFFu) / 255.0;
let stroke_a = f32(stroke_bits & 0xFFu) / 255.0;
let stroke_color = vec4<f32>(stroke_r, stroke_g, stroke_b, stroke_a);

// Inside fill region (distance < 0)
if input.distance < 0.0 {
    return input.color;
}

// Inside stroke region (0 <= distance < stroke_width)
if input.distance < stroke_width {
    // Anti-alias the outer edge of the stroke
    let edge_smoothness = 1.0;
    let alpha = 1.0 - smoothstep(stroke_width - edge_smoothness, stroke_width, input.distance);
    return vec4<f32>(stroke_color.rgb, stroke_color.a * alpha);
}

// Outside - fully transparent
return vec4<f32>(0.0, 0.0, 0.0, 0.0);
"#;

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

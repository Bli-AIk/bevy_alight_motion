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

/// Relative path to the stroked fill shader file (from assets folder)
pub const STROKED_FILL_SHADER_FILENAME: &str = "shaders/stroked_fill.wgsl";

/// Resource to hold SDF shader handles.
#[derive(Resource)]
pub struct AmSdfShaders {
    /// Handle to the base box SDF shader (fixed 50x50 half-extent).
    pub base_box_sdf: Option<Handle<Shader>>,
    /// Handle to the stroked fill shader.
    pub stroked_fill: Option<Handle<Shader>>,
    /// Path to the shader file for hot-reload
    pub shader_file_path: Option<PathBuf>,
}

impl Default for AmSdfShaders {
    fn default() -> Self {
        Self {
            base_box_sdf: None,
            stroked_fill: None,
            shader_file_path: None,
        }
    }
}

/// Stroked fill shader source, loaded from file at compile time (fallback).
pub const STROKED_FILL_SHADER_DEFAULT: &str = include_str!("../assets/shaders/stroked_fill.wgsl");

/// Initialize SDF shaders resource on startup.
pub fn setup_sdf_shaders(
    mut commands: Commands,
    mut shaders: ResMut<Assets<Shader>>,
) {
    // Create a fixed-size box SDF (50x50 half-extent, 100x100 total)
    let base_box_sdf = shaders.add_sdf_expr(
        format!("smud::sd_box(p, vec2<f32>({0}, {0}))", BASE_HALF_EXTENT)
    );
    
    // Try to find the shader file path for hot-reload (debug feature only)
    #[cfg(feature = "debug")]
    let shader_file_path = find_shader_file_path();
    #[cfg(not(feature = "debug"))]
    let shader_file_path: Option<PathBuf> = None;
    
    // Load shader content (from file if available, otherwise use embedded)
    let shader_content = if let Some(ref path) = shader_file_path {
        std::fs::read_to_string(path).unwrap_or_else(|_| STROKED_FILL_SHADER_DEFAULT.to_string())
    } else {
        STROKED_FILL_SHADER_DEFAULT.to_string()
    };
    
    let stroked_fill = shaders.add_fill_body(shader_content);
    
    #[cfg(feature = "debug")]
    if shader_file_path.is_some() {
        bevy::log::info!("[SDF] Shader hot-reload enabled. Press 'F5' to reload shader.");
    }
    
    commands.insert_resource(AmSdfShaders {
        base_box_sdf: Some(base_box_sdf),
        stroked_fill: Some(stroked_fill),
        shader_file_path,
    });
}

/// Find the shader file path by searching common locations (debug feature only).
#[cfg(feature = "debug")]
fn find_shader_file_path() -> Option<PathBuf> {
    // Try common asset paths
    let candidates = [
        // Running from workspace root
        PathBuf::from("crates/bevy_alight_motion/assets").join(STROKED_FILL_SHADER_FILENAME),
        // Running from crate directory
        PathBuf::from("assets").join(STROKED_FILL_SHADER_FILENAME),
        // Relative to current dir
        PathBuf::from(STROKED_FILL_SHADER_FILENAME),
    ];
    
    for path in &candidates {
        if path.exists() {
            return Some(path.clone());
        }
    }
    
    None
}

/// System to handle shader hot-reload when 'F5' key is pressed (debug feature only).
#[cfg(feature = "debug")]
pub fn hot_reload_shader(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut sdf_shaders: ResMut<AmSdfShaders>,
    mut smud_shapes: Query<&mut SmudShape>,
) {
    // Check if 'F5' key was just pressed (avoid conflict with 'R' for replay)
    if !keyboard.just_pressed(KeyCode::F5) {
        return;
    }
    
    let Some(ref path) = sdf_shaders.shader_file_path else {
        bevy::log::warn!("[SDF] Shader hot-reload not available (file path not found)");
        return;
    };
    
    // Read shader content from file
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            bevy::log::error!("[SDF] Failed to read shader file: {}", e);
            return;
        }
    };
    
    // Create new shader
    let new_fill = shaders.add_fill_body(&content);
    
    // Update all SmudShape entities to use the new shader
    let mut count = 0;
    for mut shape in smud_shapes.iter_mut() {
        // Only update shapes that were using our stroked fill shader
        if let Some(ref old_fill) = sdf_shaders.stroked_fill {
            if shape.fill == *old_fill {
                shape.fill = new_fill.clone();
                count += 1;
            }
        }
    }
    
    // Update the resource
    sdf_shaders.stroked_fill = Some(new_fill);
    
    bevy::log::info!("[SDF] Shader hot-reloaded! Updated {} shapes.", count);
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

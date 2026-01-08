//! SDF (Signed Distance Field) shape rendering module.
//!
//! This module provides SDF-based rendering for AM shapes using bevy_smud.
//! SDF rendering is essential for complex shape features like:
//! - Strokes/borders with customizable width
//! - Rounded corners
//! - Shadows and glows
//! - Anti-aliased edges

use bevy::asset::Assets;
use bevy::prelude::*;
use bevy_smud::prelude::*;

/// Resource to hold dynamically created SDF shader handles.
#[derive(Resource, Default)]
pub struct AmSdfShaders {
    /// Cache of dynamically created SDF shaders keyed by dimensions.
    /// Not used for caching in current implementation - SDFs are created per-shape.
    _cache: (),
}

/// Initialize SDF shaders resource on startup.
pub fn setup_sdf_shaders(mut commands: Commands) {
    commands.insert_resource(AmSdfShaders::default());
}

/// Create an SDF expression for a box with given half-dimensions.
/// Uses the built-in smud::sd_box function for reliable SDF rendering.
pub fn create_box_sdf(
    shaders: &mut Assets<Shader>,
    half_width: f32,
    half_height: f32,
) -> Handle<Shader> {
    shaders.add_sdf_expr(format!(
        "smud::sd_box(p, vec2<f32>({}, {}))",
        half_width, half_height
    ))
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
        // Resource is now a simple marker, no shader handles stored
        let _ = shaders;
    }
}

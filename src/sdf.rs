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
    /// Handle to the parametric box SDF shader (uses params for dimensions).
    pub parametric_box: Option<Handle<Shader>>,
}

/// Initialize SDF shaders resource on startup.
pub fn setup_sdf_shaders(mut commands: Commands, mut shaders: ResMut<Assets<Shader>>) {
    // Create a parametric box SDF that reads dimensions from params.xy
    // params.x = half_width, params.y = half_height
    let parametric_box = shaders.add_sdf_expr(
        "smud::sd_box(p, vec2<f32>(params.x, params.y))"
    );
    
    commands.insert_resource(AmSdfShaders {
        parametric_box: Some(parametric_box),
    });
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
        assert!(shaders.parametric_box.is_none());
    }
}

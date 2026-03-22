use bevy::prelude::*;

use super::SdfShapeType;

/// Component for AM SDF shapes that need special animation handling.
#[derive(Component, Debug, Clone)]
pub struct AmSdfShapeComponent {
    pub fill_color: Color,
    pub stroke_color: Option<Color>,
    pub stroke_width: f32,
    pub corner_radius: f32,
    pub width: f32,
    pub height: f32,
    pub shape_type: SdfShapeType,
}

/// Marker component for SDF shape entities.
#[derive(Component, Debug, Clone, Default)]
pub struct SdfShapeMarker;

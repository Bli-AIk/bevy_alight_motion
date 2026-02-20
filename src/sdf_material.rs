//! Custom SDF Material for Alight Motion shapes.
//!
//! This module provides a custom Material2d implementation for rendering SDF shapes
//! (rectangles, circles, ellipses) with strokes.

use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d},
};

/// SDF shape types supported by the material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SdfShapeType {
    /// Rectangle with round corners (using sd_box)
    #[default]
    BoxRound,
    /// Rectangle with miter/square corners (using Chebyshev distance)
    BoxMiter,
    /// Rectangle with bevel/cut corners
    BoxBevel,
    /// Circle or ellipse
    Circle,
    /// Rectangle with explicit corner radius
    RoundRect,
    /// Regular N-sided polygon
    Polygon,
    /// Star shape
    Star,
    /// Pie/arc sector
    Pie,
    /// Plus/cross shape
    Plus,
    /// Multi-leaf/multifoil shape
    Multifoil,
    /// Line segment (fill=none, stroke only)
    Line,
    /// Arc (fill=none, stroke only)
    Arc,
    /// Triangle (3 arbitrary vertices)
    Triangle,
    /// Quadrilateral (4 arbitrary vertices)
    Quad,
    /// Pentagon (5 arbitrary vertices)
    Penta,
    /// Freeform path (rendered as mesh, not SDF)
    Path,
}

impl SdfShapeType {
    pub fn to_f32(self) -> f32 {
        match self {
            Self::BoxRound => 0.0,
            Self::BoxMiter => 1.0,
            Self::BoxBevel => 2.0,
            Self::Circle => 3.0,
            Self::RoundRect => 4.0,
            Self::Polygon => 5.0,
            Self::Star => 6.0,
            Self::Pie => 7.0,
            Self::Plus => 8.0,
            Self::Multifoil => 9.0,
            Self::Line => 10.0,
            Self::Arc => 11.0,
            Self::Triangle => 12.0,
            Self::Quad => 13.0,
            Self::Penta => 14.0,
            Self::Path => 15.0,
        }
    }
}

/// Uniform data for SDF shader - must match the struct in the shader exactly
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct SdfMaterialUniform {
    /// Fill color of the shape (LinearRgba as vec4)
    pub color: Vec4,
    /// Shape parameters: (half_width, half_height, stroke_width, packed_stroke_color)
    pub params: Vec4,
    /// Mask 1 parameters: (center_x, center_y, half_width, half_height)
    /// If half_width > 5000.0, mask is disabled
    pub mask_params: Vec4,
    /// Mask 2 parameters: (center_x, center_y, half_width, half_height)
    pub mask2_params: Vec4,
    /// Shape type encoded as float (needs padding to 16 bytes for proper alignment)
    /// Also includes mask_type: 0 = no mask, 1 = rect mask, 2 = ellipse mask
    pub shape_type: f32,
    /// Mask 1 type: 0 = disabled, 1 = rectangle, 2 = ellipse, 3 = rect exclude, 4 = ellipse exclude
    pub mask_type: f32,
    /// Mask 2 type: 0 = disabled, 1 = rectangle, 2 = ellipse, 3 = rect exclude, 4 = ellipse exclude
    pub mask2_type: f32,
    /// Frame half size - the mesh quad is (frame_half * 2) x (frame_half * 2).
    /// Used by shader to convert UV to local coordinates correctly.
    pub frame_half: f32,
    /// Mask 1 rotation in radians
    pub mask_rotation: f32,
    /// Mask 2 rotation in radians
    pub mask2_rotation: f32,
    /// Border 1 direction mode: 0.0=centered, 1.0=inside, -1.0=outside
    pub border_mode: f32,
    /// Border 2 stroke width (0.0 = no second border)
    pub border2_width: f32,
    /// Border 2 packed stroke color (RGBA as u32 bits in f32)
    pub border2_packed_color: f32,
    /// Border 2 direction mode: 0.0=centered, 1.0=inside, -1.0=outside
    pub border2_mode: f32,
    /// Border anti-aliasing width in SDF units (matches AM's 1.5-step smoothstep)
    pub border_aa_width: f32,
    /// Base half-width at spawn time (used to compute scale for polygon shapes)
    pub base_half_width: f32,
    /// Shape-specific extra parameters (meaning depends on shape_type)
    /// RoundRect: (cornerRadius, 0, 0, 0)
    /// Polygon: (sideCount, radius, offsetAngle_deg, 0)
    /// Star: (pointCount, outerRadius, innerRadius, offsetAngle_deg)
    /// Pie: (startAngle_deg, endAngle_deg, radius, 0)
    /// Plus: (stemSize, 0, 0, 0) [uses params.xy for half_size]
    /// Multifoil: (pointCount, outerRadius, innerRadius, offsetAngle_deg)
    /// Line: (p1.x, p1.y, p2.x, p2.y)
    /// Arc: (startAngle_deg, endAngle_deg, radius, 0)
    /// Triangle: (p1.x, p1.y, p2.x, p2.y)
    /// Quad: (p1.x, p1.y, p2.x, p2.y)
    /// Penta: (p1.x, p1.y, p2.x, p2.y)
    pub shape_extra: Vec4,
    /// Second shape-specific extra parameters
    /// Triangle: (p3.x, p3.y, 0, 0)
    /// Quad: (p3.x, p3.y, p4.x, p4.y)
    /// Penta: (p3.x, p3.y, p4.x, p4.y)
    pub shape_extra2: Vec4,
    /// Third shape-specific extra parameters
    /// Penta: (p5.x, p5.y, 0, 0)
    pub shape_extra3: Vec4,
    /// Fourth shape-specific extra parameters (for Path with many vertices)
    /// Path: (p7.x, p7.y, p8.x, p8.y)
    pub shape_extra4: Vec4,
    /// Fifth shape-specific extra parameters
    /// Path: (p9.x, p9.y, p10.x, p10.y)
    pub shape_extra5: Vec4,
    /// Sixth shape-specific extra parameters
    /// Path: (p11.x, p11.y, p12.x, p12.y)
    pub shape_extra6: Vec4,
    /// Seventh shape-specific extra parameters
    /// Path: (p13.x, p13.y, vertex_count, 0)
    pub shape_extra7: Vec4,
    /// Gradient start color (linear RGBA). All zeros when no gradient.
    pub gradient_start_color: Vec4,
    /// Gradient end color (linear RGBA).
    pub gradient_end_color: Vec4,
    /// Gradient points: (start_x, start_y, end_x, end_y) in shape UV [0,1] space.
    pub gradient_points: Vec4,
    /// Gradient config: (gradient_type, 0, 0, 0)
    /// gradient_type: 0=none, 1=linear, 2=radial, 3=sweep
    pub gradient_config: Vec4,
    /// Mask 1 blend parameters: (fill_alpha, opacity, stroke_width, 0)
    /// fill_alpha: the mask shape's fill alpha (0.0 = transparent fill, 1.0 = opaque)
    /// opacity: the mask element's current animated opacity (0..1)
    /// stroke_width: the mask shape's stroke width in world units
    pub mask_blend: Vec4,
    /// Mask 2 blend parameters: (fill_alpha, opacity, stroke_width, 0)
    pub mask2_blend: Vec4,
}

/// Custom SDF Material for rendering shapes with optional strokes.
///
/// Params layout:
/// - `params.x`: half_width (for box) or radius_x (for circle/ellipse)
/// - `params.y`: half_height (for box) or radius_y (for circle/ellipse)
/// - `params.z`: stroke_width
/// - `params.w`: packed stroke color (RGBA as u32 bits stored in f32)
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SdfMaterial {
    /// Combined uniform data
    #[uniform(0)]
    pub uniform_data: SdfMaterialUniform,
}

// Proxy accessors for compatibility
impl SdfMaterial {
    /// Get the fill color
    pub fn color(&self) -> LinearRgba {
        LinearRgba::new(
            self.uniform_data.color.x,
            self.uniform_data.color.y,
            self.uniform_data.color.z,
            self.uniform_data.color.w,
        )
    }

    /// Set the fill color
    pub fn set_color(&mut self, color: LinearRgba) {
        self.uniform_data.color = Vec4::new(color.red, color.green, color.blue, color.alpha);
    }

    /// Get params
    pub fn params(&self) -> Vec4 {
        self.uniform_data.params
    }

    /// Set params
    pub fn set_params(&mut self, params: Vec4) {
        self.uniform_data.params = params;
    }

    /// Get shape type
    pub fn shape_type(&self) -> f32 {
        self.uniform_data.shape_type
    }
}

// For direct field access compatibility
impl SdfMaterial {
    /// Direct access to color (read)
    pub fn color_ref(&self) -> &Vec4 {
        &self.uniform_data.color
    }

    /// Direct access to params (read)  
    pub fn params_ref(&self) -> &Vec4 {
        &self.uniform_data.params
    }
}

/// Helper to access/modify color.alpha
impl SdfMaterial {
    /// Get fill alpha
    pub fn alpha(&self) -> f32 {
        self.uniform_data.color.w
    }

    /// Set fill alpha
    pub fn set_alpha(&mut self, alpha: f32) {
        self.uniform_data.color.w = alpha;
    }
}

impl Default for SdfMaterial {
    fn default() -> Self {
        // Default frame_half based on default half sizes (50, 50) with max_scale_factor=10
        let default_frame_half = 50.0 * 10.0;
        Self {
            uniform_data: SdfMaterialUniform {
                color: Vec4::new(1.0, 1.0, 1.0, 1.0),
                params: Vec4::new(50.0, 50.0, 0.0, 0.0),
                mask_params: Vec4::new(0.0, 0.0, 10000.0, 10000.0), // disabled by default
                mask2_params: Vec4::new(0.0, 0.0, 10000.0, 10000.0), // disabled by default
                shape_type: 0.0,
                mask_type: 0.0,
                mask2_type: 0.0,
                frame_half: default_frame_half,
                mask_rotation: 0.0,
                mask2_rotation: 0.0,
                border_mode: 0.0,
                border2_width: 0.0,
                border2_packed_color: 0.0,
                border2_mode: 0.0,
                border_aa_width: 0.0,
                base_half_width: 0.0,
                shape_extra: Vec4::ZERO,
                shape_extra2: Vec4::ZERO,
                shape_extra3: Vec4::ZERO,
                shape_extra4: Vec4::ZERO,
                shape_extra5: Vec4::ZERO,
                shape_extra6: Vec4::ZERO,
                shape_extra7: Vec4::ZERO,
                gradient_start_color: Vec4::ZERO,
                gradient_end_color: Vec4::ZERO,
                gradient_points: Vec4::ZERO,
                gradient_config: Vec4::ZERO,
                mask_blend: Vec4::ZERO,
                mask2_blend: Vec4::ZERO,
            },
        }
    }
}

impl SdfMaterial {
    /// Create a new SDF material with the specified shape type.
    /// Note: frame_half should be provided by the caller based on mesh size.
    pub fn new(
        shape_type: SdfShapeType,
        half_width: f32,
        half_height: f32,
        fill_color: Color,
        stroke_width: f32,
        stroke_color: Color,
    ) -> Self {
        Self::new_with_frame_half(
            shape_type,
            half_width,
            half_height,
            fill_color,
            stroke_width,
            stroke_color,
            half_width.max(half_height) * 10.0 + stroke_width * 2.0,
        )
    }

    /// Create a new SDF material with explicit frame_half.
    pub fn new_with_frame_half(
        shape_type: SdfShapeType,
        half_width: f32,
        half_height: f32,
        fill_color: Color,
        stroke_width: f32,
        stroke_color: Color,
        frame_half: f32,
    ) -> Self {
        let packed_stroke = pack_color(stroke_color);
        let linear = fill_color.to_linear();
        Self {
            uniform_data: SdfMaterialUniform {
                color: Vec4::new(linear.red, linear.green, linear.blue, linear.alpha),
                params: Vec4::new(half_width, half_height, stroke_width, packed_stroke),
                mask_params: Vec4::new(0.0, 0.0, 10000.0, 10000.0), // disabled by default
                mask2_params: Vec4::new(0.0, 0.0, 10000.0, 10000.0), // disabled by default
                shape_type: shape_type.to_f32(),
                mask_type: 0.0,
                mask2_type: 0.0,
                frame_half,
                mask_rotation: 0.0,
                mask2_rotation: 0.0,
                border_mode: 0.0,
                border2_width: 0.0,
                border2_packed_color: 0.0,
                border2_mode: 0.0,
                border_aa_width: 0.0,
                base_half_width: 0.0,
                shape_extra: Vec4::ZERO,
                shape_extra2: Vec4::ZERO,
                shape_extra3: Vec4::ZERO,
                shape_extra4: Vec4::ZERO,
                shape_extra5: Vec4::ZERO,
                shape_extra6: Vec4::ZERO,
                shape_extra7: Vec4::ZERO,
                gradient_start_color: Vec4::ZERO,
                gradient_end_color: Vec4::ZERO,
                gradient_points: Vec4::ZERO,
                gradient_config: Vec4::ZERO,
                mask_blend: Vec4::ZERO,
                mask2_blend: Vec4::ZERO,
            },
        }
    }

    /// Create a new SDF material with mask support.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_mask(
        shape_type: SdfShapeType,
        half_width: f32,
        half_height: f32,
        fill_color: Color,
        stroke_width: f32,
        stroke_color: Color,
        mask_center: Vec2,
        mask_half_size: Vec2,
        mask_is_circle: bool,
        mask_is_exclude: bool,
    ) -> Self {
        Self::new_with_mask_and_frame_half(
            shape_type,
            half_width,
            half_height,
            fill_color,
            stroke_width,
            stroke_color,
            mask_center,
            mask_half_size,
            mask_is_circle,
            mask_is_exclude,
            half_width.max(half_height) * 10.0 + stroke_width * 2.0,
        )
    }

    /// Create a new SDF material with mask support and explicit frame_half.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_mask_and_frame_half(
        shape_type: SdfShapeType,
        half_width: f32,
        half_height: f32,
        fill_color: Color,
        stroke_width: f32,
        stroke_color: Color,
        mask_center: Vec2,
        mask_half_size: Vec2,
        mask_is_circle: bool,
        mask_is_exclude: bool,
        frame_half: f32,
    ) -> Self {
        let packed_stroke = pack_color(stroke_color);
        let linear = fill_color.to_linear();
        // mask_type: 1=rect, 2=ellipse, 3=rect exclude, 4=ellipse exclude
        let base_type = if mask_is_circle { 2.0 } else { 1.0 };
        let mask_type = if mask_is_exclude {
            base_type + 2.0
        } else {
            base_type
        };
        Self {
            uniform_data: SdfMaterialUniform {
                color: Vec4::new(linear.red, linear.green, linear.blue, linear.alpha),
                params: Vec4::new(half_width, half_height, stroke_width, packed_stroke),
                mask_params: Vec4::new(
                    mask_center.x,
                    mask_center.y,
                    mask_half_size.x,
                    mask_half_size.y,
                ),
                mask2_params: Vec4::new(0.0, 0.0, 10000.0, 10000.0), // disabled by default
                shape_type: shape_type.to_f32(),
                mask_type,
                mask2_type: 0.0,
                frame_half,
                mask_rotation: 0.0,
                mask2_rotation: 0.0,
                border_mode: 0.0,
                border2_width: 0.0,
                border2_packed_color: 0.0,
                border2_mode: 0.0,
                border_aa_width: 0.0,
                base_half_width: 0.0,
                shape_extra: Vec4::ZERO,
                shape_extra2: Vec4::ZERO,
                shape_extra3: Vec4::ZERO,
                shape_extra4: Vec4::ZERO,
                shape_extra5: Vec4::ZERO,
                shape_extra6: Vec4::ZERO,
                shape_extra7: Vec4::ZERO,
                gradient_start_color: Vec4::ZERO,
                gradient_end_color: Vec4::ZERO,
                gradient_points: Vec4::ZERO,
                gradient_config: Vec4::ZERO,
                mask_blend: Vec4::ZERO,
                mask2_blend: Vec4::ZERO,
            },
        }
    }

    /// Create from LinearRgba, params, shape_type, and frame_half directly
    pub fn from_linear(color: LinearRgba, params: Vec4, shape_type: f32, frame_half: f32) -> Self {
        Self {
            uniform_data: SdfMaterialUniform {
                color: Vec4::new(color.red, color.green, color.blue, color.alpha),
                params,
                mask_params: Vec4::new(0.0, 0.0, 10000.0, 10000.0), // disabled by default
                mask2_params: Vec4::new(0.0, 0.0, 10000.0, 10000.0), // disabled by default
                shape_type,
                mask_type: 0.0,
                mask2_type: 0.0,
                frame_half,
                mask_rotation: 0.0,
                mask2_rotation: 0.0,
                border_mode: 0.0,
                border2_width: 0.0,
                border2_packed_color: 0.0,
                border2_mode: 0.0,
                border_aa_width: 0.0,
                base_half_width: 0.0,
                shape_extra: Vec4::ZERO,
                shape_extra2: Vec4::ZERO,
                shape_extra3: Vec4::ZERO,
                shape_extra4: Vec4::ZERO,
                shape_extra5: Vec4::ZERO,
                shape_extra6: Vec4::ZERO,
                shape_extra7: Vec4::ZERO,
                gradient_start_color: Vec4::ZERO,
                gradient_end_color: Vec4::ZERO,
                gradient_points: Vec4::ZERO,
                gradient_config: Vec4::ZERO,
                mask_blend: Vec4::ZERO,
                mask2_blend: Vec4::ZERO,
            },
        }
    }

    /// Create a rectangle with round corners.
    pub fn box_round(half_width: f32, half_height: f32, fill_color: Color) -> Self {
        Self::new(
            SdfShapeType::BoxRound,
            half_width,
            half_height,
            fill_color,
            0.0,
            Color::NONE,
        )
    }

    /// Create a rectangle with miter/square corners.
    pub fn box_miter(half_width: f32, half_height: f32, fill_color: Color) -> Self {
        Self::new(
            SdfShapeType::BoxMiter,
            half_width,
            half_height,
            fill_color,
            0.0,
            Color::NONE,
        )
    }

    /// Create a rectangle with bevel/cut corners.
    pub fn box_bevel(half_width: f32, half_height: f32, fill_color: Color) -> Self {
        Self::new(
            SdfShapeType::BoxBevel,
            half_width,
            half_height,
            fill_color,
            0.0,
            Color::NONE,
        )
    }

    /// Create a circle.
    pub fn circle(radius: f32, fill_color: Color) -> Self {
        Self::new(
            SdfShapeType::Circle,
            radius,
            radius,
            fill_color,
            0.0,
            Color::NONE,
        )
    }

    /// Create an ellipse.
    pub fn ellipse(radius_x: f32, radius_y: f32, fill_color: Color) -> Self {
        Self::new(
            SdfShapeType::Circle,
            radius_x,
            radius_y,
            fill_color,
            0.0,
            Color::NONE,
        )
    }

    /// Add a stroke to the shape.
    pub fn with_stroke(mut self, width: f32, color: Color) -> Self {
        self.uniform_data.params.z = width;
        self.uniform_data.params.w = pack_color(color);
        self
    }

    /// Update shape dimensions.
    pub fn set_dimensions(&mut self, half_width: f32, half_height: f32) {
        self.uniform_data.params.x = half_width;
        self.uniform_data.params.y = half_height;
    }

    /// Update stroke width.
    pub fn set_stroke_width(&mut self, width: f32) {
        self.uniform_data.params.z = width;
    }

    /// Update fill color alpha.
    pub fn set_fill_alpha(&mut self, alpha: f32) {
        self.uniform_data.color.w = alpha;
    }

    /// Update stroke color alpha (preserving RGB).
    pub fn set_stroke_alpha(&mut self, alpha: f32) {
        self.uniform_data.params.w = repack_with_alpha(self.uniform_data.params.w, alpha);
    }

    /// Get the half-width.
    pub fn half_width(&self) -> f32 {
        self.uniform_data.params.x
    }

    /// Get the half-height.
    pub fn half_height(&self) -> f32 {
        self.uniform_data.params.y
    }

    /// Get the stroke width.
    pub fn stroke_width(&self) -> f32 {
        self.uniform_data.params.z
    }
}

// Expose mutable params for animation systems
impl SdfMaterial {
    /// Get mutable access to params
    pub fn params_mut(&mut self) -> &mut Vec4 {
        &mut self.uniform_data.params
    }

    /// Get mutable access to color
    pub fn color_mut(&mut self) -> &mut Vec4 {
        &mut self.uniform_data.color
    }
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

/// Repack a color with a new alpha value.
pub fn repack_with_alpha(packed: f32, new_alpha: f32) -> f32 {
    let bits = packed.to_bits();
    let rgb = bits & 0xFFFFFF00;
    let a = ((new_alpha.clamp(0.0, 1.0) * 255.0) as u32) & 0xFF;
    f32::from_bits(rgb | a)
}

/// Component for AM SDF shapes that need special animation handling.
/// This replaces the AmSdfShape from the old sdf.rs
#[derive(Component, Debug, Clone)]
pub struct AmSdfShapeComponent {
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
    /// Shape type
    pub shape_type: SdfShapeType,
}

/// Marker component for SDF shape entities.
#[derive(Component, Debug, Clone, Default)]
pub struct SdfShapeMarker;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_color() {
        let white = Color::WHITE;
        let packed = pack_color(white);
        let bits = packed.to_bits();
        // Allow ±1 tolerance due to floating point precision
        assert!((bits >> 24) >= 254, "R should be ~255, got {}", bits >> 24); // R
        assert!(((bits >> 16) & 0xFF) >= 254, "G should be ~255"); // G
        assert!(((bits >> 8) & 0xFF) >= 254, "B should be ~255"); // B
        assert!((bits & 0xFF) >= 254, "A should be ~255"); // A

        let red = Color::srgba(1.0, 0.0, 0.0, 1.0);
        let packed = pack_color(red);
        let bits = packed.to_bits();
        assert!((bits >> 24) >= 254, "R should be ~255"); // R
        assert_eq!((bits >> 16) & 0xFF, 0); // G
        assert_eq!((bits >> 8) & 0xFF, 0); // B
        assert!((bits & 0xFF) >= 254, "A should be ~255"); // A
    }

    #[test]
    fn test_sdf_uniform_size() {
        use bevy::render::render_resource::ShaderType;
        let size = SdfMaterialUniform::min_size();
        println!("SdfMaterialUniform min_size = {}", size);
        assert_eq!(size.get(), 160, "SdfMaterialUniform size mismatch! Expected 160 bytes");
    }

    #[test]
    fn test_repack_with_alpha() {
        let white = Color::WHITE;
        let packed = pack_color(white);
        let repacked = repack_with_alpha(packed, 0.5);
        let bits = repacked.to_bits();
        assert!((bits >> 24) >= 254, "R should be ~255"); // R unchanged
        assert!(((bits >> 16) & 0xFF) >= 254, "G should be ~255"); // G unchanged
        assert!(((bits >> 8) & 0xFF) >= 254, "B should be ~255"); // B unchanged
        assert!(
            (bits & 0xFF) >= 126 && (bits & 0xFF) <= 129,
            "A should be ~127, got {}",
            bits & 0xFF
        ); // A ≈ 0.5 * 255
    }
}

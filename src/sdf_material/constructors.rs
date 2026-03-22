use bevy::prelude::*;

use super::uniform::build_base_uniform;
use super::{SdfMaterial, SdfShapeType, pack_color, repack_with_alpha};

// Convenience accessors for uniform-backed fields
impl SdfMaterial {
    pub fn color(&self) -> LinearRgba {
        LinearRgba::new(
            self.uniform_data.color.x,
            self.uniform_data.color.y,
            self.uniform_data.color.z,
            self.uniform_data.color.w,
        )
    }

    pub fn set_color(&mut self, color: LinearRgba) {
        self.uniform_data.color = Vec4::new(color.red, color.green, color.blue, color.alpha);
    }

    pub fn params(&self) -> Vec4 {
        self.uniform_data.params
    }

    pub fn set_params(&mut self, params: Vec4) {
        self.uniform_data.params = params;
    }

    pub fn shape_type(&self) -> f32 {
        self.uniform_data.shape_type
    }
}

impl SdfMaterial {
    pub fn color_ref(&self) -> &Vec4 {
        &self.uniform_data.color
    }

    pub fn params_ref(&self) -> &Vec4 {
        &self.uniform_data.params
    }
}

impl SdfMaterial {
    pub fn alpha(&self) -> f32 {
        self.uniform_data.color.w
    }

    pub fn set_alpha(&mut self, alpha: f32) {
        self.uniform_data.color.w = alpha;
    }
}

impl SdfMaterial {
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
            uniform_data: build_base_uniform(
                Vec4::new(linear.red, linear.green, linear.blue, linear.alpha),
                Vec4::new(half_width, half_height, stroke_width, packed_stroke),
                shape_type.to_f32(),
                frame_half,
            ),
        }
    }

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
        let base_type = if mask_is_circle { 2.0 } else { 1.0 };
        let mask_type = if mask_is_exclude {
            base_type + 2.0
        } else {
            base_type
        };

        let mut uniform = build_base_uniform(
            Vec4::new(linear.red, linear.green, linear.blue, linear.alpha),
            Vec4::new(half_width, half_height, stroke_width, packed_stroke),
            shape_type.to_f32(),
            frame_half,
        );
        uniform.mask_params = Vec4::new(
            mask_center.x,
            mask_center.y,
            mask_half_size.x,
            mask_half_size.y,
        );
        uniform.mask_type = mask_type;

        Self {
            uniform_data: uniform,
        }
    }

    pub fn from_linear(color: LinearRgba, params: Vec4, shape_type: f32, frame_half: f32) -> Self {
        Self {
            uniform_data: build_base_uniform(
                Vec4::new(color.red, color.green, color.blue, color.alpha),
                params,
                shape_type,
                frame_half,
            ),
        }
    }

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

    pub fn with_stroke(mut self, width: f32, color: Color) -> Self {
        self.uniform_data.params.z = width;
        self.uniform_data.params.w = pack_color(color);
        self
    }

    pub fn set_dimensions(&mut self, half_width: f32, half_height: f32) {
        self.uniform_data.params.x = half_width;
        self.uniform_data.params.y = half_height;
    }

    pub fn set_stroke_width(&mut self, width: f32) {
        self.uniform_data.params.z = width;
    }

    pub fn set_fill_alpha(&mut self, alpha: f32) {
        self.uniform_data.color.w = alpha;
    }

    pub fn set_stroke_alpha(&mut self, alpha: f32) {
        self.uniform_data.params.w = repack_with_alpha(self.uniform_data.params.w, alpha);
    }

    pub fn half_width(&self) -> f32 {
        self.uniform_data.params.x
    }

    pub fn half_height(&self) -> f32 {
        self.uniform_data.params.y
    }

    pub fn stroke_width(&self) -> f32 {
        self.uniform_data.params.z
    }
}

impl SdfMaterial {
    pub fn params_mut(&mut self) -> &mut Vec4 {
        &mut self.uniform_data.params
    }

    pub fn color_mut(&mut self) -> &mut Vec4 {
        &mut self.uniform_data.color
    }
}

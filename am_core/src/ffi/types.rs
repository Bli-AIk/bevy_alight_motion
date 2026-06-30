use std::ffi::c_char;
use std::ptr;

use crate::coord::CoordMappingConfig;

pub const MAX_EFFECTS_PER_ELEMENT: usize = 16;
pub const MAX_DELTA_CHANGES: usize = 4096;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AmCoordConfig {
    pub origin_anchor: [f32; 2],
    pub x_direction: f32,
    pub y_direction: f32,
    pub rotation_sign: f32,
    pub rotation_zero_axis: [f32; 2],
    pub engine_anchor: [f32; 2],
    pub z_spacing: f32,
    pub column_major: i32,
}

impl From<AmCoordConfig> for CoordMappingConfig {
    fn from(value: AmCoordConfig) -> Self {
        Self {
            origin_anchor: value.origin_anchor,
            x_direction: value.x_direction,
            y_direction: value.y_direction,
            rotation_sign: value.rotation_sign,
            rotation_zero_axis: value.rotation_zero_axis,
            engine_anchor: value.engine_anchor,
            z_spacing: value.z_spacing,
            column_major: value.column_major != 0,
        }
    }
}

impl From<CoordMappingConfig> for AmCoordConfig {
    fn from(value: CoordMappingConfig) -> Self {
        Self {
            origin_anchor: value.origin_anchor,
            x_direction: value.x_direction,
            y_direction: value.y_direction,
            rotation_sign: value.rotation_sign,
            rotation_zero_axis: value.rotation_zero_axis,
            engine_anchor: value.engine_anchor,
            z_spacing: value.z_spacing,
            column_major: i32::from(value.column_major),
        }
    }
}

impl AmCoordConfig {
    const fn from_coord(value: CoordMappingConfig) -> Self {
        Self {
            origin_anchor: value.origin_anchor,
            x_direction: value.x_direction,
            y_direction: value.y_direction,
            rotation_sign: value.rotation_sign,
            rotation_zero_axis: value.rotation_zero_axis,
            engine_anchor: value.engine_anchor,
            z_spacing: value.z_spacing,
            column_major: if value.column_major { 1 } else { 0 },
        }
    }
}

#[unsafe(no_mangle)]
pub static AM_COORD_AM_NATIVE: AmCoordConfig =
    AmCoordConfig::from_coord(CoordMappingConfig::AM_NATIVE);
#[unsafe(no_mangle)]
pub static AM_COORD_BEVY_2D: AmCoordConfig =
    AmCoordConfig::from_coord(CoordMappingConfig::BEVY_2D);
#[unsafe(no_mangle)]
pub static AM_COORD_UNITY_UI: AmCoordConfig =
    AmCoordConfig::from_coord(CoordMappingConfig::UNITY_UI);
#[unsafe(no_mangle)]
pub static AM_COORD_UNITY_WORLD: AmCoordConfig =
    AmCoordConfig::from_coord(CoordMappingConfig::UNITY_WORLD);
#[unsafe(no_mangle)]
pub static AM_COORD_GODOT_2D: AmCoordConfig =
    AmCoordConfig::from_coord(CoordMappingConfig::GODOT_2D);
#[unsafe(no_mangle)]
pub static AM_COORD_GODOT_CONTROL: AmCoordConfig =
    AmCoordConfig::from_coord(CoordMappingConfig::GODOT_CONTROL);
#[unsafe(no_mangle)]
pub static AM_COORD_CSS: AmCoordConfig = AmCoordConfig::from_coord(CoordMappingConfig::CSS);
#[unsafe(no_mangle)]
pub static AM_COORD_OPENGL_NDC: AmCoordConfig =
    AmCoordConfig::from_coord(CoordMappingConfig::OPENGL_NDC);

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AmMetadata {
    pub width: f32,
    pub height: f32,
    pub fps: i32,
    pub total_time_secs: f32,
    pub bgcolor: *const c_char,
    pub am_version: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct EffectInstance {
    pub effect_type: i32,
    pub params: [f32; 16],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlatElement {
    pub id: i32,
    pub parent_id: i32,
    pub layer_index: i32,
    pub world_matrix: [f32; 16],
    pub kind: i32,
    pub shape_params: [f32; 16],
    pub path_data: *const c_char,
    pub fill_type: i32,
    pub fill_color: [f32; 4],
    pub fill_gradient_start: [f32; 2],
    pub fill_gradient_end: [f32; 2],
    pub fill_gradient_start_color: [f32; 4],
    pub fill_gradient_end_color: [f32; 4],
    pub fill_image_uri: *const c_char,
    pub stroke_width: f32,
    pub stroke_color: [f32; 4],
    pub stroke_cap: i32,
    pub stroke_join: i32,
    pub stroke_miter_limit: f32,
    pub text_content: *const c_char,
    pub text_font: *const c_char,
    pub text_size: f32,
    pub text_wrap_width: f32,
    pub text_align: i32,
    pub opacity: f32,
    pub blend_mode: i32,
    pub effects_count: i32,
    pub effects: [EffectInstance; MAX_EFFECTS_PER_ELEMENT],
    pub am_position: [f32; 2],
    pub am_rotation_deg: f32,
    pub am_scale: [f32; 2],
    pub am_anchor: [f32; 2],
    pub element_width: f32,
    pub element_height: f32,
    pub canvas_width: f32,
    pub canvas_height: f32,
    pub start_time_secs: f32,
    pub end_time_secs: f32,
}

impl Default for FlatElement {
    fn default() -> Self {
        Self {
            id: 0,
            parent_id: 0,
            layer_index: 0,
            world_matrix: identity_matrix(),
            kind: 0,
            shape_params: [0.0; 16],
            path_data: ptr::null(),
            fill_type: 0,
            fill_color: [0.0, 0.0, 0.0, 0.0],
            fill_gradient_start: [0.0, 0.0],
            fill_gradient_end: [0.0, 0.0],
            fill_gradient_start_color: [0.0, 0.0, 0.0, 0.0],
            fill_gradient_end_color: [0.0, 0.0, 0.0, 0.0],
            fill_image_uri: ptr::null(),
            stroke_width: 0.0,
            stroke_color: [0.0, 0.0, 0.0, 0.0],
            stroke_cap: 0,
            stroke_join: 0,
            stroke_miter_limit: 4.0,
            text_content: ptr::null(),
            text_font: ptr::null(),
            text_size: 0.0,
            text_wrap_width: 0.0,
            text_align: 0,
            opacity: 1.0,
            blend_mode: 0,
            effects_count: 0,
            effects: [EffectInstance::default(); MAX_EFFECTS_PER_ELEMENT],
            am_position: [0.0, 0.0],
            am_rotation_deg: 0.0,
            am_scale: [1.0, 1.0],
            am_anchor: [0.0, 0.0],
            element_width: 0.0,
            element_height: 0.0,
            canvas_width: 0.0,
            canvas_height: 0.0,
            start_time_secs: 0.0,
            end_time_secs: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FrameDelta {
    pub added_count: i32,
    pub removed_count: i32,
    pub removed_ids: [i32; MAX_DELTA_CHANGES],
    pub modified_count: i32,
    pub added: *const FlatElement,
}

impl Default for FrameDelta {
    fn default() -> Self {
        Self {
            added_count: 0,
            removed_count: 0,
            removed_ids: [0; MAX_DELTA_CHANGES],
            modified_count: 0,
            added: ptr::null(),
        }
    }
}

pub(crate) fn identity_matrix() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

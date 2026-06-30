//! C-ABI exports for non-Rust engine integrations.

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char};
use std::os::raw::c_void;
use std::ptr;
use std::sync::{Mutex, OnceLock};

use glam::Vec4;

use crate::animation::{
    interpolate_color, interpolate_float, interpolate_vec2, interpolate_vec3, parse_keyframe_color,
};
use crate::coord::{CoordMappingConfig, apply_coord_mapping, multiply_4x4_column_major};
use crate::loader::{AmProject, load_project_from_path};
use crate::schema::{
    AmAnimatedColor, AmEffect, AmFillColor, AmGradient, AmKeyframe, AmLayer, AmMedia, AmProperty,
    AmTransform, parse_color, parse_vec2,
};

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

struct ProjectState {
    project: AmProject,
    coord_config: CoordMappingConfig,
    _metadata_bgcolor: CString,
    metadata: AmMetadata,
    frame_strings: Vec<CString>,
    frame_buffer: Vec<FlatElement>,
    frame_delta: FrameDelta,
}

// Raw pointers stored in FFI structs point into buffers owned by the same
// ProjectState and all access is serialized through PROJECTS' Mutex.
unsafe impl Send for ProjectState {}

struct ProjectTable {
    next_handle: i32,
    projects: HashMap<i32, Box<ProjectState>>,
}

static PROJECTS: OnceLock<Mutex<ProjectTable>> = OnceLock::new();

fn projects() -> &'static Mutex<ProjectTable> {
    PROJECTS.get_or_init(|| {
        Mutex::new(ProjectTable {
            next_handle: 1,
            projects: HashMap::new(),
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn am_project_load(path: *const c_char, coord: *const AmCoordConfig) -> i32 {
    if path.is_null() {
        return -1;
    }

    let path = unsafe {
        match CStr::from_ptr(path).to_str() {
            Ok(path) => path,
            Err(_) => return -1,
        }
    };

    let project = match load_project_from_path(path) {
        Ok(project) => project,
        Err(error) => {
            log::error!("Failed to load AM project '{}': {}", path, error);
            return -1;
        }
    };

    let coord_config = read_coord_config(coord);
    let mut table = projects().lock().unwrap();
    let handle = table.next_handle;
    table.next_handle = table.next_handle.checked_add(1).unwrap_or(1).max(1);

    let state = Box::new(ProjectState::new(project, coord_config));
    table.projects.insert(handle, state);
    handle
}

#[unsafe(no_mangle)]
pub extern "C" fn am_project_free(handle: i32) {
    if let Ok(mut table) = projects().lock() {
        table.projects.remove(&handle);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn am_project_set_coord_config(handle: i32, coord: *const AmCoordConfig) {
    let Ok(mut table) = projects().lock() else {
        return;
    };
    let Some(state) = table.projects.get_mut(&handle) else {
        return;
    };
    state.coord_config = read_coord_config(coord);
}

#[unsafe(no_mangle)]
pub extern "C" fn am_project_metadata(handle: i32) -> *const AmMetadata {
    let Ok(table) = projects().lock() else {
        return ptr::null();
    };
    table
        .projects
        .get(&handle)
        .map(|state| &state.metadata as *const AmMetadata)
        .unwrap_or(ptr::null())
}

#[unsafe(no_mangle)]
pub extern "C" fn am_project_validation_report(handle: i32) -> *mut c_char {
    let Ok(table) = projects().lock() else {
        return ptr::null_mut();
    };
    let Some(state) = table.projects.get(&handle) else {
        return ptr::null_mut();
    };
    match serde_json::to_string(&state.project.validation_report) {
        Ok(json) => into_raw_c_string(&json),
        Err(_) => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn am_string_free(value: *mut c_char) {
    if !value.is_null() {
        unsafe {
            drop(CString::from_raw(value));
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn am_query_frame(
    handle: i32,
    time_secs: f32,
    out_count: *mut i32,
) -> *const FlatElement {
    let Ok(mut table) = projects().lock() else {
        write_count(out_count, 0);
        return ptr::null();
    };
    let Some(state) = table.projects.get_mut(&handle) else {
        write_count(out_count, 0);
        return ptr::null();
    };

    state.rebuild_frame(time_secs);
    write_count(out_count, state.frame_buffer.len() as i32);
    state.frame_buffer.as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn am_query_frame_delta(
    handle: i32,
    _from_secs: f32,
    to_secs: f32,
    out_count: *mut i32,
) -> *const FrameDelta {
    let Ok(mut table) = projects().lock() else {
        write_count(out_count, 0);
        return ptr::null();
    };
    let Some(state) = table.projects.get_mut(&handle) else {
        write_count(out_count, 0);
        return ptr::null();
    };

    state.rebuild_frame(to_secs);
    state.frame_delta = FrameDelta {
        added_count: state.frame_buffer.len() as i32,
        removed_count: 0,
        removed_ids: [0; MAX_DELTA_CHANGES],
        modified_count: 0,
        added: state.frame_buffer.as_ptr(),
    };
    write_count(out_count, state.frame_delta.added_count);
    &state.frame_delta as *const FrameDelta
}

#[unsafe(no_mangle)]
pub extern "C" fn am_get_image_size(handle: i32, uri: *const c_char) -> i32 {
    resource_size(handle, uri, ResourceKind::Image)
}

#[unsafe(no_mangle)]
pub extern "C" fn am_get_image_data(
    handle: i32,
    uri: *const c_char,
    out_buffer: *mut u8,
    buffer_size: i32,
) -> i32 {
    resource_data(handle, uri, out_buffer, buffer_size, ResourceKind::Image)
}

#[unsafe(no_mangle)]
pub extern "C" fn am_get_font_size(handle: i32, font_name: *const c_char) -> i32 {
    resource_size(handle, font_name, ResourceKind::Font)
}

#[unsafe(no_mangle)]
pub extern "C" fn am_get_font_data(
    handle: i32,
    font_name: *const c_char,
    out_buffer: *mut u8,
    buffer_size: i32,
) -> i32 {
    resource_data(handle, font_name, out_buffer, buffer_size, ResourceKind::Font)
}

impl ProjectState {
    fn new(project: AmProject, coord_config: CoordMappingConfig) -> Self {
        let metadata_bgcolor = sanitized_c_string(&project.scene.bgcolor);
        let metadata = AmMetadata {
            width: project.scene.width as f32,
            height: project.scene.height as f32,
            fps: project.scene.fps as i32,
            total_time_secs: project.scene.total_time as f32 / 1000.0,
            bgcolor: metadata_bgcolor.as_ptr(),
            am_version: project.scene.amver,
        };

        Self {
            project,
            coord_config,
            _metadata_bgcolor: metadata_bgcolor,
            metadata,
            frame_strings: Vec::new(),
            frame_buffer: Vec::new(),
            frame_delta: FrameDelta::default(),
        }
    }

    fn rebuild_frame(&mut self, time_secs: f32) {
        let (strings, elements) = build_frame(&self.project, self.coord_config, time_secs);
        self.frame_strings = strings;
        self.frame_buffer = elements;
    }
}

fn build_frame(
    project: &AmProject,
    coord_config: CoordMappingConfig,
    time_secs: f32,
) -> (Vec<CString>, Vec<FlatElement>) {
    let mut strings = Vec::new();
    let mut elements = Vec::new();
    let mut layer_index = 0;
    let canvas_size = [project.scene.width as f32, project.scene.height as f32];

    collect_layers(
        &project.scene.layers,
        canvas_size,
        coord_config,
        time_secs * 1000.0,
        identity_matrix(),
        0,
        &mut layer_index,
        &mut strings,
        &mut elements,
    );

    (strings, elements)
}

#[expect(clippy::too_many_arguments)]
fn collect_layers(
    layers: &[AmLayer],
    canvas_size: [f32; 2],
    coord_config: CoordMappingConfig,
    time_ms: f32,
    parent_matrix: [f32; 16],
    fallback_parent_id: i32,
    layer_index: &mut i32,
    strings: &mut Vec<CString>,
    elements: &mut Vec<FlatElement>,
) {
    for layer in layers {
        match layer {
            AmLayer::Shape(shape) => {
                if !is_layer_active(shape.start_time, shape.end_time, shape.hidden, time_ms) {
                    continue;
                }
                let t = normalized_layer_time(shape.start_time, shape.end_time, time_ms);
                let (size, size_keyframes) =
                    property_size(&shape.properties, &shape.shape_type).unwrap_or(([100.0, 100.0], Vec::new()));
                let local = transform_values(
                    &shape.transform,
                    t,
                    [canvas_size[0] * 0.5, canvas_size[1] * 0.5],
                );
                let world_matrix = mapped_world_matrix(
                    local,
                    size,
                    canvas_size,
                    *layer_index,
                    coord_config,
                    parent_matrix,
                );
                let mut element = base_element(
                    saturating_i32(shape.id),
                    parent_id(shape.parent, fallback_parent_id),
                    *layer_index,
                    world_matrix,
                    local,
                    size,
                    canvas_size,
                    shape.start_time,
                    shape.end_time,
                );
                element.kind = shape_kind(&shape.shape_type);
                element.shape_params[0] = size[0];
                element.shape_params[1] = size[1];
                element.shape_params[2] = size_keyframes.len() as f32;
                element.path_data = shape
                    .path_element
                    .as_ref()
                    .map(|path| push_string(strings, &path.d))
                    .unwrap_or(ptr::null());
                fill_shape_fields(
                    &mut element,
                    shape.fill_type.as_str(),
                    shape.fill_image.as_str(),
                    shape.fill_color.as_ref(),
                    shape.gradient.as_ref(),
                    t,
                    strings,
                );
                fill_stroke_fields(&mut element, shape.stroke.as_ref().or(shape.borders.first()));
                fill_effect_fields(&mut element, &shape.effects);
                elements.push(element);
                *layer_index += 1;
            }
            AmLayer::Text(text) => {
                if !is_layer_active(text.start_time, text.end_time, text.hidden, time_ms) {
                    continue;
                }
                let t = normalized_layer_time(text.start_time, text.end_time, time_ms);
                let width = if text.wrap_width > 0.0 {
                    text.wrap_width
                } else {
                    text.size.max(1.0)
                };
                let size = [width, text.size.max(1.0)];
                let local = transform_values(
                    &text.transform,
                    t,
                    [canvas_size[0] * 0.5, canvas_size[1] * 0.5],
                );
                let world_matrix = mapped_world_matrix(
                    local,
                    size,
                    canvas_size,
                    *layer_index,
                    coord_config,
                    parent_matrix,
                );
                let mut element = base_element(
                    saturating_i32(text.id),
                    parent_id(text.parent, fallback_parent_id),
                    *layer_index,
                    world_matrix,
                    local,
                    size,
                    canvas_size,
                    text.start_time,
                    text.end_time,
                );
                element.kind = 6;
                element.text_content = push_string(strings, &text.content);
                element.text_font = push_string(strings, &text.font);
                element.text_size = text.size;
                element.text_wrap_width = text.wrap_width;
                element.text_align = text_align(&text.align);
                fill_shape_fields(
                    &mut element,
                    text.fill_type.as_str(),
                    "",
                    text.fill_color.as_ref(),
                    None,
                    t,
                    strings,
                );
                fill_effect_fields(&mut element, &text.effects);
                elements.push(element);
                *layer_index += 1;
            }
            AmLayer::Image(image) => {
                if !is_layer_active(image.start_time, image.end_time, image.hidden, time_ms) {
                    continue;
                }
                let t = normalized_layer_time(image.start_time, image.end_time, time_ms);
                let size = property_size(&image.properties, "").map(|(size, _)| size).unwrap_or([100.0, 100.0]);
                let local = transform_values(
                    &image.transform,
                    t,
                    [canvas_size[0] * 0.5, canvas_size[1] * 0.5],
                );
                let world_matrix = mapped_world_matrix(
                    local,
                    size,
                    canvas_size,
                    *layer_index,
                    coord_config,
                    parent_matrix,
                );
                let mut element = base_element(
                    saturating_i32(image.id),
                    parent_id(image.parent, fallback_parent_id),
                    *layer_index,
                    world_matrix,
                    local,
                    size,
                    canvas_size,
                    image.start_time,
                    image.end_time,
                );
                element.kind = 7;
                element.fill_type = 4;
                element.fill_image_uri = push_string(strings, &image.fill_image);
                fill_effect_fields(&mut element, &image.effects);
                elements.push(element);
                *layer_index += 1;
            }
            AmLayer::Nullobj(null) => {
                if !is_layer_active(null.start_time, null.end_time, null.hidden, time_ms) {
                    continue;
                }
                let t = normalized_layer_time(null.start_time, null.end_time, time_ms);
                let size = [0.0, 0.0];
                let local = transform_values(&null.transform, t, [0.0, 0.0]);
                let world_matrix = mapped_world_matrix(
                    local,
                    size,
                    canvas_size,
                    *layer_index,
                    coord_config,
                    parent_matrix,
                );
                let mut element = base_element(
                    saturating_i32(null.id),
                    parent_id(null.parent, fallback_parent_id),
                    *layer_index,
                    world_matrix,
                    local,
                    size,
                    canvas_size,
                    null.start_time,
                    null.end_time,
                );
                element.kind = 8;
                fill_effect_fields(&mut element, &null.effects);
                elements.push(element);
                *layer_index += 1;
            }
            AmLayer::EmbedScene(embed) => {
                if !is_layer_active(embed.start_time, embed.end_time, embed.hidden, time_ms) {
                    continue;
                }
                let t = normalized_layer_time(embed.start_time, embed.end_time, time_ms);
                let size = [embed.scene.width as f32, embed.scene.height as f32];
                let local = transform_values(
                    &embed.transform,
                    t,
                    [canvas_size[0] * 0.5, canvas_size[1] * 0.5],
                );
                let world_matrix = mapped_world_matrix(
                    local,
                    size,
                    canvas_size,
                    *layer_index,
                    coord_config,
                    parent_matrix,
                );
                let mut element = base_element(
                    saturating_i32(embed.id),
                    parent_id(embed.parent, fallback_parent_id),
                    *layer_index,
                    world_matrix,
                    local,
                    size,
                    canvas_size,
                    embed.start_time,
                    embed.end_time,
                );
                element.kind = 9;
                fill_shape_fields(
                    &mut element,
                    embed.fill_type.as_str(),
                    "",
                    embed.fill_color.as_ref(),
                    embed.gradient.as_ref(),
                    t,
                    strings,
                );
                fill_effect_fields(&mut element, &embed.effects);
                elements.push(element);
                let embed_parent_id = saturating_i32(embed.id);
                *layer_index += 1;

                let nested_time_ms =
                    embed.in_time.unwrap_or(0) as f32 + (time_ms - embed.start_time as f32) * embed.speed;
                collect_layers(
                    &embed.scene.layers,
                    [embed.scene.width as f32, embed.scene.height as f32],
                    coord_config,
                    nested_time_ms,
                    world_matrix,
                    embed_parent_id,
                    layer_index,
                    strings,
                    elements,
                );
            }
            AmLayer::Camera(camera) => {
                if !is_layer_active(camera.start_time, camera.end_time, camera.hidden, time_ms) {
                    continue;
                }
                let t = normalized_layer_time(camera.start_time, camera.end_time, time_ms);
                let local = transform_values(
                    &camera.transform,
                    t,
                    [canvas_size[0] * 0.5, canvas_size[1] * 0.5],
                );
                let world_matrix = mapped_world_matrix(
                    local,
                    [0.0, 0.0],
                    canvas_size,
                    *layer_index,
                    coord_config,
                    parent_matrix,
                );
                let mut element = base_element(
                    saturating_i32(camera.id),
                    parent_id(camera.parent, fallback_parent_id),
                    *layer_index,
                    world_matrix,
                    local,
                    [0.0, 0.0],
                    canvas_size,
                    camera.start_time,
                    camera.end_time,
                );
                element.kind = 10;
                elements.push(element);
                *layer_index += 1;
            }
            AmLayer::Audio(_) | AmLayer::Video(_) | AmLayer::Bookmark(_) => {}
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TransformValues {
    position: [f32; 2],
    rotation_deg: f32,
    scale: [f32; 2],
    pivot: [f32; 2],
    opacity: f32,
}

fn transform_values(transform: &AmTransform, t: f32, default_position: [f32; 2]) -> TransformValues {
    let location = interpolate_vec3(&transform.location, t)
        .or(transform.location.value)
        .unwrap_or([default_position[0], default_position[1], 0.0]);
    let rotation_deg = interpolate_float(&transform.rotation, t)
        .or(transform.rotation.value)
        .unwrap_or(0.0);
    let scale = interpolate_vec2(&transform.scale, t)
        .or(transform.scale.value)
        .unwrap_or([1.0, 1.0]);
    let pivot = interpolate_vec2(&transform.pivot, t)
        .or(transform.pivot.value)
        .unwrap_or([0.0, 0.0]);
    let opacity = interpolate_float(&transform.opacity, t)
        .or(transform.opacity.value)
        .unwrap_or(1.0);

    TransformValues {
        position: [location[0], location[1]],
        rotation_deg,
        scale,
        pivot,
        opacity,
    }
}

fn mapped_world_matrix(
    local: TransformValues,
    size: [f32; 2],
    canvas_size: [f32; 2],
    layer_index: i32,
    coord_config: CoordMappingConfig,
    parent_matrix: [f32; 16],
) -> [f32; 16] {
    let mapped = apply_coord_mapping(
        local.position,
        local.rotation_deg,
        local.scale,
        size,
        canvas_size,
        layer_index,
        &coord_config,
    );
    multiply_4x4_column_major(parent_matrix, mapped)
}

#[expect(clippy::too_many_arguments)]
fn base_element(
    id: i32,
    parent_id: i32,
    layer_index: i32,
    world_matrix: [f32; 16],
    local: TransformValues,
    size: [f32; 2],
    canvas_size: [f32; 2],
    start_time: i32,
    end_time: i32,
) -> FlatElement {
    FlatElement {
        id,
        parent_id,
        layer_index,
        world_matrix,
        opacity: local.opacity,
        am_position: local.position,
        am_rotation_deg: local.rotation_deg,
        am_scale: local.scale,
        am_anchor: local.pivot,
        element_width: size[0],
        element_height: size[1],
        canvas_width: canvas_size[0],
        canvas_height: canvas_size[1],
        start_time_secs: start_time as f32 / 1000.0,
        end_time_secs: end_time as f32 / 1000.0,
        ..Default::default()
    }
}

fn fill_shape_fields(
    element: &mut FlatElement,
    fill_type: &str,
    fill_image: &str,
    fill_color: Option<&AmFillColor>,
    gradient: Option<&AmGradient>,
    t: f32,
    strings: &mut Vec<CString>,
) {
    element.fill_type = match fill_type {
        "color" => 1,
        "gradient" => gradient
            .map(|g| if g.gradient_type == "radial" { 3 } else { 2 })
            .unwrap_or(2),
        "image" | "media" => 4,
        _ => 0,
    };

    if let Some(fill_color) = fill_color {
        element.fill_color = animated_fill_color(fill_color, t).unwrap_or(element.fill_color);
    }
    if !fill_image.is_empty() {
        element.fill_image_uri = push_string(strings, fill_image);
    }
    if let Some(gradient) = gradient {
        element.fill_gradient_start = gradient.start.unwrap_or([0.0, 0.0]);
        element.fill_gradient_end = gradient.end.unwrap_or([0.0, 0.0]);
        element.fill_gradient_start_color =
            parse_color(&gradient.start_color).unwrap_or([0.0, 0.0, 0.0, 0.0]);
        element.fill_gradient_end_color =
            parse_color(&gradient.end_color).unwrap_or([0.0, 0.0, 0.0, 0.0]);
    }
}

fn fill_stroke_fields(element: &mut FlatElement, stroke: Option<&crate::schema::AmStroke>) {
    let Some(stroke) = stroke else {
        return;
    };
    element.stroke_width = stroke
        .size
        .as_ref()
        .and_then(|size| size.value)
        .unwrap_or(0.0);
    element.stroke_color = stroke
        .color
        .as_ref()
        .and_then(|color| parse_color(&color.value).ok())
        .unwrap_or([0.0, 0.0, 0.0, 0.0]);
    element.stroke_cap = match stroke.cap.as_str() {
        "round" => 1,
        "square" => 2,
        _ => 0,
    };
    element.stroke_join = match stroke.join.as_str() {
        "round" => 1,
        "bevel" => 2,
        _ => 0,
    };
}

fn fill_effect_fields(element: &mut FlatElement, effects: &[AmEffect]) {
    let all_effects = crate::effects_registry::all_effects();
    for (idx, effect) in effects.iter().take(MAX_EFFECTS_PER_ELEMENT).enumerate() {
        let effect_type = all_effects
            .iter()
            .position(|def| def.id == effect.id)
            .map(|index| index as i32 + 1)
            .unwrap_or(0);
        element.effects[idx] = EffectInstance {
            effect_type,
            params: effect_params(effect),
        };
    }
    element.effects_count = effects.len().min(MAX_EFFECTS_PER_ELEMENT) as i32;
}

fn effect_params(effect: &AmEffect) -> [f32; 16] {
    let mut params = [0.0; 16];
    for (index, property) in effect.properties.iter().take(16).enumerate() {
        params[index] = property.value.parse::<f32>().unwrap_or(0.0);
    }
    params
}

fn animated_fill_color(fill_color: &AmFillColor, t: f32) -> Option<[f32; 4]> {
    let value = if fill_color.value.is_empty() {
        None
    } else {
        parse_keyframe_color(&fill_color.value)
    };
    let animated = AmAnimatedColor {
        value,
        keyframes: fill_color.keyframes.clone(),
    };
    interpolate_color(&animated, t).map(vec4_to_array)
}

fn property_size(properties: &[AmProperty], shape_type: &str) -> Option<([f32; 2], Vec<AmKeyframe>)> {
    for property in properties {
        if property.name != "size" || property.prop_type != "vec2" {
            continue;
        }
        let value = if property.value.is_empty() {
            property
                .keyframes
                .first()
                .and_then(|keyframe| parse_vec2(&keyframe.value).ok())
        } else {
            parse_vec2(&property.value).ok()
        }?;
        let keyframes = property
            .keyframes
            .iter()
            .map(|keyframe| {
                let value = parse_vec2(&keyframe.value)
                    .map(|size| format!("{},{}", size[0] * 2.0, size[1] * 2.0))
                    .unwrap_or_else(|_| keyframe.value.clone());
                AmKeyframe {
                    time: keyframe.time,
                    value,
                    easing: keyframe.easing.clone(),
                }
            })
            .collect();
        return Some(([(value[0] * 2.0).abs(), (value[1] * 2.0).abs()], keyframes));
    }

    infer_shape_size(shape_type).map(|size| (size, Vec::new()))
}

fn infer_shape_size(shape_type: &str) -> Option<[f32; 2]> {
    match shape_type {
        ".poly" | ".pie" | ".arc" | ".star" | ".multifoil" => Some([100.0, 100.0]),
        ".line" => Some([50.0, 1.0]),
        ".triangle" | ".quad" | ".penta" | ".arrow" => Some([100.0, 100.0]),
        _ => None,
    }
}

fn is_layer_active(start_time: i32, end_time: i32, hidden: bool, time_ms: f32) -> bool {
    if hidden || time_ms < start_time as f32 {
        return false;
    }
    end_time <= start_time || time_ms <= end_time as f32
}

fn normalized_layer_time(start_time: i32, end_time: i32, time_ms: f32) -> f32 {
    if end_time <= start_time {
        return 0.0;
    }
    ((time_ms - start_time as f32) / (end_time - start_time) as f32).clamp(0.0, 1.0)
}

fn shape_kind(shape_type: &str) -> i32 {
    match shape_type {
        ".rect" | ".roundrect" => 1,
        ".circle" | ".ellipse" => 2,
        ".poly" | ".triangle" | ".quad" | ".penta" | ".pie" | ".star" | ".multifoil"
        | ".arc" | ".ngon" | ".plus" => 3,
        ".path" => 4,
        ".line" | ".arrow" => 5,
        _ => 1,
    }
}

fn text_align(align: &str) -> i32 {
    match align {
        "center" => 1,
        "right" => 2,
        _ => 0,
    }
}

fn parent_id(layer_parent: u64, fallback_parent_id: i32) -> i32 {
    if layer_parent == 0 {
        fallback_parent_id
    } else {
        saturating_i32(layer_parent)
    }
}

fn saturating_i32(value: u64) -> i32 {
    value.min(i32::MAX as u64) as i32
}

fn vec4_to_array(value: Vec4) -> [f32; 4] {
    [value.x, value.y, value.z, value.w]
}

fn push_string(strings: &mut Vec<CString>, value: &str) -> *const c_char {
    strings.push(sanitized_c_string(value));
    strings.last().map(|value| value.as_ptr()).unwrap_or(ptr::null())
}

fn sanitized_c_string(value: &str) -> CString {
    CString::new(value.replace('\0', "")).unwrap_or_else(|_| CString::new("").unwrap())
}

fn into_raw_c_string(value: &str) -> *mut c_char {
    sanitized_c_string(value).into_raw()
}

fn read_coord_config(coord: *const AmCoordConfig) -> CoordMappingConfig {
    if coord.is_null() {
        CoordMappingConfig::AM_NATIVE
    } else {
        unsafe { *coord }.into()
    }
}

fn write_count(out_count: *mut i32, count: i32) {
    if !out_count.is_null() {
        unsafe {
            *out_count = count;
        }
    }
}

fn identity_matrix() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

enum ResourceKind {
    Image,
    Font,
}

fn resource_size(handle: i32, name: *const c_char, kind: ResourceKind) -> i32 {
    let Ok(table) = projects().lock() else {
        return -1;
    };
    let Some(data) = resource_bytes(&table, handle, name, kind) else {
        return -1;
    };
    data.len().min(i32::MAX as usize) as i32
}

fn resource_data(
    handle: i32,
    name: *const c_char,
    out_buffer: *mut u8,
    buffer_size: i32,
    kind: ResourceKind,
) -> i32 {
    if out_buffer.is_null() || buffer_size < 0 {
        return -1;
    }
    let Ok(table) = projects().lock() else {
        return -1;
    };
    let Some(data) = resource_bytes(&table, handle, name, kind) else {
        return -1;
    };
    if data.len() > buffer_size as usize {
        return data.len().min(i32::MAX as usize) as i32;
    }
    unsafe {
        ptr::copy_nonoverlapping(data.as_ptr(), out_buffer, data.len());
    }
    data.len().min(i32::MAX as usize) as i32
}

fn resource_bytes<'a>(
    table: &'a ProjectTable,
    handle: i32,
    name: *const c_char,
    kind: ResourceKind,
) -> Option<&'a Vec<u8>> {
    if name.is_null() {
        return None;
    }
    let name = unsafe { CStr::from_ptr(name).to_str().ok()? };
    let project = &table.projects.get(&handle)?.project;
    match kind {
        ResourceKind::Image => project.embedded_images.get(name),
        ResourceKind::Font => project
            .embedded_fonts
            .get(name)
            .or_else(|| find_font_by_media(project, name)),
    }
}

fn find_font_by_media<'a>(project: &'a AmProject, font_name: &str) -> Option<&'a Vec<u8>> {
    let media_filename = project.scene.media.iter().find_map(|media: &AmMedia| {
        (media.uri == font_name || media.filename == font_name).then_some(media.filename.as_str())
    })?;
    project.embedded_fonts.get(media_filename)
}

#[allow(dead_code)]
fn _opaque(_: *mut c_void) {}

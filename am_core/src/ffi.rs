//! C-ABI exports for non-Rust engine integrations.

mod element;
mod frame;
mod resources;
mod types;

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char};
use std::os::raw::c_void;
use std::ptr;
use std::sync::{Mutex, OnceLock};

use crate::coord::CoordMappingConfig;
use crate::loader::{AmProject, load_project_from_path};

use element::{into_raw_c_string, sanitized_c_string};
use frame::build_frame;
use resources::{ResourceKind, resource_data, resource_size};

pub use types::*;

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
/// # Safety
///
/// `path` must point to a valid null-terminated C string for the duration of
/// this call. `coord` may be null, otherwise it must point to a valid
/// `AmCoordConfig`.
pub unsafe extern "C" fn am_project_load(path: *const c_char, coord: *const AmCoordConfig) -> i32 {
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
/// # Safety
///
/// `value` must be null or a pointer previously returned by this library from
/// a function that transfers ownership of a C string to the caller.
pub unsafe extern "C" fn am_string_free(value: *mut c_char) {
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

#[allow(dead_code)]
fn _opaque(_: *mut c_void) {}

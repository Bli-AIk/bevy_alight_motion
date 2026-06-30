use std::ffi::{CStr, c_char};
use std::ptr;

use crate::loader::AmProject;
use crate::schema::AmMedia;

use super::{ProjectTable, projects};

pub(super) enum ResourceKind {
    Image,
    Font,
}

pub(super) fn resource_size(handle: i32, name: *const c_char, kind: ResourceKind) -> i32 {
    let Ok(table) = projects().lock() else {
        return -1;
    };
    let Some(data) = resource_bytes(&table, handle, name, kind) else {
        return -1;
    };
    data.len().min(i32::MAX as usize) as i32
}

pub(super) fn resource_data(
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

fn resource_bytes(
    table: &ProjectTable,
    handle: i32,
    name: *const c_char,
    kind: ResourceKind,
) -> Option<&Vec<u8>> {
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

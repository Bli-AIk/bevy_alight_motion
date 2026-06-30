use std::ffi::CString;
use std::ptr;

use crate::coord::CoordMappingConfig;
use crate::loader::AmProject;
use crate::schema::AmLayer;

use super::element::{
    base_element, fill_effect_fields, fill_shape_fields, fill_stroke_fields, is_layer_active,
    mapped_world_matrix, normalized_layer_time, parent_id, property_size, push_string,
    saturating_i32, shape_kind, text_align, transform_values,
};
use super::types::{FlatElement, identity_matrix};

pub(super) fn build_frame(
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
                let (size, size_keyframes) = property_size(&shape.properties, &shape.shape_type)
                    .unwrap_or(([100.0, 100.0], Vec::new()));
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
                let size = property_size(&image.properties, "")
                    .map(|(size, _)| size)
                    .unwrap_or([100.0, 100.0]);
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

                let nested_time_ms = embed.in_time.unwrap_or(0) as f32
                    + (time_ms - embed.start_time as f32) * embed.speed;
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

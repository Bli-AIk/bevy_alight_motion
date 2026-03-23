use bevy::prelude::*;

use super::super::helpers::{get_shape_float_property, get_shape_vec2_property};

pub(crate) fn extract_shape_extras(
    shape_type: &str,
    properties: &[crate::schema::AmProperty],
    path_data: &str,
) -> (Vec4, Vec4, Vec4, Vec4, Vec4, Vec4, Vec4) {
    let z = Vec4::ZERO;
    match shape_type {
        ".roundrect" => {
            let corner_radius = get_shape_float_property(properties, "cornerRadius", 25.0);
            (Vec4::new(corner_radius, 0.0, 0.0, 0.0), z, z, z, z, z, z)
        }
        ".poly" => {
            let side_count = get_shape_float_property(properties, "sideCount", 6.0);
            let radius = get_shape_float_property(properties, "radius", 100.0);
            let offset_angle = get_shape_float_property(properties, "offsetAngle", 0.0);
            (
                Vec4::new(side_count, radius, offset_angle, 0.0),
                z,
                z,
                z,
                z,
                z,
                z,
            )
        }
        ".star" => {
            let point_count = get_shape_float_property(properties, "pointCount", 5.0);
            let outer_radius = get_shape_float_property(properties, "outerRadius", 100.0);
            let inner_radius = get_shape_float_property(properties, "innerRadius", 50.0);
            let offset_angle = get_shape_float_property(properties, "offsetAngle", 0.0);
            (
                Vec4::new(point_count, outer_radius, inner_radius, offset_angle),
                z,
                z,
                z,
                z,
                z,
                z,
            )
        }
        ".pie" => {
            let start_angle = get_shape_float_property(properties, "startAngle", 0.0);
            let end_angle = get_shape_float_property(properties, "endAngle", 90.0);
            let radius = get_shape_float_property(properties, "radius", 100.0);
            (
                Vec4::new(start_angle, end_angle, radius, 0.0),
                z,
                z,
                z,
                z,
                z,
                z,
            )
        }
        ".plus" => {
            let stem_size = get_shape_float_property(properties, "stemSize", 50.0);
            (Vec4::new(stem_size, 0.0, 0.0, 0.0), z, z, z, z, z, z)
        }
        ".multifoil" => {
            let point_count = get_shape_float_property(properties, "pointCount", 5.0);
            let outer_radius = get_shape_float_property(properties, "outerRadius", 100.0);
            let inner_radius = get_shape_float_property(properties, "innerRadius", 50.0);
            let offset_angle = get_shape_float_property(properties, "offsetAngle", 0.0);
            (
                Vec4::new(point_count, outer_radius, inner_radius, offset_angle),
                z,
                z,
                z,
                z,
                z,
                z,
            )
        }
        ".line" => {
            let p1 = get_shape_vec2_property(properties, "p1", [-100.0, 0.0]);
            let p2 = get_shape_vec2_property(properties, "p2", [100.0, 0.0]);
            (Vec4::new(p1[0], p1[1], p2[0], p2[1]), z, z, z, z, z, z)
        }
        ".arrow" => {
            let start = get_shape_vec2_property(properties, "start", [0.0, 0.0]);
            let end = get_shape_vec2_property(properties, "end", [100.0, 0.0]);
            let line_width = get_shape_float_property(properties, "lineWidth", 20.0);
            let head_width = get_shape_float_property(properties, "headWidth", 40.0);
            let head_length = get_shape_float_property(properties, "headLength", 30.0);
            (
                Vec4::new(start[0], start[1], end[0], end[1]),
                Vec4::new(line_width, head_width, head_length, 0.0),
                z,
                z,
                z,
                z,
                z,
            )
        }
        ".arc" => {
            let start_angle = get_shape_float_property(properties, "startAngle", 0.0);
            let end_angle = get_shape_float_property(properties, "endAngle", 90.0);
            let radius = get_shape_float_property(properties, "radius", 100.0);
            (
                Vec4::new(start_angle, end_angle, radius, 0.0),
                z,
                z,
                z,
                z,
                z,
                z,
            )
        }
        ".triangle" => {
            let p1 = get_shape_vec2_property(properties, "p1", [-100.0, 100.0]);
            let p2 = get_shape_vec2_property(properties, "p2", [0.0, -100.0]);
            let p3 = get_shape_vec2_property(properties, "p3", [100.0, 100.0]);
            (
                Vec4::new(p1[0], p1[1], p2[0], p2[1]),
                Vec4::new(p3[0], p3[1], 0.0, 0.0),
                z,
                z,
                z,
                z,
                z,
            )
        }
        ".quad" => {
            let p1 = get_shape_vec2_property(properties, "p1", [-100.0, -100.0]);
            let p2 = get_shape_vec2_property(properties, "p2", [100.0, -100.0]);
            let p3 = get_shape_vec2_property(properties, "p3", [100.0, 100.0]);
            let p4 = get_shape_vec2_property(properties, "p4", [-100.0, 100.0]);
            (
                Vec4::new(p1[0], p1[1], p2[0], p2[1]),
                Vec4::new(p3[0], p3[1], p4[0], p4[1]),
                z,
                z,
                z,
                z,
                z,
            )
        }
        ".penta" => {
            let p1 = get_shape_vec2_property(properties, "p1", [-100.0, -100.0]);
            let p2 = get_shape_vec2_property(properties, "p2", [0.0, -100.0]);
            let p3 = get_shape_vec2_property(properties, "p3", [0.0, 0.0]);
            let p4 = get_shape_vec2_property(properties, "p4", [100.0, 100.0]);
            let p5 = get_shape_vec2_property(properties, "p5", [-100.0, 100.0]);
            (
                Vec4::new(p1[0], p1[1], p2[0], p2[1]),
                Vec4::new(p3[0], p3[1], p4[0], p4[1]),
                Vec4::new(p5[0], p5[1], 0.0, 0.0),
                z,
                z,
                z,
                z,
            )
        }
        _ if shape_type.is_empty() && !path_data.is_empty() => parse_path_extras(path_data),
        _ => (z, z, z, z, z, z, z),
    }
}

fn parse_path_token(tokens: &[&str], i: &mut usize, vertices: &mut Vec<f32>) {
    match tokens[*i] {
        "M" | "L" | "m" | "l" => {
            if *i + 2 >= tokens.len() {
                *i += 1;
                return;
            }
            if let (Ok(x), Ok(y)) = (tokens[*i + 1].parse::<f32>(), tokens[*i + 2].parse::<f32>())
                && vertices.len() < 26
            {
                vertices.push(x);
                vertices.push(y);
            }
            *i += 3;
        }
        "Z" | "z" => {
            *i += 1;
        }
        _ => {
            if *i + 1 >= tokens.len() {
                *i += 1;
                return;
            }
            let (Ok(x), Ok(y)) = (tokens[*i].parse::<f32>(), tokens[*i + 1].parse::<f32>()) else {
                *i += 1;
                return;
            };
            if vertices.len() < 26 {
                vertices.push(x);
                vertices.push(y);
            }
            *i += 2;
        }
    }
}

pub(crate) fn parse_path_extras(path_data: &str) -> (Vec4, Vec4, Vec4, Vec4, Vec4, Vec4, Vec4) {
    let mut vertices: Vec<f32> = Vec::new();
    let mut cleaned = String::with_capacity(path_data.len() + 20);
    for c in path_data.chars() {
        if c.is_ascii_alphabetic() {
            cleaned.push(' ');
            cleaned.push(c);
            cleaned.push(' ');
        } else {
            cleaned.push(c);
        }
    }
    let tokens: Vec<&str> = cleaned.split_whitespace().collect();
    let mut i = 0;
    while i < tokens.len() {
        parse_path_token(&tokens, &mut i, &mut vertices);
    }
    let vertex_count = (vertices.len() / 2) as f32;
    while vertices.len() < 26 {
        vertices.push(0.0);
    }
    (
        Vec4::new(vertices[0], vertices[1], vertices[2], vertices[3]),
        Vec4::new(vertices[4], vertices[5], vertices[6], vertices[7]),
        Vec4::new(vertices[8], vertices[9], vertices[10], vertices[11]),
        Vec4::new(vertices[12], vertices[13], vertices[14], vertices[15]),
        Vec4::new(vertices[16], vertices[17], vertices[18], vertices[19]),
        Vec4::new(vertices[20], vertices[21], vertices[22], vertices[23]),
        Vec4::new(vertices[24], vertices[25], vertex_count, 0.0),
    )
}

use bevy::prelude::*;

use super::shape_properties::{get_shape_float_property, get_shape_vec2_property};
use crate::schema::{AmAnimatedVec2, AmKeyframe, AmProperty};

fn points_bounding_size(points: &[Vec2]) -> Option<(f32, f32)> {
    let first = *points.first()?;
    let mut min_x = first.x;
    let mut max_x = first.x;
    let mut min_y = first.y;
    let mut max_y = first.y;

    for point in points.iter().skip(1) {
        min_x = min_x.min(point.x);
        max_x = max_x.max(point.x);
        min_y = min_y.min(point.y);
        max_y = max_y.max(point.y);
    }

    Some((
        (max_x - min_x).abs().max(1.0),
        (max_y - min_y).abs().max(1.0),
    ))
}

fn arrow_outline_points(
    start: Vec2,
    end: Vec2,
    line_width: f32,
    head_width: f32,
    head_length: f32,
) -> [Vec2; 7] {
    let head_width = head_width.abs();
    let line_width = line_width.abs().min(head_width);
    let mut head_length = head_length.abs();

    let delta = end - start;
    let len = delta.length();
    if len <= 0.001 {
        let half = head_width.max(line_width).max(1.0);
        return [
            end + Vec2::new(-half, -half),
            end + Vec2::new(half, -half),
            end + Vec2::new(half, 0.0),
            end + Vec2::new(half, half),
            end + Vec2::new(-half, half),
            end + Vec2::new(-half, 0.0),
            end + Vec2::new(-half, -half),
        ];
    }

    head_length = head_length.clamp(0.0, len);
    let dir = delta / len;
    let cw = Vec2::new(-dir.y, dir.x);
    let ccw = Vec2::new(dir.y, -dir.x);
    let tail_length = len - head_length;

    [
        start + cw * line_width,
        start + ccw * line_width,
        start + ccw * line_width + dir * tail_length,
        start + ccw * head_width + dir * tail_length,
        end,
        start + cw * head_width + dir * tail_length,
        start + cw * line_width + dir * tail_length,
    ]
}

fn infer_shape_size_from_properties(
    properties: &[AmProperty],
    shape_type: &str,
) -> Option<(f32, f32)> {
    match shape_type {
        ".poly" | ".pie" | ".arc" => {
            let radius = get_shape_float_property(properties, "radius", 50.0).abs();
            Some((radius * 2.0, radius * 2.0))
        }
        ".star" | ".multifoil" => {
            let outer_radius = get_shape_float_property(properties, "outerRadius", 50.0).abs();
            Some((outer_radius * 2.0, outer_radius * 2.0))
        }
        ".line" => {
            let p1 = get_shape_vec2_property(properties, "p1", [0.0, 0.0]);
            let p2 = get_shape_vec2_property(properties, "p2", [50.0, 0.0]);
            points_bounding_size(&[Vec2::new(p1[0], p1[1]), Vec2::new(p2[0], p2[1])])
        }
        ".triangle" => {
            let p1 = get_shape_vec2_property(properties, "p1", [0.0, -50.0]);
            let p2 = get_shape_vec2_property(properties, "p2", [-50.0, 50.0]);
            let p3 = get_shape_vec2_property(properties, "p3", [50.0, 50.0]);
            points_bounding_size(&[
                Vec2::new(p1[0], p1[1]),
                Vec2::new(p2[0], p2[1]),
                Vec2::new(p3[0], p3[1]),
            ])
        }
        ".quad" => {
            let p1 = get_shape_vec2_property(properties, "p1", [-50.0, -50.0]);
            let p2 = get_shape_vec2_property(properties, "p2", [50.0, -50.0]);
            let p3 = get_shape_vec2_property(properties, "p3", [50.0, 50.0]);
            let p4 = get_shape_vec2_property(properties, "p4", [-50.0, 50.0]);
            points_bounding_size(&[
                Vec2::new(p1[0], p1[1]),
                Vec2::new(p2[0], p2[1]),
                Vec2::new(p3[0], p3[1]),
                Vec2::new(p4[0], p4[1]),
            ])
        }
        ".penta" => {
            let p1 = get_shape_vec2_property(properties, "p1", [0.0, -50.0]);
            let p2 = get_shape_vec2_property(properties, "p2", [-47.5, -15.5]);
            let p3 = get_shape_vec2_property(properties, "p3", [-29.4, 40.5]);
            let p4 = get_shape_vec2_property(properties, "p4", [29.4, 40.5]);
            let p5 = get_shape_vec2_property(properties, "p5", [47.5, -15.5]);
            points_bounding_size(&[
                Vec2::new(p1[0], p1[1]),
                Vec2::new(p2[0], p2[1]),
                Vec2::new(p3[0], p3[1]),
                Vec2::new(p4[0], p4[1]),
                Vec2::new(p5[0], p5[1]),
            ])
        }
        ".arrow" => {
            let start = get_shape_vec2_property(properties, "start", [0.0, 0.0]);
            let end = get_shape_vec2_property(properties, "end", [100.0, 0.0]);
            let line_width = get_shape_float_property(properties, "lineWidth", 20.0);
            let head_width = get_shape_float_property(properties, "headWidth", 40.0);
            let head_length = get_shape_float_property(properties, "headLength", 30.0);
            let points = arrow_outline_points(
                Vec2::new(start[0], start[1]),
                Vec2::new(end[0], end[1]),
                line_width,
                head_width,
                head_length,
            );
            points_bounding_size(&points)
        }
        _ => None,
    }
}

pub(crate) fn get_shape_size(
    properties: &[AmProperty],
    shape_type: &str,
    _fill_type: &str,
) -> (f32, f32) {
    for prop in properties {
        if prop.name != "size" || prop.prop_type != "vec2" {
            continue;
        }
        if !prop.value.is_empty()
            && let Ok(size) = crate::schema::parse_vec2(&prop.value)
        {
            return ((size[0] * 2.0).abs(), (size[1] * 2.0).abs());
        }
        if prop.keyframes.is_empty() {
            continue;
        }
        let mut sorted: Vec<_> = prop.keyframes.iter().collect();
        sorted.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if let Ok(size) = crate::schema::parse_vec2(&sorted[0].value) {
            return ((size[0] * 2.0).abs(), (size[1] * 2.0).abs());
        }
    }
    infer_shape_size_from_properties(properties, shape_type).unwrap_or((100.0, 100.0))
}

pub(crate) fn get_shape_size_animation(
    properties: &[AmProperty],
    shape_type: &str,
) -> AmAnimatedVec2 {
    for prop in properties {
        if prop.name == "size" && prop.prop_type == "vec2" {
            let value = if !prop.value.is_empty() {
                crate::schema::parse_vec2(&prop.value)
                    .ok()
                    .map(|s| [s[0] * 2.0, s[1] * 2.0])
            } else {
                None
            };

            let keyframes: Vec<AmKeyframe> = prop
                .keyframes
                .iter()
                .map(|kf| {
                    let converted_value = crate::schema::parse_vec2(&kf.value)
                        .map(|s| format!("{},{}", s[0] * 2.0, s[1] * 2.0))
                        .unwrap_or_else(|_| kf.value.clone());
                    AmKeyframe {
                        time: kf.time,
                        value: converted_value,
                        easing: kf.easing.clone(),
                    }
                })
                .collect();

            return AmAnimatedVec2 { value, keyframes };
        }
    }

    let (width, height) =
        infer_shape_size_from_properties(properties, shape_type).unwrap_or((100.0, 100.0));
    AmAnimatedVec2 {
        value: Some([width, height]),
        keyframes: Vec::new(),
    }
}

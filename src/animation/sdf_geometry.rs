use bevy::prelude::*;

use crate::sdf_material::{SdfMaterialUniform, SdfShapeType};

fn vertex_scale(half_width: f32, base_half_width: f32) -> f32 {
    if base_half_width.abs() > 0.01 {
        half_width / base_half_width
    } else {
        1.0
    }
}

fn max_abs_extent(points: &[Vec2]) -> f32 {
    points.iter().fold(0.0_f32, |extent, point| {
        extent.max(point.x.abs()).max(point.y.abs())
    })
}

fn arrow_points(shape_extra: Vec4, shape_extra2: Vec4, vertex_scale: f32) -> [Vec2; 7] {
    let start = Vec2::new(shape_extra.x, shape_extra.y) * vertex_scale;
    let end = Vec2::new(shape_extra.z, shape_extra.w) * vertex_scale;
    let width_scale = vertex_scale.abs();
    let head_width = (shape_extra2.y * width_scale).abs();
    let line_width = (shape_extra2.x * width_scale).abs().min(head_width);
    let mut head_length = (shape_extra2.z * width_scale).abs();

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

pub(crate) fn compute_sdf_shape_half_extent_from_parts(
    shape_type: SdfShapeType,
    half_width: f32,
    half_height: f32,
    base_half_width: f32,
    shape_extra: Vec4,
    shape_extra2: Vec4,
    shape_extra3: Vec4,
    shape_extra4: Vec4,
    shape_extra5: Vec4,
    shape_extra6: Vec4,
    shape_extra7: Vec4,
) -> f32 {
    let point_scale = vertex_scale(half_width, base_half_width);
    let radius_scale = point_scale.abs();

    match shape_type {
        SdfShapeType::Polygon => (shape_extra.y * radius_scale).abs(),
        SdfShapeType::Star | SdfShapeType::Multifoil => (shape_extra.y * radius_scale)
            .abs()
            .max((shape_extra.z * radius_scale).abs()),
        SdfShapeType::Pie | SdfShapeType::Arc => (shape_extra.z * radius_scale).abs(),
        SdfShapeType::Line => {
            let points = [
                Vec2::new(shape_extra.x, shape_extra.y) * point_scale,
                Vec2::new(shape_extra.z, shape_extra.w) * point_scale,
            ];
            max_abs_extent(&points)
        }
        SdfShapeType::Triangle => {
            let points = [
                Vec2::new(shape_extra.x, shape_extra.y) * point_scale,
                Vec2::new(shape_extra.z, shape_extra.w) * point_scale,
                Vec2::new(shape_extra2.x, shape_extra2.y) * point_scale,
            ];
            max_abs_extent(&points)
        }
        SdfShapeType::Quad => {
            let points = [
                Vec2::new(shape_extra.x, shape_extra.y) * point_scale,
                Vec2::new(shape_extra.z, shape_extra.w) * point_scale,
                Vec2::new(shape_extra2.x, shape_extra2.y) * point_scale,
                Vec2::new(shape_extra2.z, shape_extra2.w) * point_scale,
            ];
            max_abs_extent(&points)
        }
        SdfShapeType::Penta => {
            let points = [
                Vec2::new(shape_extra.x, shape_extra.y) * point_scale,
                Vec2::new(shape_extra.z, shape_extra.w) * point_scale,
                Vec2::new(shape_extra2.x, shape_extra2.y) * point_scale,
                Vec2::new(shape_extra2.z, shape_extra2.w) * point_scale,
                Vec2::new(shape_extra3.x, shape_extra3.y) * point_scale,
            ];
            max_abs_extent(&points)
        }
        SdfShapeType::Path => {
            let points = [
                Vec2::new(shape_extra.x, shape_extra.y) * point_scale,
                Vec2::new(shape_extra.z, shape_extra.w) * point_scale,
                Vec2::new(shape_extra2.x, shape_extra2.y) * point_scale,
                Vec2::new(shape_extra2.z, shape_extra2.w) * point_scale,
                Vec2::new(shape_extra3.x, shape_extra3.y) * point_scale,
                Vec2::new(shape_extra3.z, shape_extra3.w) * point_scale,
                Vec2::new(shape_extra4.x, shape_extra4.y) * point_scale,
                Vec2::new(shape_extra4.z, shape_extra4.w) * point_scale,
                Vec2::new(shape_extra5.x, shape_extra5.y) * point_scale,
                Vec2::new(shape_extra5.z, shape_extra5.w) * point_scale,
                Vec2::new(shape_extra6.x, shape_extra6.y) * point_scale,
                Vec2::new(shape_extra6.z, shape_extra6.w) * point_scale,
                Vec2::new(shape_extra7.x, shape_extra7.y) * point_scale,
            ];
            max_abs_extent(&points)
        }
        SdfShapeType::Arrow => {
            max_abs_extent(&arrow_points(shape_extra, shape_extra2, point_scale))
        }
        _ => half_width.abs().max(half_height.abs()),
    }
}

pub(crate) fn compute_sdf_shape_half_extent(uniform: &SdfMaterialUniform) -> f32 {
    let shape_type = match uniform.shape_type.round() as i32 {
        0 => SdfShapeType::BoxRound,
        1 => SdfShapeType::BoxMiter,
        2 => SdfShapeType::BoxBevel,
        3 => SdfShapeType::Circle,
        4 => SdfShapeType::RoundRect,
        5 => SdfShapeType::Polygon,
        6 => SdfShapeType::Star,
        7 => SdfShapeType::Pie,
        8 => SdfShapeType::Plus,
        9 => SdfShapeType::Multifoil,
        10 => SdfShapeType::Line,
        11 => SdfShapeType::Arc,
        12 => SdfShapeType::Triangle,
        13 => SdfShapeType::Quad,
        14 => SdfShapeType::Penta,
        15 => SdfShapeType::Path,
        16 => SdfShapeType::Arrow,
        _ => SdfShapeType::BoxRound,
    };

    compute_sdf_shape_half_extent_from_parts(
        shape_type,
        uniform.params.x,
        uniform.params.y,
        uniform.base_half_width,
        uniform.shape_extra,
        uniform.shape_extra2,
        uniform.shape_extra3,
        uniform.shape_extra4,
        uniform.shape_extra5,
        uniform.shape_extra6,
        uniform.shape_extra7,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_extent_uses_tip_and_head_width() {
        let extent = compute_sdf_shape_half_extent_from_parts(
            SdfShapeType::Arrow,
            50.0,
            50.0,
            50.0,
            Vec4::new(0.0, 0.0, 100.0, 0.0),
            Vec4::new(20.0, 80.0, 30.0, 0.0),
            Vec4::ZERO,
            Vec4::ZERO,
            Vec4::ZERO,
            Vec4::ZERO,
            Vec4::ZERO,
        );

        assert!((extent - 100.0).abs() < 0.001);
    }

    #[test]
    fn pie_extent_uses_radius_slot() {
        let extent = compute_sdf_shape_half_extent_from_parts(
            SdfShapeType::Pie,
            50.0,
            50.0,
            50.0,
            Vec4::new(0.0, 270.0, 123.0, 0.0),
            Vec4::ZERO,
            Vec4::ZERO,
            Vec4::ZERO,
            Vec4::ZERO,
            Vec4::ZERO,
            Vec4::ZERO,
        );

        assert!((extent - 123.0).abs() < 0.001);
    }
}

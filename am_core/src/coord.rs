//! Coordinate mapping presets and transform matrix construction.

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoordMappingConfig {
    pub origin_anchor: [f32; 2],
    pub x_direction: f32,
    pub y_direction: f32,
    pub rotation_sign: f32,
    pub rotation_zero_axis: [f32; 2],
    pub engine_anchor: [f32; 2],
    pub z_spacing: f32,
    pub column_major: bool,
}

impl CoordMappingConfig {
    pub const AM_NATIVE: Self = Self {
        origin_anchor: [0.0, 0.0],
        x_direction: 1.0,
        y_direction: 1.0,
        rotation_sign: -1.0,
        rotation_zero_axis: [1.0, 0.0],
        engine_anchor: [0.5, 0.5],
        z_spacing: 0.001,
        column_major: true,
    };

    pub const BEVY_2D: Self = Self {
        origin_anchor: [0.5, 0.5],
        x_direction: 1.0,
        y_direction: -1.0,
        rotation_sign: 1.0,
        rotation_zero_axis: [1.0, 0.0],
        engine_anchor: [0.5, 0.5],
        z_spacing: 0.001,
        column_major: true,
    };

    pub const UNITY_UI: Self = Self {
        origin_anchor: [0.0, 1.0],
        x_direction: 1.0,
        y_direction: -1.0,
        rotation_sign: 1.0,
        rotation_zero_axis: [1.0, 0.0],
        engine_anchor: [0.5, 0.5],
        z_spacing: 1.0,
        column_major: false,
    };

    pub const UNITY_WORLD: Self = Self {
        origin_anchor: [0.5, 0.5],
        x_direction: 1.0,
        y_direction: -1.0,
        rotation_sign: 1.0,
        rotation_zero_axis: [1.0, 0.0],
        engine_anchor: [0.5, 0.5],
        z_spacing: 1.0,
        column_major: false,
    };

    pub const GODOT_2D: Self = Self {
        origin_anchor: [0.0, 0.0],
        x_direction: 1.0,
        y_direction: 1.0,
        rotation_sign: -1.0,
        rotation_zero_axis: [1.0, 0.0],
        engine_anchor: [0.5, 0.5],
        z_spacing: 0.001,
        column_major: true,
    };

    pub const GODOT_CONTROL: Self = Self {
        origin_anchor: [0.0, 0.0],
        x_direction: 1.0,
        y_direction: 1.0,
        rotation_sign: -1.0,
        rotation_zero_axis: [1.0, 0.0],
        engine_anchor: [0.0, 0.0],
        z_spacing: 0.001,
        column_major: true,
    };

    pub const CSS: Self = Self {
        origin_anchor: [0.0, 0.0],
        x_direction: 1.0,
        y_direction: 1.0,
        rotation_sign: -1.0,
        rotation_zero_axis: [1.0, 0.0],
        engine_anchor: [0.0, 0.0],
        z_spacing: 1.0,
        column_major: true,
    };

    pub const OPENGL_NDC: Self = Self {
        origin_anchor: [0.5, 0.5],
        x_direction: 1.0,
        y_direction: -1.0,
        rotation_sign: 1.0,
        rotation_zero_axis: [1.0, 0.0],
        engine_anchor: [0.5, 0.5],
        z_spacing: 0.001,
        column_major: true,
    };
}

impl Default for CoordMappingConfig {
    fn default() -> Self {
        Self::AM_NATIVE
    }
}

pub fn apply_coord_mapping(
    am_position: [f32; 2],
    am_rotation_deg: f32,
    am_scale: [f32; 2],
    element_size: [f32; 2],
    canvas_size: [f32; 2],
    layer_index: i32,
    config: &CoordMappingConfig,
) -> [f32; 16] {
    let origin_x = config.origin_anchor[0] * canvas_size[0];
    let origin_y = config.origin_anchor[1] * canvas_size[1];

    let anchor_dx = (0.5 - config.engine_anchor[0]) * element_size[0];
    let anchor_dy = (0.5 - config.engine_anchor[1]) * element_size[1];

    let target_x = (am_position[0] + anchor_dx - origin_x) * config.x_direction;
    let target_y = (am_position[1] + anchor_dy - origin_y) * config.y_direction;
    let target_rotation = am_rotation_deg * config.rotation_sign;
    let z = layer_index as f32 * config.z_spacing;

    let matrix = build_transform_matrix(
        target_x,
        target_y,
        target_rotation,
        config.rotation_zero_axis,
        am_scale,
        z,
    );

    if config.column_major {
        matrix
    } else {
        transpose_4x4(matrix)
    }
}

pub fn build_transform_matrix(
    x: f32,
    y: f32,
    rotation_deg: f32,
    rotation_zero_axis: [f32; 2],
    scale: [f32; 2],
    z: f32,
) -> [f32; 16] {
    let zero_angle = rotation_zero_axis[1].atan2(rotation_zero_axis[0]);
    let angle = zero_angle + rotation_deg.to_radians();
    let cos = angle.cos();
    let sin = angle.sin();
    let sx = scale[0];
    let sy = scale[1];

    [
        cos * sx,
        sin * sx,
        0.0,
        0.0,
        -sin * sy,
        cos * sy,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        x,
        y,
        z,
        1.0,
    ]
}

pub fn multiply_4x4_column_major(a: [f32; 16], b: [f32; 16]) -> [f32; 16] {
    let mut out = [0.0; 16];
    for col in 0..4 {
        for row in 0..4 {
            out[col * 4 + row] = a[row] * b[col * 4]
                + a[4 + row] * b[col * 4 + 1]
                + a[8 + row] * b[col * 4 + 2]
                + a[12 + row] * b[col * 4 + 3];
        }
    }
    out
}

pub fn transpose_4x4(matrix: [f32; 16]) -> [f32; 16] {
    [
        matrix[0], matrix[4], matrix[8], matrix[12], matrix[1], matrix[5], matrix[9], matrix[13],
        matrix[2], matrix[6], matrix[10], matrix[14], matrix[3], matrix[7], matrix[11],
        matrix[15],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bevy_preset_maps_center_to_origin() {
        let matrix = apply_coord_mapping(
            [640.0, 480.0],
            0.0,
            [1.0, 1.0],
            [100.0, 100.0],
            [1280.0, 960.0],
            0,
            &CoordMappingConfig::BEVY_2D,
        );

        assert!((matrix[12] - 0.0).abs() < 0.001);
        assert!((matrix[13] - 0.0).abs() < 0.001);
    }
}

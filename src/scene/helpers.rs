//! # helpers.rs
//!
//! # 场景辅助函数模块
//!
//! Helper functions for scene building.
//! 场景构建辅助函数。

use bevy::prelude::*;

use super::components::AmSceneConfig;
use crate::schema::{AmAnimatedFloat, AmAnimatedVec2, AmAnimatedVec3};

pub fn am_to_bevy_coords(x: f32, y: f32, config: &AmSceneConfig) -> (f32, f32) {
    let bx = x - config.canvas_width / 2.0;
    let by = if config.flip_y {
        config.canvas_height / 2.0 - y
    } else {
        y - config.canvas_height / 2.0
    };
    (bx, by)
}
pub(crate) fn get_initial_location(
    prop: &AmAnimatedVec3,
    config: &AmSceneConfig,
    has_parent: bool,
) -> (f32, f32) {
    let (x, y) = if let Some(val) = &prop.value {
        (val[0], val[1])
    } else if !prop.keyframes.is_empty() {
        // Sort keyframes by time and get the first one
        let mut sorted: Vec<_> = prop.keyframes.iter().collect();
        sorted.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        crate::schema::parse_vec3(&sorted[0].value)
            .map(|v| (v[0], v[1]))
            .unwrap_or((0.0, 0.0))
    } else if has_parent {
        (0.0, 0.0) // Local origin for children
    } else {
        (config.canvas_width / 2.0, config.canvas_height / 2.0) // Canvas center for root
    };

    if has_parent {
        // For layers with parents, use local coordinates
        // Only flip Y axis (AM Y-down -> Bevy Y-up)
        (x, -y)
    } else {
        // For root layers, convert from canvas coordinates
        am_to_bevy_coords(x, y, config)
    }
}

/// Get initial rotation from animated property.
pub(crate) fn get_initial_rotation(prop: &AmAnimatedFloat) -> f32 {
    if let Some(val) = prop.value {
        -val // Negate for Bevy's coordinate system
    } else if !prop.keyframes.is_empty() {
        // Sort keyframes by time and get the first one
        let mut sorted: Vec<_> = prop.keyframes.iter().collect();
        sorted.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        -sorted[0].value.parse().unwrap_or(0.0)
    } else {
        0.0
    }
}

/// Get initial scale from animated property.
pub(crate) fn get_initial_scale(prop: &AmAnimatedVec2) -> (f32, f32) {
    if let Some(val) = &prop.value {
        (val[0], val[1])
    } else if !prop.keyframes.is_empty() {
        // Sort keyframes by time and get the first one
        let mut sorted: Vec<_> = prop.keyframes.iter().collect();
        sorted.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        crate::schema::parse_vec2(&sorted[0].value)
            .unwrap_or([1.0, 1.0])
            .into()
    } else {
        (1.0, 1.0)
    }
}

/// Get initial pivot from animated property.
pub(crate) fn get_initial_pivot(prop: &AmAnimatedVec2) -> (f32, f32) {
    if let Some(val) = &prop.value {
        (val[0], val[1])
    } else if !prop.keyframes.is_empty() {
        let mut sorted: Vec<_> = prop.keyframes.iter().collect();
        sorted.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        crate::schema::parse_vec2(&sorted[0].value)
            .unwrap_or([0.0, 0.0])
            .into()
    } else {
        (0.0, 0.0)
    }
}

/// Calculate pivot compensation for non-unit scale.
/// AM transforms around (location + pivot), Bevy transforms around entity origin.
/// This function calculates the position compensation needed when scale != 1.
///
/// Note: Rotation is handled by Bevy's transform system - we don't need to compensate for it here.
/// The key insight is that pivot compensation is about WHERE the scale happens, not about rotation.
///
/// Returns (compensation_x, compensation_y) in Bevy coordinates.
/// Calculate the final position for an entity with pivot-based rotation and scaling.
///
/// In AM, pivot defines the rotation/scaling center relative to the object's location.
/// When rotation and scaling are applied around the pivot, the object's visual center
/// moves to a new position.
///
/// This function calculates the position compensation so that the entity's Transform.translation
/// results in the correct visual position after Bevy applies rotation and scaling.
pub(crate) fn calculate_embed_position_compensation(
    pivot: (f32, f32),
    scale: (f32, f32),
    rotation_deg: f32,
    has_parent: bool,
) -> (f32, f32) {
    // Convert pivot to Bevy coordinates (X same, Y flipped if root)
    let pivot_x = pivot.0;
    let pivot_y = if has_parent { pivot.1 } else { -pivot.1 };

    // Object offset from rotation center is -pivot (in Bevy coords)
    // After scaling
    let scaled_offset_x = -pivot_x * scale.0;
    let scaled_offset_y = -pivot_y * scale.1;

    // After rotation (Bevy uses opposite rotation direction)
    let rotation_rad = (-rotation_deg).to_radians();
    let rotated_offset_x =
        scaled_offset_x * rotation_rad.cos() - scaled_offset_y * rotation_rad.sin();
    let rotated_offset_y =
        scaled_offset_x * rotation_rad.sin() + scaled_offset_y * rotation_rad.cos();

    // The compensation is: rotated_offset - original_offset
    // original_offset is -pivot, so: rotated_offset + pivot
    let comp_x = rotated_offset_x + pivot_x;
    let comp_y = rotated_offset_y + pivot_y;

    (comp_x, comp_y)
}

#[allow(dead_code)]
pub(crate) fn calculate_pivot_compensation(
    pivot: (f32, f32),
    scale: (f32, f32),
    _rotation_deg: f32, // Kept for API compatibility, but not used
    has_parent: bool,
) -> (f32, f32) {
    let pivot_x = pivot.0;
    let pivot_y = pivot.1;

    // Compensation formula: pivot * (1 - scale)
    // This moves the entity position to keep the visual center correct after scaling
    let comp_x = pivot_x * (1.0 - scale.0);
    let comp_y = if has_parent {
        pivot_y * (1.0 - scale.1) // Y already flipped in parent coordinate system
    } else {
        -pivot_y * (1.0 - scale.1) // Flip Y for Bevy (AM Y-down, Bevy Y-up)
    };

    (comp_x, comp_y)
}

/// Get initial opacity from animated property.
pub(crate) fn get_initial_opacity(prop: &AmAnimatedFloat) -> f32 {
    if let Some(val) = prop.value {
        val
    } else if !prop.keyframes.is_empty() {
        // Sort keyframes by time and get the first one
        let mut sorted: Vec<_> = prop.keyframes.iter().collect();
        sorted.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted[0].value.parse().unwrap_or(1.0)
    } else {
        1.0
    }
}
pub(crate) fn get_shape_size(
    properties: &[crate::schema::AmProperty],
    _fill_type: &str,
) -> (f32, f32) {
    for prop in properties {
        if prop.name != "size" || prop.prop_type != "vec2" {
            continue;
        }
        // Check static value first
        // AM's size property represents half-extents, multiply by 2 for full dimensions
        if !prop.value.is_empty()
            && let Ok(size) = crate::schema::parse_vec2(&prop.value)
        {
            return ((size[0] * 2.0).abs(), (size[1] * 2.0).abs());
        }
        // If no static value, check first keyframe
        if prop.keyframes.is_empty() {
            continue;
        }
        // Find earliest keyframe
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
    (100.0, 100.0)
}

/// Get shape size animation data from properties.
/// AM's size property represents half-extents, so we multiply by 2 for full dimensions.
/// Returns AmAnimatedVec2 with values in full dimensions (width, height).
pub(crate) fn get_shape_size_animation(
    properties: &[crate::schema::AmProperty],
) -> crate::schema::AmAnimatedVec2 {
    use crate::schema::{AmAnimatedVec2, AmKeyframe};

    for prop in properties {
        if prop.name == "size" && prop.prop_type == "vec2" {
            // Convert static value (half-extents to full dimensions)
            let value = if !prop.value.is_empty() {
                crate::schema::parse_vec2(&prop.value)
                    .ok()
                    .map(|s| [s[0] * 2.0, s[1] * 2.0])
            } else {
                None
            };

            // Convert keyframes (half-extents to full dimensions)
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

    // Default: 100x100 (full dimensions)
    AmAnimatedVec2 {
        value: Some([100.0, 100.0]),
        keyframes: Vec::new(),
    }
}
pub(crate) fn get_stroke_width_animation(
    stroke: Option<&crate::schema::AmStroke>,
) -> crate::schema::AmAnimatedFloat {
    use crate::schema::AmAnimatedFloat;

    if let Some(stroke) = stroke {
        // First check for <size> element (animated or static)
        if let Some(ref size) = stroke.size {
            // Check if there are keyframes
            if !size.keyframes.is_empty() {
                return AmAnimatedFloat {
                    value: size.value,
                    keyframes: size.keyframes.clone(),
                };
            }
            // Static value only
            return AmAnimatedFloat {
                value: size.value,
                keyframes: Vec::new(),
            };
        }

        // Fall back: AM's default stroke size for path-stroke is 4.0
        // (from KeyableEdgeDecoration.NO_STROKE template)
        return AmAnimatedFloat {
            value: Some(4.0),
            keyframes: Vec::new(),
        };
    }

    // Default: no stroke width
    AmAnimatedFloat {
        value: Some(0.0),
        keyframes: Vec::new(),
    }
}
/// Get the base alpha from fill color, considering no_fill flag.
/// When no_fill is true, returns 0.0 regardless of fillColor value.
pub(crate) fn get_base_alpha(
    fill_color: &Option<crate::schema::AmFillColor>,
    no_fill: bool,
) -> f32 {
    // fillType="none" means no fill, alpha should be 0
    if no_fill {
        return 0.0;
    }

    if let Some(fc) = fill_color {
        if !fc.value.is_empty() {
            if let Ok(c) = crate::schema::parse_color(&fc.value) {
                return c[3]; // alpha is the 4th component
            }
        } else if !fc.keyframes.is_empty() {
            // For animated fill color, use the first keyframe's alpha
            let mut sorted: Vec<_> = fc.keyframes.iter().collect();
            sorted.sort_by(|a, b| {
                a.time
                    .partial_cmp(&b.time)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            if let Ok(c) = crate::schema::parse_color(&sorted[0].value) {
                return c[3];
            }
        }
    }
    1.0 // Default to fully opaque
}

pub(crate) fn get_initial_fill_color_rgba(
    fill_color: &Option<crate::schema::AmFillColor>,
    no_fill: bool,
) -> [f32; 4] {
    if no_fill {
        return [0.0; 4];
    }
    if let Some(fc) = fill_color {
        if !fc.value.is_empty() {
            if let Ok(c) = crate::schema::parse_color(&fc.value) {
                return c;
            }
        } else if !fc.keyframes.is_empty() {
            let mut sorted: Vec<_> = fc.keyframes.iter().collect();
            sorted.sort_by(|a, b| {
                a.time
                    .partial_cmp(&b.time)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            if let Ok(c) = crate::schema::parse_color(&sorted[0].value) {
                return c;
            }
        }
    }
    [1.0, 1.0, 1.0, 1.0]
}

pub(crate) fn pivot_to_anchor_and_offset(
    pivot_x: f32,
    pivot_y: f32,
    width: f32,
    height: f32,
) -> (bevy::sprite::Anchor, f32, f32) {
    if pivot_x == 0.0 && pivot_y == 0.0 {
        return (bevy::sprite::Anchor::CENTER, 0.0, 0.0);
    }

    // Convert pixel offset to normalized anchor
    // AM: pivot (px, py) means: "the anchor point is at (center + pivot)"
    // Bevy: anchor value of 0.5 corresponds to half the sprite size
    // So anchor = pivot / size (where size is the full dimension)
    let anchor_x = if width > 0.0 { pivot_x / width } else { 0.0 };
    let anchor_y = if height > 0.0 {
        // Y is inverted: AM Y-down, Bevy Y-up
        -pivot_y / height
    } else {
        0.0
    };

    // Position compensation: when anchor is not center, we need to offset position
    // so that the sprite center stays at the same world position.
    // Bevy draws sprite such that anchor point is at translation.
    // To keep center at (tx, ty), we need to move translation by anchor * size.
    // In Bevy coords: compensation = (anchor_x * width, anchor_y * height)
    let comp_x = anchor_x * width;
    let comp_y = anchor_y * height;

    (
        bevy::sprite::Anchor(Vec2::new(anchor_x, anchor_y)),
        comp_x,
        comp_y,
    )
}
pub(crate) fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
pub(crate) fn get_scale_at_normalized_time(
    prop: &crate::schema::AmAnimatedVec2,
    t: f32,
) -> (f32, f32) {
    // If there's a static value, use it
    if let Some(val) = &prop.value {
        return (val[0], val[1]);
    }

    // If no keyframes, default to 1.0
    if prop.keyframes.is_empty() {
        return (1.0, 1.0);
    }

    // Sort keyframes by time
    let mut sorted: Vec<_> = prop.keyframes.iter().collect();
    sorted.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // If t is before or at the first keyframe, use the first keyframe value
    if t <= sorted[0].time {
        return crate::schema::parse_vec2(&sorted[0].value)
            .map(|v| (v[0], v[1]))
            .unwrap_or((1.0, 1.0));
    }

    // If t is after or at the last keyframe, use the last keyframe value
    let last = sorted.last().unwrap();
    if t >= last.time {
        return crate::schema::parse_vec2(&last.value)
            .map(|v| (v[0], v[1]))
            .unwrap_or((1.0, 1.0));
    }

    // Find the surrounding keyframes and interpolate
    for i in 0..sorted.len() - 1 {
        let kf_prev = sorted[i];
        let kf_next = sorted[i + 1];

        if t >= kf_prev.time && t <= kf_next.time {
            let v_prev = crate::schema::parse_vec2(&kf_prev.value).unwrap_or([1.0, 1.0]);
            let v_next = crate::schema::parse_vec2(&kf_next.value).unwrap_or([1.0, 1.0]);

            let span = kf_next.time - kf_prev.time;
            let local_t = if span > 0.0 {
                (t - kf_prev.time) / span
            } else {
                0.0
            };

            return (
                v_prev[0] + (v_next[0] - v_prev[0]) * local_t,
                v_prev[1] + (v_next[1] - v_prev[1]) * local_t,
            );
        }
    }

    // Fallback
    (1.0, 1.0)
}

/// Extract a float property from shape properties by name.
pub(crate) fn get_shape_float_property(
    properties: &[crate::schema::AmProperty],
    name: &str,
    default: f32,
) -> f32 {
    for prop in properties {
        if prop.name == name && prop.prop_type == "float" {
            if !prop.value.is_empty()
                && let Ok(v) = prop.value.parse::<f32>()
            {
                return v;
            }
            if let Some(kf) = prop.keyframes.first()
                && let Ok(v) = kf.value.parse::<f32>()
            {
                return v;
            }
        }
    }
    default
}

/// Extract a float property animation from shape properties by name.
pub(crate) fn get_shape_float_animation(
    properties: &[crate::schema::AmProperty],
    name: &str,
    default: f32,
) -> crate::schema::AmAnimatedFloat {
    use crate::schema::AmAnimatedFloat;
    for prop in properties {
        if prop.name == name && prop.prop_type == "float" {
            let value = if !prop.value.is_empty() {
                prop.value.parse::<f32>().ok()
            } else {
                None
            };
            return AmAnimatedFloat {
                value: value.or(Some(default)),
                keyframes: prop.keyframes.clone(),
            };
        }
    }
    AmAnimatedFloat {
        value: Some(default),
        keyframes: Vec::new(),
    }
}

/// Extract a vec2 property from shape properties by name.
pub(crate) fn get_shape_vec2_property(
    properties: &[crate::schema::AmProperty],
    name: &str,
    default: [f32; 2],
) -> [f32; 2] {
    for prop in properties {
        if prop.name == name && prop.prop_type == "vec2" {
            if !prop.value.is_empty()
                && let Ok(v) = crate::schema::parse_vec2(&prop.value)
            {
                return v;
            }
            if let Some(kf) = prop.keyframes.first()
                && let Ok(v) = crate::schema::parse_vec2(&kf.value)
            {
                return v;
            }
        }
    }
    default
}

/// Extract a vec2 property animation from shape properties by name.
pub(crate) fn get_shape_vec2_animation(
    properties: &[crate::schema::AmProperty],
    name: &str,
    default: [f32; 2],
) -> crate::schema::AmAnimatedVec2 {
    use crate::schema::{AmAnimatedVec2, AmKeyframe};
    for prop in properties {
        if prop.name == name && prop.prop_type == "vec2" {
            let value = if !prop.value.is_empty() {
                crate::schema::parse_vec2(&prop.value).ok()
            } else {
                None
            };
            let keyframes: Vec<AmKeyframe> = prop.keyframes.clone();
            return AmAnimatedVec2 {
                value: value.or(Some(default)),
                keyframes,
            };
        }
    }
    AmAnimatedVec2 {
        value: Some(default),
        keyframes: Vec::new(),
    }
}

/// Extract animated shape properties based on shape type.
/// Returns ([4 float props], [5 vec2 points]) with animation keyframes.
pub(crate) fn extract_shape_animations(
    shape_type: &str,
    properties: &[crate::schema::AmProperty],
) -> (
    [crate::schema::AmAnimatedFloat; 4],
    [crate::schema::AmAnimatedVec2; 5],
) {
    use crate::schema::{AmAnimatedFloat, AmAnimatedVec2};
    let df = || AmAnimatedFloat {
        value: Some(0.0),
        keyframes: Vec::new(),
    };
    let dv = || AmAnimatedVec2 {
        value: Some([0.0, 0.0]),
        keyframes: Vec::new(),
    };

    let props = match shape_type {
        ".roundrect" => [
            get_shape_float_animation(properties, "cornerRadius", 0.0),
            df(),
            df(),
            df(),
        ],
        ".poly" => [
            get_shape_float_animation(properties, "sideCount", 6.0),
            get_shape_float_animation(properties, "radius", 50.0),
            get_shape_float_animation(properties, "offsetAngle", 0.0),
            df(),
        ],
        ".star" => [
            get_shape_float_animation(properties, "pointCount", 5.0),
            get_shape_float_animation(properties, "outerRadius", 50.0),
            get_shape_float_animation(properties, "innerRadius", 25.0),
            get_shape_float_animation(properties, "offsetAngle", 0.0),
        ],
        ".pie" => [
            get_shape_float_animation(properties, "startAngle", 0.0),
            get_shape_float_animation(properties, "endAngle", 270.0),
            get_shape_float_animation(properties, "radius", 50.0),
            df(),
        ],
        ".plus" => [
            get_shape_float_animation(properties, "stemSize", 50.0),
            df(),
            df(),
            df(),
        ],
        ".multifoil" => [
            get_shape_float_animation(properties, "pointCount", 5.0),
            get_shape_float_animation(properties, "outerRadius", 50.0),
            get_shape_float_animation(properties, "innerRadius", 25.0),
            get_shape_float_animation(properties, "offsetAngle", 0.0),
        ],
        ".arc" => [
            get_shape_float_animation(properties, "startAngle", 0.0),
            get_shape_float_animation(properties, "endAngle", 270.0),
            get_shape_float_animation(properties, "radius", 50.0),
            df(),
        ],
        ".line" => {
            let p = [df(), df(), df(), df()];
            return (
                p,
                [
                    get_shape_vec2_animation(properties, "p1", [0.0, 0.0]),
                    get_shape_vec2_animation(properties, "p2", [50.0, 0.0]),
                    dv(),
                    dv(),
                    dv(),
                ],
            );
        }
        ".triangle" => {
            let p = [df(), df(), df(), df()];
            return (
                p,
                [
                    get_shape_vec2_animation(properties, "p1", [0.0, -50.0]),
                    get_shape_vec2_animation(properties, "p2", [-50.0, 50.0]),
                    get_shape_vec2_animation(properties, "p3", [50.0, 50.0]),
                    dv(),
                    dv(),
                ],
            );
        }
        ".quad" => {
            let p = [df(), df(), df(), df()];
            return (
                p,
                [
                    get_shape_vec2_animation(properties, "p1", [-50.0, -50.0]),
                    get_shape_vec2_animation(properties, "p2", [50.0, -50.0]),
                    get_shape_vec2_animation(properties, "p3", [50.0, 50.0]),
                    get_shape_vec2_animation(properties, "p4", [-50.0, 50.0]),
                    dv(),
                ],
            );
        }
        ".penta" => {
            let p = [df(), df(), df(), df()];
            return (
                p,
                [
                    get_shape_vec2_animation(properties, "p1", [0.0, -50.0]),
                    get_shape_vec2_animation(properties, "p2", [-47.5, -15.5]),
                    get_shape_vec2_animation(properties, "p3", [-29.4, 40.5]),
                    get_shape_vec2_animation(properties, "p4", [29.4, 40.5]),
                    get_shape_vec2_animation(properties, "p5", [47.5, -15.5]),
                ],
            );
        }
        _ => [df(), df(), df(), df()],
    };
    (props, [dv(), dv(), dv(), dv(), dv()])
}

/// Extract gradient data from an AmGradient into uniform-ready values.
/// Returns (gradient_type, start_color, end_color, points).
pub(crate) fn extract_gradient_data(
    gradient: &Option<crate::schema::AmGradient>,
) -> (u8, bevy::math::Vec4, bevy::math::Vec4, bevy::math::Vec4) {
    use bevy::math::Vec4;
    if let Some(g) = gradient {
        let grad_type = match g.gradient_type.as_str() {
            "linear" => 1u8,
            "radial" => 2u8,
            "sweep" => 3u8,
            _ => 0u8,
        };
        if grad_type == 0 {
            return (0, Vec4::ZERO, Vec4::ZERO, Vec4::ZERO);
        }
        let start_color = crate::schema::parse_color(&g.start_color)
            .map(|c| {
                // Store in sRGB space for sRGB-space interpolation (matching AM's NanoVG)
                Vec4::new(c[0], c[1], c[2], c[3])
            })
            .unwrap_or(Vec4::ZERO);
        let end_color = crate::schema::parse_color(&g.end_color)
            .map(|c| Vec4::new(c[0], c[1], c[2], c[3]))
            .unwrap_or(Vec4::ZERO);
        let start_pt = g.start.unwrap_or([0.0, 0.0]);
        let end_pt = g.end.unwrap_or([1.0, 1.0]);
        let points = Vec4::new(start_pt[0], start_pt[1], end_pt[0], end_pt[1]);
        (grad_type, start_color, end_color, points)
    } else {
        (0, Vec4::ZERO, Vec4::ZERO, Vec4::ZERO)
    }
}

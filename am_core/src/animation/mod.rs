//! Animation math helpers shared by renderers and FFI consumers.

pub mod interpolation;

pub use interpolation::{
    find_keyframes, find_keyframes_internal, interpolate_color, interpolate_float,
    interpolate_float_reverse, interpolate_vec2, interpolate_vec2_reverse, interpolate_vec3,
    interpolate_vec3_reverse, interpolate_vec3_with_extrapolation, lerp, parse_keyframe_color,
    parse_keyframe_vec2, parse_keyframe_vec3,
};

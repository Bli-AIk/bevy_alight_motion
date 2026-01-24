//! # interpolation.rs
//!
//! # 插值模块
//!
//! Interpolation functions for animation keyframes.
//! Contains interpolate_vec3, interpolate_vec2, interpolate_float and related helpers.
//!
//! 动画关键帧的插值函数。
//! 包含 interpolate_vec3、interpolate_vec2、interpolate_float 及相关辅助函数。

use crate::schema::{AmAnimatedFloat, AmAnimatedVec2, AmAnimatedVec3, AmKeyframe, Easing};

/// Interpolate a Vec3 property at normalized time t.
pub fn interpolate_vec3(prop: &AmAnimatedVec3, t: f32) -> Option<[f32; 3]> {
    interpolate_vec3_internal(prop, t, false)
}

/// Interpolate a Vec3 property at normalized time t.
/// Before the first keyframe, holds the first keyframe value (AM behavior).
pub fn interpolate_vec3_with_extrapolation(prop: &AmAnimatedVec3, t: f32) -> Option<[f32; 3]> {
    // AM behavior: hold first keyframe value before first keyframe, don't extrapolate
    interpolate_vec3_internal(prop, t, false)
}

fn interpolate_vec3_internal(prop: &AmAnimatedVec3, t: f32, extrapolate: bool) -> Option<[f32; 3]> {
    if prop.keyframes.is_empty() {
        return prop.value;
    }

    let (kf_prev, kf_next, local_t) = find_keyframes_internal(&prop.keyframes, t, extrapolate);

    let v_prev = parse_keyframe_vec3(&kf_prev.value).unwrap_or([0.0, 0.0, 0.0]);
    let v_next = parse_keyframe_vec3(&kf_next.value).unwrap_or(v_prev);

    // Easing is defined on the "target" keyframe (describes how to arrive at it)
    // For extrapolation (local_t < 0), use linear interpolation
    let eased_t = if local_t < 0.0 {
        local_t
    } else {
        let easing = kf_next
            .easing
            .as_ref()
            .map(|e| Easing::parse(e))
            .unwrap_or_default();
        easing.evaluate(local_t)
    };

    Some([
        lerp(v_prev[0], v_next[0], eased_t),
        lerp(v_prev[1], v_next[1], eased_t),
        lerp(v_prev[2], v_next[2], eased_t),
    ])
}

/// Interpolate a Vec2 property at normalized time t.
/// Before the first keyframe, holds the first keyframe value (AM behavior).
pub fn interpolate_vec2(prop: &AmAnimatedVec2, t: f32) -> Option<[f32; 2]> {
    if prop.keyframes.is_empty() {
        return prop.value;
    }

    // AM behavior: hold first/last keyframe value outside keyframe range (no extrapolation)
    let (kf_prev, kf_next, local_t) = find_keyframes_internal(&prop.keyframes, t, false);

    let v_prev = parse_keyframe_vec2(&kf_prev.value).unwrap_or([1.0, 1.0]);
    let v_next = parse_keyframe_vec2(&kf_next.value).unwrap_or(v_prev);

    // Easing is defined on the "target" keyframe (describes how to arrive at it)
    let easing = kf_next
        .easing
        .as_ref()
        .map(|e| Easing::parse(e))
        .unwrap_or_default();
    let eased_t = easing.evaluate(local_t);

    Some([
        lerp(v_prev[0], v_next[0], eased_t),
        lerp(v_prev[1], v_next[1], eased_t),
    ])
}

/// Interpolate a float property at normalized time t.
/// Before the first keyframe, holds the first keyframe value (AM behavior).
pub fn interpolate_float(prop: &AmAnimatedFloat, t: f32) -> Option<f32> {
    if prop.keyframes.is_empty() {
        return prop.value;
    }

    // AM behavior: hold first/last keyframe value outside keyframe range (no extrapolation)
    let (kf_prev, kf_next, local_t) = find_keyframes_internal(&prop.keyframes, t, false);

    let v_prev: f32 = kf_prev.value.parse().unwrap_or(0.0);
    let v_next: f32 = kf_next.value.parse().unwrap_or(v_prev);

    // Easing is defined on the "target" keyframe (describes how to arrive at it)
    let easing = kf_next
        .easing
        .as_ref()
        .map(|e| Easing::parse(e))
        .unwrap_or_default();
    let eased_t = easing.evaluate(local_t);

    // Debug log for cyclic easing
    if matches!(easing, Easing::Cyclic { .. }) {
        bevy::log::trace!(
            "[interpolate_float] Cyclic easing: local_t={:.4}, eased_t={:.4}, v_prev={}, v_next={}, result={:.4}",
            local_t,
            eased_t,
            v_prev,
            v_next,
            lerp(v_prev, v_next, eased_t)
        );
    }

    Some(lerp(v_prev, v_next, eased_t))
}

/// Find the surrounding keyframes for a given time.
#[allow(dead_code)]
pub fn find_keyframes(keyframes: &[AmKeyframe], t: f32) -> (&AmKeyframe, &AmKeyframe, f32) {
    find_keyframes_internal(keyframes, t, true)
}

/// Find the surrounding keyframes for a given time with optional extrapolation.
pub fn find_keyframes_internal(
    keyframes: &[AmKeyframe],
    t: f32,
    extrapolate: bool,
) -> (&AmKeyframe, &AmKeyframe, f32) {
    // Sort keyframes by time (in case they're not sorted)
    let mut sorted: Vec<_> = keyframes.iter().collect();
    sorted.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Handle edge cases
    if sorted.len() == 1 {
        return (sorted[0], sorted[0], 0.0);
    }

    // Find surrounding keyframes
    for i in 0..sorted.len() - 1 {
        let kf_prev = sorted[i];
        let kf_next = sorted[i + 1];

        if t >= kf_prev.time && t <= kf_next.time {
            let span = kf_next.time - kf_prev.time;
            let local_t = if span > 0.0 {
                (t - kf_prev.time) / span
            } else {
                0.0
            };
            return (kf_prev, kf_next, local_t);
        }
    }

    // Before first keyframe
    if t < sorted[0].time {
        if extrapolate && sorted.len() >= 2 {
            // Extrapolate backwards using first two keyframes
            let kf_first = sorted[0];
            let kf_second = sorted[1];
            let span = kf_second.time - kf_first.time;
            let local_t = if span > 0.0 {
                (t - kf_first.time) / span // Will be negative
            } else {
                0.0
            };
            return (kf_first, kf_second, local_t);
        }
        return (sorted[0], sorted[0], 0.0);
    }

    // After last keyframe
    let last = sorted.last().unwrap();
    (last, last, 0.0)
}

/// Linear interpolation.
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Parse Vec3 from keyframe value string.
pub fn parse_keyframe_vec3(s: &str) -> Option<[f32; 3]> {
    crate::schema::parse_vec3(s).ok()
}

/// Parse Vec2 from keyframe value string.
pub fn parse_keyframe_vec2(s: &str) -> Option<[f32; 2]> {
    crate::schema::parse_vec2(s).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_keyframe(t: f32, v: &str, e: Option<&str>) -> AmKeyframe {
        AmKeyframe {
            time: t,
            value: v.to_string(),
            easing: e.map(String::from),
        }
    }

    #[test]
    fn test_interpolate_float_static() {
        let prop = AmAnimatedFloat {
            value: Some(0.5),
            keyframes: vec![],
        };
        assert_eq!(interpolate_float(&prop, 0.0), Some(0.5));
        assert_eq!(interpolate_float(&prop, 0.5), Some(0.5));
        assert_eq!(interpolate_float(&prop, 1.0), Some(0.5));
    }

    #[test]
    fn test_interpolate_float_linear() {
        let prop = AmAnimatedFloat {
            value: None,
            keyframes: vec![
                make_keyframe(0.0, "0.0", None),
                make_keyframe(1.0, "1.0", None),
            ],
        };

        let v = interpolate_float(&prop, 0.0).unwrap();
        assert!((v - 0.0).abs() < 0.001);

        let v = interpolate_float(&prop, 0.5).unwrap();
        assert!((v - 0.5).abs() < 0.001);

        let v = interpolate_float(&prop, 1.0).unwrap();
        assert!((v - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_interpolate_float_step() {
        // Easing is on the target keyframe (describes how to arrive at it)
        let prop = AmAnimatedFloat {
            value: None,
            keyframes: vec![
                make_keyframe(0.0, "1.0", None),
                make_keyframe(1.0, "0.0", Some("step 1.0 0.0")),
            ],
        };

        let v = interpolate_float(&prop, 0.0).unwrap();
        assert!((v - 1.0).abs() < 0.001, "At t=0.0, expected 1.0, got {}", v);

        let v = interpolate_float(&prop, 0.5).unwrap();
        assert!(
            (v - 1.0).abs() < 0.001,
            "At t=0.5, expected 1.0 (step), got {}",
            v
        );

        let v = interpolate_float(&prop, 0.99).unwrap();
        assert!(
            (v - 1.0).abs() < 0.001,
            "At t=0.99, expected 1.0 (step), got {}",
            v
        );
    }

    #[test]
    fn test_interpolate_vec3_linear() {
        let prop = AmAnimatedVec3 {
            value: None,
            keyframes: vec![
                make_keyframe(0.0, "0.0,0.0,0.0", None),
                make_keyframe(1.0, "100.0,200.0,0.0", None),
            ],
        };

        let v = interpolate_vec3(&prop, 0.5).unwrap();
        assert!((v[0] - 50.0).abs() < 0.1);
        assert!((v[1] - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_interpolate_boundary() {
        let prop = AmAnimatedFloat {
            value: None,
            keyframes: vec![
                make_keyframe(0.2, "0.0", None),
                make_keyframe(0.8, "1.0", None),
            ],
        };

        // Before first keyframe
        let v = interpolate_float(&prop, 0.0).unwrap();
        assert!((v - 0.0).abs() < 0.001);

        // After last keyframe
        let v = interpolate_float(&prop, 1.0).unwrap();
        assert!((v - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_interpolate_cubic_bezier() {
        // Easing is on the target keyframe (describes how to arrive at it)
        let prop = AmAnimatedFloat {
            value: None,
            keyframes: vec![
                make_keyframe(0.0, "0.0", None),
                make_keyframe(1.0, "100.0", Some("cubicBezier 0.0 0.0 0.58 1.0")),
            ],
        };

        let v_mid = interpolate_float(&prop, 0.5).unwrap();
        // ease-out should be faster at the start, so at t=0.5, value should be > 50
        assert!(v_mid > 50.0, "Expected > 50.0 for ease-out, got {}", v_mid);
    }
}

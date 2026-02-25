//! # animation module
//!
//! # 动画模块
//!
//! Animation systems for interpolating keyframes in Alight Motion projects.
//! This module provides components, systems, and utilities for playback control,
//! transform animation, opacity animation, SDF shape animation, and layer lifecycle management.
//!
//! 用于在 Alight Motion 项目中插值关键帧的动画系统。
//! 本模块提供用于播放控制、变换动画、不透明度动画、SDF 形状动画和图层生命周期管理的组件、系统和工具。

// Sub-modules
mod components;
mod effects;
mod helpers;
mod interpolation;
mod lifecycle;
mod sdf;
mod sdf_spawn;
mod spawn;
mod systems;
mod visual;

// Re-export components
pub use components::{
    AmAnimated, AmCameraLayer, AmPathRepeat, AmPlayback, AmSdfFillParams, AmSdfParams,
    AmSdfShapeParent, AmSdfStrokeParams, DEBUG_NEGATIVE_HEIGHT_SCALE,
};

// Re-export systems
pub use effects::{
    animate_path_repeat_system, animate_rtt_blur_system, animate_text_progress_system,
    animate_text_spacing_system, animate_unified_effect_system, fix_rtl_line_alignment_system,
    update_unified_mask_system,
};
pub use lifecycle::manage_layer_lifecycle_system;
pub use sdf::{
    animate_sdf_opacity_system, animate_sdf_scale_system, apply_mask_clipping_system,
    update_sdf_mask_system,
};
pub use systems::{
    advance_playback_system, animate_am_camera_system, animate_opacity_system, animate_size_system,
    animate_text_opacity_system, animate_transform_system,
};

// Re-export interpolation functions
pub use interpolation::{
    interpolate_float, interpolate_vec2, interpolate_vec3, interpolate_vec3_with_extrapolation,
};

// Internal re-exports for other modules in this crate

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{AmAnimatedFloat, AmKeyframe};

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
    }
}

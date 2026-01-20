//! # scene.rs
//!
//! # 场景模块
//!
//! Scene building and coordinate transformation.
//! AM scene loading, entity spawning, and layer management.
//!
//! 场景构建和坐标转换。
//! AM 场景加载、实体生成和图层管理。

mod components;
mod helpers;
mod effects;
mod spawn;
mod spawn_visual;
mod collect;
mod collect_types;

// Re-export public types
pub use components::{
    AmEmbedContent, AmEmbedContentMarker, AmProjectBundle, AmProjectRoot, AmPendingLayers,
    AmLayerMarker, AmVisualSpawned, AmLayersContainer, AmEmbedContentsContainer, AmRttCamerasContainer,
    AmLayerSpec, AmBlendingMode, AmMaskEntry, AmMaskInfo, PendingLayer, AmSceneConfig, AmPaletteMapParams,
};

pub use helpers::am_to_bevy_coords;
pub use effects::{WipeEffectParams, StretchSegmentParams, GaussianBlurParams, PaletteMapParams};
pub use spawn::spawn_scene;
pub use collect::collect_pending_layers;

// Internal re-exports for other modules in this crate
pub(crate) use helpers::{
    get_initial_location, get_initial_rotation, get_initial_scale, get_initial_pivot,
    get_initial_opacity, get_shape_size, get_shape_size_animation, get_stroke_width_animation,
    get_base_alpha, pivot_to_anchor_and_offset, truncate_string, get_scale_at_normalized_time,
    calculate_embed_position_compensation, calculate_pivot_compensation,
};
pub(crate) use effects::{
    extract_effect_animations, extract_wipe_effect, extract_stretch_segment_effect,
    extract_gaussian_blur_effect, extract_palette_map_effect,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_am_to_bevy_coords() {
        let config = AmSceneConfig {
            canvas_width: 1280.0,
            canvas_height: 960.0,
            flip_y: true,
            z_spacing: 0.001,
            time_offset: 0,
            speed_multiplier: 1.0,
            nesting_depth: 0,
            lifecycle_offset: 0,
        };

        // Center of AM canvas should be at Bevy origin
        let (x, y) = am_to_bevy_coords(640.0, 480.0, &config);
        assert!((x - 0.0).abs() < 0.01, "Center X should be 0, got {}", x);
        assert!((y - 0.0).abs() < 0.01, "Center Y should be 0, got {}", y);

        // Top-left of AM canvas
        let (x, y) = am_to_bevy_coords(0.0, 0.0, &config);
        assert!(
            (x - (-640.0)).abs() < 0.01,
            "Top-left X should be -640, got {}",
            x
        );
        assert!(
            (y - 480.0).abs() < 0.01,
            "Top-left Y should be 480, got {}",
            y
        );

        // Bottom-right of AM canvas
        let (x, y) = am_to_bevy_coords(1280.0, 960.0, &config);
        assert!(
            (x - 640.0).abs() < 0.01,
            "Bottom-right X should be 640, got {}",
            x
        );
        assert!(
            (y - (-480.0)).abs() < 0.01,
            "Bottom-right Y should be -480, got {}",
            y
        );
    }

    #[test]
    fn test_get_shape_size() {
        let props = vec![crate::schema::AmProperty {
            name: "size".to_string(),
            prop_type: "vec2".to_string(),
            value: "200.0,300.0".to_string(),
            keyframes: vec![],
        }];

        // Size is always doubled (half-extent to full size)
        let (w, h) = get_shape_size(&props, "media");
        assert!((w - 400.0).abs() < 0.01);
        assert!((h - 600.0).abs() < 0.01);

        let (w, h) = get_shape_size(&props, "color");
        assert!((w - 400.0).abs() < 0.01);
        assert!((h - 600.0).abs() < 0.01);
    }
}

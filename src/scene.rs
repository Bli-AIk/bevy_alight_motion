//! # scene.rs
//!
//! # 场景模块
//!
//! Scene building and coordinate transformation.
//! AM scene loading, entity spawning, and layer management.
//!
//! 场景构建和坐标转换。
//! AM 场景加载、实体生成和图层管理。

mod collect;
mod collect_camera;
mod collect_embed;
mod collect_image;
mod collect_shape;
mod collect_types;
mod components;
pub(crate) mod effects;
mod helpers;
mod spawn;
mod spawn_embed;
mod spawn_null;
mod spawn_shape;
mod spawn_visual;

// Re-export public types
pub use components::{
    AmBlendingMode,
    AmElement,
    // 2.2 扩展钩子系统
    AmElementType,
    AmEmbedContent,
    AmEmbedContentMarker,
    AmEmbedContentsContainer,
    AmEntitySpawned,
    AmForceHidden,
    AmLayerMarker,
    // 2.3 标识与查询标准化
    AmLayerName,
    AmLayerSpec,
    AmLayersContainer,
    AmMaskEntry,
    AmMaskInfo,
    AmPaletteMapParams,
    AmPendingLayers,
    AmProjectBundle,
    AmProjectRoot,
    AmRttCamerasContainer,
    AmSceneConfig,
    AmSpawnSettings,
    AmVisualSpawned,
    // 2.1 元素过滤机制
    LayerFilter,
    PendingLayer,
};

pub use collect::collect_pending_layers;
pub use effects::{GaussianBlurParams, PaletteMapParams, StretchSegmentParams, WipeEffectParams};
pub use helpers::am_to_bevy_coords;
pub use spawn::spawn_scene;

// Internal re-exports for other modules in this crate

#[cfg(test)]
mod tests {
    use super::helpers::get_shape_size;
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
            scene_fps: 30.0,
            scene_total_time: 0.0,
            retime: None,
            echo_time_shift_ms: 0.0,
            echo_alpha_config: None,
            render_fps: 30.0,
            repeat_alpha_factor: 1.0,
            repeat_offset: bevy::math::Vec2::ZERO,
            repeat_rotation_deg: 0.0,
            repeat_scale_factor: 1.0,
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

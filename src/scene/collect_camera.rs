//! # collect_camera.rs
//!
//! # 摄像机收集模块
//!
//! Functions for collecting camera layer data.
//! 摄像机图层数据收集函数。

use bevy::prelude::*;

use crate::animation::AmAnimated;
use crate::schema::AmCamera;

use super::components::*;
use super::helpers::*;

/// Collect a camera layer's data for lazy spawning.
pub(crate) fn collect_camera(
    camera: &AmCamera,
    config: &AmSceneConfig,
    z: f32,
) -> Option<PendingLayer> {
    let has_parent = camera.parent != 0;
    let (tx, ty) = get_initial_location(&camera.transform.location, config, has_parent);

    // Extract base Z from first location keyframe (or use default)
    let base_z = camera
        .transform
        .location
        .keyframes
        .first()
        .and_then(|kf| {
            let parts: Vec<&str> = kf.value.split(',').collect();
            parts.get(2).and_then(|s| s.trim().parse::<f32>().ok())
        })
        .or_else(|| camera.transform.location.value.as_ref().map(|v| v[2]))
        .unwrap_or(-1247.0);

    let transform = Transform {
        translation: Vec3::new(tx, ty, z),
        ..Default::default()
    };

    Some(PendingLayer {
        id: camera.id,
        label: camera.label.clone(),
        parent: camera.parent,
        start_time: camera.start_time,
        end_time: camera.end_time,
        transform,
        animated: AmAnimated {
            layer_id: camera.id,
            start_time: camera.start_time,
            end_time: camera.end_time,
            time_offset: config.time_offset,
            lifecycle_offset: config.lifecycle_offset,
            location: camera.transform.location.clone(),
            pivot: camera.transform.pivot.clone(),
            rotation: camera.transform.rotation.clone(),
            scale: camera.transform.scale.clone(),
            opacity: camera.transform.opacity.clone(),
            canvas_width: config.canvas_width,
            canvas_height: config.canvas_height,
            has_parent,
            parent_layer_id: camera.parent,
            speed_multiplier: config.speed_multiplier,
            element_speed: 1.0,
            scene_fps: config.scene_fps,
            retime: config.retime.clone(),
            echo_time_shift_ms: config.echo_time_shift_ms,
            echo_alpha_config: config.echo_alpha_config.clone(),
            repeat_rotation_offset_deg: 0.0,
            repeat_scale_factor: 1.0,
            repeat_position_offset: Vec2::ZERO,
            embed_inner_total_time: None,
            ..Default::default()
        },
        spec: AmLayerSpec::Camera {
            fov: camera.fov.clone(),
            base_z,
        },
        z_index: z,
        children: Vec::new(),
        blending_mode: AmBlendingMode::Normal,
        mask_info: None,
        palette_params: None,
        embed_scene_size: None,
        containing_embed_id: 0,
        from_deeply_nested_scene: config.nesting_depth > 1,
        echo_runtime: None,
        group_fill: None,
        embed_inner_total_time: None,
    })
}

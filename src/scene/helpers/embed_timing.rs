//! Centralizes embed-scene timing derivation so collection and spawn paths
//! share one implementation for nested scene time, lifecycle, retime, and FPS.
//!
//! 集中管理嵌套场景的时间推导，让 collect 与 spawn 共享同一套 nested
//! scene 时间、生命周期、retime 与渲染 FPS 计算逻辑。

use crate::animation::{AmRetimeInfo, RetimeMode};
use crate::scene::components::AmSceneConfig;
use crate::schema::AmEmbedScene;

#[derive(Debug, Clone)]
pub(crate) struct EmbedSceneTimingPlan {
    pub in_time: f32,
    pub effective_speed: f32,
    pub global_start: f32,
    pub nested_time_offset: f32,
    pub nested_lifecycle_offset: f32,
    pub nested_config: AmSceneConfig,
}

pub(crate) fn build_embed_scene_timing_plan(
    embed: &AmEmbedScene,
    config: &AmSceneConfig,
) -> EmbedSceneTimingPlan {
    let in_time = embed.in_time.unwrap_or(0) as f32;
    let effective_speed = config.speed_multiplier * embed.speed;
    let global_start = if config.speed_multiplier > 0.0 {
        config.time_offset + embed.start_time as f32 / config.speed_multiplier
    } else {
        config.time_offset + embed.start_time as f32
    };
    let nested_time_offset = if effective_speed > 0.0 {
        global_start - in_time / effective_speed
    } else {
        global_start
    };
    let nested_lifecycle_offset = global_start - in_time;
    let nested_z_spacing = config.z_spacing / 100.0;

    let retime_mode = RetimeMode::parse(&embed.scene.retime);
    let retime_info = build_retime_info(embed, config, retime_mode, global_start, effective_speed);
    let nested_render_fps = calculate_nested_render_fps(embed, config, effective_speed);

    let nested_config = AmSceneConfig {
        canvas_width: embed.scene.width as f32,
        canvas_height: embed.scene.height as f32,
        time_offset: nested_time_offset,
        lifecycle_offset: nested_lifecycle_offset as i32,
        z_spacing: nested_z_spacing,
        nesting_depth: config.nesting_depth + 1,
        speed_multiplier: effective_speed,
        scene_fps: embed.scene.fps as f32,
        scene_total_time: embed.scene.total_time as f32,
        retime: retime_info,
        render_fps: nested_render_fps,
        repeat_offset: bevy::math::Vec2::ZERO,
        repeat_rotation_deg: 0.0,
        repeat_scale_factor: 1.0,
        comparison_frame_center_bias_ms: config.comparison_frame_center_bias_ms,
        ..config.clone()
    };

    EmbedSceneTimingPlan {
        in_time,
        effective_speed,
        global_start,
        nested_time_offset,
        nested_lifecycle_offset,
        nested_config,
    }
}

fn build_retime_info(
    embed: &AmEmbedScene,
    config: &AmSceneConfig,
    retime_mode: RetimeMode,
    global_start: f32,
    effective_speed: f32,
) -> Option<AmRetimeInfo> {
    if retime_mode == RetimeMode::Off {
        return config.retime.clone();
    }

    let container_duration = (embed.end_time - embed.start_time) as f32;
    let nested_total = embed.scene.total_time as f32;
    Some(AmRetimeInfo {
        mode: retime_mode,
        embed_global_start: global_start,
        container_duration_ms: container_duration,
        nested_total_time_ms: nested_total,
        embed_speed: effective_speed,
        comparison_frame_center_bias_ms: config.comparison_frame_center_bias_ms,
    })
}

fn calculate_nested_render_fps(
    embed: &AmEmbedScene,
    config: &AmSceneConfig,
    effective_speed: f32,
) -> f32 {
    let element_duration = (embed.end_time - embed.start_time) as f64;
    let inner_total_time = embed.scene.total_time as f64;
    let duration_factor = if inner_total_time > 0.0 {
        (element_duration / inner_total_time).max(1.0).ceil() as u32
    } else {
        1
    };
    let speed_factor = if effective_speed < 0.99999 {
        (1.0 / effective_speed.max(1e-6)).round().max(1.0) as u32
    } else {
        1
    };
    let parent_fphs = (config.render_fps * 100.0) as u32;
    let nested_fphs = (parent_fphs * duration_factor * speed_factor * 16).min(192000);
    nested_fphs as f32 / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::components::AmSceneConfig;
    use crate::schema::{AmEmbedScene, AmScene, AmTransform};

    fn make_scene() -> Box<AmScene> {
        Box::new(AmScene {
            title: "nested".into(),
            width: 640,
            height: 360,
            export_width: 640,
            export_height: 360,
            fps: 30,
            total_time: 600,
            bgcolor: "#ff000000".into(),
            amver: 0,
            retime: String::new(),
            precompose: String::new(),
            media: vec![],
            layers: vec![],
        })
    }

    fn make_embed() -> AmEmbedScene {
        AmEmbedScene {
            id: 7,
            label: "embed".into(),
            start_time: 900,
            end_time: 2100,
            parent: 0,
            hidden: false,
            in_time: Some(300),
            out_time: None,
            speed: 0.5,
            transform: AmTransform::default(),
            fill_type: "intrinsic".into(),
            fill_color: None,
            effects: vec![],
            gradient: None,
            blending: "normal".into(),
            scene: make_scene(),
        }
    }

    #[test]
    fn test_embed_scene_timing_plan_accounts_for_parent_speed_and_in_time() {
        let embed = make_embed();
        let config = AmSceneConfig {
            time_offset: 120.0,
            speed_multiplier: 2.0,
            render_fps: 30.0,
            ..Default::default()
        };

        let plan = build_embed_scene_timing_plan(&embed, &config);

        assert!((plan.global_start - 570.0).abs() < f32::EPSILON);
        assert!((plan.nested_time_offset - 270.0).abs() < f32::EPSILON);
        assert!((plan.nested_lifecycle_offset - 270.0).abs() < f32::EPSILON);
        assert!((plan.nested_config.speed_multiplier - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_embed_scene_timing_plan_resets_repeat_transform_for_children() {
        let embed = make_embed();
        let config = AmSceneConfig {
            repeat_offset: bevy::math::Vec2::new(10.0, 20.0),
            repeat_rotation_deg: 45.0,
            repeat_scale_factor: 1.5,
            ..Default::default()
        };

        let plan = build_embed_scene_timing_plan(&embed, &config);

        assert_eq!(plan.nested_config.repeat_offset, bevy::math::Vec2::ZERO);
        assert_eq!(plan.nested_config.repeat_rotation_deg, 0.0);
        assert_eq!(plan.nested_config.repeat_scale_factor, 1.0);
    }
}

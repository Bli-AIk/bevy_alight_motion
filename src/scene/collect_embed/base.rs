//! Builds the shared base state for collected embed scenes.
//! It computes the outer transform, retime information, and nested child layers
//! before specialized embed features such as group fill or RTT strategy are added.
//!
//! 负责构建嵌套场景收集阶段的公共基础状态。它会先计算外层变换、retime
//! 信息和嵌套子图层，然后再让 group fill、RTT 策略等更具体的嵌套场景特性叠加上去。

use bevy::prelude::*;
use std::collections::HashMap;

use crate::loader::FontMetrics;

use super::super::collect::collect_pending_layers;
use super::super::components::*;
use super::super::effects::*;
use super::super::helpers::*;

pub(super) struct EmbedCollectedBase {
    pub(super) has_parent: bool,
    pub(super) transform: Transform,
    pub(super) children: Vec<PendingLayer>,
}

pub(super) fn collect_embed_base(
    embed: &crate::schema::AmEmbedScene,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    config: &AmSceneConfig,
    z: f32,
) -> EmbedCollectedBase {
    let has_parent = embed.parent != 0;
    let (mut tx, mut ty) = get_initial_location(&embed.transform.location, config, has_parent);
    let mut rotation = get_initial_rotation(&embed.transform.rotation);
    let (mut sx, mut sy) = get_initial_scale(&embed.transform.scale);
    let pivot = get_initial_pivot(&embed.transform.pivot);

    rotation += -config.repeat_rotation_deg;
    sx *= config.repeat_scale_factor;
    sy *= config.repeat_scale_factor;

    let (comp_x, comp_y) =
        calculate_embed_position_compensation(pivot, (sx, sy), rotation, has_parent);
    tx += comp_x;
    ty += comp_y;

    tx += config.repeat_offset.x;
    ty += config.repeat_offset.y;

    let transform = Transform {
        translation: Vec3::new(tx, ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    let timing = build_embed_scene_timing_plan(embed, config);
    if let Some(retime) = &timing.nested_config.retime {
        bevy::log::debug!(
            "  [Retime] embed '{}': mode={:?}, container={}, total={}, speed={}",
            embed.label,
            retime.mode,
            retime.container_duration_ms,
            retime.nested_total_time_ms,
            retime.embed_speed,
        );
    }

    bevy::log::trace!(
        "  [TimeOffset] embed '{}': parent_offset={}, start_time={}, global_start={}, in_time={}, speed={}, nested_offset={}, lifecycle_offset={}, nested_speed={}",
        embed.label,
        config.time_offset,
        embed.start_time,
        timing.global_start,
        timing.in_time,
        timing.effective_speed,
        timing.nested_time_offset,
        timing.nested_lifecycle_offset,
        timing.nested_config.speed_multiplier
    );

    let mut children =
        collect_pending_layers(&embed.scene, fonts, font_metrics, &timing.nested_config);

    let inner_total = embed.scene.total_time as f32;
    for child in &mut children {
        child.embed_inner_total_time = Some(inner_total);
        child.animated.embed_inner_total_time = Some(inner_total);
    }

    let embed_replace = extract_replace_color_effect(&embed.effects);
    if embed.fill_type != "intrinsic" && embed_replace.old_color != Vec4::ZERO {
        for child in &mut children {
            if child.animated.replace_old_color == Vec4::ZERO {
                child.animated.replace_old_color = embed_replace.old_color;
                child.animated.replace_new_color = embed_replace.new_color.clone();
                child.animated.replace_threshold = embed_replace.threshold.clone();
                child.animated.replace_feather = embed_replace.feather.clone();
                child.animated.replace_alpha = embed_replace.alpha.clone();
                child.animated.replace_lock_luminance = embed_replace.lock_luminance;
            }
        }
    }

    let embed_pixelate = extract_pixelate_effect(&embed.effects);
    if let Some(embed_pix_size) = embed_pixelate.size.value
        && embed_pix_size > 1.0
    {
        for child in &mut children {
            if child.animated.pixelate_size.value.is_some() {
                continue;
            }
            if child.animated.pixelate_size.keyframes.is_empty() {
                child.animated.pixelate_size = embed_pixelate.size.clone();
                child.animated.pixelate_stretch = embed_pixelate.stretch.clone();
                child.animated.pixelate_angle = embed_pixelate.angle.clone();
                child.animated.pixelate_vignette = embed_pixelate.vignette.clone();
                child.animated.pixelate_threshold = embed_pixelate.threshold.clone();
                child.animated.pixelate_saturation = embed_pixelate.saturation.clone();
                child.animated.pixelate_screen_space = embed_pixelate.screen_space;
            }
        }
    }

    EmbedCollectedBase {
        has_parent,
        transform,
        children,
    }
}

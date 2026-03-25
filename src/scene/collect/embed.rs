//! Collects embed-scene layers into pending runtime structures.
//! It expands nested scenes, applies repeat-copy semantics, extends child
//! lifecycles when needed, and packages the result so embed scenes behave like
//! first-class layers in the outer timeline.
//!
//! 负责把嵌套场景图层收集成待生成的运行时结构。它会展开子场景、应用
//! repeat copy 语义、在需要时延长子节点生命周期，并把结果打包成外层时间轴中的
//! 一级图层表现。

use bevy::prelude::*;
use std::collections::HashMap;

use crate::loader::FontMetrics;
use crate::schema::AmEmbedScene;

use super::super::collect_echo::{apply_echo_copy_transform, remap_echo_pl_ids};
use super::super::collect_embed::collect_embed_scene;
use super::super::components::{AmSceneConfig, PendingLayer};
use super::super::effects::{self, RepeatParams};

fn extend_children_lifecycle(pl: &mut PendingLayer, extension_ms: f32) {
    let ext = extension_ms as i32;
    for child in &mut pl.children {
        child.end_time += ext;
        extend_children_lifecycle(child, extension_ms);
    }
}

fn collect_repeat_copies(
    pending: &mut Vec<PendingLayer>,
    embed: &AmEmbedScene,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    config: &AmSceneConfig,
    z: f32,
    repeat: &RepeatParams,
    count: usize,
) {
    let time_val = repeat.time.value.unwrap_or(0.0);
    let offset_val = repeat.offset.value.unwrap_or([0.0, 0.0]);
    let angle_val = repeat.angle.value.unwrap_or(0.0);
    let scale_val = repeat.scale.value.unwrap_or(1.0);
    let alpha_val = repeat.alpha.value.unwrap_or(1.0);

    let frame_duration_ms = 1000.0 / config.render_fps;
    let is_mask_embed = embed.blending == "mask" || embed.blending == "exclude";

    let mut acc_offset = Vec2::ZERO;
    let mut acc_angle: f32 = 0.0;
    let mut acc_scale: f32 = 1.0;
    let mut acc_alpha: f32 = 1.0;
    let mut acc_time: f32 = 0.0;

    let mut base_pl: Option<PendingLayer> = None;

    for i in 0..count {
        if acc_alpha <= 0.0 && i > 0 {
            break;
        }

        let mut copy_config = config.clone();

        let rounded_frames = acc_time.round();
        let frac = acc_time - acc_time.trunc();
        let sub_frame_ms = (frac * config.scene_fps).round();
        let time_shift_ms = rounded_frames * frame_duration_ms + sub_frame_ms;
        copy_config.echo_time_shift_ms += time_shift_ms;
        copy_config.repeat_alpha_factor *= acc_alpha;
        copy_config.repeat_offset = acc_offset;
        copy_config.repeat_rotation_deg = acc_angle;
        copy_config.repeat_scale_factor = acc_scale;

        let copy_z = z + i as f32 * config.z_spacing * 0.001;

        let mut pl = collect_embed_scene(embed, fonts, font_metrics, &copy_config, copy_z);

        if time_shift_ms > 0.0 {
            extend_children_lifecycle(&mut pl, time_shift_ms);
        }

        if is_mask_embed {
            if i == 0 {
                base_pl = Some(pl);
            } else {
                remap_echo_pl_ids(&mut pl);
                apply_echo_copy_transform(&mut pl, acc_scale, acc_offset, acc_angle);
                let base = base_pl.as_mut().expect("copy 0 must set base_pl");
                base.children.extend(pl.children);
            }
        } else {
            if i > 0 {
                remap_echo_pl_ids(&mut pl);
            }
            pending.push(pl);
        }

        acc_offset += Vec2::new(offset_val[0], -offset_val[1]);
        acc_angle += angle_val;
        acc_scale *= scale_val;
        acc_alpha -= 1.0 - alpha_val;
        acc_time += time_val;
    }

    if let Some(pl) = base_pl {
        pending.push(pl);
    }
}

pub(super) fn collect_embed_layer(
    pending: &mut Vec<PendingLayer>,
    embed: &AmEmbedScene,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    config: &AmSceneConfig,
    z: f32,
) {
    let echokf = effects::extract_echokf_effect(&embed.effects);
    let max_count = echokf.max_count();

    if !echokf.enabled || max_count == 0 {
        let repeat = effects::extract_repeat_effect(&embed.effects);
        let repeat_count = repeat.count.value.unwrap_or(0.0) as i32;

        if repeat_count > 1 {
            collect_repeat_copies(
                pending,
                embed,
                fonts,
                font_metrics,
                config,
                z,
                &repeat,
                repeat_count as usize,
            );
            return;
        }

        let pl = collect_embed_scene(embed, fonts, font_metrics, config, z);
        bevy::log::trace!(
            "  Collected embed '{}' (id={}, time={}..{}ms, inTime={:?}, outTime={:?}, children={})",
            embed.label,
            embed.id,
            embed.start_time,
            embed.end_time,
            embed.in_time,
            embed.out_time,
            pl.children.len()
        );
        pending.push(pl);
        return;
    }

    let seconds = echokf.static_seconds();
    let is_dynamic = echokf.is_dynamic() || !echokf.alpha.keyframes.is_empty();

    let base_echo_alpha = crate::animation::EchoAlphaConfig {
        alpha_keyframes: echokf.alpha.clone(),
        fraction: 0.0,
        parent_start: embed.start_time,
        parent_end: embed.end_time,
        parent_time_offset: config.time_offset,
        parent_speed: config.speed_multiplier,
    };

    let echo_rt_template = if is_dynamic {
        Some(crate::animation::AmEchoRuntime {
            echo_index: 0,
            max_count,
            mode: echokf.mode,
            count_kf: echokf.count.clone(),
            seconds_kf: echokf.seconds.clone(),
            alpha_kf: echokf.alpha.clone(),
            embed_start: embed.start_time as f32,
            embed_end: embed.end_time as f32,
            embed_time_offset: config.time_offset,
            embed_speed: config.speed_multiplier,
        })
    } else {
        None
    };

    if echokf.mode == 0 {
        let pl = collect_embed_scene(embed, fonts, font_metrics, config, z);
        pending.push(pl);

        for i in 0..max_count {
            let echo_index = (max_count - 1 - i) as f32;
            let fraction = echo_index / max_count as f32;
            let time_shift_ms = (1.0 - fraction) * seconds * 1000.0;
            let echo_z = z + (i as f32 + 1.0) * config.z_spacing * 0.001;

            let mut echo_config = config.clone();
            echo_config.echo_time_shift_ms += time_shift_ms;
            echo_config.echo_alpha_config = Some(crate::animation::EchoAlphaConfig {
                fraction,
                ..base_echo_alpha.clone()
            });

            let mut echo_pl = collect_embed_scene(embed, fonts, font_metrics, &echo_config, echo_z);
            remap_echo_pl_ids(&mut echo_pl);
            if let Some(ref template) = echo_rt_template {
                echo_pl.echo_runtime = Some(crate::animation::AmEchoRuntime {
                    echo_index: echo_index as u32,
                    ..template.clone()
                });
            }
            pending.push(echo_pl);
        }
    } else {
        for i in 0..max_count {
            let echo_index = i as f32;
            let fraction = echo_index / max_count as f32;
            let time_shift_ms = (1.0 - fraction) * seconds * 1000.0;
            let echo_z = z - (max_count - i) as f32 * config.z_spacing * 0.001;

            let mut echo_config = config.clone();
            echo_config.echo_time_shift_ms += time_shift_ms;
            echo_config.echo_alpha_config = Some(crate::animation::EchoAlphaConfig {
                fraction,
                ..base_echo_alpha.clone()
            });

            let mut echo_pl = collect_embed_scene(embed, fonts, font_metrics, &echo_config, echo_z);
            remap_echo_pl_ids(&mut echo_pl);
            if let Some(ref template) = echo_rt_template {
                echo_pl.echo_runtime = Some(crate::animation::AmEchoRuntime {
                    echo_index: echo_index as u32,
                    ..template.clone()
                });
            }
            pending.push(echo_pl);
        }

        let pl = collect_embed_scene(embed, fonts, font_metrics, config, z);
        pending.push(pl);
    }
}

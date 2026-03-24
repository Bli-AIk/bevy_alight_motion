//! This file defines the main per-entity animation payload used at runtime.
//! `AmAnimated` is the dense component that carries timeline data, effect
//! parameters, repeat settings, and various precomputed fields needed by the
//! transform, opacity, text, and unified-effect systems every frame.
//!
//! 这个文件定义了运行时最核心的逐实体动画载荷。`AmAnimated` 是那个高密度组件，
//! 它承载时间轴数据、效果参数、重复设置，以及变换、透明度、文本和统一特效系统
//! 每帧都会读取的各种预计算字段。

use bevy::prelude::*;

use crate::scene::AmBlendingMode;
use crate::scene::effects::PathRepeatParams;
use crate::schema::{AmAnimatedFloat, AmAnimatedVec2, AmAnimatedVec3};

use super::runtime::{AmRetimeInfo, EchoAlphaConfig, RetimeMode};

/// DEBUG: 拉伸效果乘数，用于调试编组内图片的拉伸计算
pub const DEBUG_NEGATIVE_HEIGHT_SCALE: f32 = 1.05;

/// Component marking an entity as part of an AM animation.
#[derive(Component, Debug, Clone, Default)]
pub struct AmAnimated {
    pub layer_id: u64,
    pub start_time: i32,
    pub end_time: i32,
    pub time_offset: f32,
    pub lifecycle_offset: i32,
    pub location: AmAnimatedVec3,
    pub pivot: AmAnimatedVec2,
    pub rotation: AmAnimatedFloat,
    pub scale: AmAnimatedVec2,
    pub opacity: AmAnimatedFloat,
    pub canvas_width: f32,
    pub canvas_height: f32,
    pub has_parent: bool,
    pub parent_layer_id: u64,
    pub effect_pos_x: AmAnimatedFloat,
    pub effect_pos_y: AmAnimatedFloat,
    pub effect_posz: AmAnimatedFloat,
    pub effect_angle: AmAnimatedFloat,
    pub effect_xinv: bool,
    pub effect_yinv: bool,
    pub effect_zinv: bool,
    pub effect_ainv: bool,
    pub extra_transform2: Vec<crate::scene::effects::Transform2Params>,
    pub font_y_offset: f32,
    pub size: AmAnimatedVec2,
    pub anchor_offset: Vec2,
    pub wipe_start: AmAnimatedFloat,
    pub wipe_end: AmAnimatedFloat,
    pub wipe_angle: AmAnimatedFloat,
    pub wipe_feather: AmAnimatedFloat,
    pub stretch_angle: AmAnimatedFloat,
    pub stretch_amount: AmAnimatedFloat,
    pub stretch_offset: AmAnimatedFloat,
    pub stretch_smooth: AmAnimatedFloat,
    pub stretch_seg2_angle: AmAnimatedFloat,
    pub stretch_seg2_amount: AmAnimatedFloat,
    pub stretch_seg2_offset: AmAnimatedFloat,
    pub stretch_seg2_smooth: AmAnimatedFloat,
    pub blur_strength: AmAnimatedFloat,
    pub speed_multiplier: f32,
    pub element_speed: f32,
    pub scene_fps: f32,
    pub embed_offset: Vec2,
    pub inv_fit_scale: f32,
    pub stroke_width: AmAnimatedFloat,
    pub base_alpha: f32,
    pub fade_in_time: AmAnimatedFloat,
    pub fade_out_time: AmAnimatedFloat,
    pub fade_layer_duration_ms: f32,
    pub palette_alpha: AmAnimatedFloat,
    pub scale_assist: AmAnimatedFloat,
    pub scale_assist_damp: AmAnimatedFloat,
    pub scale_assist_axis: i32,
    pub parenthelper_scale_mode: i32,
    pub parenthelper_rotate_mode: i32,
    pub parenthelper_scale_weight: AmAnimatedFloat,
    pub parenthelper_rotate_weight: AmAnimatedFloat,
    pub parenthelper_auto_rotate: i32,
    pub parenthelper_radius_adjust: AmAnimatedFloat,
    pub parenthelper_has_effect: bool,
    pub stretch2_scale: AmAnimatedFloat,
    pub stretch2_angle: AmAnimatedFloat,
    pub stretch2_content_only: bool,
    pub wavewarp2_phase: AmAnimatedFloat,
    pub wavewarp2_a1d: AmAnimatedFloat,
    pub wavewarp2_m1: AmAnimatedFloat,
    pub wavewarp2_m2: AmAnimatedFloat,
    pub wavewarp2_a2d: AmAnimatedFloat,
    pub wavewarp2_damping: AmAnimatedFloat,
    pub wavewarp2_damping_space: AmAnimatedFloat,
    pub wavewarp2_damping_origin: AmAnimatedFloat,
    pub wavewarp2_screen_space: bool,
    pub wavewarp2_has_effect: bool,
    pub mirror_type: i32,
    pub mirror_blend_mode: i32,
    pub mirror_alpha: AmAnimatedFloat,
    pub mirror_offset: AmAnimatedFloat,
    pub mirror_has_effect: bool,
    pub lift_fill: AmAnimatedFloat,
    pub lift_has_effect: bool,
    pub rays_center_x: AmAnimatedFloat,
    pub rays_center_y: AmAnimatedFloat,
    pub rays_strength: AmAnimatedFloat,
    pub rays_intensity: AmAnimatedFloat,
    pub rays_threshold: AmAnimatedFloat,
    pub rays_threshold_color: Vec4,
    pub rays_fill_color: Vec4,
    pub rays_blend: AmAnimatedFloat,
    pub rays_quality: AmAnimatedFloat,
    pub rays_has_effect: bool,
    pub replace_old_color: Vec4,
    pub replace_new_color: crate::schema::AmAnimatedColor,
    pub replace_threshold: AmAnimatedFloat,
    pub replace_feather: AmAnimatedFloat,
    pub replace_alpha: AmAnimatedFloat,
    pub replace_lock_luminance: bool,
    pub repeat_count: AmAnimatedFloat,
    pub repeat_offset: AmAnimatedVec2,
    pub repeat_angle: AmAnimatedFloat,
    pub repeat_scale: AmAnimatedFloat,
    pub repeat_alpha: AmAnimatedFloat,
    pub linear_repeat_count: AmAnimatedFloat,
    pub linear_repeat_position: AmAnimatedVec2,
    pub linear_repeat_offset: AmAnimatedVec2,
    pub linear_repeat_angle: AmAnimatedFloat,
    pub linear_repeat_scale: AmAnimatedFloat,
    pub linear_repeat_alpha: AmAnimatedFloat,
    pub linear_repeat_fill_color: crate::schema::AmAnimatedColor,
    pub linear_repeat_blend: AmAnimatedFloat,
    pub linear_repeat_color_alt_copies: bool,
    pub linear_repeat_start: AmAnimatedFloat,
    pub linear_repeat_end: AmAnimatedFloat,
    pub linear_repeat_phase: AmAnimatedFloat,
    pub linear_repeat_ease_in: AmAnimatedFloat,
    pub linear_repeat_ease_out: AmAnimatedFloat,
    pub linear_repeat_overlap: AmAnimatedFloat,
    pub linear_repeat_shape: i32,
    pub linear_repeat_invert: bool,
    pub linear_repeat_random_order: bool,
    pub linear_repeat_seed: AmAnimatedFloat,
    pub linear_repeat2: Option<Box<crate::scene::effects::LinearRepeatParams>>,
    pub radial_repeat_count: AmAnimatedFloat,
    pub radial_repeat_radius: AmAnimatedFloat,
    pub radial_repeat_orientation: AmAnimatedFloat,
    pub radial_repeat_start_angle: AmAnimatedFloat,
    pub radial_repeat_sweep: AmAnimatedFloat,
    pub radial_repeat_base_scale: AmAnimatedFloat,
    pub radial_repeat_offset: AmAnimatedVec2,
    pub radial_repeat_angle: AmAnimatedFloat,
    pub radial_repeat_scale: AmAnimatedFloat,
    pub radial_repeat_alpha: AmAnimatedFloat,
    pub radial_repeat_fill_color: crate::schema::AmAnimatedColor,
    pub radial_repeat_blend: AmAnimatedFloat,
    pub radial_repeat_color_alt_copies: bool,
    pub radial_repeat_start: AmAnimatedFloat,
    pub radial_repeat_end: AmAnimatedFloat,
    pub radial_repeat_phase: AmAnimatedFloat,
    pub radial_repeat_ease_in: AmAnimatedFloat,
    pub radial_repeat_ease_out: AmAnimatedFloat,
    pub radial_repeat_overlap: AmAnimatedFloat,
    pub radial_repeat_shape: i32,
    pub radial_repeat_invert: bool,
    pub radial_repeat_random_order: bool,
    pub radial_repeat_seed: f32,
    pub oscillate_direction: i32,
    pub oscillate_angle: AmAnimatedFloat,
    pub oscillate_freq: AmAnimatedFloat,
    pub oscillate_mag: AmAnimatedFloat,
    pub oscillate_wave_type: i32,
    pub oscillate_phase: AmAnimatedFloat,
    pub swing_freq: AmAnimatedFloat,
    pub swing_a1: AmAnimatedFloat,
    pub swing_a2: AmAnimatedFloat,
    pub swing_phase: AmAnimatedFloat,
    pub swing_type: i32,
    pub spin_rpm: AmAnimatedFloat,
    pub threshold_value: AmAnimatedFloat,
    pub threshold_feather: AmAnimatedFloat,
    pub threshold_invert: bool,
    pub threshold_blend_mode: i32,
    pub grid_position: AmAnimatedVec2,
    pub grid_spacing: AmAnimatedFloat,
    pub grid_width: AmAnimatedFloat,
    pub grid_color: crate::schema::AmAnimatedColor,
    pub grid_punchout: bool,
    pub grid_smoothing: AmAnimatedFloat,
    pub grid_screen_space: bool,
    pub pixelate_size: AmAnimatedFloat,
    pub pixelate_stretch: AmAnimatedVec2,
    pub pixelate_angle: AmAnimatedFloat,
    pub pixelate_vignette: AmAnimatedFloat,
    pub pixelate_threshold: AmAnimatedFloat,
    pub pixelate_saturation: AmAnimatedFloat,
    pub pixelate_screen_space: bool,
    pub solid_color: crate::schema::AmAnimatedColor,
    pub solid_color_alpha: AmAnimatedFloat,
    pub solid_color_blend_mode: i32,
    pub base_fill_color: [f32; 4],
    pub fill_color: crate::schema::AmAnimatedColor,
    pub path_repeat: Option<PathRepeatParams>,
    pub textspacing_letter: AmAnimatedFloat,
    pub textspacing_line: AmAnimatedFloat,
    pub textprogress_start: AmAnimatedFloat,
    pub textprogress_end: AmAnimatedFloat,
    pub textprogress_cursor: i32,
    pub textprogress_blink: bool,
    pub counter_offset: AmAnimatedFloat,
    pub counter_scale: AmAnimatedFloat,
    pub shape_props: [AmAnimatedFloat; 4],
    pub shape_points: [AmAnimatedVec2; 5],
    pub jitter_enabled: bool,
    pub jitter_angle: AmAnimatedFloat,
    pub jitter_freq: AmAnimatedFloat,
    pub jitter_mag: AmAnimatedFloat,
    pub jitter_seed: AmAnimatedFloat,
    pub jitter_slack: AmAnimatedFloat,
    pub jitter_zjitter: AmAnimatedFloat,
    pub sd_enabled: bool,
    pub sd_mag: AmAnimatedFloat,
    pub sd_evolution: AmAnimatedFloat,
    pub sd_seed: AmAnimatedFloat,
    pub sd_scatter: AmAnimatedFloat,
    pub rgb_split_enabled: bool,
    pub rgb_split_strength: AmAnimatedFloat,
    pub rgb_split_angle: AmAnimatedFloat,
    pub rgb_split_center: i32,
    pub rgb_split_mode: i32,
    pub exposure_value: AmAnimatedFloat,
    pub exposure_gamma: AmAnimatedFloat,
    pub exposure_offset: AmAnimatedFloat,
    pub exposure_has_effect: bool,
    pub chromakey_enabled: bool,
    pub chromakey_key_color: crate::schema::AmAnimatedColor,
    pub chromakey_threshold: AmAnimatedFloat,
    pub chromakey_feather: AmAnimatedFloat,
    pub chromakey_defringe: bool,
    pub chromakey_invert: bool,
    pub blend_mode: AmBlendingMode,
    pub retime: Option<AmRetimeInfo>,
    pub echo_time_shift_ms: f32,
    pub echo_alpha_config: Option<EchoAlphaConfig>,
    pub repeat_rotation_offset_deg: f32,
    pub repeat_scale_factor: f32,
    pub repeat_position_offset: Vec2,
    pub embed_inner_total_time: Option<f32>,
}

impl AmAnimated {
    fn renderable_nested_total(total_ms: f32, scene_fps: f32) -> f32 {
        if total_ms <= 0.0 {
            return 0.0;
        }
        let total_ms_i = total_ms.floor().max(0.0) as i32;
        let sample_ms = total_ms_i.saturating_sub(1);
        let fphs = (scene_fps * 100.0).round().max(1.0) as i32;
        let frame_number = (sample_ms * fphs) / 100_000;
        (((frame_number * 100_000) + 50_000) / fphs) as f32
    }

    fn apply_retime(&self, global_time: f32) -> Option<f32> {
        let rt = self.retime.as_ref()?;
        if rt.mode == RetimeMode::Off {
            return None;
        }
        let comparison_time = match rt.mode {
            RetimeMode::Stretch | RetimeMode::LoopStretch => {
                global_time + rt.comparison_frame_center_bias_ms
            }
            _ => global_time,
        };
        let embed_elapsed = (comparison_time - rt.embed_global_start) * rt.embed_speed;
        if embed_elapsed < 0.0 {
            return Some(0.0);
        }
        let total = rt.nested_total_time_ms;
        if total <= 0.0 {
            return Some(0.0);
        }
        let renderable_total = Self::renderable_nested_total(total, self.scene_fps)
            .max(0.0)
            .min(total);
        let nested_time = match rt.mode {
            RetimeMode::Off => return None,
            RetimeMode::Stretch => {
                let container = rt.container_duration_ms;
                if container > 0.0 {
                    (embed_elapsed / container) * total
                } else {
                    embed_elapsed
                }
            }
            RetimeMode::Freeze => embed_elapsed.min(renderable_total.max(0.0)),
            RetimeMode::Loop => {
                let loop_total = renderable_total.max(1.0);
                embed_elapsed.rem_euclid(loop_total)
            }
            RetimeMode::LoopStretch => {
                Self::calc_loop_stretch_time(rt.container_duration_ms, total, embed_elapsed)
            }
            RetimeMode::Blank => {
                if embed_elapsed > renderable_total.max(0.0) {
                    return Some(-1.0);
                }
                embed_elapsed
            }
        };
        if let Some(trace_ids) =
            std::env::var_os("AM_RETIME_TRACE_IDS").and_then(|value| value.into_string().ok())
        {
            let should_trace = trace_ids
                .split(',')
                .filter_map(|value| value.trim().parse::<u64>().ok())
                .any(|id| id == self.layer_id);
            if should_trace {
                bevy::log::warn!(
                    "[RetimeTrace] id={} mode={:?} global={:.3} comparison={:.3} bias={:.3} embed_start={:.3} embed_elapsed={:.3} total={:.3} renderable_total={:.3} nested={:.3} scene_fps={:.3}",
                    self.layer_id,
                    rt.mode,
                    global_time,
                    comparison_time,
                    rt.comparison_frame_center_bias_ms,
                    rt.embed_global_start,
                    embed_elapsed,
                    total,
                    renderable_total,
                    nested_time,
                    self.scene_fps,
                );
            }
        }
        Some(nested_time)
    }

    fn calc_loop_stretch_time(container: f32, total: f32, embed_elapsed: f32) -> f32 {
        if container <= 0.0 {
            return embed_elapsed;
        }
        let loops = (container / total).ceil().max(1.0);
        let stride = container / loops;
        if stride > 0.0 {
            ((embed_elapsed % stride) / stride) * total
        } else {
            embed_elapsed
        }
    }

    pub fn calc_local_time(&self, global_time: f32) -> f32 {
        if let Some(nested_time) = self.apply_retime(global_time) {
            return nested_time;
        }
        (global_time - self.time_offset) * self.speed_multiplier
    }

    pub fn calc_lifecycle_time(&self, global_time: f32) -> f32 {
        if let Some(nested_time) = self.apply_retime(global_time) {
            return nested_time;
        }
        global_time - self.lifecycle_offset as f32
    }

    pub fn is_active(&self, local_time: f32) -> bool {
        local_time >= self.start_time as f32
            && local_time <= self.end_time as f32 + self.echo_time_shift_ms
    }

    pub fn calc_layer_time(&self, local_time: f32) -> f32 {
        let effective_time = local_time - self.echo_time_shift_ms;
        let duration = (self.end_time - self.start_time) as f32;
        if duration > 0.0 {
            (effective_time - self.start_time as f32) / duration * self.element_speed
        } else {
            0.0
        }
    }

    pub fn calc_fade_alpha(&self, layer_time: f32) -> f32 {
        use super::super::interpolation::interpolate_float;

        let has_fade = self.fade_in_time.value.is_some()
            || !self.fade_in_time.keyframes.is_empty()
            || self.fade_out_time.value.is_some()
            || !self.fade_out_time.keyframes.is_empty();

        if !has_fade {
            return 1.0;
        }

        let duration_secs = self.fade_layer_duration_ms / 1000.0;
        if duration_secs <= 0.0 {
            return 1.0;
        }

        let in_time_secs = interpolate_float(&self.fade_in_time, layer_time).unwrap_or(0.0);
        let out_time_secs = interpolate_float(&self.fade_out_time, layer_time).unwrap_or(0.0);
        let in_time = in_time_secs / duration_secs;
        let out_time = out_time_secs / duration_secs;

        let mut alpha = 1.0_f32;
        if in_time > 0.0 && layer_time < in_time {
            alpha *= ease_in_out_quad(layer_time / in_time);
        }
        if out_time > 0.0 && layer_time > 1.0 - out_time {
            alpha *= ease_in_out_quad((1.0 - layer_time) / out_time);
        }

        alpha.clamp(0.0, 1.0)
    }
}

fn ease_in_out_quad(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        2.0 * t * t
    } else {
        -1.0 + (4.0 - 2.0 * t) * t
    }
}

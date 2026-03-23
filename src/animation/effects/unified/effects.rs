//! This file updates the effect uniforms used by the unified material path.
//! It converts animated wipe, stretch, wavewarp, mirror, palette, and other
//! effect parameters into GPU-ready uniform fields on `UnifiedEffectMaterial`.
//!
//! 这个文件负责更新统一材质路径使用的特效 uniform。它会把 wipe、stretch、
//! wavewarp、mirror、palette 等动画参数转换成 `UnifiedEffectMaterial`
//! 可直接提交给 GPU 的 uniform 字段。

use bevy::prelude::*;

use crate::animation::components::AmAnimated;
use crate::animation::interpolation::{interpolate_color, interpolate_float, interpolate_vec2};

use super::super::unified_support::srgb_to_linear;

pub(super) fn update_wipe(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    layer_time: f32,
    has_wipe: bool,
) {
    if has_wipe {
        material.set_wipe_enabled(true);
        let wipe_start = interpolate_float(&animated.wipe_start, layer_time).unwrap_or(0.0);
        let wipe_end = interpolate_float(&animated.wipe_end, layer_time).unwrap_or(1.0);
        let wipe_angle = interpolate_float(&animated.wipe_angle, layer_time).unwrap_or(0.0);
        let wipe_feather = interpolate_float(&animated.wipe_feather, layer_time).unwrap_or(0.0);
        material.uniform_data.wipe_params =
            Vec4::new(wipe_start, wipe_end, wipe_angle, wipe_feather);
    } else {
        material.set_wipe_enabled(false);
    }
}

pub(super) fn update_stretch2_uniform(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    has_stretch2: bool,
    s2_scale: f32,
    s2_angle_rad: f32,
) {
    if has_stretch2 {
        let s2_content_only = if animated.stretch2_content_only {
            1.0
        } else {
            0.0
        };
        bevy::log::trace!(
            "[stretch2] layer_id={} scale={:.4} angle_rad={:.4} content_only={}",
            animated.layer_id,
            s2_scale,
            s2_angle_rad,
            animated.stretch2_content_only
        );
        material.uniform_data.stretch2_params =
            Vec4::new(s2_scale, s2_angle_rad, s2_content_only, 0.0);
    } else {
        material.uniform_data.stretch2_params = Vec4::ZERO;
    }
}

pub(super) fn update_wavewarp2(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    layer_time: f32,
    orig_width: f32,
    orig_height: f32,
) {
    if animated.wavewarp2_has_effect {
        let phase = interpolate_float(&animated.wavewarp2_phase, layer_time).unwrap_or(0.0);
        let a1d_rad = interpolate_float(&animated.wavewarp2_a1d, layer_time)
            .unwrap_or(0.0)
            .to_radians();
        let m1 = interpolate_float(&animated.wavewarp2_m1, layer_time).unwrap_or(20.0);
        let m2 = interpolate_float(&animated.wavewarp2_m2, layer_time).unwrap_or(4.0);
        let a2d = interpolate_float(&animated.wavewarp2_a2d, layer_time).unwrap_or(90.0);
        let a2_rad = (a1d_rad.to_degrees() + a2d).to_radians();
        let damping_val = interpolate_float(&animated.wavewarp2_damping, layer_time).unwrap_or(0.0);
        let damping_space =
            interpolate_float(&animated.wavewarp2_damping_space, layer_time).unwrap_or(0.0);
        let damping_origin =
            interpolate_float(&animated.wavewarp2_damping_origin, layer_time).unwrap_or(0.5);

        material.uniform_data.wavewarp2_params1 = Vec4::new(phase, a1d_rad, m1, m2);
        material.uniform_data.wavewarp2_params2 =
            Vec4::new(a2_rad, damping_val, damping_space, damping_origin);
        let mag_x = animated.canvas_width / orig_width.max(1.0);
        let mag_y = animated.canvas_height / orig_height.max(1.0);
        material.uniform_data.wavewarp2_flags = Vec4::new(
            if animated.wavewarp2_screen_space {
                1.0
            } else {
                0.0
            },
            1.0,
            mag_x,
            mag_y,
        );
    } else {
        material.uniform_data.wavewarp2_params1 = Vec4::ZERO;
        material.uniform_data.wavewarp2_params2 = Vec4::ZERO;
        material.uniform_data.wavewarp2_flags = Vec4::ZERO;
    }
}

pub(super) fn update_mirror(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    layer_time: f32,
) {
    if animated.mirror_has_effect {
        let alpha = interpolate_float(&animated.mirror_alpha, layer_time).unwrap_or(1.0);
        let offset = interpolate_float(&animated.mirror_offset, layer_time).unwrap_or(0.0);
        let type_plus_1 = (animated.mirror_type + 1) as f32;
        material.uniform_data.mirror_params = Vec4::new(
            type_plus_1,
            animated.mirror_blend_mode as f32,
            alpha,
            offset,
        );
    } else {
        material.uniform_data.mirror_params = Vec4::ZERO;
    }
}

pub(super) fn update_lift(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    layer_time: f32,
) {
    if animated.lift_has_effect {
        let fill = interpolate_float(&animated.lift_fill, layer_time).unwrap_or(0.0);
        material.uniform_data.lift_params =
            Vec4::new(fill, animated.canvas_width, animated.canvas_height, 1.0);
    } else {
        material.uniform_data.lift_params = Vec4::ZERO;
    }
}

pub(super) fn update_rays(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    embed_marker: Option<&crate::scene::AmEmbedContentMarker>,
    parent_animated_query: &Query<(&AmAnimated, Option<&ChildOf>)>,
    global_time: f32,
) {
    let mut has_rays = animated.rays_has_effect;
    let rays_src = if has_rays {
        animated
    } else if let Some(marker) = embed_marker
        && let Ok((parent_anim, _)) = parent_animated_query.get(marker.embed_entity)
        && parent_anim.rays_has_effect
    {
        has_rays = true;
        parent_anim
    } else {
        animated
    };

    if has_rays {
        let rt = rays_src.calc_layer_time(rays_src.calc_local_time(global_time));
        let strength = interpolate_float(&rays_src.rays_strength, rt).unwrap_or(0.15);
        let intensity = interpolate_float(&rays_src.rays_intensity, rt).unwrap_or(1.0);
        let threshold = interpolate_float(&rays_src.rays_threshold, rt).unwrap_or(0.6);
        let quality = interpolate_float(&rays_src.rays_quality, rt).unwrap_or(150.0);
        let blend = interpolate_float(&rays_src.rays_blend, rt).unwrap_or(0.0);
        let cx = 0.5 + interpolate_float(&rays_src.rays_center_x, rt).unwrap_or(0.0) / 500.0;
        let cy = 0.5 - interpolate_float(&rays_src.rays_center_y, rt).unwrap_or(0.0) / 500.0;
        material.uniform_data.rays_params1 = Vec4::new(strength, intensity, threshold, quality);
        material.uniform_data.rays_params2 = Vec4::new(blend, cx, cy, 1.0);
        material.uniform_data.rays_threshold_color = rays_src.rays_threshold_color;
        material.uniform_data.rays_fill_color = rays_src.rays_fill_color;
    } else {
        material.uniform_data.rays_params1 = Vec4::ZERO;
        material.uniform_data.rays_params2 = Vec4::ZERO;
        material.uniform_data.rays_threshold_color = Vec4::ZERO;
        material.uniform_data.rays_fill_color = Vec4::ZERO;
    }
}

pub(super) fn update_rgb_split(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    layer_time: f32,
) {
    if animated.rgb_split_enabled {
        let strength = interpolate_float(&animated.rgb_split_strength, layer_time).unwrap_or(0.15);
        let angle_deg = interpolate_float(&animated.rgb_split_angle, layer_time).unwrap_or(0.0);
        let angle_rad = angle_deg.to_radians();
        let adj_strength = strength / 8.0;
        let offset_x = angle_rad.cos() * adj_strength;
        let offset_y = angle_rad.sin() * adj_strength;
        material.uniform_data.rgb_split_params = Vec4::new(
            offset_x,
            offset_y,
            animated.rgb_split_center as f32,
            animated.rgb_split_mode as f32,
        );
    } else {
        material.uniform_data.rgb_split_params = Vec4::new(0.0, 0.0, 0.0, -1.0);
    }
}

pub(super) fn update_exposure(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    embed_marker: Option<&crate::scene::AmEmbedContentMarker>,
    parent_animated_query: &Query<(&AmAnimated, Option<&ChildOf>)>,
    global_time: f32,
    layer_time: f32,
) {
    let (mut exp_val, mut gam_val, mut off_val, mut has_exp) = if animated.exposure_has_effect {
        (
            interpolate_float(&animated.exposure_value, layer_time).unwrap_or(0.0),
            interpolate_float(&animated.exposure_gamma, layer_time).unwrap_or(1.0),
            interpolate_float(&animated.exposure_offset, layer_time).unwrap_or(0.0),
            true,
        )
    } else {
        (0.0, 1.0, 0.0, false)
    };

    if let Some(marker) = embed_marker
        && let Ok((parent_anim, _)) = parent_animated_query.get(marker.embed_entity)
        && parent_anim.exposure_has_effect
    {
        let pt = parent_anim.calc_local_time(global_time);
        let plt = parent_anim.calc_layer_time(pt);
        let pe = interpolate_float(&parent_anim.exposure_value, plt).unwrap_or(0.0);
        let pg = interpolate_float(&parent_anim.exposure_gamma, plt).unwrap_or(1.0);
        let po = interpolate_float(&parent_anim.exposure_offset, plt).unwrap_or(0.0);
        exp_val += pe;
        gam_val *= pg;
        off_val += po;
        has_exp = true;
    }

    material.set_exposure_gamma(exp_val, gam_val, off_val, has_exp);
}

pub(super) fn update_chromakey(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    layer_time: f32,
) {
    if animated.chromakey_enabled {
        let key_color = interpolate_color(&animated.chromakey_key_color, layer_time)
            .unwrap_or(Vec4::new(0.0, 1.0, 0.0, 1.0));
        let threshold = interpolate_float(&animated.chromakey_threshold, layer_time).unwrap_or(0.1);
        let feather = interpolate_float(&animated.chromakey_feather, layer_time).unwrap_or(0.05);
        let linear_key = Vec4::new(
            srgb_to_linear(key_color.x),
            srgb_to_linear(key_color.y),
            srgb_to_linear(key_color.z),
            key_color.w,
        );
        material.set_chromakey(
            linear_key,
            threshold,
            feather,
            animated.chromakey_defringe,
            animated.chromakey_invert,
        );
    }
}

pub(super) fn update_blend(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
) {
    if animated.blend_mode.is_blend() {
        material.set_blend_mode(
            animated.blend_mode.as_f32(),
            animated.canvas_width,
            animated.canvas_height,
        );
    }
}

pub(super) fn update_solidcolor(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    layer_time: f32,
) {
    let sc_alpha_val = interpolate_float(&animated.solid_color_alpha, layer_time).unwrap_or(0.0);
    if sc_alpha_val > 0.0 {
        let sc_color = interpolate_color(&animated.solid_color, layer_time).unwrap_or(Vec4::ZERO);
        material.uniform_data.solid_color_params = Vec4::new(
            srgb_to_linear(sc_color.x),
            srgb_to_linear(sc_color.y),
            srgb_to_linear(sc_color.z),
            animated.solid_color_blend_mode as f32,
        );
        material.uniform_data.solid_color_alpha = Vec4::new(sc_alpha_val, 0.0, 0.0, 0.0);
    } else {
        material.uniform_data.solid_color_alpha = Vec4::ZERO;
    }
}

pub(super) fn update_palette(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    layer_time: f32,
    has_palette: bool,
) {
    if has_palette && material.is_palette_enabled() {
        let palette_alpha = interpolate_float(&animated.palette_alpha, layer_time).unwrap_or(1.0);
        material.set_palette_alpha(palette_alpha);
    }
}

pub(super) fn update_replace_color(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    layer_time: f32,
    has_replace_color: bool,
) {
    bevy::log::debug!(
        "[ReplaceColor Check] layer={} has_replace={} old_color={:?}",
        animated.layer_id,
        has_replace_color,
        animated.replace_old_color
    );
    if has_replace_color {
        let new_color = interpolate_color(&animated.replace_new_color, layer_time)
            .unwrap_or(animated.replace_old_color);
        let threshold = interpolate_float(&animated.replace_threshold, layer_time).unwrap_or(0.25);
        let feather = interpolate_float(&animated.replace_feather, layer_time).unwrap_or(0.25);
        let alpha = interpolate_float(&animated.replace_alpha, layer_time).unwrap_or(1.0);

        bevy::log::debug!(
            "[ReplaceColor Apply] layer={} old={:?} new={:?} threshold={:.3} feather={:.3} alpha={:.3}",
            animated.layer_id,
            animated.replace_old_color,
            new_color,
            threshold,
            feather,
            alpha
        );

        material.set_replace_color(
            animated.replace_old_color,
            new_color,
            threshold,
            feather,
            alpha,
            animated.replace_lock_luminance,
        );
    }
}

pub(super) fn update_threshold(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    layer_time: f32,
) {
    let has_threshold =
        animated.threshold_value.value.is_some() || !animated.threshold_value.keyframes.is_empty();
    if has_threshold {
        let threshold = interpolate_float(&animated.threshold_value, layer_time).unwrap_or(0.5);
        let feather = interpolate_float(&animated.threshold_feather, layer_time).unwrap_or(0.0);
        material.set_threshold(
            true,
            threshold,
            feather,
            animated.threshold_invert,
            animated.threshold_blend_mode,
        );
    }
}

pub(super) fn update_grid(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    layer_time: f32,
) {
    let has_grid =
        animated.grid_spacing.value.is_some() || !animated.grid_spacing.keyframes.is_empty();
    if has_grid {
        let position = interpolate_vec2(&animated.grid_position, layer_time).unwrap_or([0.0, 0.0]);
        let spacing = interpolate_float(&animated.grid_spacing, layer_time).unwrap_or(0.1);
        let width = interpolate_float(&animated.grid_width, layer_time).unwrap_or(0.02);
        let smoothing = interpolate_float(&animated.grid_smoothing, layer_time).unwrap_or(0.0);
        let color = interpolate_color(&animated.grid_color, layer_time)
            .unwrap_or(Vec4::new(1.0, 1.0, 1.0, 1.0));

        material.set_grid(
            true,
            animated.grid_punchout,
            animated.grid_screen_space,
            position[0],
            position[1],
            spacing,
            width,
            smoothing,
            color,
        );
    }
}

pub(super) fn update_pixelate(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    layer_time: f32,
    global_transform: &GlobalTransform,
    root_scale: f32,
    has_pixelate: bool,
) {
    if has_pixelate {
        let size = interpolate_float(&animated.pixelate_size, layer_time).unwrap_or(1.0);
        let stretch =
            interpolate_vec2(&animated.pixelate_stretch, layer_time).unwrap_or([1.0, 1.0]);
        let angle = interpolate_float(&animated.pixelate_angle, layer_time).unwrap_or(0.0);
        let vignette = interpolate_float(&animated.pixelate_vignette, layer_time).unwrap_or(0.0);
        let threshold = interpolate_float(&animated.pixelate_threshold, layer_time).unwrap_or(0.5);
        let saturation =
            interpolate_float(&animated.pixelate_saturation, layer_time).unwrap_or(1.0);

        bevy::log::debug!(
            "[Pixelate] layer={} time={:.2} size={:.1} stretch=({:.2},{:.2}) angle={:.1}",
            animated.layer_id,
            layer_time,
            size,
            stretch[0],
            stretch[1],
            angle
        );

        material.set_pixelate(
            true,
            animated.pixelate_screen_space,
            size,
            stretch[0],
            stretch[1],
            angle,
            vignette,
            threshold,
            saturation,
        );

        let origin = global_transform.translation();
        let local_x_world = global_transform.transform_point(Vec3::X) - origin;
        let local_y_world = global_transform.transform_point(Vec3::Y) - origin;
        let scene_scale_x = local_x_world.length() / root_scale;
        let scene_scale_y = local_y_world.length() / root_scale;
        material.uniform_data.pixelate_flags.z = scene_scale_x;
        material.uniform_data.pixelate_flags.w = scene_scale_y;

        let local_x_world = global_transform.transform_point(Vec3::X) - origin;
        let scene_rotation = local_x_world.y.atan2(local_x_world.x);
        material.uniform_data.pixelate_params2.w = scene_rotation;
    }
}

use std::collections::HashMap;

use bevy::asset::Assets;
use bevy::prelude::*;

use crate::scene::PendingLayer;
use crate::sdf_material::SdfMaterial;

use super::super::helpers::get_initial_scale_from_animated;
use super::super::interpolation::{interpolate_float, interpolate_vec2, parse_keyframe_color};
use super::super::visual::add_visual_components;

#[expect(clippy::too_many_arguments)] // reason: visual spawn wires a large cross-section of runtime state
pub(super) fn spawn_visuals_for_layer(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    unified_materials: &mut Assets<crate::masked_sprite::UnifiedEffectMaterial>,
    color_materials: &mut Assets<ColorMaterial>,
    sdf_materials: &mut Assets<SdfMaterial>,
    layer: &PendingLayer,
    entity: Entity,
    images: &HashMap<String, Handle<Image>>,
    fonts: &HashMap<String, Handle<Font>>,
    white_pixel: Option<&Handle<Image>>,
    inv_fit_scale: f32,
    has_child_layers: bool,
    layer_time: f32,
    initial_rotation: Quat,
    global_time: f32,
) {
    let initial_scale = get_initial_scale_from_animated(&layer.animated.scale);

    let has_wipe = layer.animated.wipe_end.value != Some(1.0)
        || !layer.animated.wipe_end.keyframes.is_empty()
        || layer.animated.wipe_start.value.is_some()
        || !layer.animated.wipe_start.keyframes.is_empty();

    let has_stretch = layer.animated.stretch_amount.value.is_some()
        || !layer.animated.stretch_amount.keyframes.is_empty()
        || layer.animated.stretch_angle.value.is_some()
        || !layer.animated.stretch_angle.keyframes.is_empty()
        || layer.animated.stretch_offset.value.is_some()
        || !layer.animated.stretch_offset.keyframes.is_empty()
        || layer.animated.stretch_smooth.value.is_some()
        || !layer.animated.stretch_smooth.keyframes.is_empty()
        || layer.animated.stretch_seg2_amount.value.is_some()
        || !layer.animated.stretch_seg2_amount.keyframes.is_empty()
        || layer.animated.stretch_seg2_angle.value.is_some()
        || !layer.animated.stretch_seg2_angle.keyframes.is_empty();

    let has_blur = layer.animated.blur_strength.value.is_some()
        || !layer.animated.blur_strength.keyframes.is_empty();
    let has_stretch2 = layer.animated.stretch2_scale.value.is_some()
        || !layer.animated.stretch2_scale.keyframes.is_empty();

    let initial_wipe = if has_wipe {
        let wipe_start = layer.animated.wipe_start.value.unwrap_or(0.0);
        let wipe_end = layer.animated.wipe_end.value.unwrap_or(1.0);
        let wipe_angle = layer.animated.wipe_angle.value.unwrap_or(0.0);
        let wipe_feather = layer.animated.wipe_feather.value.unwrap_or(0.0);
        Some(Vec4::new(wipe_start, wipe_end, wipe_angle, wipe_feather))
    } else {
        None
    };

    let initial_stretch = if has_stretch {
        let angle_deg = layer.animated.stretch_angle.value.unwrap_or(0.0);
        let angle_rad = angle_deg.to_radians();
        let stretch_px = layer.animated.stretch_amount.value.unwrap_or(0.0);
        let stretch_uv = stretch_px / 500.0;
        let offset_px = layer.animated.stretch_offset.value.unwrap_or(0.0);
        let offset_uv = offset_px / 500.0;
        let smooth = layer.animated.stretch_smooth.value.unwrap_or(0.0);
        let smooth_width = smooth * 0.3;
        Some(Vec4::new(angle_rad, stretch_uv, offset_uv, smooth_width))
    } else {
        None
    };

    let initial_blur = if has_blur {
        let blur_strength = layer.animated.blur_strength.value.unwrap_or(0.0);
        Some(Vec4::new(blur_strength * 80.0, 0.0, 0.0, 0.0))
    } else {
        None
    };

    let max_blur_radius = if has_blur {
        let max_strength = layer
            .animated
            .blur_strength
            .keyframes
            .iter()
            .filter_map(|kf| kf.value.parse::<f32>().ok())
            .fold(layer.animated.blur_strength.value.unwrap_or(0.0), f32::max);
        max_strength * 80.0
    } else {
        0.0
    };

    let (initial_mesh_offset, initial_stretch_mesh_bounds) = if has_stretch {
        let sprite_size =
            interpolate_vec2(&layer.animated.size, layer_time).unwrap_or([100.0, 100.0]);
        let scale = interpolate_vec2(&layer.animated.scale, layer_time).unwrap_or([1.0, 1.0]);
        let orig_width = (sprite_size[0] * scale[0]).abs().max(1.0);
        let orig_height = (sprite_size[1] * scale[1]).abs().max(1.0);

        let angle_deg = interpolate_float(&layer.animated.stretch_angle, layer_time).unwrap_or(0.0);
        let transform_rotation_rad = initial_rotation.to_euler(bevy::math::EulerRot::XYZ).2;
        let angle_rad = angle_deg.to_radians();
        let stretch_raw =
            interpolate_float(&layer.animated.stretch_amount, layer_time).unwrap_or(0.0);
        let offset_raw =
            interpolate_float(&layer.animated.stretch_offset, layer_time).unwrap_or(0.0);

        let scene_width = layer.animated.canvas_width;
        let scene_height = layer.animated.canvas_height;
        let adj_stretch = stretch_raw / 500.0;
        let _offset_norm = offset_raw / 1000.0;

        let dx_screen = angle_rad.cos().abs() * adj_stretch * scene_width;
        let dy_screen = angle_rad.sin().abs() * adj_stretch * scene_height;
        let rot_cos = transform_rotation_rad.cos().abs();
        let rot_sin = transform_rotation_rad.sin().abs();
        let max_dx = rot_cos * dx_screen + rot_sin * dy_screen;
        let max_dy = rot_sin * dx_screen + rot_cos * dy_screen;

        let hw = orig_width / 2.0;
        let hh = orig_height / 2.0;
        let min_x = -hw - max_dx;
        let max_x = hw + max_dx;
        let min_y = -hh - max_dy;
        let max_y = hh + max_dy;

        bevy::log::trace!(
            "[SpawnStretch] layer '{}' orig=({:.1},{:.1}) adj_stretch={:.4} expansion=({:.2},{:.2})",
            layer.label,
            orig_width,
            orig_height,
            adj_stretch,
            max_dx,
            max_dy
        );

        (
            Some(Vec4::new(
                transform_rotation_rad,
                0.0,
                scene_width,
                scene_height,
            )),
            Some((min_x, max_x, min_y, max_y)),
        )
    } else {
        (None, None)
    };

    let initial_replace_color = if layer.animated.replace_old_color != Vec4::ZERO
        || layer.animated.replace_new_color.value.is_some()
        || !layer.animated.replace_new_color.keyframes.is_empty()
    {
        let new_color_srgb = if let Some(val) = layer.animated.replace_new_color.value {
            val
        } else if !layer.animated.replace_new_color.keyframes.is_empty() {
            let first_kf = &layer.animated.replace_new_color.keyframes[0];
            parse_keyframe_color(&first_kf.value).unwrap_or(Vec4::new(1.0, 1.0, 1.0, 1.0))
        } else {
            Vec4::new(1.0, 1.0, 1.0, 1.0)
        };

        let threshold = layer.animated.replace_threshold.value.unwrap_or(0.25);
        let feather = layer.animated.replace_feather.value.unwrap_or(0.25);
        let alpha = layer.animated.replace_alpha.value.unwrap_or(1.0);
        let lock_lum = if layer.animated.replace_lock_luminance {
            1.0
        } else {
            0.0
        };

        Some((
            Vec4::new(1.0, lock_lum, 0.0, 0.0),
            layer.animated.replace_old_color,
            new_color_srgb,
            Vec4::new(threshold, feather, alpha, 0.0),
        ))
    } else {
        None
    };

    add_visual_components(
        commands,
        meshes,
        unified_materials,
        color_materials,
        sdf_materials,
        entity,
        &layer.spec,
        &layer.mask_info,
        layer.palette_params.as_ref(),
        images,
        fonts,
        white_pixel,
        &layer.label,
        layer.id,
        initial_scale,
        initial_wipe,
        initial_stretch,
        initial_blur,
        layer.embed_scene_size,
        1.0,
        max_blur_radius,
        initial_mesh_offset,
        initial_stretch_mesh_bounds,
        1.0 / inv_fit_scale,
        layer.containing_embed_id != 0,
        !layer.animated.scale.keyframes.is_empty(),
        layer.animated.scale_assist_axis != 0,
        layer.animated.repeat_count.value.is_some_and(|v| v > 0.0)
            || !layer.animated.repeat_count.keyframes.is_empty()
            || layer
                .animated
                .linear_repeat_count
                .value
                .is_some_and(|v| v > 0.0)
            || !layer.animated.linear_repeat_count.keyframes.is_empty()
            || layer.animated.linear_repeat2.is_some()
            || layer
                .animated
                .radial_repeat_count
                .value
                .is_some_and(|v| v > 0.0)
            || !layer.animated.radial_repeat_count.keyframes.is_empty(),
        layer.animated.threshold_value.value.is_some()
            || !layer.animated.threshold_value.keyframes.is_empty(),
        layer.animated.grid_spacing.value.is_some()
            || !layer.animated.grid_spacing.keyframes.is_empty(),
        layer.animated.pixelate_size.value.is_some()
            || !layer.animated.pixelate_size.keyframes.is_empty(),
        has_stretch2,
        layer.animated.solid_color_alpha.value.is_some()
            || !layer.animated.solid_color_alpha.keyframes.is_empty(),
        layer.animated.wavewarp2_has_effect,
        layer.animated.mirror_has_effect,
        layer.animated.lift_has_effect,
        layer.animated.rays_has_effect,
        layer.animated.rgb_split_enabled,
        layer.animated.exposure_has_effect,
        layer.blending_mode.is_blend(),
        layer.animated.chromakey_enabled,
        layer.animated.parenthelper_has_effect,
        has_child_layers,
        if layer.animated.rgb_split_enabled {
            let max_strength = layer
                .animated
                .rgb_split_strength
                .keyframes
                .iter()
                .filter_map(|kf| kf.value.parse::<f32>().ok())
                .fold(
                    layer
                        .animated
                        .rgb_split_strength
                        .value
                        .unwrap_or(0.15)
                        .abs(),
                    |acc, v| acc.max(v.abs()),
                );
            max_strength / 8.0
        } else {
            0.0
        },
        {
            let max_size = layer
                .animated
                .pixelate_size
                .keyframes
                .iter()
                .filter_map(|kf| kf.value.parse::<f32>().ok())
                .fold(layer.animated.pixelate_size.value.unwrap_or(0.0), f32::max);
            let mut max_stretch = 1.0f32;
            if let Some(v) = layer.animated.pixelate_stretch.value {
                max_stretch = max_stretch.max(v[0].abs()).max(v[1].abs());
            }
            max_stretch = layer
                .animated
                .pixelate_stretch
                .keyframes
                .iter()
                .filter_map(|kf| {
                    let mut parts = kf.value.split(',');
                    let x = parts.next()?.trim().parse::<f32>().ok()?;
                    let y = parts.next()?.trim().parse::<f32>().ok()?;
                    Some(x.abs().max(y.abs()))
                })
                .fold(max_stretch, f32::max);
            max_size * max_stretch / 2.0
        },
        {
            let mut max_m2 = layer.animated.wavewarp2_m2.value.unwrap_or(0.0);
            for kf in &layer.animated.wavewarp2_m2.keyframes {
                max_m2 = max_m2.max(kf.value.parse::<f32>().unwrap_or(0.0).abs());
            }
            max_m2
        },
        {
            let mut max_off = layer.animated.mirror_offset.value.unwrap_or(0.0).abs();
            for kf in &layer.animated.mirror_offset.keyframes {
                max_off = max_off.max(kf.value.parse::<f32>().unwrap_or(0.0).abs());
            }
            max_off
        },
        global_time as u64,
        initial_replace_color,
        {
            let mut max_s = initial_scale.0.abs().max(initial_scale.1.abs());
            max_s = layer
                .animated
                .scale
                .keyframes
                .iter()
                .filter_map(|kf| {
                    let mut parts = kf.value.split(',');
                    let sx = parts.next()?.parse::<f32>().ok()?;
                    let sy = parts.next()?.parse::<f32>().ok()?;
                    Some(sx.abs().max(sy.abs()))
                })
                .fold(max_s, f32::max);
            let base_half = match &layer.spec {
                crate::scene::AmLayerSpec::SdfShape { width, height, .. } => {
                    (*width / 2.0).max(*height / 2.0).max(1.0)
                }
                _ => 1.0,
            };
            let max_size_ratio = layer
                .animated
                .size
                .keyframes
                .iter()
                .filter_map(|kf| {
                    let mut parts = kf.value.split(',');
                    let w = parts.next()?.parse::<f32>().ok()?;
                    let h = parts.next()?.parse::<f32>().ok()?;
                    Some((w / 2.0).max(h / 2.0) / base_half)
                })
                .fold(1.0f32, f32::max);
            let max_stroke = layer
                .animated
                .stroke_width
                .keyframes
                .iter()
                .filter_map(|kf| kf.value.parse::<f32>().ok())
                .fold(layer.animated.stroke_width.value.unwrap_or(0.0), f32::max);
            let stroke_direction = match &layer.spec {
                crate::scene::AmLayerSpec::SdfShape {
                    stroke_direction, ..
                } => stroke_direction.as_str(),
                _ => "inside",
            };
            let stroke_expansion = match stroke_direction {
                "outside" => max_stroke,
                "centered" => max_stroke * 0.5,
                _ => 0.0,
            };
            let border2_expansion = match &layer.spec {
                crate::scene::AmLayerSpec::SdfShape {
                    border2_width,
                    border2_direction,
                    ..
                } => match border2_direction.as_str() {
                    "outside" => *border2_width,
                    "centered" => *border2_width * 0.5,
                    _ => 0.0,
                },
                _ => 0.0,
            };
            let total_expansion = stroke_expansion + border2_expansion;
            let expansion_ratio = if base_half > 0.0 {
                (base_half + total_expansion) / base_half
            } else {
                1.0
            };
            max_s * max_size_ratio * expansion_ratio
        },
    );

    if let Some(ref fill) = layer.group_fill {
        commands.entity(entity).insert(fill.clone());
    }
}

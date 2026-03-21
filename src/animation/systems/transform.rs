use bevy::prelude::*;

use crate::animation::components::{
    AmAnimated, AmPlayback, AmSdfShapeParent, AmUnifiedUsesTransformScale,
};
use crate::animation::interpolation::{
    interpolate_float, interpolate_vec2, interpolate_vec2_reverse,
    interpolate_vec3_reverse,
};
use crate::animation::noise_effects::{compute_jitter, compute_simplex_displace};
use crate::scene::{AmLayerMarker, AmLayerSpec};

use super::shared::{
    apply_oscillate, compute_normalized_frame_delta, invert_transform_component,
    resolve_unwrapped_rotation_deg,
};

fn apply_sdf_linear_repeat(
    sdf_parent: Option<&AmSdfShapeParent>,
    animated: &AmAnimated,
    layer_time: f32,
    bx: &mut f32,
    by: &mut f32,
) {
    if sdf_parent.is_none() {
        return;
    }
    let Some(d) = crate::animation::effects::repeat::compute_sdf_linear_repeat_displacement(
        animated, layer_time,
    ) else {
        return;
    };
    if d[0].is_nan() {
        *bx = -99999.0;
        *by = -99999.0;
    } else {
        *bx += d[0];
        *by -= d[1];
    }
}

fn apply_pivot_offset(
    animated: &AmAnimated,
    layer_time: f32,
    layer_spec: &AmLayerSpec,
    sdf_parent: Option<&AmSdfShapeParent>,
    current_scale: [f32; 2],
    bx: &mut f32,
    by: &mut f32,
) {
    let Some(pivot) = interpolate_vec2(&animated.pivot, layer_time) else {
        return;
    };
    let pivot_x = pivot[0];
    let pivot_y = pivot[1];

    let is_sdf_shape = sdf_parent.is_some() || matches!(layer_spec, AmLayerSpec::SdfShape { .. });

    if is_sdf_shape {
        *bx += pivot_x;
        *by -= pivot_y;
    } else if matches!(layer_spec, AmLayerSpec::EmbedScene | AmLayerSpec::Null) {
        let rotation_deg = interpolate_float(&animated.rotation, layer_time).unwrap_or(0.0);
        let rotation_rad = (-rotation_deg + animated.repeat_rotation_offset_deg).to_radians();
        let pivot_bevy_y = -pivot_y;
        let scaled_offset_x = -pivot_x * current_scale[0];
        let scaled_offset_y = -pivot_bevy_y * current_scale[1];
        let rotated_offset_x =
            scaled_offset_x * rotation_rad.cos() - scaled_offset_y * rotation_rad.sin();
        let rotated_offset_y =
            scaled_offset_x * rotation_rad.sin() + scaled_offset_y * rotation_rad.cos();

        *bx += rotated_offset_x + pivot_x;
        *by += rotated_offset_y + pivot_bevy_y;
    }
}

pub fn animate_transform_system(
    playback: Res<AmPlayback>,
    mut query: Query<(
        &AmAnimated,
        &mut Transform,
        &AmLayerMarker,
        &AmLayerSpec,
        Option<&AmSdfShapeParent>,
        Option<&crate::masked_sprite::UnifiedEffectMarker>,
        Option<&AmUnifiedUsesTransformScale>,
        Option<&crate::scene::AmEmbedContentMarker>,
    )>,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (
        animated,
        mut transform,
        _marker,
        layer_spec,
        sdf_parent,
        effect_marker,
        unified_transform_scale,
        embed_content_marker,
    ) in query.iter_mut()
    {
        let local_time = animated.calc_local_time(global_time);
        if !animated.is_active(local_time) {
            continue;
        }

        let layer_time = animated.calc_layer_time(local_time);
        let frame_delta = compute_normalized_frame_delta(animated);

        let mut actual_scale = interpolate_vec2_reverse(&animated.scale, layer_time, frame_delta)
            .unwrap_or([1.0, 1.0]);

        if animated.scale_assist_axis != 0
            && let Some(scale_param) = crate::animation::interpolation::interpolate_float(
                &animated.scale_assist,
                layer_time,
            )
        {
            let damp_param = crate::animation::interpolation::interpolate_float(
                &animated.scale_assist_damp,
                layer_time,
            )
            .unwrap_or(1.0);

            const SCALE_POWER: f32 = 1.71;
            const DAMP_COEFF: f32 = 2.75;
            const DAMP_POWER: f32 = 1.93;

            match animated.scale_assist_axis {
                1 => actual_scale[1] *= scale_param,
                2 => actual_scale[0] *= scale_param,
                3 => {
                    let damp_exp = 1.0 + DAMP_COEFF * (damp_param - 1.0).powf(DAMP_POWER);
                    let damp_factor = damp_param.powf(damp_exp);
                    let scale_divisor = scale_param.powf(SCALE_POWER) * damp_factor;
                    actual_scale[0] *= scale_param;
                    actual_scale[1] /= scale_divisor;
                }
                _ => {}
            }
        }

        let mut posz_offset = 0.0_f32;
        if let Some(mut posz) = interpolate_float(&animated.effect_posz, layer_time) {
            if animated.effect_zinv {
                posz = 2.0 - posz;
            }
            posz_offset += posz - 1.0;
        }
        for extra in &animated.extra_transform2 {
            let Some(mut posz) = interpolate_float(&extra.pos_z, layer_time) else {
                continue;
            };
            if extra.zinv {
                posz = 2.0 - posz;
            }
            posz_offset += posz - 1.0;
        }
        let combined_posz = 1.0 + posz_offset;
        actual_scale[0] *= combined_posz;
        actual_scale[1] *= combined_posz;

        let unified_scale_baked = effect_marker.is_some() && unified_transform_scale.is_none();
        let current_scale = if sdf_parent.is_some() || unified_scale_baked {
            [1.0_f32, 1.0_f32]
        } else {
            actual_scale
        };

        let loc =
            interpolate_vec3_reverse(&animated.location, layer_time, frame_delta).or_else(|| {
                if animated.has_parent
                    && sdf_parent.is_none()
                    && matches!(layer_spec, AmLayerSpec::SdfShape { .. })
                {
                    Some([0.0, 0.0, 0.0])
                } else {
                    None
                }
            });

        let mut oscillate_z_zoom = 1.0_f32;
        if let Some(loc) = loc {
            let (mut bx, mut by) = if animated.has_parent {
                (loc[0], -loc[1])
            } else {
                (
                    loc[0] - animated.canvas_width / 2.0,
                    animated.canvas_height / 2.0 - loc[1],
                )
            };

            if animated.layer_id == 347000343 {
                trace!(
                    "[PosCalc] layer={} is_embed_content={} speed_mul={:.2} time_offset={} | global_time={:.1} local_time={:.1} layer_time={:.4} | AM_loc=({:.2},{:.2}) canvas=({:.0},{:.0}) has_parent={} | bevy=({:.2},{:.2})",
                    animated.layer_id,
                    embed_content_marker.is_some(),
                    animated.speed_multiplier,
                    animated.time_offset,
                    global_time,
                    local_time,
                    layer_time,
                    loc[0],
                    loc[1],
                    animated.canvas_width,
                    animated.canvas_height,
                    animated.has_parent,
                    bx,
                    by
                );
            }

            apply_pivot_offset(
                animated,
                layer_time,
                layer_spec,
                sdf_parent,
                current_scale,
                &mut bx,
                &mut by,
            );

            if let Some(effect_x) = interpolate_float(&animated.effect_pos_x, layer_time) {
                bx += invert_transform_component(effect_x, animated.effect_xinv);
            }
            if let Some(effect_y) = interpolate_float(&animated.effect_pos_y, layer_time) {
                by -= invert_transform_component(effect_y, animated.effect_yinv);
            }
            for extra in &animated.extra_transform2 {
                bx += interpolate_float(&extra.pos_x, layer_time)
                    .map(|x| invert_transform_component(x, extra.xinv))
                    .unwrap_or(0.0);
                by -= interpolate_float(&extra.pos_y, layer_time)
                    .map(|y| invert_transform_component(y, extra.yinv))
                    .unwrap_or(0.0);
            }

            if !animated.has_parent {
                by -= animated.font_y_offset;
            }

            if matches!(layer_spec, AmLayerSpec::Text { .. }) {
                bx -= animated.inv_fit_scale;
            }

            if !matches!(layer_spec, AmLayerSpec::SdfShape { .. }) && sdf_parent.is_none() {
                bx += animated.anchor_offset.x;
                by += animated.anchor_offset.y;
            }

            oscillate_z_zoom = apply_oscillate(animated, layer_time, &mut bx, &mut by);

            if animated.jitter_enabled {
                let (jdx, jdy, jz) = compute_jitter(animated, local_time);
                bx = (bx + jdx) * jz;
                by = (by + jdy) * jz;
                oscillate_z_zoom *= jz;
            }

            if animated.sd_enabled {
                let (sdx, sdy) = compute_simplex_displace(animated, layer_time, bx, by);
                bx += sdx;
                by += sdy;
            }

            bx += animated.repeat_position_offset.x;
            by += animated.repeat_position_offset.y;

            apply_sdf_linear_repeat(sdf_parent, animated, layer_time, &mut bx, &mut by);

            transform.translation = Vec3::new(bx, by, transform.translation.z);
        }

        let final_rotation = resolve_unwrapped_rotation_deg(animated, layer_time, frame_delta);
        transform.rotation = Quat::from_rotation_z(final_rotation.to_radians());

        if sdf_parent.is_none() && effect_marker.is_none() {
            transform.scale = Vec3::new(
                current_scale[0] * oscillate_z_zoom * animated.repeat_scale_factor,
                current_scale[1] * oscillate_z_zoom * animated.repeat_scale_factor,
                1.0,
            );
        } else if unified_scale_baked {
            let sign_x = actual_scale[0].signum();
            let sign_y = actual_scale[1].signum();
            transform.scale = Vec3::new(
                sign_x * combined_posz * oscillate_z_zoom,
                sign_y * combined_posz * oscillate_z_zoom,
                1.0,
            );
        }
    }
}

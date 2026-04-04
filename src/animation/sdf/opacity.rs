//! Animates opacity and solid-color blending for SDF-backed visuals.
//! 驱动基于 SDF 的可视对象透明度与纯色混合动画。
//!
//! SDF children do not use Bevy's standard sprite alpha path, so opacity, fade curves, echo alpha,
//! and solid-color overlays must be resolved into shader uniforms here. This module converts the
//! layer timeline into packed material data and keeps hidden/force-hidden states in sync with the
//! visibility expected by the rest of the animation pipeline.
//! SDF 子对象不走 Bevy 默认的 sprite 透明度路径，所以透明度、淡入淡出、echo alpha 和纯色叠加
//! 都必须在这里折算成 shader uniform。这个模块把图层时间线转换成材质数据，并同步隐藏 /
//! 强制隐藏状态，让可见性行为与整个动画管线保持一致。

use bevy::prelude::*;

use crate::sdf_material::{SdfMaterial, repack_with_alpha};

use super::super::components::{AmAnimated, AmPlayback, AmSdfParams, AmSdfShapeParent};
use super::super::interpolation::{interpolate_color, interpolate_float};
use super::super::sdf_helpers::{apply_solidcolor_blend, trace_sdf_once};

pub fn animate_sdf_opacity_system(
    playback: Res<AmPlayback>,
    parent_query: Query<
        (
            &AmAnimated,
            &Children,
            &crate::scene::AmLayerMarker,
            Option<&crate::scene::AmForceHidden>,
        ),
        With<AmSdfShapeParent>,
    >,
    mut sdf_query: Query<(
        &MeshMaterial2d<SdfMaterial>,
        &AmSdfParams,
        &mut Visibility,
        &GlobalTransform,
        Option<&ChildOf>,
    )>,
    mut materials: ResMut<Assets<SdfMaterial>>,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (animated, children, marker, force_hidden) in parent_query.iter() {
        let local_time = animated.calc_local_time(global_time);
        let layer_time = animated.calc_layer_time(local_time);
        let opacity = interpolate_float(&animated.opacity, layer_time).unwrap_or(1.0);
        let is_force_hidden = force_hidden.is_some();

        for child in children.iter() {
            let Ok((material_handle, sdf_params, mut visibility, child_gt, child_of)) =
                sdf_query.get_mut(child)
            else {
                continue;
            };

            if !animated.is_active(local_time) {
                *visibility = Visibility::Hidden;
                // Only mutate material if values actually differ
                #[expect(clippy::excessive_nesting)]
                // reason: guard against spurious GPU re-upload inside inactive-layer branch
                if let Some(mat_ref) = materials.get(&material_handle.0) {
                    let zero_packed = repack_with_alpha(sdf_params.packed_stroke, 0.0);
                    if mat_ref.uniform_data.color.w != 0.0
                        || mat_ref.uniform_data.params.w != zero_packed
                    {
                        let material = materials.get_mut(&material_handle.0).unwrap();
                        material.uniform_data.color.w = 0.0;
                        material.uniform_data.params.w = zero_packed;
                    }
                }
                continue;
            }

            if is_force_hidden {
                *visibility = Visibility::Hidden;
            } else {
                *visibility = Visibility::Inherited;
            }

            let Some(mat_ref) = materials.get(&material_handle.0) else {
                continue;
            };
            let mut new_uniform = mat_ref.uniform_data;

            if let Some(fc_srgb) = interpolate_color(&animated.fill_color, layer_time) {
                new_uniform.color.x = fc_srgb.x.powf(2.2);
                new_uniform.color.y = fc_srgb.y.powf(2.2);
                new_uniform.color.z = fc_srgb.z.powf(2.2);
            }

            let mut final_alpha = opacity * animated.base_alpha;
            final_alpha *= animated.calc_fade_alpha(layer_time);
            let echo_mult = if let Some(ref echo_cfg) = animated.echo_alpha_config {
                echo_cfg.evaluate(global_time)
            } else {
                1.0
            };
            final_alpha *= echo_mult;
            new_uniform.color.w = final_alpha.clamp(0.0, 1.0);

            let final_stroke_alpha =
                (sdf_params.base_stroke_alpha * opacity * echo_mult).clamp(0.0, 1.0);
            new_uniform.params.w = repack_with_alpha(sdf_params.packed_stroke, final_stroke_alpha);

            if marker.label.starts_with("Rectangle 1 Copy") {
                let parent = child_of.map(|c| c.parent());
                #[expect(clippy::excessive_nesting)]
                // reason: keep the targeted Rectangle 1 Copy trace beside the opacity update
                trace_sdf_once(format!("{}:{}", marker.id, marker.label), || {
                    format!(
                        "[SDF] layer_id={} label='{}' parent={:?} vis={:?} fill_alpha={:.3} global_z={:.4} stroke_width={:.3} frame_half={:.3}",
                        marker.id,
                        marker.label,
                        parent,
                        *visibility,
                        new_uniform.color.w,
                        child_gt.translation().z,
                        new_uniform.params.z,
                        new_uniform.frame_half,
                    )
                });
            }

            let sc_alpha =
                interpolate_float(&animated.solid_color_alpha, layer_time).unwrap_or(0.0);
            if sc_alpha > 0.0 {
                let sc_color =
                    interpolate_color(&animated.solid_color, layer_time).unwrap_or(Vec4::ZERO);
                apply_solidcolor_blend(
                    &mut new_uniform.color,
                    &animated.base_fill_color,
                    sc_color,
                    sc_alpha,
                    animated.solid_color_blend_mode,
                );
            }

            let pix_thresh =
                interpolate_float(&animated.pixelate_threshold, layer_time).unwrap_or(0.0);
            new_uniform.gradient_config.y = pix_thresh;

            // Only mutate if changed
            if new_uniform != mat_ref.uniform_data {
                let material = materials.get_mut(&material_handle.0).unwrap();
                material.uniform_data = new_uniform;
            }
        }
    }
}

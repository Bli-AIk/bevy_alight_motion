//! Main unified effect animation system.
//!
//! Handles wipe, blur, stretch, opacity, palette, replace color, threshold,
//! grid, and pixelate effects. Repeat and linear repeat processing is
//! delegated to the `repeat` submodule.

use bevy::prelude::*;

mod effects;
mod mesh;
mod post_effects;

use self::effects::{
    update_blend, update_chromakey, update_exposure, update_lift, update_mirror, update_palette,
    update_rays, update_rgb_split, update_solidcolor, update_stretch2_uniform, update_wavewarp2,
    update_wipe,
};
use self::mesh::{update_base_mesh, update_blur_mesh, update_stretch_mesh};
use self::post_effects::{update_grid, update_pixelate, update_replace_color, update_threshold};
use super::unified_support::{compute_ancestor_scale, trace_parenthelper_unified_state};
use crate::animation::components::{AmAnimated, AmPlayback, AmUnifiedMeshState};
use crate::animation::interpolation::{interpolate_float, interpolate_vec2};

fn force_white_tint_enabled(layer_id: u64) -> bool {
    std::env::var_os("AM_FORCE_WHITE_TINT_IDS")
        .and_then(|value| value.into_string().ok())
        .is_some_and(|ids| {
            ids.split(',')
                .filter_map(|value| value.trim().parse::<u64>().ok())
                .any(|id| id == layer_id)
        })
}

fn trace_unified_color_enabled(layer_id: u64) -> bool {
    std::env::var_os("AM_TRACE_UNIFIED_COLOR_IDS")
        .and_then(|value| value.into_string().ok())
        .is_some_and(|ids| {
            ids.split(',')
                .filter_map(|value| value.trim().parse::<u64>().ok())
                .any(|id| id == layer_id)
        })
}

/// System to animate effects on sprites using UnifiedEffectMaterial.
/// This system handles all effect types (wipe, stretch segment, mask, blur) in a single pass.
/// It is designed for the RTT architecture where effects are stackable.
pub fn animate_unified_effect_system(
    playback: Res<AmPlayback>,
    mut query: Query<(
        Entity,
        &AmAnimated,
        &crate::scene::AmLayerMarker,
        &MeshMaterial2d<crate::masked_sprite::UnifiedEffectMaterial>,
        &Transform,
        &GlobalTransform,
        &bevy::mesh::Mesh2d,
        &mut AmUnifiedMeshState,
        Option<&crate::animation::components::AmUnifiedUsesTransformScale>,
        Option<&crate::scene::AmEmbedContentMarker>,
        Option<&Visibility>,
        Option<&bevy::camera::visibility::RenderLayers>,
        Option<&ChildOf>,
    )>,
    parent_animated_query: Query<(&AmAnimated, Option<&ChildOf>)>,
    effect_marker_query: Query<(), With<crate::masked_sprite::UnifiedEffectMarker>>,
    root_query: Query<&Transform, With<crate::scene::AmProjectRoot>>,
    _embed_gt_query: Query<&GlobalTransform>,
    mut materials: ResMut<Assets<crate::masked_sprite::UnifiedEffectMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let global_time = playback.current_time_ms;
    // Get the FitWindow root scale (uniform scaling applied to project root entity).
    let root_scale = root_query
        .iter()
        .next()
        .map(|t| t.scale.x)
        .unwrap_or(1.0)
        .max(0.001);

    for (
        entity,
        animated,
        marker,
        material_handle,
        transform,
        global_transform,
        mesh2d,
        mut mesh_state,
        unified_transform_scale,
        _embed_marker,
        visibility,
        render_layers,
        child_of,
    ) in query.iter_mut()
    {
        if unified_transform_scale.is_some() {
            continue;
        }

        // Use local time for visibility check (affected by speed)
        let local_time = animated.calc_local_time(global_time);

        // Get material to update alpha
        if let Some(material) = materials.get_mut(&material_handle.0) {
            if !animated.is_active(local_time) {
                // Hide layer by setting alpha to 0
                material.uniform_data.color.w = 0.0;
                continue;
            }

            // Layer is active - restore alpha (will be updated by opacity below)
            let layer_time = animated.calc_layer_time(local_time);
            let opacity = interpolate_float(&animated.opacity, layer_time).unwrap_or(1.0);
            let fade_alpha = animated.calc_fade_alpha(layer_time);
            material.uniform_data.color.w = opacity * animated.base_alpha * fade_alpha;
        } else if !animated.is_active(local_time) {
            continue;
        }

        // Use animation local time for interpolation (affected by speed)
        let layer_time = animated.calc_layer_time(local_time);

        // Get sprite base size and scale
        let sprite_size = interpolate_vec2(&animated.size, layer_time).unwrap_or([100.0, 100.0]);
        let mut scale = interpolate_vec2(&animated.scale, layer_time).unwrap_or([1.0, 1.0]);

        // Apply scale_assist effect
        // Formula derived from reference video analysis:
        //   axis=1 (Y only): scale_y *= scale_param
        //   axis=2 (X only): scale_x *= scale_param
        //   axis=3 (Both):   scale_x *= scale_param
        //                    scale_y /= (scale_param^SCALE_POWER * damp_factor)
        //                    where damp_factor = damp^(1 + DAMP_COEFF*(damp-1)^DAMP_POWER)
        if animated.scale_assist_axis != 0
            && let Some(scale_param) = interpolate_float(&animated.scale_assist, layer_time)
        {
            let damp_param =
                interpolate_float(&animated.scale_assist_damp, layer_time).unwrap_or(1.0);

            // Debug log for scale_assist with cyclic easing
            bevy::log::trace!(
                "[scale_assist] layer_time={:.4}, scale_param={:.4}, damp={:.4}, axis={}",
                layer_time,
                scale_param,
                damp_param,
                animated.scale_assist_axis
            );

            // Constants derived from empirical analysis of AM reference videos
            // scale divisor = scale_param^SCALE_POWER
            // damp factor = damp^(1 + DAMP_COEFF*(damp-1)^DAMP_POWER)
            const SCALE_POWER: f32 = 1.71; // = ln(2) / ln(1.501), makes scale_y=0.5 when scale_param=1.501
            const DAMP_COEFF: f32 = 2.75;
            const DAMP_POWER: f32 = 1.93;

            match animated.scale_assist_axis {
                1 => {
                    // Y only (vertical stretch)
                    scale[1] *= scale_param;
                }
                2 => {
                    // X only (horizontal stretch)
                    scale[0] *= scale_param;
                }
                3 => {
                    // Both axes - X stretches, Y compresses
                    // This creates the characteristic "line stretch" effect
                    let damp_exp = 1.0 + DAMP_COEFF * (damp_param - 1.0).powf(DAMP_POWER);
                    let damp_factor = damp_param.powf(damp_exp);
                    let scale_divisor = scale_param.powf(SCALE_POWER) * damp_factor;
                    scale[0] *= scale_param;
                    scale[1] /= scale_divisor;
                }
                _ => {}
            }
        }

        // Actual rendered size = base size * scale * accumulated ancestor scale
        // Use abs() because negative size in AM behaves same as positive (no flip)
        // Ancestor scale accounts for parent hierarchy scale that effect sprites bake into
        // mesh size rather than Transform.scale, ensuring children match screen-space dimensions.
        let mut ancestor_scale = compute_ancestor_scale(
            entity,
            &parent_animated_query,
            &effect_marker_query,
            global_time,
        );
        if animated.parenthelper_has_effect {
            let parenthelper_scale_factor = match animated.parenthelper_scale_mode {
                1 => 0.0,
                2 => interpolate_float(&animated.parenthelper_scale_weight, layer_time)
                    .unwrap_or(1.0),
                _ => 1.0,
            };
            ancestor_scale[0] = 1.0 + (ancestor_scale[0] - 1.0) * parenthelper_scale_factor;
            ancestor_scale[1] = 1.0 + (ancestor_scale[1] - 1.0) * parenthelper_scale_factor;
        }
        let orig_width = (sprite_size[0] * scale[0]).abs().max(1.0) * ancestor_scale[0].abs();
        let orig_height = (sprite_size[1] * scale[1]).abs().max(1.0) * ancestor_scale[1].abs();

        // NOTE: inv_fit_scale is NOT applied to RTT content dimensions
        // RTT content renders at scene's internal resolution, and the final
        // display size is determined by embed's transform scale and main scene's fit_scale.
        // Applying inv_fit_scale here would incorrectly enlarge the content.

        // Stretch operates in screen space, so nested/embed content needs the composed
        // world rotation instead of only the layer's local transform rotation.
        let (_, global_rotation, _) = global_transform.to_scale_rotation_translation();
        let transform_rotation_rad = global_rotation.to_euler(bevy::math::EulerRot::ZYX).0;

        // Calculate "world-space" dimensions for stretch calculations
        // When element is rotated, its local width/height swap in world space
        let rot_cos = transform_rotation_rad.cos().abs();
        let rot_sin = transform_rotation_rad.sin().abs();
        let _world_width = orig_width * rot_cos + orig_height * rot_sin;
        let world_height = orig_width * rot_sin + orig_height * rot_cos;
        let _ = world_height; // Reserved for future use

        // Check which effects are active
        let has_wipe = animated.wipe_end.value != Some(1.0)
            || !animated.wipe_end.keyframes.is_empty()
            || animated.wipe_start.value.is_some()
            || !animated.wipe_start.keyframes.is_empty();

        let has_stretch = animated.stretch_amount.value.is_some()
            || !animated.stretch_amount.keyframes.is_empty()
            || animated.stretch_angle.value.is_some()
            || !animated.stretch_angle.keyframes.is_empty()
            || animated.stretch_offset.value.is_some()
            || !animated.stretch_offset.keyframes.is_empty()
            || animated.stretch_smooth.value.is_some()
            || !animated.stretch_smooth.keyframes.is_empty()
            || animated.stretch_seg2_amount.value.is_some()
            || !animated.stretch_seg2_amount.keyframes.is_empty()
            || animated.stretch_seg2_angle.value.is_some()
            || !animated.stretch_seg2_angle.keyframes.is_empty()
            || animated.stretch_seg2_offset.value.is_some()
            || !animated.stretch_seg2_offset.keyframes.is_empty()
            || animated.stretch_seg2_smooth.value.is_some()
            || !animated.stretch_seg2_smooth.keyframes.is_empty();

        let has_stretch_seg2 = animated.stretch_seg2_amount.value.is_some()
            || !animated.stretch_seg2_amount.keyframes.is_empty()
            || animated.stretch_seg2_angle.value.is_some()
            || !animated.stretch_seg2_angle.keyframes.is_empty();

        let has_pixelate =
            animated.pixelate_size.value.is_some() || !animated.pixelate_size.keyframes.is_empty();

        let Some(material) = materials.get_mut(&material_handle.0) else {
            continue;
        };

        if trace_unified_color_enabled(animated.layer_id) {
            bevy::log::warn!(
                "[UnifiedColorTrace][pre-update] id={} label='{}' layer_time={:.6} color={:?} replace_flags={:?} threshold={:?}",
                animated.layer_id,
                marker.label,
                layer_time,
                material.uniform_data.color,
                material.uniform_data.replace_color_flags,
                material.uniform_data.threshold_params
            );
        }

        if force_white_tint_enabled(animated.layer_id) {
            material.uniform_data.color.x = 1.0;
            material.uniform_data.color.y = 1.0;
            material.uniform_data.color.z = 1.0;
        }

        if std::env::var_os("AM_PARENTHELPER_UNIFIED_TRACE").is_some()
            && (marker.label == "长方形 1" || marker.label == "长方形 2")
        {
            trace_parenthelper_unified_state(
                marker,
                material,
                transform,
                global_transform,
                visibility,
                render_layers,
                child_of.map(|c| c.parent()),
            );
        }

        // Always set original_size so pixelate (and other effects) have correct display dimensions
        material.uniform_data.original_size =
            Vec4::new(orig_width, orig_height, orig_width, orig_height);

        update_wipe(material, animated, layer_time, has_wipe);

        // Update stretch2 parameters (directional UV stretch)
        let has_stretch2 = animated.stretch2_scale.value.is_some()
            || !animated.stretch2_scale.keyframes.is_empty();
        let s2_scale = interpolate_float(&animated.stretch2_scale, layer_time).unwrap_or(1.0);
        let s2_angle_rad = interpolate_float(&animated.stretch2_angle, layer_time)
            .unwrap_or(0.0)
            .to_radians();
        update_stretch2_uniform(material, animated, has_stretch2, s2_scale, s2_angle_rad);

        update_wavewarp2(material, animated, layer_time, orig_width, orig_height);

        update_mirror(material, animated, layer_time);

        update_lift(material, animated, layer_time);

        update_rays(
            material,
            animated,
            _embed_marker,
            &parent_animated_query,
            global_time,
        );

        update_rgb_split(material, animated, layer_time);

        update_exposure(
            material,
            animated,
            _embed_marker,
            &parent_animated_query,
            global_time,
            layer_time,
        );

        update_chromakey(material, animated, layer_time);

        update_blend(material, animated);

        update_solidcolor(material, animated, layer_time);

        let has_blur =
            animated.blur_strength.value.is_some() || !animated.blur_strength.keyframes.is_empty();

        update_blur_mesh(
            material,
            animated,
            layer_time,
            orig_width,
            orig_height,
            mesh2d,
            &mut mesh_state,
            &mut meshes,
        );

        update_stretch_mesh(
            material,
            animated,
            layer_time,
            has_stretch,
            has_stretch_seg2,
            transform_rotation_rad,
            sprite_size,
            scale,
            ancestor_scale,
            orig_width,
            orig_height,
            global_transform,
            mesh2d,
            &mut mesh_state,
            &mut meshes,
        );

        update_base_mesh(
            material,
            animated,
            layer_time,
            has_stretch,
            has_blur,
            has_pixelate,
            has_stretch2,
            s2_scale,
            s2_angle_rad,
            orig_width,
            orig_height,
            mesh2d,
            &mut mesh_state,
            &mut meshes,
        );

        // Update palette map alpha if present
        let has_palette =
            animated.palette_alpha.value.is_some() || !animated.palette_alpha.keyframes.is_empty();
        update_palette(material, animated, layer_time, has_palette);

        let has_replace_color = animated.replace_old_color.w > 0.0;
        update_replace_color(material, animated, layer_time, has_replace_color);

        update_threshold(material, animated, layer_time);

        update_grid(material, animated, layer_time);
        update_pixelate(
            material,
            animated,
            layer_time,
            global_transform,
            root_scale,
            has_pixelate,
        );

        // Process repeat and linear repeat effects (delegated to repeat module)
        super::repeat::process_repeat_effect(
            animated,
            layer_time,
            material,
            orig_width,
            orig_height,
            mesh2d,
            &mut meshes,
        );

        super::repeat::process_linear_repeat_effect(
            animated,
            layer_time,
            material,
            orig_width,
            orig_height,
            mesh2d,
            &mut meshes,
        );

        super::repeat::process_radial_repeat_effect(
            animated,
            layer_time,
            material,
            orig_width,
            orig_height,
            mesh2d,
            &mut meshes,
        );

        if trace_unified_color_enabled(animated.layer_id) {
            bevy::log::warn!(
                "[UnifiedColorTrace][post-update] id={} label='{}' layer_time={:.6} color={:?} replace_flags={:?} threshold={:?}",
                animated.layer_id,
                marker.label,
                layer_time,
                material.uniform_data.color,
                material.uniform_data.replace_color_flags,
                material.uniform_data.threshold_params
            );
        }
    }
}

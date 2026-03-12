//! Main unified effect animation system.
//!
//! Handles wipe, blur, stretch, opacity, palette, replace color, threshold,
//! grid, and pixelate effects. Repeat and linear repeat processing is
//! delegated to the `repeat` submodule.

use bevy::prelude::*;

use crate::animation::components::{AmAnimated, AmPlayback};
use crate::animation::interpolation::{interpolate_color, interpolate_float, interpolate_vec2};

/// Convert sRGB component to linear for shader (colors from AM are sRGB).
fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Compute accumulated ancestor visual scale by walking up the entity hierarchy.
/// Only accumulates scale from ancestors that have UnifiedEffectMarker,
/// because those entities bake their animated scale into mesh size (not Transform.scale).
/// Regular group/shape parents put scale into Transform.scale, which children
/// already inherit through Bevy's transform hierarchy.
fn compute_ancestor_scale(
    entity: Entity,
    parent_query: &Query<(&AmAnimated, Option<&ChildOf>)>,
    effect_check: &Query<(), With<crate::masked_sprite::UnifiedEffectMarker>>,
    global_time: f32,
) -> [f32; 2] {
    let mut acc_scale = [1.0f32, 1.0f32];

    // Get entity's parent
    let parent_entity = match parent_query.get(entity) {
        Ok((_, Some(child_of))) => child_of.parent(),
        _ => return acc_scale,
    };

    // Walk up from the parent, accumulating animated scales only from effect sprites
    let mut current = parent_entity;
    while let Ok((animated, child_of_ref)) = parent_query.get(current) {
        // Only accumulate scale from effect sprites (scale baked into mesh, not Transform)
        if effect_check.contains(current) {
            let local_time = animated.calc_local_time(global_time);
            let layer_time = animated.calc_layer_time(local_time);
            let s = interpolate_vec2(&animated.scale, layer_time).unwrap_or([1.0, 1.0]);
            acc_scale[0] *= s[0];
            acc_scale[1] *= s[1];
        }

        if let Some(child_of) = child_of_ref {
            current = child_of.parent();
        } else {
            break;
        }
    }

    acc_scale
}

/// System to animate effects on sprites using UnifiedEffectMaterial.
/// This system handles all effect types (wipe, stretch segment, mask, blur) in a single pass.
/// It is designed for the RTT architecture where effects are stackable.
pub fn animate_unified_effect_system(
    playback: Res<AmPlayback>,
    mut commands: Commands,
    query: Query<(
        Entity,
        &AmAnimated,
        &MeshMaterial2d<crate::masked_sprite::UnifiedEffectMaterial>,
        &Transform,
        &GlobalTransform,
        &bevy::mesh::Mesh2d,
        Option<&crate::scene::AmEmbedContentMarker>,
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

    for (entity, animated, material_handle, transform, global_transform, _mesh2d, _embed_marker) in
        query.iter()
    {
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
        let ancestor_scale = compute_ancestor_scale(
            entity,
            &parent_animated_query,
            &effect_marker_query,
            global_time,
        );
        let orig_width = (sprite_size[0] * scale[0]).abs().max(1.0) * ancestor_scale[0].abs();
        let orig_height = (sprite_size[1] * scale[1]).abs().max(1.0) * ancestor_scale[1].abs();

        // NOTE: inv_fit_scale is NOT applied to RTT content dimensions
        // RTT content renders at scene's internal resolution, and the final
        // display size is determined by embed's transform scale and main scene's fit_scale.
        // Applying inv_fit_scale here would incorrectly enlarge the content.

        // Get transform rotation angle for effect compensation
        // In Bevy, rotation is stored as Quat, extract Z rotation
        let (_, _, transform_rotation_rad) = transform.rotation.to_euler(bevy::math::EulerRot::XYZ);

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
        // Always set original_size so pixelate (and other effects) have correct display dimensions
        material.uniform_data.original_size =
            Vec4::new(orig_width, orig_height, orig_width, orig_height);

        // Update wipe parameters if needed
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

        // Update stretch2 parameters (directional UV stretch)
        let has_stretch2 = animated.stretch2_scale.value.is_some()
            || !animated.stretch2_scale.keyframes.is_empty();
        let s2_scale = interpolate_float(&animated.stretch2_scale, layer_time).unwrap_or(1.0);
        let s2_angle_rad = interpolate_float(&animated.stretch2_angle, layer_time)
            .unwrap_or(0.0)
            .to_radians();
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

        // Update wavewarp2 parameters (波浪歪曲 / Wave Warp)
        if animated.wavewarp2_has_effect {
            let phase = interpolate_float(&animated.wavewarp2_phase, layer_time).unwrap_or(0.0);
            let a1d_rad = interpolate_float(&animated.wavewarp2_a1d, layer_time)
                .unwrap_or(0.0)
                .to_radians();
            let m1 = interpolate_float(&animated.wavewarp2_m1, layer_time).unwrap_or(20.0);
            let m2 = interpolate_float(&animated.wavewarp2_m2, layer_time).unwrap_or(4.0);
            let a2d = interpolate_float(&animated.wavewarp2_a2d, layer_time).unwrap_or(90.0);
            let a2_rad = (a1d_rad.to_degrees() + a2d).to_radians();
            let damping_val =
                interpolate_float(&animated.wavewarp2_damping, layer_time).unwrap_or(0.0);
            let damping_space =
                interpolate_float(&animated.wavewarp2_damping_space, layer_time).unwrap_or(0.0);
            let damping_origin =
                interpolate_float(&animated.wavewarp2_damping_origin, layer_time).unwrap_or(0.5);

            material.uniform_data.wavewarp2_params1 = Vec4::new(phase, a1d_rad, m1, m2);
            material.uniform_data.wavewarp2_params2 =
                Vec4::new(a2_rad, damping_val, damping_space, damping_origin);
            // AM computes offset in acLayerNorm but applies to acScreenNorm,
            // causing magnification by (canvas_size / layer_display_size).
            // Pass per-axis scale factors so the shader can replicate this.
            let mag_x = animated.canvas_width / orig_width.max(1.0);
            let mag_y = animated.canvas_height / orig_height.max(1.0);
            material.uniform_data.wavewarp2_flags = Vec4::new(
                if animated.wavewarp2_screen_space {
                    1.0
                } else {
                    0.0
                },
                1.0, // enabled
                mag_x,
                mag_y,
            );
        } else {
            material.uniform_data.wavewarp2_params1 = Vec4::ZERO;
            material.uniform_data.wavewarp2_params2 = Vec4::ZERO;
            material.uniform_data.wavewarp2_flags = Vec4::ZERO;
        }

        // Update mirror effect (镜子)
        if animated.mirror_has_effect {
            let alpha = interpolate_float(&animated.mirror_alpha, layer_time).unwrap_or(1.0);
            let offset = interpolate_float(&animated.mirror_offset, layer_time).unwrap_or(0.0);
            // Encode type+1: 0=disabled, 1=horizontal, 2=vertical
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

        // Update lift (copy background) effect
        if animated.lift_has_effect {
            let fill = interpolate_float(&animated.lift_fill, layer_time).unwrap_or(0.0);
            material.uniform_data.lift_params = Vec4::new(
                fill,
                animated.canvas_width,
                animated.canvas_height,
                1.0, // enabled
            );
        } else {
            material.uniform_data.lift_params = Vec4::ZERO;
        }

        // Update rays (volumetric light rays) effect / 更新射线效果
        if animated.rays_has_effect {
            let strength = interpolate_float(&animated.rays_strength, layer_time).unwrap_or(0.15);
            let intensity = interpolate_float(&animated.rays_intensity, layer_time).unwrap_or(1.0);
            let threshold = interpolate_float(&animated.rays_threshold, layer_time).unwrap_or(0.6);
            let quality = interpolate_float(&animated.rays_quality, layer_time).unwrap_or(150.0);
            let blend = interpolate_float(&animated.rays_blend, layer_time).unwrap_or(0.0);
            let center_x = interpolate_float(&animated.rays_center_x, layer_time).unwrap_or(0.0);
            let center_y = interpolate_float(&animated.rays_center_y, layer_time).unwrap_or(0.0);

            // Convert AM center coords to normalized (AM uses ±500 range)
            let center_x_norm = 0.5 + center_x / 500.0;
            let center_y_norm = 0.5 - center_y / 500.0;

            material.uniform_data.rays_params1 = Vec4::new(strength, intensity, threshold, quality);
            material.uniform_data.rays_params2 =
                Vec4::new(blend, center_x_norm, center_y_norm, 1.0); // w=1.0 → enabled
            material.uniform_data.rays_threshold_color = animated.rays_threshold_color;
            material.uniform_data.rays_fill_color = animated.rays_fill_color;
        } else {
            material.uniform_data.rays_params1 = Vec4::ZERO;
            material.uniform_data.rays_params2 = Vec4::ZERO;
            material.uniform_data.rays_threshold_color = Vec4::ZERO;
            material.uniform_data.rays_fill_color = Vec4::ZERO;
        }

        // Update solidcolor effect
        let sc_alpha_val =
            interpolate_float(&animated.solid_color_alpha, layer_time).unwrap_or(0.0);
        if sc_alpha_val > 0.0 {
            let sc_color =
                interpolate_color(&animated.solid_color, layer_time).unwrap_or(Vec4::ZERO);
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

        // For content_only=false, compute mesh expansion so content extends beyond
        // original layer boundary (matching AM's screen-space stretch behavior).
        // We compute the bounding box of the inverse stretch transform of [0,1]²
        // to determine how much larger the mesh needs to be and what UV range to use.
        let (s2_expand_x, s2_expand_y, s2_uv_min_x, s2_uv_min_y) =
            if has_stretch2 && !animated.stretch2_content_only && (s2_scale - 1.0).abs() > 0.001 {
                let cos_a = s2_angle_rad.cos();
                let sin_a = s2_angle_rad.sin();
                let corners = [(-0.5_f32, -0.5_f32), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)];
                let (mut min_x, mut min_y, mut max_x, mut max_y) =
                    (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
                for (cx, cy) in corners {
                    // rotate into stretch-axis space
                    let rx = cx * cos_a - cy * sin_a;
                    let ry = cx * sin_a + cy * cos_a;
                    // apply inverse of 1/scale → multiply by scale
                    let ux = rx * s2_scale;
                    let uy = ry;
                    // rotate back
                    let mx = ux * cos_a + uy * sin_a;
                    let my = -ux * sin_a + uy * cos_a;
                    min_x = min_x.min(mx + 0.5);
                    min_y = min_y.min(my + 0.5);
                    max_x = max_x.max(mx + 0.5);
                    max_y = max_y.max(my + 0.5);
                }
                (max_x - min_x, max_y - min_y, min_x, min_y)
            } else {
                (1.0, 1.0, 0.0, 0.0)
            };

        // Update blur parameters if needed
        let has_blur =
            animated.blur_strength.value.is_some() || !animated.blur_strength.keyframes.is_empty();
        if has_blur {
            let blur_strength =
                interpolate_float(&animated.blur_strength, layer_time).unwrap_or(0.0);
            if blur_strength > 0.001 {
                material.set_blur_enabled(true);
                // AM strength 2.0 produces very strong blur
                // Testing shows AM blur is much stronger than expected
                // Use strength * 80 for closer match to AM's blur intensity
                let blur_radius_px = blur_strength * 80.0;

                // Expand mesh to allow blur overflow (circular glow effect)
                // The blur samples beyond the texture boundary, so mesh needs to be larger
                // AM's blur glow extends significantly - use 2x radius for full coverage
                let blur_expansion = blur_radius_px * 2.0;

                // Pass blur parameters to shader
                // blur_params.x = blur radius in pixels
                // blur_params.y = original width (for UV calculations)
                // blur_params.z = original height (for UV calculations)
                // blur_params.w = blur expansion in pixels
                material.uniform_data.blur_params =
                    Vec4::new(blur_radius_px, orig_width, orig_height, blur_expansion);

                // Update mesh bounds for blur overflow
                // Create new mesh with expanded bounds (similar to stretch segment approach)
                let half_w = orig_width / 2.0;
                let half_h = orig_height / 2.0;

                // Vertices expand outward by blur_expansion
                let min_x = -half_w - blur_expansion;
                let max_x = half_w + blur_expansion;
                let min_y = -half_h - blur_expansion;
                let max_y = half_h + blur_expansion;

                // Calculate UV coordinates that extend beyond 0-1 for blur sampling
                // The shader will treat out-of-bounds samples as transparent
                let uv_expand_x = blur_expansion / orig_width;
                let uv_expand_y = blur_expansion / orig_height;

                let vertices = vec![
                    [min_x, min_y, 0.0],
                    [max_x, min_y, 0.0],
                    [max_x, max_y, 0.0],
                    [min_x, max_y, 0.0],
                ];
                let normals = vec![
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0],
                ];
                // UV coords extend beyond [0,1] to sample the expanded blur area
                let uvs = vec![
                    [-uv_expand_x, 1.0 + uv_expand_y],      // bottom-left
                    [1.0 + uv_expand_x, 1.0 + uv_expand_y], // bottom-right
                    [1.0 + uv_expand_x, -uv_expand_y],      // top-right
                    [-uv_expand_x, -uv_expand_y],           // top-left
                ];
                let indices = vec![0u32, 1, 2, 0, 2, 3];

                let mut new_mesh = Mesh::new(
                    bevy::mesh::PrimitiveTopology::TriangleList,
                    bevy::asset::RenderAssetUsages::RENDER_WORLD
                        | bevy::asset::RenderAssetUsages::MAIN_WORLD,
                );
                new_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
                new_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                new_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                new_mesh.insert_indices(bevy::mesh::Indices::U32(indices));

                let new_mesh_handle = meshes.add(new_mesh);
                commands
                    .entity(entity)
                    .insert(bevy::mesh::Mesh2d(new_mesh_handle));
            } else {
                material.set_blur_enabled(false);
                // Reset mesh to original bounds when blur is disabled
                // This ensures no leftover expansion from previous frames
            }
        } else {
            material.set_blur_enabled(false);
        }

        // Update stretch segment parameters if needed
        if has_stretch {
            material.set_stretch_enabled(true);

            let angle_deg = interpolate_float(&animated.stretch_angle, layer_time).unwrap_or(0.0);
            // Pass original AM angle to shader (NOT rotation-compensated).
            // The shader rotates local coords to screen space using transform_rotation,
            // applies the AM stretch formula, then rotates back. This correctly handles
            // the anisotropic scene-norm space (scene_width != scene_height).
            let angle_rad = angle_deg.to_radians();
            let stretch_raw =
                interpolate_float(&animated.stretch_amount, layer_time).unwrap_or(0.0);
            let offset_raw = interpolate_float(&animated.stretch_offset, layer_time).unwrap_or(0.0);
            let smooth_raw = interpolate_float(&animated.stretch_smooth, layer_time).unwrap_or(0.0);

            // AM stretch-segment formula (from stretchsegment.xml):
            //   adjStretch = stretch / 500.0  (scene-normalized coords)
            //   offset_norm = offset / 1000.0  (scene-normalized coords)
            //   smooth is passed raw (0..1)
            // The shader converts pixel coords to scene-norm, applies AM's formula, converts back.
            let scene_width = animated.canvas_width;
            let scene_height = animated.canvas_height;
            let adj_stretch = stretch_raw / 500.0;
            let offset_norm = offset_raw / 1000.0;

            // Compute second stretch-segment params if present
            let (adj_stretch2, angle_rad2, offset_norm2, smooth_raw2) = if has_stretch_seg2 {
                let a2_deg =
                    interpolate_float(&animated.stretch_seg2_angle, layer_time).unwrap_or(0.0);
                let a2_rad = a2_deg.to_radians();
                let s2_raw =
                    interpolate_float(&animated.stretch_seg2_amount, layer_time).unwrap_or(0.0);
                let o2_raw =
                    interpolate_float(&animated.stretch_seg2_offset, layer_time).unwrap_or(0.0);
                let sm2_raw =
                    interpolate_float(&animated.stretch_seg2_smooth, layer_time).unwrap_or(0.0);
                (s2_raw / 500.0, a2_rad, o2_raw / 1000.0, sm2_raw)
            } else {
                (0.0, 0.0, 0.0, 0.0)
            };

            // Mesh bounds: compute max displacement in SCREEN space using original AM angle,
            // then rotate back to LOCAL space for mesh expansion.
            // This correctly handles the anisotropic scene-norm space for rotated sprites.
            let cos_a1 = angle_rad.cos().abs();
            let sin_a1 = angle_rad.sin().abs();
            let dx1_screen = cos_a1 * adj_stretch * scene_width;
            let dy1_screen = sin_a1 * adj_stretch * scene_height;
            let (dx2_screen, dy2_screen) = if has_stretch_seg2 {
                (
                    angle_rad2.cos().abs() * adj_stretch2 * scene_width,
                    angle_rad2.sin().abs() * adj_stretch2 * scene_height,
                )
            } else {
                (0.0, 0.0)
            };
            let total_dx_screen = dx1_screen + dx2_screen;
            let total_dy_screen = dy1_screen + dy2_screen;

            // Rotate screen-space displacement bounding box back to local space
            let rot_cos = transform_rotation_rad.cos().abs();
            let rot_sin = transform_rotation_rad.sin().abs();
            let total_dx = rot_cos * total_dx_screen + rot_sin * total_dy_screen;
            let total_dy = rot_sin * total_dx_screen + rot_cos * total_dy_screen;

            let new_width = orig_width + 2.0 * total_dx;
            let new_height = orig_height + 2.0 * total_dy;

            // Mesh vertex bounds (centered, expanded)
            let half_nw = new_width / 2.0;
            let half_nh = new_height / 2.0;
            let min_x = -half_nw;
            let max_x = half_nw;
            let min_y = -half_nh;
            let max_y = half_nh;

            // Debug: log stretch calculation details
            if stretch_raw > 0.1 {
                trace!(
                    "[Stretch] layer_id={} scene=({:.0},{:.0}) adj_stretch={:.4} new_sz=({:.1},{:.1})",
                    animated.layer_id,
                    scene_width,
                    scene_height,
                    adj_stretch,
                    new_width,
                    new_height,
                );
            }

            // Pass raw AM params to shader: (angle, adjStretch, offset_norm, smooth)
            // mesh_offset.x carries transform_rotation for screen-space conversion
            material.uniform_data.stretch_params =
                Vec4::new(angle_rad, adj_stretch, offset_norm, smooth_raw);
            material.uniform_data.original_size =
                Vec4::new(orig_width, orig_height, new_width, new_height);
            material.uniform_data.mesh_offset =
                Vec4::new(transform_rotation_rad, 0.0, scene_width, scene_height);

            // Update second stretch-segment material parameters
            if has_stretch_seg2 {
                material.uniform_data.stretch_seg2_params =
                    Vec4::new(angle_rad2, adj_stretch2, offset_norm2, smooth_raw2);
            } else {
                material.uniform_data.stretch_seg2_params = Vec4::ZERO;
            }

            // Create new mesh with expanded bounds
            let vertices = vec![
                [min_x, min_y, 0.0],
                [max_x, min_y, 0.0],
                [max_x, max_y, 0.0],
                [min_x, max_y, 0.0],
            ];
            let normals = vec![
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
            ];
            let uvs = vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
            let indices = vec![0u32, 1, 2, 0, 2, 3];

            let mut new_mesh = Mesh::new(
                bevy::mesh::PrimitiveTopology::TriangleList,
                bevy::asset::RenderAssetUsages::RENDER_WORLD
                    | bevy::asset::RenderAssetUsages::MAIN_WORLD,
            );
            new_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
            new_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
            new_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
            new_mesh.insert_indices(bevy::mesh::Indices::U32(indices));

            let new_mesh_handle = meshes.add(new_mesh);
            commands
                .entity(entity)
                .insert(bevy::mesh::Mesh2d(new_mesh_handle));
        } else {
            material.set_stretch_enabled(false);
        }

        // For effect sprites without blur/stretch, still need to update mesh size
        // when scale/size animation changes. This ensures content scales correctly.
        // This applies to BOTH regular content AND embed content.
        // Bounds clipping (if needed) is handled separately by apply_embed_bounds_clipping_system.
        if !has_stretch && !has_blur {
            // Apply stretch2 mesh expansion for content_only=false.
            // The mesh grows so content extends beyond original layer boundary,
            // matching AM's screen-space sampling. UV range is expanded so the
            // shader's UV stretch maps the full content onto the larger mesh.
            let half_w = orig_width / 2.0 * s2_expand_x;
            let half_h = orig_height / 2.0 * s2_expand_y;

            // Get original size (before scale_assist) from the animated size property
            let orig_size = interpolate_vec2(&animated.size, 0.0).unwrap_or([100.0, 100.0]);
            let orig_w = orig_size[0].abs().max(1.0);
            let orig_h = orig_size[1].abs().max(1.0);

            // anchor = anchor_offset / orig_size (approximately)
            let anchor_x = if orig_w > 0.0 {
                animated.anchor_offset.x / orig_w
            } else {
                0.0
            };
            let anchor_y = if orig_h > 0.0 {
                animated.anchor_offset.y / orig_h
            } else {
                0.0
            };

            // mesh offset = -anchor * current_size (based on content dimensions, not expanded mesh)
            let offset_x = -anchor_x * orig_width;
            let offset_y = -anchor_y * orig_height;

            // Pixelate expansion: edge blocks extend half a cell beyond content area
            let pix_expansion = if has_pixelate {
                let size = interpolate_float(&animated.pixelate_size, layer_time).unwrap_or(1.0);
                let stretch =
                    interpolate_vec2(&animated.pixelate_stretch, layer_time).unwrap_or([1.0, 1.0]);
                size * stretch[0].abs().max(stretch[1].abs()) / 2.0
            } else {
                0.0
            };

            // Wavewarp2 expansion: wave displacement can push content beyond original bounds
            let warp_expansion = if animated.wavewarp2_has_effect {
                let m2 = interpolate_float(&animated.wavewarp2_m2, layer_time)
                    .unwrap_or(0.0)
                    .abs();
                // AM applies displacement in acScreenNorm but computes in acLayerNorm,
                // causing magnification by (canvas_size / layer_display_size).
                // We replicate this: expansion = m2/100 * magnification * content_size
                let mag = animated.canvas_height / orig_height.max(1.0);
                m2 / 100.0 * mag * orig_width.max(orig_height)
            } else {
                0.0
            };
            // Mirror offset pushes reflected content beyond layer bounds
            let mirror_expansion = if animated.mirror_has_effect {
                let offset = interpolate_float(&animated.mirror_offset, layer_time)
                    .unwrap_or(0.0)
                    .abs();
                offset * orig_width.max(orig_height)
            } else {
                0.0
            };
            let total_expansion = pix_expansion + warp_expansion + mirror_expansion;

            let vertices = vec![
                [
                    offset_x - half_w - total_expansion,
                    offset_y - half_h - total_expansion,
                    0.0,
                ],
                [
                    offset_x + half_w + total_expansion,
                    offset_y - half_h - total_expansion,
                    0.0,
                ],
                [
                    offset_x + half_w + total_expansion,
                    offset_y + half_h + total_expansion,
                    0.0,
                ],
                [
                    offset_x - half_w - total_expansion,
                    offset_y + half_h + total_expansion,
                    0.0,
                ],
            ];
            let normals = vec![
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
            ];
            // UV range: expanded by stretch2 for content_only=false, plus pixelate/wavewarp margin
            let uv_exp_x = total_expansion / orig_width;
            let uv_exp_y = total_expansion / orig_height;
            let uvs = vec![
                [s2_uv_min_x - uv_exp_x, (1.0 - s2_uv_min_y) + uv_exp_y],
                [
                    (s2_uv_min_x + s2_expand_x) + uv_exp_x,
                    (1.0 - s2_uv_min_y) + uv_exp_y,
                ],
                [
                    (s2_uv_min_x + s2_expand_x) + uv_exp_x,
                    (1.0 - s2_uv_min_y) - s2_expand_y - uv_exp_y,
                ],
                [
                    s2_uv_min_x - uv_exp_x,
                    (1.0 - s2_uv_min_y) - s2_expand_y - uv_exp_y,
                ],
            ];
            let indices = vec![0u32, 1, 2, 0, 2, 3];

            let mut new_mesh = Mesh::new(
                bevy::mesh::PrimitiveTopology::TriangleList,
                bevy::asset::RenderAssetUsages::RENDER_WORLD
                    | bevy::asset::RenderAssetUsages::MAIN_WORLD,
            );
            new_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
            new_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
            new_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
            new_mesh.insert_indices(bevy::mesh::Indices::U32(indices));

            let new_mesh_handle = meshes.add(new_mesh);
            commands
                .entity(entity)
                .insert(bevy::mesh::Mesh2d(new_mesh_handle));
        }

        // Update palette map alpha if present
        let has_palette =
            animated.palette_alpha.value.is_some() || !animated.palette_alpha.keyframes.is_empty();
        let palette_enabled = material.is_palette_enabled();
        if has_palette && palette_enabled {
            let palette_alpha =
                interpolate_float(&animated.palette_alpha, layer_time).unwrap_or(1.0);
            material.set_palette_alpha(palette_alpha);
        }

        // Update replace color effect if present
        let has_replace_color = animated.replace_old_color.w > 0.0;
        bevy::log::debug!(
            "[ReplaceColor Check] layer={} has_replace={} old_color={:?}",
            animated.layer_id,
            has_replace_color,
            animated.replace_old_color
        );
        if has_replace_color {
            let new_color = interpolate_color(&animated.replace_new_color, layer_time)
                .unwrap_or(animated.replace_old_color);
            let threshold =
                interpolate_float(&animated.replace_threshold, layer_time).unwrap_or(0.25);
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

            // Pass colors directly - shader will handle color space
            material.set_replace_color(
                animated.replace_old_color,
                new_color,
                threshold,
                feather,
                alpha,
                animated.replace_lock_luminance,
            );
        }

        // Update threshold effect if present
        let has_threshold = animated.threshold_value.value.is_some()
            || !animated.threshold_value.keyframes.is_empty();
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

        // Update grid effect if present
        let has_grid =
            animated.grid_spacing.value.is_some() || !animated.grid_spacing.keyframes.is_empty();
        if has_grid {
            let position =
                interpolate_vec2(&animated.grid_position, layer_time).unwrap_or([0.0, 0.0]);
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

        // Update pixelate effect if present
        if has_pixelate {
            let size = interpolate_float(&animated.pixelate_size, layer_time).unwrap_or(1.0);
            let stretch =
                interpolate_vec2(&animated.pixelate_stretch, layer_time).unwrap_or([1.0, 1.0]);
            let angle = interpolate_float(&animated.pixelate_angle, layer_time).unwrap_or(0.0);
            let vignette =
                interpolate_float(&animated.pixelate_vignette, layer_time).unwrap_or(0.0);
            let threshold =
                interpolate_float(&animated.pixelate_threshold, layer_time).unwrap_or(0.5);
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

            // Store scene_scale for potential future use; currently unused in shader
            // as pixelation uses inner-scene-space coordinates.
            let origin = global_transform.translation();
            let local_x_world = global_transform.transform_point(Vec3::X) - origin;
            let local_y_world = global_transform.transform_point(Vec3::Y) - origin;
            let scene_scale_x = local_x_world.length() / root_scale;
            let scene_scale_y = local_y_world.length() / root_scale;
            material.uniform_data.pixelate_flags.z = scene_scale_x;
            material.uniform_data.pixelate_flags.w = scene_scale_y;

            // Compute parent rotation for grid angle compensation.
            let local_x_world = global_transform.transform_point(Vec3::X) - origin;
            let scene_rotation = local_x_world.y.atan2(local_x_world.x);
            material.uniform_data.pixelate_params2.w = scene_rotation;
        }

        // Process repeat and linear repeat effects (delegated to repeat module)
        super::repeat::process_repeat_effect(
            animated,
            layer_time,
            material,
            orig_width,
            orig_height,
            entity,
            &mut meshes,
            &mut commands,
        );

        super::repeat::process_linear_repeat_effect(
            animated,
            layer_time,
            material,
            orig_width,
            orig_height,
            entity,
            &mut meshes,
            &mut commands,
        );

        super::repeat::process_radial_repeat_effect(
            animated,
            layer_time,
            material,
            orig_width,
            orig_height,
            entity,
            &mut meshes,
            &mut commands,
        );
    }
}

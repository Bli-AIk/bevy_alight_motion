//! Main unified effect animation system.
//!
//! Handles wipe, blur, stretch, opacity, palette, replace color, threshold,
//! grid, and pixelate effects. Repeat and linear repeat processing is
//! delegated to the `repeat` submodule.

use bevy::prelude::*;

use crate::animation::components::{AmAnimated, AmPlayback, DEBUG_NEGATIVE_HEIGHT_SCALE};
use crate::animation::interpolation::{interpolate_color, interpolate_float, interpolate_vec2};

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
#[allow(clippy::type_complexity)]
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
        Option<&super::repeat::RepeatMeshBounds>,
    )>,
    parent_animated_query: Query<(&AmAnimated, Option<&ChildOf>)>,
    effect_marker_query: Query<(), With<crate::masked_sprite::UnifiedEffectMarker>>,
    root_query: Query<&Transform, With<crate::scene::AmProjectRoot>>,
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
        material_handle,
        transform,
        global_transform,
        _mesh2d,
        embed_marker,
        repeat_bounds,
    ) in query.iter()
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
            material.uniform_data.color.w = opacity * animated.base_alpha;
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
            || !animated.stretch_smooth.keyframes.is_empty();

        let has_pixelate =
            animated.pixelate_size.value.is_some() || !animated.pixelate_size.keyframes.is_empty();

        if let Some(material) = materials.get_mut(&material_handle.0) {
            // Always set original_size so pixelate (and other effects) have correct display dimensions
            material.uniform_data.original_size =
                Vec4::new(orig_width, orig_height, orig_width, orig_height);

            // Update wipe parameters if needed
            if has_wipe {
                material.set_wipe_enabled(true);
                let wipe_start = interpolate_float(&animated.wipe_start, layer_time).unwrap_or(0.0);
                let wipe_end = interpolate_float(&animated.wipe_end, layer_time).unwrap_or(1.0);
                let wipe_angle = interpolate_float(&animated.wipe_angle, layer_time).unwrap_or(0.0);
                let wipe_feather =
                    interpolate_float(&animated.wipe_feather, layer_time).unwrap_or(0.0);
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

            // Update solidcolor effect
            let sc_alpha_val =
                interpolate_float(&animated.solid_color_alpha, layer_time).unwrap_or(0.0);
            if sc_alpha_val > 0.0 {
                let sc_color =
                    interpolate_color(&animated.solid_color, layer_time).unwrap_or(Vec4::ZERO);
                // Convert sRGB to linear for shader (colors from AM are sRGB)
                fn srgb_to_linear(c: f32) -> f32 {
                    if c <= 0.04045 {
                        c / 12.92
                    } else {
                        ((c + 0.055) / 1.055).powf(2.4)
                    }
                }
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
            let (s2_expand_x, s2_expand_y, s2_uv_min_x, s2_uv_min_y) = if has_stretch2
                && !animated.stretch2_content_only
                && (s2_scale - 1.0).abs() > 0.001
            {
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
            let has_blur = animated.blur_strength.value.is_some()
                || !animated.blur_strength.keyframes.is_empty();
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

                let angle_deg =
                    interpolate_float(&animated.stretch_angle, layer_time).unwrap_or(0.0);
                // Compensate for transform rotation: subtract transform rotation from effect angle
                // This ensures the stretch effect is applied in world space, not local space
                // Note: transform rotation is already negated in animate_transform_system (for Bevy's coord system)
                // So we add it back here to get the original AM rotation value
                let angle_rad = angle_deg.to_radians() + transform_rotation_rad;
                let stretch_px =
                    interpolate_float(&animated.stretch_amount, layer_time).unwrap_or(0.0);
                let offset_px =
                    interpolate_float(&animated.stretch_offset, layer_time).unwrap_or(0.0);
                let smooth = interpolate_float(&animated.stretch_smooth, layer_time).unwrap_or(0.0);
                let smooth_width = smooth * 0.3;

                // Calculate mesh expansion for stretch segment effect
                //
                // The base_size determines how much stretch_px translates to actual pixel stretch.
                // Through black-box testing, we found that the formula depends on the aspect ratio:
                //
                // - For wide shapes (width >= height): use orig_width directly
                // - For tall shapes (width < height): use weighted formula with rotation
                //
                // Special case: when size.y is negative (AM uses this for certain flip/transform
                // operations), the stretch calculation needs to use the diagonal length instead.
                let has_negative_size_y = sprite_size[1] < 0.0;

                // Debug: log raw values for negative height embed content
                if has_negative_size_y && embed_marker.is_some() {
                    info!(
                        "[StretchDebug] layer_id={} sprite_size=({:.2},{:.2}) scale=({:.2},{:.2}) orig=({:.2},{:.2})",
                        animated.layer_id,
                        sprite_size[0],
                        sprite_size[1],
                        scale[0],
                        scale[1],
                        orig_width,
                        orig_height
                    );
                }

                let base_size = if has_negative_size_y {
                    // For negative height, use diagonal length as base, with optional scale factor
                    (orig_width * orig_width + orig_height * orig_height).sqrt()
                        * DEBUG_NEGATIVE_HEIGHT_SCALE
                } else if orig_width >= orig_height {
                    // Wide shape: use original width
                    orig_width
                } else {
                    // Tall shape: use weighted formula with rotation
                    let rot_cos = transform_rotation_rad.cos().abs();
                    let rot_sin = transform_rotation_rad.sin().abs();
                    let world_w = orig_width * rot_cos + orig_height * rot_sin;
                    0.8 * world_w + 0.2 * orig_width
                };
                let base_divisor = base_size / 4.27; // Best match for reference
                let stretch_factor = 1.0 + stretch_px / base_divisor;

                let mut actual_stretch_px = orig_width * stretch_factor - orig_width;

                // Hack: Compensate for RTT stretch issue in groups
                // The issue causes grouped elements to appear shorter/less stretched than expected
                // This seems related to the ratio between RTT canvas height and the standard 960.0 height
                if embed_marker.is_some() {
                    let ratio = animated.canvas_height / 960.0;
                    actual_stretch_px *= ratio;
                }

                let angle_factor = 1.0 - 0.1 * angle_rad.sin().abs();
                let half_gap = actual_stretch_px * 0.5 * angle_factor;

                let rotate = |x: f32, y: f32, angle: f32| -> (f32, f32) {
                    let c = angle.cos();
                    let s = angle.sin();
                    (x * c - y * s, x * s + y * c)
                };

                let transform_vertex = |vx: f32, vy: f32| -> (f32, f32) {
                    let (rx, ry) = rotate(vx, vy, angle_rad);
                    let shifted_x = rx + offset_px;
                    let pushed_x = rx + shifted_x.signum() * half_gap;
                    rotate(pushed_x, ry, -angle_rad)
                };

                let hw = orig_width / 2.0;
                let hh = orig_height / 2.0;
                let corners = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)];

                let mut min_x = f32::MAX;
                let mut max_x = f32::MIN;
                let mut min_y = f32::MAX;
                let mut max_y = f32::MIN;

                for (cx, cy) in corners {
                    let (tx, ty) = transform_vertex(cx, cy);
                    min_x = min_x.min(tx);
                    max_x = max_x.max(tx);
                    min_y = min_y.min(ty);
                    max_y = max_y.max(ty);
                }

                // No padding - the calculated bounds should be exact for stretch effect
                // Padding would cause sample_uv to go outside [0,1] range

                let new_width = max_x - min_x;
                let new_height = max_y - min_y;
                let center_offset_x = (min_x + max_x) / 2.0;
                let center_offset_y = (min_y + max_y) / 2.0;

                // Debug: log stretch calculation details (trace level)
                if stretch_px > 0.1 {
                    let is_embed_content = animated.embed_offset != Vec2::ZERO;
                    trace!(
                        "[Stretch] layer_id={} is_embed={} canvas=({:.0},{:.0}) stretch_px={:.1} actual={:.1} new_h={:.1} neg_h={} base_size={:.1}",
                        animated.layer_id,
                        is_embed_content,
                        animated.canvas_width,
                        animated.canvas_height,
                        stretch_px,
                        actual_stretch_px,
                        new_height,
                        has_negative_size_y,
                        base_size
                    );
                }

                // Update material parameters
                material.uniform_data.stretch_params =
                    Vec4::new(angle_rad, actual_stretch_px, offset_px, smooth_width);
                material.uniform_data.original_size =
                    Vec4::new(orig_width, orig_height, new_width, new_height);
                material.uniform_data.mesh_offset =
                    Vec4::new(center_offset_x, center_offset_y, 0.0, 0.0);

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

                // For effect sprites without blur/stretch, still need to update mesh size
                // when scale/size animation changes. This ensures content scales correctly.
                // This applies to BOTH regular content AND embed content.
                // Bounds clipping (if needed) is handled separately by apply_embed_bounds_clipping_system.
                if !has_blur {
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
                        let size =
                            interpolate_float(&animated.pixelate_size, layer_time).unwrap_or(1.0);
                        let stretch = interpolate_vec2(&animated.pixelate_stretch, layer_time)
                            .unwrap_or([1.0, 1.0]);
                        size * stretch[0].abs().max(stretch[1].abs()) / 2.0
                    } else {
                        0.0
                    };

                    let vertices = vec![
                        [
                            offset_x - half_w - pix_expansion,
                            offset_y - half_h - pix_expansion,
                            0.0,
                        ],
                        [
                            offset_x + half_w + pix_expansion,
                            offset_y - half_h - pix_expansion,
                            0.0,
                        ],
                        [
                            offset_x + half_w + pix_expansion,
                            offset_y + half_h + pix_expansion,
                            0.0,
                        ],
                        [
                            offset_x - half_w - pix_expansion,
                            offset_y + half_h + pix_expansion,
                            0.0,
                        ],
                    ];
                    let normals = vec![
                        [0.0, 0.0, 1.0],
                        [0.0, 0.0, 1.0],
                        [0.0, 0.0, 1.0],
                        [0.0, 0.0, 1.0],
                    ];
                    // UV range: expanded by stretch2 for content_only=false, plus pixelate margin
                    let uv_exp_x = pix_expansion / orig_width;
                    let uv_exp_y = pix_expansion / orig_height;
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
            }

            // Update palette map alpha if present
            let has_palette = animated.palette_alpha.value.is_some()
                || !animated.palette_alpha.keyframes.is_empty();
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
                let feather =
                    interpolate_float(&animated.replace_feather, layer_time).unwrap_or(0.25);
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
                let threshold =
                    interpolate_float(&animated.threshold_value, layer_time).unwrap_or(0.5);
                let feather =
                    interpolate_float(&animated.threshold_feather, layer_time).unwrap_or(0.0);

                material.set_threshold(
                    true,
                    threshold,
                    feather,
                    animated.threshold_invert,
                    animated.threshold_blend_mode,
                );
            }

            // Update grid effect if present
            let has_grid = animated.grid_spacing.value.is_some()
                || !animated.grid_spacing.keyframes.is_empty();
            if has_grid {
                let position =
                    interpolate_vec2(&animated.grid_position, layer_time).unwrap_or([0.0, 0.0]);
                let spacing = interpolate_float(&animated.grid_spacing, layer_time).unwrap_or(0.1);
                let width = interpolate_float(&animated.grid_width, layer_time).unwrap_or(0.02);
                let smoothing =
                    interpolate_float(&animated.grid_smoothing, layer_time).unwrap_or(0.0);
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

                // Compute AM-scene parent scale (excluding FitWindow root scale).
                let origin = global_transform.translation();
                let local_x_world = global_transform.transform_point(Vec3::X) - origin;
                let local_y_world = global_transform.transform_point(Vec3::Y) - origin;
                let scene_scale_x = local_x_world.length() / root_scale;
                let scene_scale_y = local_y_world.length() / root_scale;
                let scene_rotation = local_x_world.y.atan2(local_x_world.x);
                debug!(
                    "[Pixelate] layer={} orig_size=({:.1},{:.1}) sprite_size=({:.1},{:.1}) scale=({:.4},{:.4}) rot={:.4}",
                    animated.layer_id,
                    orig_width,
                    orig_height,
                    sprite_size[0],
                    sprite_size[1],
                    scale[0],
                    scale[1],
                    scene_rotation
                );
                material.uniform_data.pixelate_flags.z = scene_scale_x;
                material.uniform_data.pixelate_flags.w = scene_scale_y;
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
                repeat_bounds,
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
                repeat_bounds,
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
                repeat_bounds,
            );
        }
    }
}

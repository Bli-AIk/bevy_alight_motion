//! Repeat, linear repeat, and radial repeat effect processing helpers.

use bevy::prelude::*;

use crate::animation::components::AmAnimated;
use crate::animation::interpolation::{interpolate_color, interpolate_float, interpolate_vec2};

/// Compute Java Random initial state from AM seed value.
/// Returns (state_hi_16bits, state_lo_32bits) packed as f32 via bitcast.
/// Uses f64 precision for seed formula to match Java's double arithmetic.
fn compute_java_random_state_packed(seed: f32) -> (f32, f32) {
    let seed_f64 = seed as f64;
    let am_seed = (15234322.0_f64 + 35432882176.0_f64 * seed_f64) as i64;
    let multiplier: i64 = 0x5DEECE66D;
    let init_state = ((am_seed ^ multiplier) as u64) & ((1u64 << 48) - 1);
    let state_hi = ((init_state >> 32) & 0xFFFF) as u32;
    let state_lo = (init_state & 0xFFFFFFFF) as u32;
    (f32::from_bits(state_lo), f32::from_bits(state_hi))
}

/// Process the repeat effect for an entity, updating material params and mesh bounds.
pub(super) fn process_repeat_effect(
    animated: &AmAnimated,
    layer_time: f32,
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    orig_width: f32,
    orig_height: f32,
    entity: Entity,
    meshes: &mut Assets<Mesh>,
    commands: &mut Commands,
) {
    let has_repeat = animated.repeat_count.value.is_some_and(|v| v > 0.0)
        || animated
            .repeat_count
            .keyframes
            .iter()
            .any(|kf| kf.value.parse::<f32>().unwrap_or(0.0) > 0.0);
    if has_repeat {
        let count = interpolate_float(&animated.repeat_count, layer_time).unwrap_or(0.0);
        let offset = interpolate_vec2(&animated.repeat_offset, layer_time).unwrap_or([0.0, 0.0]);
        let angle = interpolate_float(&animated.repeat_angle, layer_time).unwrap_or(0.0);
        let repeat_scale = interpolate_float(&animated.repeat_scale, layer_time).unwrap_or(1.0);
        let alpha = interpolate_float(&animated.repeat_alpha, layer_time).unwrap_or(1.0);

        bevy::log::debug!(
            "[RepeatEffect] layer={} time={:.2} count={:.1} offset=({:.1},{:.1}) angle={:.1} scale={:.2} alpha={:.2}",
            animated.layer_id,
            layer_time,
            count,
            offset[0],
            offset[1],
            angle,
            repeat_scale,
            alpha
        );

        material.uniform_data.repeat_params1 = Vec4::new(count, offset[0], offset[1], angle);
        material.uniform_data.repeat_params2 = Vec4::new(repeat_scale, alpha, 0.0, 0.0);

        // Calculate mesh expansion needed to show all copies
        // Each copy is offset by (offset_x * i, offset_y * i) and scaled by scale^i
        // We need to find the bounding box of all copies
        // AM's count means total copies, so we iterate 0 to count-1
        // Use floor to match shader's i32(count) truncation behavior
        let n = (count.floor() as i32 - 1).max(0);
        let angle_rad = angle.to_radians();

        // Calculate the total offset for each corner of each copy
        // For simplicity, calculate a conservative bounding box
        let mut min_x = -orig_width / 2.0;
        let mut max_x = orig_width / 2.0;
        let mut min_y = -orig_height / 2.0;
        let mut max_y = orig_height / 2.0;

        for i in 0..=n {
            let fi = i as f32;
            // AM linear alpha: 1.0 - i*(1-alpha), skip when <= 0
            let cum_alpha = 1.0 - fi * (1.0 - alpha);
            if cum_alpha <= 0.0 {
                break;
            }
            let cum_offset_x = offset[0] * fi;
            let cum_offset_y = -offset[1] * fi; // Y flipped for Bevy
            let cum_scale = repeat_scale.powf(fi);
            let cum_angle = angle_rad * fi;

            // Calculate the four corners of this copy
            let half_w = orig_width / 2.0 * cum_scale;
            let half_h = orig_height / 2.0 * cum_scale;

            // Corners in local space (before rotation)
            let corners = [
                (-half_w, -half_h),
                (half_w, -half_h),
                (half_w, half_h),
                (-half_w, half_h),
            ];

            // Apply rotation and offset to each corner
            let cos_a = cum_angle.cos();
            let sin_a = cum_angle.sin();
            for (cx, cy) in corners {
                let rx = cx * cos_a - cy * sin_a + cum_offset_x;
                let ry = cx * sin_a + cy * cos_a + cum_offset_y;
                min_x = min_x.min(rx);
                max_x = max_x.max(rx);
                min_y = min_y.min(ry);
                max_y = max_y.max(ry);
            }
        }

        // Add some padding for safety
        let padding = 10.0;
        min_x -= padding;
        max_x += padding;
        min_y -= padding;
        max_y += padding;

        bevy::log::debug!(
            "[RepeatEffect] mesh bounds: ({:.1},{:.1}) to ({:.1},{:.1})",
            min_x,
            min_y,
            max_x,
            max_y
        );

        // Calculate UV coordinates matching the standard mesh Y-flip convention:
        // UV.x = world_x / orig_width + 0.5
        // UV.y = 0.5 - world_y / orig_height  (Y-inverted: bottom vertex → large UV.y)
        let uv_min_x = min_x / orig_width + 0.5;
        let uv_max_x = max_x / orig_width + 0.5;
        let uv_at_bottom = 0.5 - min_y / orig_height; // min_y is negative → UV > 0.5
        let uv_at_top = 0.5 - max_y / orig_height; // max_y is positive → UV < 0.5

        // Update mesh bounds
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
        // UV.y is flipped to match standard mesh convention (texture Y-down)
        let uvs = vec![
            [uv_min_x, uv_at_bottom], // bottom-left
            [uv_max_x, uv_at_bottom], // bottom-right
            [uv_max_x, uv_at_top],    // top-right
            [uv_min_x, uv_at_top],    // top-left
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
        // Reset repeat params when effect is disabled
        material.uniform_data.repeat_params1 = Vec4::ZERO;
        material.uniform_data.repeat_params2 = Vec4::new(1.0, 1.0, 0.0, 0.0);
    }
}

/// Process the linear repeat effect for an entity, updating material params and mesh bounds.
pub(super) fn process_linear_repeat_effect(
    animated: &AmAnimated,
    layer_time: f32,
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    orig_width: f32,
    orig_height: f32,
    entity: Entity,
    meshes: &mut Assets<Mesh>,
    commands: &mut Commands,
) {
    let has_linear_repeat = animated.linear_repeat_count.value.is_some_and(|v| v > 0.0)
        || animated
            .linear_repeat_count
            .keyframes
            .iter()
            .any(|kf| kf.value.parse::<f32>().unwrap_or(0.0) > 0.0);
    if has_linear_repeat {
        let count = interpolate_float(&animated.linear_repeat_count, layer_time).unwrap_or(0.0);
        let position =
            interpolate_vec2(&animated.linear_repeat_position, layer_time).unwrap_or([0.0, 0.0]);
        let offset =
            interpolate_vec2(&animated.linear_repeat_offset, layer_time).unwrap_or([0.0, 0.0]);
        let angle = interpolate_float(&animated.linear_repeat_angle, layer_time).unwrap_or(0.0);
        let scale = interpolate_float(&animated.linear_repeat_scale, layer_time).unwrap_or(1.0);
        let alpha = interpolate_float(&animated.linear_repeat_alpha, layer_time).unwrap_or(1.0);
        let fill_color_srgb = interpolate_color(&animated.linear_repeat_fill_color, layer_time)
            .unwrap_or(Vec4::new(1.0, 1.0, 1.0, 1.0));
        // Convert sRGB to Linear to match uniforms.color
        let fill_color = Vec4::new(
            fill_color_srgb.x.powf(2.2),
            fill_color_srgb.y.powf(2.2),
            fill_color_srgb.z.powf(2.2),
            fill_color_srgb.w, // alpha stays the same
        );
        let blend = interpolate_float(&animated.linear_repeat_blend, layer_time).unwrap_or(0.0);
        let start = interpolate_float(&animated.linear_repeat_start, layer_time).unwrap_or(0.0);
        let end = interpolate_float(&animated.linear_repeat_end, layer_time).unwrap_or(1.0);
        let phase = interpolate_float(&animated.linear_repeat_phase, layer_time).unwrap_or(0.0);
        let ease_in = interpolate_float(&animated.linear_repeat_ease_in, layer_time).unwrap_or(0.0);
        let ease_out =
            interpolate_float(&animated.linear_repeat_ease_out, layer_time).unwrap_or(0.0);
        let overlap = interpolate_float(&animated.linear_repeat_overlap, layer_time).unwrap_or(0.0);

        // Pack shape, invert, and color_alt_copies into a single int
        let shape_invert_alt = animated.linear_repeat_shape * 100
            + if animated.linear_repeat_invert { 10 } else { 0 }
            + if animated.linear_repeat_color_alt_copies {
                1
            } else {
                0
            };

        // Use round for count to get integer copy counts
        let count_rounded = count.round();

        // Debug: log count calculation for specific time ranges
        // Frame 39 corresponds to layer_time around 0.062-0.065
        if layer_time > 0.060 && layer_time < 0.070 {
            bevy::log::info!(
                "[LinearRepeat DEBUG] layer={} layer_time={:.6} count_raw={:.4} count_rounded={:.0}",
                animated.layer_id,
                layer_time,
                count,
                count_rounded
            );
        }

        bevy::log::debug!(
            "[LinearRepeat] layer={} time={:.2} count={:.1} position=({:.1},{:.1}) offset=({:.1},{:.1}) angle={:.1} scale={:.2} alpha={:.2}",
            animated.layer_id,
            layer_time,
            count,
            position[0],
            position[1],
            offset[0],
            offset[1],
            angle,
            scale,
            alpha
        );

        // Debug for frame 42 region (layer_time ~0.068)
        if layer_time > 0.065 && layer_time < 0.075 {
            bevy::log::info!(
                "[LinearRepeat FRAME42 DEBUG] layer={} layer_time={:.4} count={:.1} position=({:.1},{:.1})",
                animated.layer_id,
                layer_time,
                count,
                position[0],
                position[1]
            );
        }

        // Debug for frame 454 region (layer_time ~0.728)
        if layer_time > 0.725 && layer_time < 0.735 {
            bevy::log::info!(
                "[LinearRepeat FRAME454 DEBUG] layer={} layer_time={:.4} count={:.0} position=({:.1},{:.1}) offset=({:.1},{:.1}) phase={:.2} easeIn={:.2} easeOut={:.2}",
                animated.layer_id,
                layer_time,
                count,
                position[0],
                position[1],
                offset[0],
                offset[1],
                phase,
                ease_in,
                ease_out
            );
        }

        material.uniform_data.linear_repeat_params1 =
            Vec4::new(count_rounded, position[0], position[1], angle);
        material.uniform_data.linear_repeat_params2 = Vec4::new(offset[0], offset[1], scale, alpha);
        material.uniform_data.linear_repeat_params3 = Vec4::new(start, end, phase, overlap);
        material.uniform_data.linear_repeat_params4 =
            Vec4::new(ease_in, ease_out, blend, shape_invert_alt as f32);
        material.uniform_data.linear_repeat_params5 = if animated.linear_repeat_random_order {
            let seed = interpolate_float(&animated.linear_repeat_seed, layer_time).unwrap_or(0.0);
            let (state_lo_bits, state_hi_bits) = compute_java_random_state_packed(seed);
            Vec4::new(1.0, state_lo_bits, state_hi_bits, 0.0)
        } else {
            Vec4::new(0.0, 0.0, 0.0, 0.0)
        };
        material.uniform_data.linear_repeat_fill_color = fill_color;

        // Process second linear repeat effect if present
        let (has_lr2, count2_rounded, position2, offset2, angle2, scale2) = if let Some(ref lr2) =
            animated.linear_repeat2
        {
            let c2 = interpolate_float(&lr2.count, layer_time).unwrap_or(0.0);
            let p2 = interpolate_vec2(&lr2.position, layer_time).unwrap_or([0.0, 0.0]);
            let o2 = interpolate_vec2(&lr2.offset, layer_time).unwrap_or([0.0, 0.0]);
            let a2 = interpolate_float(&lr2.angle, layer_time).unwrap_or(0.0);
            let s2 = interpolate_float(&lr2.scale, layer_time).unwrap_or(1.0);
            let al2 = interpolate_float(&lr2.alpha, layer_time).unwrap_or(1.0);
            let fc2_srgb = interpolate_color(&lr2.fill_color, layer_time)
                .unwrap_or(Vec4::new(1.0, 1.0, 1.0, 1.0));
            let fc2 = Vec4::new(
                fc2_srgb.x.powf(2.2),
                fc2_srgb.y.powf(2.2),
                fc2_srgb.z.powf(2.2),
                fc2_srgb.w,
            );
            let bl2 = interpolate_float(&lr2.blend, layer_time).unwrap_or(0.0);
            let st2 = interpolate_float(&lr2.start, layer_time).unwrap_or(0.0);
            let en2 = interpolate_float(&lr2.end, layer_time).unwrap_or(1.0);
            let ph2 = interpolate_float(&lr2.phase, layer_time).unwrap_or(0.0);
            let ei2 = interpolate_float(&lr2.ease_in, layer_time).unwrap_or(0.0);
            let eo2 = interpolate_float(&lr2.ease_out, layer_time).unwrap_or(0.0);
            let ov2 = interpolate_float(&lr2.overlap, layer_time).unwrap_or(0.0);
            let sia2 = lr2.shape * 100
                + if lr2.invert { 10 } else { 0 }
                + if lr2.color_alt_copies { 1 } else { 0 };
            let c2r = c2.round();

            material.uniform_data.linear_repeat2_params1 = Vec4::new(c2r, p2[0], p2[1], a2);
            material.uniform_data.linear_repeat2_params2 = Vec4::new(o2[0], o2[1], s2, al2);
            material.uniform_data.linear_repeat2_params3 = Vec4::new(st2, en2, ph2, ov2);
            material.uniform_data.linear_repeat2_params4 = Vec4::new(ei2, eo2, bl2, sia2 as f32);
            material.uniform_data.linear_repeat2_params5 = if lr2.random_order {
                let lr2_seed = interpolate_float(&lr2.seed, layer_time).unwrap_or(0.0);
                let (state_lo_bits, state_hi_bits) = compute_java_random_state_packed(lr2_seed);
                Vec4::new(1.0, state_lo_bits, state_hi_bits, 0.0)
            } else {
                Vec4::new(0.0, 0.0, 0.0, 0.0)
            };
            material.uniform_data.linear_repeat2_fill_color = fc2;
            (true, c2r, p2, o2, a2, s2)
        } else {
            material.uniform_data.linear_repeat2_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
            material.uniform_data.linear_repeat2_params2 = Vec4::new(0.0, 0.0, 1.0, 1.0);
            material.uniform_data.linear_repeat2_params3 = Vec4::new(0.0, 1.0, 0.0, 0.0);
            material.uniform_data.linear_repeat2_params4 = Vec4::ZERO;
            material.uniform_data.linear_repeat2_params5 = Vec4::ZERO;
            material.uniform_data.linear_repeat2_fill_color = Vec4::new(1.0, 1.0, 1.0, 1.0);
            (false, 0.0, [0.0, 0.0], [0.0, 0.0], 0.0, 1.0)
        };

        // Calculate mesh expansion using AM's repeatWithEasing algorithm
        // This must match the shader's calculation exactly
        let n = count_rounded as i32;
        let angle_rad = angle.to_radians();

        let mut min_x = -orig_width / 2.0;
        let mut max_x = orig_width / 2.0;
        let mut min_y = -orig_height / 2.0;
        let mut max_y = orig_height / 2.0;

        // Compute bounding box for effect 1 copies
        // When dual effects exist, this will be further expanded by effect 2
        let interp_progress = 1.0; // Use max for bounding box

        // Helper: compute displacement for a single effect's copy
        let compute_displacement =
            |idx: i32, count: i32, pos: [f32; 2], off: [f32; 2]| -> (f32, f32) {
                let base = if count > 1 {
                    idx as f32 / (count as f32 - 1.0)
                } else {
                    0.0
                };
                (
                    pos[0] * base + off[0] * interp_progress,
                    pos[1] * base + off[1] * interp_progress,
                )
            };

        // Effect 2 iteration (or just 1 iteration if no effect 2)
        let n2 = if has_lr2 { count2_rounded as i32 } else { 1 };
        let angle2_rad = angle2.to_radians();

        for j in 0..n2 {
            // Effect 2 displacement (0,0 if no effect 2)
            let (d2x, d2y) = if has_lr2 {
                compute_displacement(j, n2, position2, offset2)
            } else {
                (0.0, 0.0)
            };
            let cum_scale2 = if has_lr2 {
                1.0 + (scale2 - 1.0) * interp_progress
            } else {
                1.0
            };
            let cum_angle2 = if has_lr2 {
                angle2_rad * interp_progress
            } else {
                0.0
            };

            for i in 0..n {
                let (d1x, d1y) = compute_displacement(i, n, position, offset);
                let cum_scale1 = 1.0 + (scale - 1.0) * interp_progress;
                let cum_angle1 = angle_rad * interp_progress;

                // Combined transform: effect2(effect1(shape))
                // In world space: displacement = d2 + rotate2(scale2 * d1)
                let scaled_d1x = d1x * cum_scale2;
                let scaled_d1y = d1y * cum_scale2;
                let (rot_d1x, rot_d1y) = if cum_angle2.abs() > 0.001 {
                    let c = cum_angle2.cos();
                    let s = cum_angle2.sin();
                    (
                        scaled_d1x * c - scaled_d1y * s,
                        scaled_d1x * s + scaled_d1y * c,
                    )
                } else {
                    (scaled_d1x, scaled_d1y)
                };
                let total_dx = d2x + rot_d1x;
                let total_dy = d2y + rot_d1y;

                // Convert to Bevy coords (flip Y)
                let cum_offset_x = total_dx;
                let cum_offset_y = -total_dy;

                let total_scale = cum_scale1 * cum_scale2;
                let total_angle = cum_angle1 + cum_angle2;

                let half_w = orig_width / 2.0 * total_scale;
                let half_h = orig_height / 2.0 * total_scale;
                let corners = [
                    (-half_w, -half_h),
                    (half_w, -half_h),
                    (half_w, half_h),
                    (-half_w, half_h),
                ];
                let cos_a = total_angle.cos();
                let sin_a = total_angle.sin();
                for (cx, cy) in corners {
                    let rx = cx * cos_a - cy * sin_a + cum_offset_x;
                    let ry = cx * sin_a + cy * cos_a + cum_offset_y;
                    min_x = min_x.min(rx);
                    max_x = max_x.max(rx);
                    min_y = min_y.min(ry);
                    max_y = max_y.max(ry);
                }
            }
        }

        // Add padding for safety - larger padding to handle edge cases
        // Also scale padding by the maximum possible scale factor
        let max_scale = scale.abs().max(1.0) * (if has_lr2 { scale2.abs().max(1.0) } else { 1.0 });
        let padding = 20.0 * max_scale
            + offset[0].abs()
            + offset[1].abs()
            + if has_lr2 {
                offset2[0].abs() + offset2[1].abs()
            } else {
                0.0
            };
        min_x -= padding;
        max_x += padding;
        min_y -= padding;
        max_y += padding;

        // Calculate new mesh dimensions
        let new_width = max_x - min_x;
        let new_height = max_y - min_y;

        // Update original_size uniform so shader knows the original shape dimensions
        // x,y = original shape size (for pixel coordinate calculation)
        // z,w = expanded mesh size (for reference)
        material.uniform_data.original_size =
            Vec4::new(orig_width, orig_height, new_width, new_height);

        // UV mapping: match standard mesh Y-flip convention
        let uv_min_x = min_x / orig_width + 0.5;
        let uv_max_x = max_x / orig_width + 0.5;
        let uv_at_bottom = 0.5 - min_y / orig_height;
        let uv_at_top = 0.5 - max_y / orig_height;

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
        let uvs = vec![
            [uv_min_x, uv_at_bottom],
            [uv_max_x, uv_at_bottom],
            [uv_max_x, uv_at_top],
            [uv_min_x, uv_at_top],
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
        // Reset linear repeat params when effect is disabled
        // Use count=-1.0 to indicate "not activated" (distinguishes from count=0 which means "activated but hide")
        material.uniform_data.linear_repeat_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
        material.uniform_data.linear_repeat_params2 = Vec4::new(0.0, 0.0, 1.0, 1.0);
        material.uniform_data.linear_repeat_params3 = Vec4::new(0.0, 1.0, 0.0, 0.0);
        material.uniform_data.linear_repeat_params4 = Vec4::ZERO;
        material.uniform_data.linear_repeat_params5 = Vec4::ZERO;
        material.uniform_data.linear_repeat_fill_color = Vec4::new(1.0, 1.0, 1.0, 1.0);
        material.uniform_data.linear_repeat2_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
        material.uniform_data.linear_repeat2_params2 = Vec4::new(0.0, 0.0, 1.0, 1.0);
        material.uniform_data.linear_repeat2_params3 = Vec4::new(0.0, 1.0, 0.0, 0.0);
        material.uniform_data.linear_repeat2_params4 = Vec4::ZERO;
        material.uniform_data.linear_repeat2_params5 = Vec4::ZERO;
        material.uniform_data.linear_repeat2_fill_color = Vec4::new(1.0, 1.0, 1.0, 1.0);
    }
}

/// Process the radial repeat effect for an entity, updating material params and mesh bounds.
pub(super) fn process_radial_repeat_effect(
    animated: &AmAnimated,
    layer_time: f32,
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    orig_width: f32,
    orig_height: f32,
    entity: Entity,
    meshes: &mut Assets<Mesh>,
    commands: &mut Commands,
) {
    let has_radial_repeat = animated.radial_repeat_count.value.is_some_and(|v| v > 0.0)
        || animated
            .radial_repeat_count
            .keyframes
            .iter()
            .any(|kf| kf.value.parse::<f32>().unwrap_or(0.0) > 0.0);
    if has_radial_repeat {
        let count = interpolate_float(&animated.radial_repeat_count, layer_time).unwrap_or(0.0);
        let radius = interpolate_float(&animated.radial_repeat_radius, layer_time).unwrap_or(0.0);
        let orientation =
            interpolate_float(&animated.radial_repeat_orientation, layer_time).unwrap_or(0.0);
        let start_angle =
            interpolate_float(&animated.radial_repeat_start_angle, layer_time).unwrap_or(0.0);
        let sweep = interpolate_float(&animated.radial_repeat_sweep, layer_time).unwrap_or(360.0);
        let base_scale =
            interpolate_float(&animated.radial_repeat_base_scale, layer_time).unwrap_or(1.0);
        let offset =
            interpolate_vec2(&animated.radial_repeat_offset, layer_time).unwrap_or([0.0, 0.0]);
        let angle = interpolate_float(&animated.radial_repeat_angle, layer_time).unwrap_or(0.0);
        let scale = interpolate_float(&animated.radial_repeat_scale, layer_time).unwrap_or(1.0);
        let alpha = interpolate_float(&animated.radial_repeat_alpha, layer_time).unwrap_or(1.0);
        let fill_color_srgb = interpolate_color(&animated.radial_repeat_fill_color, layer_time)
            .unwrap_or(Vec4::new(1.0, 1.0, 1.0, 1.0));
        let fill_color = Vec4::new(
            fill_color_srgb.x.powf(2.2),
            fill_color_srgb.y.powf(2.2),
            fill_color_srgb.z.powf(2.2),
            fill_color_srgb.w,
        );
        let blend = interpolate_float(&animated.radial_repeat_blend, layer_time).unwrap_or(0.0);
        let start = interpolate_float(&animated.radial_repeat_start, layer_time).unwrap_or(0.0);
        let end = interpolate_float(&animated.radial_repeat_end, layer_time).unwrap_or(1.0);
        let phase = interpolate_float(&animated.radial_repeat_phase, layer_time).unwrap_or(0.0);
        let ease_in = interpolate_float(&animated.radial_repeat_ease_in, layer_time).unwrap_or(0.0);
        let ease_out =
            interpolate_float(&animated.radial_repeat_ease_out, layer_time).unwrap_or(0.0);
        let overlap = interpolate_float(&animated.radial_repeat_overlap, layer_time).unwrap_or(0.0);

        let shape_invert_alt = animated.radial_repeat_shape * 100
            + if animated.radial_repeat_invert { 10 } else { 0 }
            + if animated.radial_repeat_color_alt_copies {
                1
            } else {
                0
            };

        // Pack into uniforms
        // Use -1 as sentinel: tells shader "effect is present, but 0 copies" (render nothing)
        // Pass raw (unrounded) count — AM uses raw count in position formula, rounded only for loop
        let count_for_shader = if count.round() <= 0.0 { -1.0 } else { count };

        material.uniform_data.radial_repeat_params1 =
            Vec4::new(count_for_shader, radius, orientation, start_angle);
        material.uniform_data.radial_repeat_params2 = Vec4::new(sweep, base_scale, angle, scale);
        material.uniform_data.radial_repeat_params3 = Vec4::new(alpha, offset[0], offset[1], blend);
        material.uniform_data.radial_repeat_params4 = Vec4::new(start, end, phase, overlap);
        material.uniform_data.radial_repeat_params5 = Vec4::new(
            ease_in,
            ease_out,
            shape_invert_alt as f32,
            if animated.radial_repeat_random_order {
                animated.radial_repeat_seed + 0.5
            } else {
                animated.radial_repeat_seed
            },
        );
        material.uniform_data.radial_repeat_fill_color = fill_color;

        // Mesh expansion: use conservative bounds covering ALL possible copy positions.
        // Copies sit on a circle of the given radius, so maximum extent is:
        //   radius + half_element * max_scale + offset
        let max_mix = scale.abs().max(1.0);
        let visual_scale = (max_mix * base_scale).abs().max(1.0);
        let max_extent = radius.abs() * max_mix
            + (orig_width.max(orig_height)) / 2.0 * visual_scale
            + offset[0].abs()
            + offset[1].abs();
        let mut min_x = -max_extent;
        let mut max_x = max_extent;
        let mut min_y = -max_extent;
        let mut max_y = max_extent;

        let padding = 20.0;
        min_x -= padding;
        max_x += padding;
        min_y -= padding;
        max_y += padding;

        let new_width = max_x - min_x;
        let new_height = max_y - min_y;

        material.uniform_data.original_size =
            Vec4::new(orig_width, orig_height, new_width, new_height);

        let uv_min_x = min_x / orig_width + 0.5;
        let uv_max_x = max_x / orig_width + 0.5;
        let uv_at_bottom = 0.5 - min_y / orig_height;
        let uv_at_top = 0.5 - max_y / orig_height;

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
        let uvs = vec![
            [uv_min_x, uv_at_bottom],
            [uv_max_x, uv_at_bottom],
            [uv_max_x, uv_at_top],
            [uv_min_x, uv_at_top],
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
        material.uniform_data.radial_repeat_params1 = Vec4::ZERO;
        material.uniform_data.radial_repeat_params2 = Vec4::new(360.0, 1.0, 0.0, 1.0);
        material.uniform_data.radial_repeat_params3 = Vec4::new(1.0, 0.0, 0.0, 0.0);
        material.uniform_data.radial_repeat_params4 = Vec4::new(0.0, 1.0, 0.0, 0.0);
        material.uniform_data.radial_repeat_params5 = Vec4::ZERO;
        material.uniform_data.radial_repeat_fill_color = Vec4::new(1.0, 1.0, 1.0, 1.0);
    }
}

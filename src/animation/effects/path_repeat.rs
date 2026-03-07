//! Path Repeat effect animation system.
//! Places copies along the outline of a source element's shape.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use crate::animation::components::{AmAnimated, AmPathRepeat, AmPlayback};
use crate::animation::interpolation::{
    interpolate_color, interpolate_float, interpolate_vec2, interpolate_vec3,
};
use crate::plugin::AmWhitePixel;
use crate::scene::effects::PathRepeatParams;

/// Compute the repeat-with-easing distribution for path repeat.
/// Returns a Vec of (progress, ease_factor) for each copy.
fn repeat_with_easing(count: i32, params: &PathRepeatParams, layer_time: f32) -> Vec<(f32, f32)> {
    if count <= 0 {
        return Vec::new();
    }

    let start = interpolate_float(&params.start, layer_time).unwrap_or(0.0);
    let end_val = interpolate_float(&params.end, layer_time).unwrap_or(1.0);
    let phase = interpolate_float(&params.phase, layer_time).unwrap_or(0.0);
    let ease_in = interpolate_float(&params.ease_in, layer_time).unwrap_or(0.0);
    let ease_out = interpolate_float(&params.ease_out, layer_time).unwrap_or(0.0);
    let overlap_raw = interpolate_float(&params.overlap, layer_time).unwrap_or(0.0);
    let overlap = overlap_raw + 1.0;
    let shape = params.shape;
    let invert = params.invert;

    let count_f = count as f32;
    let denom = 2.0 * overlap + count_f - 1.0;
    let step = 1.0 / denom;

    // Build index list (potentially shuffled for random order)
    let indices: Vec<i32> = if params.random_order {
        // Match Java's f32 arithmetic: (long)(15234322 + 35432882176L * seedValue)
        let seed = (15234322.0_f32 + 35432882176.0_f32 * params.seed) as u64;
        let mut idx_list: Vec<i32> = (0..count).collect();
        // Simple Fisher-Yates shuffle with deterministic seed
        let mut rng = seed;
        for i in (1..idx_list.len()).rev() {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let j = (rng >> 33) as usize % (i + 1);
            idx_list.swap(i, j);
        }
        idx_list
    } else {
        (0..count).collect()
    };

    let mut results = Vec::with_capacity(count as usize);

    for i in 0..count {
        let idx = indices[i as usize];
        let center = ((idx as f32 + overlap) / denom) + phase;
        let width = step * overlap;
        let mid = center + width / 2.0;

        // Compute ease factor (f7) based on shape type
        let mut ease = match shape {
            1 => {
                // Square shape
                let coerce_in1 = ((center - start) / width).clamp(0.0, 1.0);
                let coerce_in2 = ((end_val - center) / width).clamp(0.0, 1.0);
                if start < end_val {
                    coerce_in1.min(coerce_in2)
                } else {
                    1.0 - coerce_in1.max(coerce_in2)
                }
            }
            2 => {
                // Smooth (gaussian-like)
                let range = end_val - start;
                if mid >= start && mid <= end_val && range > 0.0 {
                    let normalized = ((mid - start) / range - 0.5) * 2.0 * std::f32::consts::PI;
                    std::f64::consts::E.powf(-(normalized as f64 * normalized as f64) / 2.0) as f32
                } else {
                    0.0
                }
            }
            3 => {
                // Triangle
                let range = end_val - start;
                if mid >= start && mid <= end_val && range > 0.0 {
                    let t = (mid - start) / range;
                    2.0 * t.min(1.0 - t)
                } else {
                    0.0
                }
            }
            _ => {
                // Ramp (default, shape=0)
                let range = (end_val - start).max(0.0).max(f32::EPSILON);
                (mid - start) / range
            }
        };

        // Apply cubic bezier easing if ease_in or ease_out are non-zero
        if ease_in != 0.0 || ease_out != 0.0 {
            ease = cubic_bezier_ease(ease, ease_in, ease_out);
        }

        if invert {
            ease = 1.0 - ease;
        }

        ease = ease.clamp(0.0, 1.0);

        // Progress for positioning
        let progress = if count <= 1 {
            0.0
        } else {
            i as f32 / (count_f - 1.0)
        };

        results.push((progress, ease));
    }

    results
}

/// Simple cubic bezier easing approximation.
fn cubic_bezier_ease(t: f32, ease_in: f32, ease_out: f32) -> f32 {
    let _p1x = (ease_in / 2.0).max(0.0);
    let p1y = ((-ease_in) / 2.0).max(0.0);
    let _p2x = 1.0 - (ease_out / 2.0).max(0.0);
    let p2y = 1.0 - ((-ease_out) / 2.0).max(0.0);

    // Simple approximation: evaluate bezier Y at parameter t
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    let t2 = t * t;
    let t3 = t2 * t;
    mt3 * 0.0 + 3.0 * mt2 * t * p1y + 3.0 * mt * t2 * p2y + t3 * 1.0
}

/// Hide all copy entities by setting their visibility to Hidden.
fn hide_copies(
    copy_entities: &[Entity],
    copy_query: &mut Query<(&mut Transform, &mut Visibility, &mut Sprite), Without<AmPathRepeat>>,
) {
    for &copy_e in copy_entities {
        let Ok((_, mut vis, _)) = copy_query.get_mut(copy_e) else {
            continue;
        };
        *vis = Visibility::Hidden;
    }
}

/// Sample a position and tangent along a rectangle's perimeter.
/// The rectangle is defined by half-extents (half_w, half_h).
/// Distance is measured along the perimeter (total = 2*(w+h)).
/// Returns (position_x, position_y, tangent_x, tangent_y) in local coords.
fn sample_rect_path(half_w: f32, half_h: f32, distance: f32) -> (f32, f32, f32, f32) {
    let w = half_w * 2.0;
    let h = half_h * 2.0;
    let perimeter = 2.0 * (w + h);
    if perimeter < f32::EPSILON {
        return (0.0, 0.0, 1.0, 0.0);
    }
    // Normalize distance to [0, perimeter)
    let d = ((distance % perimeter) + perimeter) % perimeter;

    // AM rect path order: top-left → top-right → bottom-right → bottom-left → top-left
    // In AM coords (Y-down): TL(-hw,-hh), TR(hw,-hh), BR(hw,hh), BL(-hw,hh)
    if d < w {
        // Top edge: left to right
        let t = d / w;
        (-half_w + t * w, -half_h, 1.0, 0.0)
    } else if d < w + h {
        // Right edge: top to bottom
        let t = (d - w) / h;
        (half_w, -half_h + t * h, 0.0, 1.0)
    } else if d < 2.0 * w + h {
        // Bottom edge: right to left
        let t = (d - w - h) / w;
        (half_w - t * w, half_h, -1.0, 0.0)
    } else {
        // Left edge: bottom to top
        let t = (d - 2.0 * w - h) / h;
        (-half_w, half_h - t * h, 0.0, -1.0)
    }
}

/// Path repeat animation system.
/// For each entity with AmPathRepeat, compute copy positions along the source shape's outline.
pub fn animate_path_repeat_system(
    mut commands: Commands,
    playback: Res<AmPlayback>,
    white_pixel: Option<Res<AmWhitePixel>>,
    mut path_repeat_query: Query<(
        Entity,
        &mut AmPathRepeat,
        &AmAnimated,
        &Transform,
        &mut Visibility,
        Option<&ChildOf>,
    )>,
    mut copy_query: Query<(&mut Transform, &mut Visibility, &mut Sprite), Without<AmPathRepeat>>,
) {
    let white_pixel_handle = match &white_pixel {
        Some(wp) => wp.0.clone(),
        None => return,
    };
    let current_time = playback.current_time_ms;

    for (_entity, mut path_repeat, animated, transform, mut visibility, child_of) in
        path_repeat_query.iter_mut()
    {
        let path_params = match &animated.path_repeat {
            Some(p) => p,
            None => continue,
        };

        let local_time = animated.calc_local_time(current_time);
        let layer_time = animated.calc_layer_time(local_time);
        let is_active = animated.is_active(local_time);

        if !is_active {
            // Hide copies when layer is inactive
            hide_copies(&path_repeat.copy_entities, &mut copy_query);
            continue;
        }

        // Use source's animated data stored in AmPathRepeat
        // (available even after source entity is despawned)
        let source_animated = &path_repeat.source_animated;

        // Get the current count
        let count_raw = interpolate_float(&path_params.count, layer_time).unwrap_or(0.0);
        let count = count_raw.round() as i32;

        if count <= 0 {
            hide_copies(&path_repeat.copy_entities, &mut copy_query);
            continue;
        }

        // Hide the original entity's visual (copies replace it)
        *visibility = Visibility::Hidden;

        // Get source shape properties from its AmAnimated data
        let source_local_time = source_animated.calc_local_time(current_time);
        let source_layer_time = source_animated.calc_layer_time(source_local_time);
        let source_size =
            interpolate_vec2(&source_animated.size, source_layer_time).unwrap_or([100.0, 100.0]);
        let source_width = source_size[0];
        let source_height = source_size[1];
        let source_half_w = source_width / 2.0;
        let source_half_h = source_height / 2.0;
        let perimeter = 2.0 * (source_width + source_height);

        if perimeter < f32::EPSILON {
            continue;
        }

        // Get source's transform properties from its AmAnimated data
        let src_loc = interpolate_vec3(&source_animated.location, source_layer_time)
            .unwrap_or([0.0, 0.0, 0.0]);
        let src_scale_vals =
            interpolate_vec2(&source_animated.scale, source_layer_time).unwrap_or([1.0, 1.0]);
        let src_rotation_deg =
            interpolate_float(&source_animated.rotation, source_layer_time).unwrap_or(0.0);

        // Convert source location from AM coords to Bevy coords
        let src_bevy_x = src_loc[0] - source_animated.canvas_width / 2.0;
        let src_bevy_y = source_animated.canvas_height / 2.0 - src_loc[1];
        let src_rot_quat = Quat::from_rotation_z(-src_rotation_deg.to_radians());

        // Get path-repeat specific params
        let start_pos = interpolate_float(&path_params.start_pos, layer_time).unwrap_or(0.0);
        let end_pos = interpolate_float(&path_params.end_pos, layer_time).unwrap_or(1.0);
        let path_phase = interpolate_float(&path_params.path_phase, layer_time).unwrap_or(0.0);
        let offset = interpolate_vec2(&path_params.offset, layer_time).unwrap_or([0.0, 0.0]);
        let angle_param = interpolate_float(&path_params.angle, layer_time).unwrap_or(0.0);
        let scale_param = interpolate_float(&path_params.scale, layer_time).unwrap_or(1.0);
        let alpha_param = interpolate_float(&path_params.alpha, layer_time).unwrap_or(1.0);
        let tangent = path_params.tangent;

        // Get current element's AM location
        let cur_loc = interpolate_vec3(&animated.location, layer_time).unwrap_or([0.0, 0.0, 0.0]);
        let cur_bevy_x = cur_loc[0] - animated.canvas_width / 2.0;
        let cur_bevy_y = animated.canvas_height / 2.0 - cur_loc[1];

        // Compute easing distribution
        let copies = repeat_with_easing(count, path_params, layer_time);

        // Compute fill color for copies
        let fill_color_srgb = interpolate_color(&path_params.fill_color, layer_time);
        let blend = interpolate_float(&path_params.blend, layer_time).unwrap_or(0.0);

        // Get the element's base fill color
        let base_color = Color::srgba(
            animated.base_fill_color[0],
            animated.base_fill_color[1],
            animated.base_fill_color[2],
            animated.base_fill_color[3],
        );

        // Ensure we have enough copy entities
        let needed = count as usize;
        while path_repeat.copy_entities.len() < needed {
            let mut copy_cmds = commands.spawn((
                Sprite {
                    image: white_pixel_handle.clone(),
                    color: base_color,
                    custom_size: Some(Vec2::new(100.0, 100.0)),
                    ..default()
                },
                Transform::default(),
                GlobalTransform::default(),
                Visibility::Hidden,
                InheritedVisibility::default(),
                ViewVisibility::default(),
                RenderLayers::layer(0),
            ));
            // Parent copies to the same parent as the target entity
            // so they inherit the project's fit_scale transform
            if let Some(parent_ref) = child_of {
                copy_cmds.insert(ChildOf(parent_ref.0));
            }
            let copy_entity = copy_cmds.id();
            path_repeat.copy_entities.push(copy_entity);
        }

        // Get current element's size and scale
        let elem_size = interpolate_vec2(&animated.size, layer_time).unwrap_or([100.0, 100.0]);
        let elem_width = elem_size[0];
        let elem_height = elem_size[1];
        let elem_scale = interpolate_vec2(&animated.scale, layer_time).unwrap_or([1.0, 1.0]);

        // Position each copy
        for (copy_idx, &(progress, ease)) in copies.iter().enumerate() {
            if copy_idx >= path_repeat.copy_entities.len() {
                break;
            }
            let copy_entity = path_repeat.copy_entities[copy_idx];

            // Compute distance along path
            // AM formula: ((startPos * L + range * L * progress) + (phase + 1000) * L) % L
            let range_len = (end_pos - start_pos) * perimeter;
            let distance =
                (start_pos * perimeter + range_len * progress) + (path_phase + 1000.0) * perimeter;

            // Sample position and tangent on source rect (AM coords, Y-down, center-origin)
            let (path_x, path_y, tan_x, tan_y) =
                sample_rect_path(source_half_w, source_half_h, distance);

            // Transform path point by source's scale and rotation (in AM coords)
            let scaled_x = path_x * src_scale_vals[0];
            let scaled_y = path_y * src_scale_vals[1];

            // Convert to Bevy coords (flip Y) then apply source rotation
            let bevy_local = Vec3::new(scaled_x, -scaled_y, 0.0);
            let rotated = src_rot_quat * bevy_local;

            // World position = source_location + rotated_path_point
            let world_x = src_bevy_x + rotated.x;
            let world_y = src_bevy_y + rotated.y;

            // Copy offset relative to current element position
            // AM formula: pathPos - currentElement.location + offset * ease
            let copy_x = world_x - cur_bevy_x + offset[0] * ease;
            let copy_y = world_y - cur_bevy_y - offset[1] * ease;

            // Per-copy scale
            let copy_scale = 1.0 + (scale_param - 1.0) * ease;
            // Per-copy alpha
            let copy_alpha = 1.0 + (alpha_param - 1.0) * ease;
            // Per-copy rotation
            let tangent_angle = if tangent {
                let bevy_tan = src_rot_quat * Vec3::new(tan_x, -tan_y, 0.0);
                -bevy_tan.x.atan2(bevy_tan.y).to_degrees()
            } else {
                0.0
            };
            let copy_rotation = tangent_angle + angle_param * ease;

            // Set copy transform - position is relative to current element,
            // but since copies are root entities, we use the current element's position as base
            let Ok((mut copy_transform, mut copy_vis, mut sprite)) =
                copy_query.get_mut(copy_entity)
            else {
                continue;
            };

            // Copies are root entities, so their Transform IS their world position
            // We position them at: current_element_bevy_pos + copy_offset
            copy_transform.translation = Vec3::new(
                cur_bevy_x + copy_x,
                cur_bevy_y + copy_y,
                transform.translation.z,
            );
            copy_transform.scale =
                Vec3::new(elem_scale[0] * copy_scale, elem_scale[1] * copy_scale, 1.0);
            copy_transform.rotation = Quat::from_rotation_z(copy_rotation.to_radians());
            sprite.custom_size = Some(Vec2::new(elem_width, elem_height));

            // Compute color with blend
            let mut color = base_color;
            if blend > 0.0
                && let Some(fc) = &fill_color_srgb
            {
                let fc_color = Color::srgba(fc.x, fc.y, fc.z, fc.w);
                let t = (blend * ease).clamp(0.0, 1.0);
                let base_linear = color.to_linear();
                let fill_linear = fc_color.to_linear();
                color = Color::LinearRgba(LinearRgba::new(
                    base_linear.red * (1.0 - t) + fill_linear.red * t,
                    base_linear.green * (1.0 - t) + fill_linear.green * t,
                    base_linear.blue * (1.0 - t) + fill_linear.blue * t,
                    1.0,
                ));
            }
            let a = copy_alpha.clamp(0.0, 1.0);
            sprite.color = color.with_alpha(a);

            *copy_vis = Visibility::Inherited;
        }

        // Hide excess copies
        for i in needed..path_repeat.copy_entities.len() {
            let copy_entity = path_repeat.copy_entities[i];
            if let Ok((_, mut vis, _)) = copy_query.get_mut(copy_entity) {
                *vis = Visibility::Hidden;
            }
        }
    }
}

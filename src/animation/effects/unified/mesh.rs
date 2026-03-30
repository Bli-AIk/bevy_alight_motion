//! Adjusts the render mesh used by unified-effect visuals.
//! When blur, stretch, or other geometry-expanding effects are active, the code
//! here grows or reshapes the quad so shader-side sampling has enough space and
//! effect edges do not get clipped.
//!
//! 负责调整统一特效视觉对象使用的渲染网格。当 blur、stretch 或其他会
//! 扩张几何边界的效果启用时，这里的逻辑会放大或重塑 quad，确保 shader 采样空间
//! 足够，特效边缘不会被提前裁掉。

use bevy::prelude::*;

use crate::animation::components::AmAnimated;
use crate::animation::interpolation::{interpolate_float, interpolate_vec2};

use super::super::unified_support::{trace_stretch_once, update_quad_mesh};

fn trace_unified_mesh_layer(layer_id: u64) -> bool {
    std::env::var_os("AM_TRACE_EFFECT_IDS")
        .and_then(|value| value.into_string().ok())
        .is_some_and(|ids| {
            ids.split(',')
                .filter_map(|value| value.trim().parse::<u64>().ok())
                .any(|id| id == layer_id)
        })
}

pub(super) fn update_blur_mesh(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    layer_time: f32,
    orig_width: f32,
    orig_height: f32,
    mesh2d: &bevy::mesh::Mesh2d,
    mesh_state: &mut crate::animation::components::AmUnifiedMeshState,
    meshes: &mut Assets<Mesh>,
) {
    let has_blur =
        animated.blur_strength.value.is_some() || !animated.blur_strength.keyframes.is_empty();
    if has_blur {
        let blur_strength = interpolate_float(&animated.blur_strength, layer_time).unwrap_or(0.0);
        if blur_strength > 0.001 {
            material.set_blur_enabled(true);
            let blur_radius_px = blur_strength * 80.0;
            let blur_expansion = blur_radius_px * 2.0;
            material.uniform_data.blur_params =
                Vec4::new(blur_radius_px, orig_width, orig_height, blur_expansion);

            let half_w = orig_width / 2.0;
            let half_h = orig_height / 2.0;
            let min_x = -half_w - blur_expansion;
            let max_x = half_w + blur_expansion;
            let min_y = -half_h - blur_expansion;
            let max_y = half_h + blur_expansion;
            let uv_expand_x = blur_expansion / orig_width;
            let uv_expand_y = blur_expansion / orig_height;

            update_quad_mesh(
                meshes,
                mesh2d,
                mesh_state,
                [min_x, max_x, min_y, max_y],
                [
                    -uv_expand_x,
                    1.0 + uv_expand_x,
                    -uv_expand_y,
                    1.0 + uv_expand_y,
                ],
            );
        } else {
            material.set_blur_enabled(false);
        }
    } else {
        material.set_blur_enabled(false);
    }
}

pub(super) fn update_stretch_mesh(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    layer_time: f32,
    has_stretch: bool,
    has_stretch_seg2: bool,
    transform_rotation_rad: f32,
    sprite_size: [f32; 2],
    scale: [f32; 2],
    ancestor_scale: [f32; 2],
    orig_width: f32,
    orig_height: f32,
    layer_scale: Vec2,
    global_transform: &GlobalTransform,
    mesh2d: &bevy::mesh::Mesh2d,
    mesh_state: &mut crate::animation::components::AmUnifiedMeshState,
    meshes: &mut Assets<Mesh>,
) {
    if has_stretch {
        material.set_stretch_enabled(true);

        let angle_deg = interpolate_float(&animated.stretch_angle, layer_time).unwrap_or(0.0);
        let angle_rad = angle_deg.to_radians();
        let stretch_raw = interpolate_float(&animated.stretch_amount, layer_time).unwrap_or(0.0);
        let offset_raw = interpolate_float(&animated.stretch_offset, layer_time).unwrap_or(0.0);
        let smooth_raw = interpolate_float(&animated.stretch_smooth, layer_time).unwrap_or(0.0);

        let scene_width = animated.canvas_width;
        let scene_height = animated.canvas_height;
        let adj_stretch = stretch_raw / 500.0;
        let offset_norm = offset_raw / 1000.0;

        let (adj_stretch2, angle_rad2, offset_norm2, smooth_raw2) = if has_stretch_seg2 {
            let a2_deg = interpolate_float(&animated.stretch_seg2_angle, layer_time).unwrap_or(0.0);
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

        let rot_cos = transform_rotation_rad.cos().abs();
        let rot_sin = transform_rotation_rad.sin().abs();
        let total_dx = rot_cos * total_dx_screen + rot_sin * total_dy_screen;
        let total_dy = rot_sin * total_dx_screen + rot_cos * total_dy_screen;

        let new_width = orig_width + 2.0 * total_dx;
        let new_height = orig_height + 2.0 * total_dy;

        // Convert screen-space dimensions to local-space for mesh vertices and shader.
        // For SDF layers (layer_scale=1,1): no change.
        // For non-SDF layers: mesh must be larger in local space so that
        // mesh_local * Transform.scale covers the correct screen area.
        let local_orig_w = orig_width / layer_scale.x;
        let local_orig_h = orig_height / layer_scale.y;
        let local_new_w = new_width / layer_scale.x;
        let local_new_h = new_height / layer_scale.y;

        let global_scale = global_transform.to_scale_rotation_translation().0;
        let _ = global_scale;
        trace_stretch_once(animated.layer_id, || {
            format!(
                "[STRETCH] layer_id={} parent={} canvas=({:.0},{:.0}) sprite=({:.2},{:.2}) scale=({:.4},{:.4}) ancestor=({:.4},{:.4}) global_scale=({:.4},{:.4}) orig=({:.2},{:.2}) screen_mesh=({:.2},{:.2}) local_mesh=({:.2},{:.2}) layer_scale=({:.4},{:.4}) angle={:.2} stretch={:.2}",
                animated.layer_id,
                animated.parent_layer_id,
                scene_width,
                scene_height,
                sprite_size[0],
                sprite_size[1],
                scale[0],
                scale[1],
                ancestor_scale[0],
                ancestor_scale[1],
                global_scale.x,
                global_scale.y,
                orig_width,
                orig_height,
                new_width,
                new_height,
                local_new_w,
                local_new_h,
                layer_scale.x,
                layer_scale.y,
                angle_deg,
                stretch_raw,
            )
        });

        let aa_pad: f32 = 4.0;
        let half_nw = local_new_w / 2.0 + aa_pad;
        let half_nh = local_new_h / 2.0 + aa_pad;

        if stretch_raw > 0.1 {
            bevy::log::trace!(
                "[Stretch] layer_id={} scene=({:.0},{:.0}) adj_stretch={:.4} new_sz=({:.1},{:.1})",
                animated.layer_id,
                scene_width,
                scene_height,
                adj_stretch,
                local_new_w,
                local_new_h,
            );
        }

        material.uniform_data.stretch_params =
            Vec4::new(angle_rad, adj_stretch, offset_norm, smooth_raw);
        material.uniform_data.original_size =
            Vec4::new(local_orig_w, local_orig_h, local_new_w, local_new_h);
        let stretch_sign_code =
            (if scale[0] < 0.0 { 1.0 } else { 0.0 }) + (if scale[1] < 0.0 { 2.0 } else { 0.0 });
        material.uniform_data.mesh_offset = Vec4::new(
            transform_rotation_rad,
            stretch_sign_code,
            scene_width,
            scene_height,
        );
        // Store layer_scale in solid_color_alpha.yz for the shader's local↔screen conversion
        material.uniform_data.solid_color_alpha.y = layer_scale.x;
        material.uniform_data.solid_color_alpha.z = layer_scale.y;

        if has_stretch_seg2 {
            material.uniform_data.stretch_seg2_params =
                Vec4::new(angle_rad2, adj_stretch2, offset_norm2, smooth_raw2);
        } else {
            material.uniform_data.stretch_seg2_params = Vec4::ZERO;
        }

        let u_pad = aa_pad / local_new_w;
        let v_pad = aa_pad / local_new_h;
        update_quad_mesh(
            meshes,
            mesh2d,
            mesh_state,
            [-half_nw, half_nw, -half_nh, half_nh],
            [-u_pad, 1.0 + u_pad, -v_pad, 1.0 + v_pad],
        );
    } else {
        material.set_stretch_enabled(false);
        // Default layer_scale for non-stretch layers
        material.uniform_data.solid_color_alpha.y = 1.0;
        material.uniform_data.solid_color_alpha.z = 1.0;
    }
}

pub(super) fn update_base_mesh(
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
    animated: &AmAnimated,
    layer_time: f32,
    has_stretch: bool,
    has_blur: bool,
    has_pixelate: bool,
    has_stretch2: bool,
    s2_scale: f32,
    s2_angle_rad: f32,
    orig_width: f32,
    orig_height: f32,
    mesh2d: &bevy::mesh::Mesh2d,
    mesh_state: &mut crate::animation::components::AmUnifiedMeshState,
    meshes: &mut Assets<Mesh>,
) {
    if !has_stretch && !has_blur {
        let trace_layer = trace_unified_mesh_layer(animated.layer_id);
        let (s2_uv_expand_x, s2_uv_expand_y, s2_uv_min_x, s2_uv_min_y) =
            if has_stretch2 && !animated.stretch2_content_only && (s2_scale - 1.0).abs() > 0.001 {
                let cos_a = s2_angle_rad.cos();
                let sin_a = s2_angle_rad.sin();
                let corners = [(-0.5_f32, -0.5_f32), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)];
                let (mut min_x, mut min_y, mut max_x, mut max_y) =
                    (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
                for (cx, cy) in corners {
                    let rx = cx * cos_a - cy * sin_a;
                    let ry = cx * sin_a + cy * cos_a;
                    let ux = rx * s2_scale;
                    let uy = ry;
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

        // AM's stretch2 grows the visible footprint more aggressively than the raw UV bbox.
        // Keep UV remap conservative, but give the mesh twice the overflow so late-frame bars
        // are not clipped back to the unstretched thickness.
        let s2_mesh_expand_x = 1.0 + (s2_uv_expand_x - 1.0) * 2.0;
        let s2_mesh_expand_y = 1.0 + (s2_uv_expand_y - 1.0) * 2.0;

        let half_w = orig_width / 2.0 * s2_mesh_expand_x;
        let half_h = orig_height / 2.0 * s2_mesh_expand_y;

        let orig_size = interpolate_vec2(&animated.size, 0.0).unwrap_or([100.0, 100.0]);
        let orig_w = orig_size[0].abs().max(1.0);
        let orig_h = orig_size[1].abs().max(1.0);
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
        let offset_x = -anchor_x * orig_width;
        let offset_y = -anchor_y * orig_height;

        let pix_expansion = if has_pixelate {
            let size = interpolate_float(&animated.pixelate_size, layer_time).unwrap_or(1.0);
            let stretch =
                interpolate_vec2(&animated.pixelate_stretch, layer_time).unwrap_or([1.0, 1.0]);
            size * stretch[0].abs().max(stretch[1].abs()) / 2.0
        } else {
            0.0
        };
        let warp_expansion = if animated.wavewarp2_has_effect {
            let m2 = interpolate_float(&animated.wavewarp2_m2, layer_time)
                .unwrap_or(0.0)
                .abs();
            let mag = animated.canvas_height / orig_height.max(1.0);
            m2 / 100.0 * mag * orig_width.max(orig_height)
        } else {
            0.0
        };
        let mirror_expansion = if animated.mirror_has_effect {
            let offset = interpolate_float(&animated.mirror_offset, layer_time)
                .unwrap_or(0.0)
                .abs();
            offset * orig_width.max(orig_height)
        } else {
            0.0
        };
        let rgb_split_expansion = if animated.rgb_split_enabled && !animated.lift_has_effect {
            let strength =
                interpolate_float(&animated.rgb_split_strength, layer_time).unwrap_or(0.15);
            let adj_strength = (strength / 8.0).abs();
            adj_strength * orig_width.max(orig_height)
        } else {
            0.0
        };
        let total_expansion =
            pix_expansion + warp_expansion + mirror_expansion + rgb_split_expansion;

        let (lx, rx) = (
            offset_x - half_w - total_expansion,
            offset_x + half_w + total_expansion,
        );
        let (by, ty) = (
            offset_y - half_h - total_expansion,
            offset_y + half_h + total_expansion,
        );
        let uv_exp_x = total_expansion / orig_width;
        let uv_exp_y = total_expansion / orig_height;
        let bounds = [lx, rx, by, ty];
        let mesh_width = rx - lx;
        let mesh_height = ty - by;
        let uv_rect = [
            s2_uv_min_x - uv_exp_x,
            (s2_uv_min_x + s2_uv_expand_x) + uv_exp_x,
            (1.0 - s2_uv_min_y) - s2_uv_expand_y - uv_exp_y,
            (1.0 - s2_uv_min_y) + uv_exp_y,
        ];
        if trace_layer {
            bevy::log::warn!(
                "[UnifiedMeshTrace] layer={} orig=({:.1},{:.1}) anchor_offset=({:.1},{:.1}) total_expansion={:.3} bounds={:?} uv_rect={:?}",
                animated.layer_id,
                orig_width,
                orig_height,
                animated.anchor_offset.x,
                animated.anchor_offset.y,
                total_expansion,
                bounds,
                uv_rect
            );
        }
        material.uniform_data.original_size =
            Vec4::new(orig_width, orig_height, mesh_width, mesh_height);
        update_quad_mesh(meshes, mesh2d, mesh_state, bounds, uv_rect);
    }
}

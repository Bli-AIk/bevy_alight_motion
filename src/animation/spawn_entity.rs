//! # spawn_entity.rs
//!
//! # 图层实体生成
//!
//! Spawning a complete entity from a PendingLayer with all components.
//! 从 PendingLayer 生成完整实体及所有组件。

use bevy::asset::Assets;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use std::collections::HashMap;

use crate::scene::{
    AmBlendingMode, AmElement, AmElementType, AmEntitySpawned, AmLayerMarker, AmLayerName,
    PendingLayer,
};
use crate::sdf_material::SdfMaterial;

use super::helpers::get_initial_scale_from_animated;
use super::interpolation::{
    interpolate_float, interpolate_vec2, interpolate_vec3_with_extrapolation,
};
use super::visual::add_visual_components;

/// Check if a layer is a descendant of another layer (direct or nested).
/// Spawn a complete entity from a PendingLayer.
///
/// For spatial decoupling of embed content:
/// - If `containing_embed_id != 0`, the entity is made a child of embed_contents_container
/// - But its coordinates remain in world space (relative to RTT camera at origin)
/// - The container has identity Transform so GlobalTransform equals Transform
/// - This provides organization while maintaining correct rendering
pub(super) fn spawn_layer_entity(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    unified_materials: &mut Assets<crate::masked_sprite::UnifiedEffectMaterial>,
    sdf_materials: &mut Assets<SdfMaterial>,
    layer: &PendingLayer,
    images: &HashMap<String, Handle<Image>>,
    fonts: &HashMap<String, Handle<Font>>,
    white_pixel: Option<&Handle<Image>>,
    parent_entity: Entity,
    _embed_contents_container: Option<Entity>,
    inv_fit_scale: f32,
    spawned_entities: &HashMap<u64, Entity>,
    global_time: f32,
) -> Entity {
    let entity_name = format!("Layer[{}]: {}", layer.id, layer.label);

    // Check if layer has any effects that need scale baking
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

    let has_mask = layer.mask_info.is_some();
    let has_stretch2 = layer.animated.stretch2_scale.value.is_some()
        || !layer.animated.stretch2_scale.keyframes.is_empty();
    let needs_effect = has_wipe || has_stretch || has_mask || has_blur || has_stretch2;

    // Calculate correct initial position at spawn time (to prevent frame jump)
    // Use the same logic as animate_transform_system
    let animated = &layer.animated;

    // Calculate local time for animation interpolation
    let local_time = animated.calc_local_time(global_time);

    bevy::log::trace!(
        "[SpawnTime] '{}' global_time={:.1}, local_time={:.1}, start_time={}, end_time={}, time_offset={:.1}, speed={:.2}",
        layer.label,
        global_time,
        local_time,
        layer.start_time,
        layer.end_time,
        animated.time_offset,
        animated.speed_multiplier
    );

    // Calculate normalized time within layer duration
    let layer_time = animated.calc_layer_time(local_time);

    // Get current scale for pivot compensation
    // For effect layers and SDF shapes, magnitude is baked into mesh, but we need the sign for flipping
    let actual_scale = interpolate_vec2(&animated.scale, layer_time).unwrap_or([1.0, 1.0]);
    let current_scale =
        if matches!(layer.spec, crate::scene::AmLayerSpec::SdfShape { .. }) || needs_effect {
            [1.0_f32, 1.0_f32]
        } else {
            actual_scale
        };

    // Calculate initial position using animation interpolation
    // Use extrapolation for location to improve accuracy before first keyframe
    let initial_position = if let Some(loc) =
        interpolate_vec3_with_extrapolation(&animated.location, layer_time)
    {
        let (mut bx, mut by) = if animated.has_parent {
            // For layers with parents, use local coordinates
            (loc[0], -loc[1])
        } else {
            // For root layers, convert from canvas coordinates
            (
                loc[0] - animated.canvas_width / 2.0,
                animated.canvas_height / 2.0 - loc[1],
            )
        };

        // Apply pivot compensation (simplified - full logic is in animate_transform_system)
        if let Some(pivot) = interpolate_vec2(&animated.pivot, layer_time) {
            let pivot_x = pivot[0];
            let pivot_y = pivot[1];

            if matches!(layer.spec, crate::scene::AmLayerSpec::SdfShape { .. }) {
                // SDF shapes: translation is at transform center
                bx += pivot_x;
                by -= pivot_y;
            } else if matches!(layer.spec, crate::scene::AmLayerSpec::EmbedScene) {
                // Embed scenes: need rotation-aware pivot compensation
                let rotation_deg = interpolate_float(&animated.rotation, layer_time).unwrap_or(0.0);
                let rotation_rad = (-rotation_deg).to_radians();
                let pivot_bevy_y = -pivot_y;
                let scaled_offset_x = -pivot_x * current_scale[0];
                let scaled_offset_y = -pivot_bevy_y * current_scale[1];
                let rotated_offset_x =
                    scaled_offset_x * rotation_rad.cos() - scaled_offset_y * rotation_rad.sin();
                let rotated_offset_y =
                    scaled_offset_x * rotation_rad.sin() + scaled_offset_y * rotation_rad.cos();
                bx += pivot_x + rotated_offset_x;
                by += pivot_bevy_y + rotated_offset_y;
            } else {
                // Standard shapes: pivot offset is already applied in collect_types.rs
                // via pivot_to_anchor_and_offset and transform.translation
                // No additional compensation needed here
            }
        }

        // Apply effect position offsets (transform2 effect)
        if let Some(mut effect_x) = interpolate_float(&animated.effect_pos_x, layer_time) {
            if animated.effect_xinv {
                effect_x = -effect_x;
            }
            bx += effect_x;
        }
        if let Some(mut effect_y) = interpolate_float(&animated.effect_pos_y, layer_time) {
            if animated.effect_yinv {
                effect_y = -effect_y;
            }
            by -= effect_y; // Y is inverted
        }
        // Apply extra stacked transform2 position offsets
        for extra in &animated.extra_transform2 {
            let ex = interpolate_float(&extra.pos_x, layer_time).unwrap_or(0.0);
            bx += if extra.xinv { -ex } else { ex };
            let ey = interpolate_float(&extra.pos_y, layer_time).unwrap_or(0.0);
            by -= if extra.yinv { -ey } else { ey };
        }

        // Apply font Y offset for text layers (to compensate for different font metrics)
        if !animated.has_parent {
            by -= animated.font_y_offset;
        }

        // Apply anchor offset compensation for SpriteShape with non-center pivot
        // NOTE: Skip for SDF shapes - their pivot is already handled above via `by -= pivot_y`
        if !matches!(layer.spec, crate::scene::AmLayerSpec::SdfShape { .. }) {
            bx += animated.anchor_offset.x;
            by += animated.anchor_offset.y;
        }

        Vec3::new(bx, by, layer.transform.translation.z)
    } else {
        layer.transform.translation
    };

    // Calculate initial rotation
    let initial_rotation = if let Some(rot_deg) = interpolate_float(&animated.rotation, layer_time)
    {
        Quat::from_rotation_z((-rot_deg).to_radians())
    } else {
        layer.transform.rotation
    };

    // Calculate initial scale
    let initial_scale =
        if needs_effect || matches!(layer.spec, crate::scene::AmLayerSpec::SdfShape { .. }) {
            // For effect layers and SDF shapes, keep only the sign of scale for flipping
            // The magnitude is baked into the mesh
            Vec3::new(actual_scale[0].signum(), actual_scale[1].signum(), 1.0)
        } else {
            Vec3::new(current_scale[0], current_scale[1], 1.0)
        };

    bevy::log::debug!(
        "[SpawnInit] '{}' layer_time={:.4}, pos=({:.1},{:.1},{:.4}), rot={:.2}°, scale=({:.3},{:.3})",
        layer.label,
        layer_time,
        initial_position.x,
        initial_position.y,
        initial_position.z,
        initial_rotation
            .to_euler(bevy::math::EulerRot::ZYX)
            .0
            .to_degrees(),
        initial_scale.x,
        initial_scale.y
    );

    // Create transform with calculated initial values
    let transform_to_use = Transform {
        translation: initial_position,
        rotation: initial_rotation,
        scale: initial_scale,
    };

    // Clone animated component and set inv_fit_scale for embed children
    // Use containing_embed_id to detect embed content, not embed_offset
    // (embed_offset can be ZERO when embed is at canvas center)
    let mut animated = layer.animated.clone();
    if animated.scale_assist_axis != 0 {
        bevy::log::info!(
            "[SPAWN] Layer '{}' has scale_assist_axis={}, keyframes={}",
            layer.label,
            animated.scale_assist_axis,
            animated.scale_assist.keyframes.len()
        );
    }
    if layer.containing_embed_id != 0 {
        animated.inv_fit_scale = inv_fit_scale;
    }

    // **Hybrid Rendering Pipeline**:
    // All content starts visible and renders to Layer 0 (main camera).
    // For Composite strategy embeds, content will later be reassigned to RTT layers.
    // This ensures content is always visible and eliminates the first-frame hidden issue.
    //
    // Note: embed content that WAS using containing_embed_id for spatial decoupling
    // now uses Bevy parent-child hierarchy for RenderLayers propagation.
    let initial_visibility = Visibility::Inherited;

    // Determine element type based on layer spec
    // 根据图层规格确定元素类型
    let element_type = match &layer.spec {
        crate::scene::AmLayerSpec::SpriteShape { .. } => AmElementType::Shape,
        crate::scene::AmLayerSpec::SdfShape { .. } => AmElementType::Shape,
        crate::scene::AmLayerSpec::Text { .. } => AmElementType::Text,
        crate::scene::AmLayerSpec::Image { .. } => AmElementType::Image,
        crate::scene::AmLayerSpec::Null => AmElementType::Null,
        crate::scene::AmLayerSpec::EmbedScene => AmElementType::EmbedScene,
        crate::scene::AmLayerSpec::Camera { .. } => AmElementType::Null,
    };

    // Create base entity with common components
    // Include RenderLayers::layer(0) by default - Direct strategy content stays on Layer 0
    // 创建带有通用组件的基础实体
    let entity = commands
        .spawn((
            Name::new(entity_name),
            AmLayerMarker {
                id: layer.id,
                label: layer.label.clone(),
            },
            // 2.3 标识与查询标准化 (Identification & Query Standardization)
            AmLayerName::new(layer.label.clone()),
            AmElement, // Marker for all AM-generated entities
            animated,
            layer.spec.clone(),
            transform_to_use,
            GlobalTransform::default(),
            initial_visibility,
            InheritedVisibility::default(),
            ViewVisibility::default(),
            RenderLayers::layer(0), // Default to Layer 0 (main camera)
        ))
        .id();

    // 2.2 扩展钩子系统 - 触发 AmEntitySpawned 事件
    // (Hook System - trigger AmEntitySpawned event)
    commands.trigger(AmEntitySpawned {
        entity,
        layer_name: layer.label.clone(),
        layer_id: layer.id,
        element_type,
    });

    // Add mask info component if this layer is affected by a mask
    if let Some(mask_info) = &layer.mask_info {
        commands.entity(entity).insert(mask_info.clone());
        bevy::log::debug!(
            "[Lifecycle] Layer '{}' has {} mask(s)",
            layer.label,
            mask_info.masks.len()
        );
    }

    // Add camera layer component if this is a camera layer
    if let crate::scene::AmLayerSpec::Camera { ref fov, base_z } = layer.spec {
        commands
            .entity(entity)
            .insert(crate::animation::AmCameraLayer {
                fov: fov.clone(),
                base_z,
                scene_width: layer.animated.canvas_width,
                scene_height: layer.animated.canvas_height,
            });
        bevy::log::info!(
            "[Lifecycle] Camera layer '{}' spawned (base_z={:.1})",
            layer.label,
            base_z
        );
    }

    // Add visual components based on spec (skip for mask and camera layers)
    bevy::log::debug!(
        "[spawn_layer_entity] '{}' blending_mode={:?}, checking visual spawn",
        layer.label,
        layer.blending_mode
    );
    if layer.blending_mode != AmBlendingMode::Mask
        && layer.blending_mode != AmBlendingMode::Exclude
        && !matches!(layer.spec, crate::scene::AmLayerSpec::Camera { .. })
    {
        // Extract initial scale from animated data for SDF shapes
        // (transform.scale is set to 1.0 for SDF shapes, actual scale is in animated)
        let initial_scale = get_initial_scale_from_animated(&layer.animated.scale);

        // Check if layer has wipe effect
        let has_wipe = layer.animated.wipe_end.value != Some(1.0)
            || !layer.animated.wipe_end.keyframes.is_empty()
            || layer.animated.wipe_start.value.is_some()
            || !layer.animated.wipe_start.keyframes.is_empty();

        // Check if layer has stretch segment effect
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

        // Check if layer has blur effect
        let has_blur = layer.animated.blur_strength.value.is_some()
            || !layer.animated.blur_strength.keyframes.is_empty();

        // Get initial wipe params
        let initial_wipe = if has_wipe {
            let wipe_start = layer.animated.wipe_start.value.unwrap_or(0.0);
            let wipe_end = layer.animated.wipe_end.value.unwrap_or(1.0);
            let wipe_angle = layer.animated.wipe_angle.value.unwrap_or(0.0);
            let wipe_feather = layer.animated.wipe_feather.value.unwrap_or(0.0);
            Some(Vec4::new(wipe_start, wipe_end, wipe_angle, wipe_feather))
        } else {
            None
        };

        // Get initial stretch segment params
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

        // Get initial blur params and calculate max blur for mesh expansion
        let initial_blur = if has_blur {
            let blur_strength = layer.animated.blur_strength.value.unwrap_or(0.0);
            // AM strength 2.0 produces very strong blur
            // Use strength * 80 to match animate_unified_effect_system
            let blur_radius = blur_strength * 80.0;
            Some(Vec4::new(blur_radius, 0.0, 0.0, 0.0))
        } else {
            None
        };

        // Calculate maximum blur strength from keyframes for mesh expansion
        let max_blur_radius = if has_blur {
            let max_strength = layer
                .animated
                .blur_strength
                .keyframes
                .iter()
                .filter_map(|kf| kf.value.parse::<f32>().ok())
                .fold(layer.animated.blur_strength.value.unwrap_or(0.0), f32::max);
            // Same multiplier as used in animation system
            max_strength * 80.0
        } else {
            0.0
        };

        // For embed content rendered to RTT, use original size (no scaling)
        // The final display size will be affected by embed's inherited fit_scale
        let size_scale = 1.0;

        // Calculate initial stretch mesh bounds and mesh_offset to prevent first frame jump
        // This replicates the logic from animate_unified_effect_system
        let (initial_mesh_offset, initial_stretch_mesh_bounds) = if has_stretch {
            // Use interpolation at layer_time to match animate_unified_effect_system
            let sprite_size =
                interpolate_vec2(&layer.animated.size, layer_time).unwrap_or([100.0, 100.0]);
            let scale = interpolate_vec2(&layer.animated.scale, layer_time).unwrap_or([1.0, 1.0]);
            let orig_width = (sprite_size[0] * scale[0]).abs().max(1.0);
            let orig_height = (sprite_size[1] * scale[1]).abs().max(1.0);

            // Get stretch parameters using interpolation
            let angle_deg =
                interpolate_float(&layer.animated.stretch_angle, layer_time).unwrap_or(0.0);
            let transform_rotation_rad = initial_rotation.to_euler(bevy::math::EulerRot::XYZ).2;
            // Pass original AM angle to shader (NOT rotation-compensated)
            let angle_rad = angle_deg.to_radians();
            let stretch_raw =
                interpolate_float(&layer.animated.stretch_amount, layer_time).unwrap_or(0.0);
            let offset_raw =
                interpolate_float(&layer.animated.stretch_offset, layer_time).unwrap_or(0.0);

            // AM formula: convert to scene-normalized coords
            let scene_width = layer.animated.canvas_width;
            let scene_height = layer.animated.canvas_height;
            let adj_stretch = stretch_raw / 500.0;
            let offset_norm = offset_raw / 1000.0;

            // Mesh bounds: compute displacement in screen space, rotate to local space
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

        // Get initial replace color params
        let initial_replace_color = if layer.animated.replace_old_color != Vec4::ZERO
            || layer.animated.replace_new_color.value.is_some()
            || !layer.animated.replace_new_color.keyframes.is_empty()
        {
            // Extract initial new_color value
            // If has keyframes, get color from first keyframe; otherwise use static value or default
            let new_color_srgb = if let Some(val) = layer.animated.replace_new_color.value {
                val
            } else if !layer.animated.replace_new_color.keyframes.is_empty() {
                // Parse first keyframe's color value (format: "r,g,b,a")
                let first_kf = &layer.animated.replace_new_color.keyframes[0];
                super::interpolation::parse_keyframe_color(&first_kf.value)
                    .unwrap_or(Vec4::new(1.0, 1.0, 1.0, 1.0))
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

            let flags = Vec4::new(1.0, lock_lum, 0.0, 0.0); // enabled, lock_luminance
            let params = Vec4::new(threshold, feather, alpha, 0.0);

            // Pass colors directly in sRGB - shader will convert to linear
            Some((
                flags,
                layer.animated.replace_old_color,
                new_color_srgb,
                params,
            ))
        } else {
            None
        };

        add_visual_components(
            commands,
            meshes,
            unified_materials,
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
            size_scale,
            max_blur_radius,
            initial_mesh_offset,
            initial_stretch_mesh_bounds,
            1.0 / inv_fit_scale,            // fit_scale for mask coordinates
            layer.containing_embed_id != 0, // is_embed_content - force effect material for bounds clipping
            !layer.animated.scale.keyframes.is_empty(), // has_scale_animation - needs bounds clipping
            layer.animated.scale_assist_axis != 0, // has_scale_assist - needs UnifiedEffectMaterial for dynamic sizing
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
                || !layer.animated.radial_repeat_count.keyframes.is_empty(), // has_repeat - needs UnifiedEffectMaterial
            layer.animated.threshold_value.value.is_some()
                || !layer.animated.threshold_value.keyframes.is_empty(), // has_threshold - needs UnifiedEffectMaterial
            layer.animated.grid_spacing.value.is_some()
                || !layer.animated.grid_spacing.keyframes.is_empty(), // has_grid - needs UnifiedEffectMaterial
            layer.animated.pixelate_size.value.is_some()
                || !layer.animated.pixelate_size.keyframes.is_empty(), // has_pixelate - needs UnifiedEffectMaterial
            has_stretch2, // has_stretch2 - needs UnifiedEffectMaterial
            layer.animated.solid_color_alpha.value.is_some()
                || !layer.animated.solid_color_alpha.keyframes.is_empty(), // has_solidcolor - needs UnifiedEffectMaterial
            {
                // Calculate max pixelate expansion for mesh sizing
                // Edge blocks extend up to half a grid cell beyond the content area
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
            }, // pixelate_expansion
            global_time as u64,    // current playback time for mask initialization
            initial_replace_color, // replace color params
            {
                // Compute max animated scale for SDF mesh sizing
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
                // Also account for max animated size relative to initial size
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
                // Account for border/stroke expansion in mesh sizing
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
                // Also account for border2 (static)
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
            }, // max_animated_scale
        );

        // Insert group fill component if present (for embed scenes with fillType)
        if let Some(ref fill) = layer.group_fill {
            commands.entity(entity).insert(fill.clone());
        }
    } else {
        bevy::log::trace!(
            "[Lifecycle] Skipping visual for mask layer '{}' (id={})",
            layer.label,
            layer.id
        );
    }

    // **Hybrid Rendering Pipeline**:
    // In Direct strategy, ALL content inherits transforms from their embed ancestors.
    // We make content a Bevy child of its parent (from pending.layers), NOT of embed_contents_container.
    // This allows proper Transform propagation through the hierarchy.
    //
    // Note: We still add AmEmbedContentMarker for lifecycle management,
    // but the content is parented to its actual parent (not the container).
    if layer.containing_embed_id != 0 {
        // This is embed content - make it a child of its parent entity
        // This ensures proper Transform inheritance through the Bevy hierarchy
        commands.entity(parent_entity).add_child(entity);

        // Look up the embed entity and add marker for lifecycle management
        if let Some(&embed_entity) = spawned_entities.get(&layer.containing_embed_id) {
            commands
                .entity(entity)
                .insert(crate::scene::AmEmbedContentMarker {
                    embed_entity,
                    embed_id: layer.containing_embed_id,
                });
            bevy::log::debug!(
                "[Lifecycle] Embed content '{}' parented to {:?}, belongs to embed {} ({:?})",
                layer.label,
                parent_entity,
                layer.containing_embed_id,
                embed_entity
            );
        } else {
            // Even if embed lookup fails, still parent correctly for Transform inheritance
            bevy::log::debug!(
                "[Lifecycle] Embed content '{}' parented to {:?} (embed {} not in spawned_entities)",
                layer.label,
                parent_entity,
                layer.containing_embed_id
            );
        }
    } else {
        // Regular layer - add as child of parent
        commands.entity(parent_entity).add_child(entity);
    }

    // Insert echo runtime component if present
    if let Some(ref echo_runtime) = layer.echo_runtime {
        commands.entity(entity).insert(echo_runtime.clone());
    }

    entity
}

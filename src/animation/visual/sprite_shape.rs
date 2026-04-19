//! Builds runtime visuals for sprite-shape layers.
//! 为 SpriteShape 图层构建运行时可视对象。
//!
//! Sprite-shape layers sit between plain images and true SDF shapes: they may render a fill image,
//! a solid-color fallback, unified shader effects, or transform-scale compensation. This file owns
//! that branch of the spawn pipeline so the caller can treat sprite-shape layers as one case even
//! though the final rendering setup has several sub-paths.
//! SpriteShape 图层介于普通图片和真正的 SDF 形状之间：它既可能渲染填充图片，也可能渲染纯色兜底，
//! 还可能叠加统一 shader 效果或变换缩放补偿。这个文件负责这条 spawn 分支，让上层调用方可以把
//! SpriteShape 当成一个案例处理，而底层渲染配置仍能按不同子路径展开。

use bevy::asset::Assets;
use bevy::prelude::*;
use std::collections::HashMap;

use crate::effects::TextureSourceContract;
use crate::scene::{AmMaskInfo, AmPaletteMapParams, AmVisualSpawned};

use super::super::components::AmUnifiedUsesTransformScale;
use super::super::visual_helpers::{create_stretch_bounds_mesh, extract_fill_color};
use super::material::create_unified_material;
use super::mesh::create_anchored_rectangle_with_blur;

fn force_plain_sprite_for_label(label: &str) -> bool {
    std::env::var_os("AM_FORCE_PLAIN_SPRITE_LABELS")
        .and_then(|value| value.into_string().ok())
        .is_some_and(|labels| labels.split(',').any(|value| value.trim() == label))
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

#[expect(clippy::too_many_arguments)] // reason: sprite-shape visuals require effect/resource fan-in
pub(super) fn handle_sprite_shape_visual(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    unified_materials: &mut Assets<crate::masked_sprite::UnifiedEffectMaterial>,
    entity: Entity,
    layer_id: u64,
    image_uri: &str,
    is_media: bool,
    fill_color: &Option<crate::schema::AmFillColor>,
    width: f32,
    height: f32,
    anchor: &bevy::sprite::Anchor,
    mask_info: &Option<AmMaskInfo>,
    palette_params: Option<&AmPaletteMapParams>,
    images: &HashMap<String, Handle<Image>>,
    white_pixel: Option<&Handle<Image>>,
    label: &str,
    initial_scale: (f32, f32),
    wipe_params: Option<Vec4>,
    stretch_params: Option<Vec4>,
    blur_params: Option<Vec4>,
    size_scale: f32,
    initial_mesh_offset: Option<Vec4>,
    initial_stretch_mesh_bounds: Option<(f32, f32, f32, f32)>,
    fit_scale: f32,
    global_time_ms: u64,
    replace_color_params: Option<(Vec4, Vec4, Vec4, Vec4)>,
    unified_uses_transform_scale: bool,
    needs_any_effect: bool,
    needs_mask: bool,
    needs_wipe: bool,
    needs_stretch: bool,
    needs_blur: bool,
    needs_palette: bool,
    has_wavewarp2: bool,
    has_mirror: bool,
    has_rgb_split: bool,
    pixelate_expansion: f32,
    wavewarp2_max_m2: f32,
    mirror_max_offset: f32,
    rgb_split_max_offset: f32,
) {
    use crate::masked_sprite::UnifiedEffectMarker;

    let base_width = width * size_scale;
    let base_height = height * size_scale;

    if is_media && !image_uri.is_empty() {
        bevy::log::debug!(
            "[add_visual_components] SpriteShape '{}': is_media=true, image_uri='{}', image_found={}",
            label,
            image_uri,
            images.contains_key(image_uri)
        );
    }

    let blur_only = needs_blur && !needs_mask && !needs_wipe && !needs_stretch;
    let media_handle = (is_media && !image_uri.is_empty())
        .then(|| images.get(image_uri))
        .flatten();

    if let Some(handle) = media_handle.filter(|_| blur_only) {
        let scaled_width = base_width * initial_scale.0.abs();
        let scaled_height = base_height * initial_scale.1.abs();
        let blur_radius = blur_params.map(|bp| bp.x).unwrap_or(0.0);

        commands.entity(entity).insert((
            Sprite {
                image: handle.clone(),
                color: Color::WHITE,
                custom_size: Some(Vec2::new(scaled_width, scaled_height)),
                ..default()
            },
            *anchor,
            crate::gaussian_blur::GaussianBlurEffect {
                radius: blur_radius,
                width: scaled_width,
                height: scaled_height,
                rtt_ready: false,
            },
            AmVisualSpawned,
        ));

        bevy::log::trace!(
            "[Visual] Spawned sprite '{}' with RTT Gaussian blur: size=({:.1},{:.1}), radius={:.1}",
            label,
            scaled_width,
            scaled_height,
            blur_radius
        );
        return;
    }

    if let Some(handle) = media_handle.filter(|_| force_plain_sprite_for_label(label)) {
        commands.entity(entity).insert((
            Sprite {
                image: handle.clone(),
                color: Color::WHITE,
                custom_size: Some(Vec2::new(base_width, base_height)),
                ..default()
            },
            *anchor,
            AmVisualSpawned,
        ));
        bevy::log::warn!(
            "[VisualDebug] forcing plain sprite path for '{}' via AM_FORCE_PLAIN_SPRITE_LABELS",
            label
        );
        return;
    }

    if let Some(handle) = media_handle.filter(|_| needs_any_effect) {
        let scaled_width = base_width * initial_scale.0.abs();
        let scaled_height = base_height * initial_scale.1.abs();
        let material_width = if unified_uses_transform_scale {
            base_width
        } else {
            scaled_width
        };
        let material_height = if unified_uses_transform_scale {
            base_height
        } else {
            scaled_height
        };
        let blur_expansion = pixelate_expansion
            + if has_wavewarp2 {
                let exp = wavewarp2_max_m2 / 100.0 * scaled_width.max(scaled_height);
                bevy::log::warn!(
                    "[wavewarp2 mesh] expansion={:.2} max_m2={:.2} scaled=({:.1},{:.1}) base=({:.1},{:.1}) scale=({:.4},{:.4})",
                    exp,
                    wavewarp2_max_m2,
                    scaled_width,
                    scaled_height,
                    base_width,
                    base_height,
                    initial_scale.0,
                    initial_scale.1
                );
                exp
            } else {
                0.0
            }
            + if has_mirror && mirror_max_offset > 0.001 {
                mirror_max_offset * scaled_width.max(scaled_height)
            } else {
                0.0
            }
            + if has_rgb_split && rgb_split_max_offset > 0.001 {
                rgb_split_max_offset * scaled_width.max(scaled_height)
            } else {
                0.0
            };

        let stretch_mesh = initial_stretch_mesh_bounds.map(|(min_x, max_x, min_y, max_y)| {
            create_stretch_bounds_mesh(meshes, min_x, max_x, min_y, max_y)
        });
        let mesh = stretch_mesh.unwrap_or_else(|| {
            create_anchored_rectangle_with_blur(
                meshes,
                material_width,
                material_height,
                anchor,
                blur_expansion,
            )
        });

        let blur_params_with_expansion =
            blur_params.map(|bp| Vec4::new(bp.x, material_width, material_height, blur_expansion));
        let mesh_size = initial_stretch_mesh_bounds
            .map(|(min_x, max_x, min_y, max_y)| (max_x - min_x, max_y - min_y));

        let material = create_unified_material(
            unified_materials,
            handle.clone(),
            LinearRgba::WHITE,
            material_width,
            material_height,
            mask_info,
            wipe_params,
            stretch_params,
            blur_params_with_expansion,
            palette_params,
            initial_mesh_offset,
            mesh_size,
            fit_scale,
            global_time_ms,
            replace_color_params,
            TextureSourceContract::layer_texture(),
        );

        if trace_unified_color_enabled(layer_id)
            && let Some(material_ref) = unified_materials.get(&material)
        {
            bevy::log::warn!(
                "[UnifiedColorTrace][spawn-media] id={} label='{}' color={:?} replace_flags={:?} threshold={:?}",
                layer_id,
                label,
                material_ref.uniform_data.color,
                material_ref.uniform_data.replace_color_flags,
                material_ref.uniform_data.threshold_params
            );
        }

        commands.entity(entity).insert((
            Mesh2d(mesh),
            MeshMaterial2d(material),
            UnifiedEffectMarker,
            crate::animation::components::AmUnifiedMeshState::default(),
            AmVisualSpawned,
        ));
        if unified_uses_transform_scale {
            commands.entity(entity).insert(AmUnifiedUsesTransformScale);
        }

        bevy::log::trace!(
            "[Visual] Spawned sprite '{}' with unified effect: scaled_size=({:.1},{:.1}), blur_exp={:.1}, mask={}, wipe={}, stretch={}, blur={}, palette={}, has_stretch_bounds={}",
            label,
            scaled_width,
            scaled_height,
            blur_expansion,
            needs_mask,
            needs_wipe,
            needs_stretch,
            needs_blur,
            needs_palette,
            initial_stretch_mesh_bounds.is_some()
        );
        return;
    }

    if let Some(handle) = media_handle {
        commands.entity(entity).insert((
            Sprite {
                image: handle.clone(),
                color: Color::WHITE,
                custom_size: Some(Vec2::new(base_width, base_height)),
                ..default()
            },
            *anchor,
            AmVisualSpawned,
        ));
        return;
    }

    if let Some(wp) = white_pixel
        && needs_any_effect
    {
        bevy::log::trace!(
            "[Visual] Spawning fill sprite '{}' with white_pixel, color fill",
            label
        );
        let color = extract_fill_color(fill_color, false);
        let scaled_width = base_width * initial_scale.0.abs();
        let scaled_height = base_height * initial_scale.1.abs();
        let mesh_width = if unified_uses_transform_scale {
            base_width
        } else {
            scaled_width
        };
        let mesh_height = if unified_uses_transform_scale {
            base_height
        } else {
            scaled_height
        };

        let blur_expansion = pixelate_expansion
            + if has_wavewarp2 {
                let exp = wavewarp2_max_m2 / 100.0 * scaled_width.max(scaled_height);
                bevy::log::warn!(
                    "[wavewarp2 mesh fill] expansion={:.2} max_m2={:.2} scaled=({:.1},{:.1})",
                    exp,
                    wavewarp2_max_m2,
                    scaled_width,
                    scaled_height
                );
                exp
            } else {
                0.0
            }
            + if has_mirror && mirror_max_offset > 0.001 {
                mirror_max_offset * scaled_width.max(scaled_height)
            } else {
                0.0
            };
        let stretch_mesh = initial_stretch_mesh_bounds.map(|(min_x, max_x, min_y, max_y)| {
            create_stretch_bounds_mesh(meshes, min_x, max_x, min_y, max_y)
        });
        let mesh = stretch_mesh.unwrap_or_else(|| {
            create_anchored_rectangle_with_blur(
                meshes,
                mesh_width,
                mesh_height,
                anchor,
                blur_expansion,
            )
        });

        let blur_params_with_expansion =
            blur_params.map(|bp| Vec4::new(bp.x, mesh_width, mesh_height, blur_expansion));
        let mesh_size = initial_stretch_mesh_bounds
            .map(|(min_x, max_x, min_y, max_y)| (max_x - min_x, max_y - min_y));

        let material = create_unified_material(
            unified_materials,
            wp.clone(),
            color.to_linear(),
            base_width,
            base_height,
            mask_info,
            wipe_params,
            stretch_params,
            blur_params_with_expansion.or(blur_params),
            palette_params,
            initial_mesh_offset,
            mesh_size,
            fit_scale,
            global_time_ms,
            replace_color_params,
            TextureSourceContract::layer_texture(),
        );

        if trace_unified_color_enabled(layer_id)
            && let Some(material_ref) = unified_materials.get(&material)
        {
            bevy::log::warn!(
                "[UnifiedColorTrace][spawn-fill] id={} label='{}' color={:?} replace_flags={:?} threshold={:?}",
                layer_id,
                label,
                material_ref.uniform_data.color,
                material_ref.uniform_data.replace_color_flags,
                material_ref.uniform_data.threshold_params
            );
        }

        commands.entity(entity).insert((
            Mesh2d(mesh),
            MeshMaterial2d(material),
            UnifiedEffectMarker,
            crate::animation::components::AmUnifiedMeshState::default(),
            AmVisualSpawned,
        ));
        if unified_uses_transform_scale {
            commands.entity(entity).insert(AmUnifiedUsesTransformScale);
        }

        bevy::log::trace!(
            "[Visual] Spawned fill sprite '{}' with unified effect: base_size=({:.1},{:.1}), has_stretch_bounds={}",
            label,
            base_width,
            base_height,
            initial_stretch_mesh_bounds.is_some()
        );
        return;
    }

    if let Some(wp) = white_pixel {
        bevy::log::trace!(
            "[Visual] Spawning fill sprite '{}' with white_pixel, color fill",
            label
        );
        let color = extract_fill_color(fill_color, false);
        commands.entity(entity).insert((
            Sprite {
                image: wp.clone(),
                color,
                custom_size: Some(Vec2::new(base_width, base_height)),
                ..default()
            },
            *anchor,
            AmVisualSpawned,
        ));
        return;
    }

    bevy::log::warn!(
        "[Visual] Cannot spawn fill sprite '{}': white_pixel is None! is_media={}, image_uri='{}'",
        label,
        is_media,
        image_uri
    );
}

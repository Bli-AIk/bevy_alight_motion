//! # visual.rs
//!
//! # 视觉组件模块
//!
//! Visual component creation for AM layers.
//! AM 图层的视觉组件创建。

use bevy::asset::Assets;
use bevy::prelude::*;
use std::collections::HashMap;

mod image;
mod material;
mod mesh;
mod sprite_shape;
mod text;

use crate::scene::{AmLayerMarker, AmLayerSpec, AmMaskInfo, AmPaletteMapParams, AmVisualSpawned};
use crate::sdf_material::SdfMaterial;

use self::image::handle_image_visual;
use self::sprite_shape::handle_sprite_shape_visual;
use self::text::handle_text_visual;
use super::sdf_spawn::spawn_sdf_visual;
use super::visual_helpers::trace_visual_path_once;

#[expect(clippy::too_many_arguments)] // reason: visual setup requires many GPU resource handles
pub(crate) fn add_visual_components(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    unified_materials: &mut Assets<crate::masked_sprite::UnifiedEffectMaterial>,
    _color_materials: &mut Assets<ColorMaterial>,
    sdf_materials: &mut Assets<SdfMaterial>,
    entity: Entity,
    spec: &AmLayerSpec,
    mask_info: &Option<AmMaskInfo>,
    palette_params: Option<&AmPaletteMapParams>,
    images: &HashMap<String, Handle<Image>>,
    fonts: &HashMap<String, Handle<Font>>,
    white_pixel: Option<&Handle<Image>>,
    label: &str,
    id: u64,
    initial_scale: (f32, f32),
    wipe_params: Option<Vec4>,
    stretch_params: Option<Vec4>,
    blur_params: Option<Vec4>,
    embed_scene_size: Option<(f32, f32)>,
    size_scale: f32,
    _max_blur_radius: f32,
    initial_mesh_offset: Option<Vec4>,
    initial_stretch_mesh_bounds: Option<(f32, f32, f32, f32)>,
    fit_scale: f32,
    is_embed_content: bool,
    has_scale_animation: bool,
    has_scale_assist: bool,
    has_repeat: bool,
    has_threshold: bool,
    has_grid: bool,
    has_pixelate: bool,
    has_stretch2: bool,
    has_solidcolor: bool,
    has_wavewarp2: bool,
    has_mirror: bool,
    has_lift: bool,
    has_rays: bool,
    has_rgb_split: bool,
    has_exposure: bool,
    has_blend: bool,
    has_chromakey: bool,
    _has_parenthelper: bool,
    has_child_layers: bool,
    rgb_split_max_offset: f32,
    pixelate_expansion: f32,
    wavewarp2_max_m2: f32,
    mirror_max_offset: f32,
    global_time_ms: u64,
    replace_color_params: Option<(Vec4, Vec4, Vec4, Vec4)>,
    max_animated_scale: f32,
) {
    bevy::log::debug!(
        "[add_visual_components] Called for '{}' (id={}), spec={:?}, is_embed_content={}, has_scale_assist={}, has_repeat={}, has_threshold={}, has_grid={}, has_pixelate={}",
        label,
        id,
        std::mem::discriminant(spec),
        is_embed_content,
        has_scale_assist,
        has_repeat,
        has_threshold,
        has_grid,
        has_pixelate
    );

    let needs_stretch = stretch_params.is_some();
    let needs_wipe = wipe_params.is_some();
    let needs_mask = mask_info.is_some();
    let needs_blur = blur_params.is_some();
    let needs_palette = palette_params.is_some();
    let needs_replace_color = replace_color_params.is_some();

    let unified_uses_transform_scale = is_embed_content
        && !needs_stretch
        && !needs_wipe
        && !needs_mask
        && !needs_blur
        && !needs_palette
        && !needs_replace_color
        && !has_scale_assist
        && !has_repeat
        && !has_threshold
        && !has_grid
        && !has_pixelate
        && !has_stretch2
        && !has_solidcolor
        && !has_wavewarp2
        && !has_mirror
        && !has_lift
        && !has_rays
        && !has_rgb_split
        && !has_exposure
        && !has_blend
        && !has_chromakey;

    let needs_any_effect = needs_stretch
        || needs_wipe
        || needs_mask
        || needs_blur
        || needs_palette
        || needs_replace_color
        || has_scale_assist
        || has_repeat
        || has_threshold
        || has_grid
        || has_pixelate
        || has_stretch2
        || has_solidcolor
        || has_wavewarp2
        || has_mirror
        || has_lift
        || has_rays
        || has_rgb_split
        || has_exposure
        || has_blend
        || has_chromakey;

    trace_visual_path_once(format!("{id}:{label}"), || {
        format!(
            "[VISUAL-PATH] id={} label='{}' embed_content={} fast_transform_scale={} has_child_layers={} has_parenthelper={} needs_any_effect={} stretch={} wipe={} mask={} blur={} repeat={} pixelate={} wavewarp2={} mirror={} blend={}",
            id,
            label,
            is_embed_content,
            unified_uses_transform_scale,
            has_child_layers,
            _has_parenthelper,
            needs_any_effect,
            needs_stretch,
            needs_wipe,
            needs_mask,
            needs_blur,
            has_repeat,
            has_pixelate,
            has_wavewarp2,
            has_mirror,
            has_blend,
        )
    });

    let direct_embed_size_scale = if is_embed_content && !needs_any_effect {
        fit_scale
    } else {
        size_scale
    };

    match spec {
        AmLayerSpec::SpriteShape {
            image_uri,
            is_media,
            fill_color,
            width,
            height,
            anchor,
        } => {
            handle_sprite_shape_visual(
                commands,
                meshes,
                unified_materials,
                entity,
                image_uri,
                *is_media,
                fill_color,
                *width,
                *height,
                anchor,
                mask_info,
                palette_params,
                images,
                white_pixel,
                label,
                initial_scale,
                wipe_params,
                stretch_params,
                blur_params,
                direct_embed_size_scale,
                initial_mesh_offset,
                initial_stretch_mesh_bounds,
                fit_scale,
                global_time_ms,
                replace_color_params,
                unified_uses_transform_scale,
                needs_any_effect,
                needs_mask,
                needs_wipe,
                needs_stretch,
                needs_blur,
                needs_palette,
                has_wavewarp2,
                has_mirror,
                has_rgb_split,
                pixelate_expansion,
                wavewarp2_max_m2,
                mirror_max_offset,
                rgb_split_max_offset,
            );
        }
        AmLayerSpec::SdfShape {
            fill_color,
            stroke_color_value,
            stroke_width,
            stroke_join,
            stroke_direction,
            border2_color_value,
            border2_width,
            border2_direction,
            width,
            height,
            pivot_x,
            pivot_y,
            shape_type,
            no_fill,
            shape_extra,
            shape_extra2,
            shape_extra3,
            shape_extra4,
            shape_extra5,
            shape_extra6,
            shape_extra7,
            gradient_type,
            gradient_start_color,
            gradient_end_color,
            gradient_points,
        } => {
            bevy::log::info!("[Visual] Spawning SdfShape for '{}'", label);
            spawn_sdf_visual(
                commands,
                meshes,
                sdf_materials,
                entity,
                fill_color,
                stroke_color_value,
                *stroke_width,
                stroke_join,
                stroke_direction,
                border2_color_value,
                *border2_width,
                border2_direction,
                *width,
                *height,
                *pivot_x,
                *pivot_y,
                shape_type,
                &AmLayerMarker {
                    id,
                    label: label.to_string(),
                },
                initial_scale,
                mask_info,
                global_time_ms,
                fit_scale,
                *no_fill,
                *shape_extra,
                *shape_extra2,
                *shape_extra3,
                *shape_extra4,
                *shape_extra5,
                *shape_extra6,
                *shape_extra7,
                *gradient_type,
                *gradient_start_color,
                *gradient_end_color,
                *gradient_points,
                max_animated_scale,
            );
        }
        AmLayerSpec::Image {
            image_uri,
            width,
            height,
            anchor,
        } => {
            handle_image_visual(
                commands,
                meshes,
                unified_materials,
                entity,
                image_uri,
                *width,
                *height,
                anchor,
                images,
                label,
                mask_info,
                palette_params,
                wipe_params,
                stretch_params,
                blur_params,
                direct_embed_size_scale,
                initial_mesh_offset,
                initial_stretch_mesh_bounds,
                fit_scale,
                global_time_ms,
                replace_color_params,
                needs_any_effect,
            );
        }
        AmLayerSpec::Text {
            content,
            font_name,
            font_size,
            align,
            fill_color,
            wrap_width,
            line_height_ratio,
        } => {
            handle_text_visual(
                commands,
                entity,
                content,
                font_name,
                *font_size,
                align,
                fill_color,
                *wrap_width,
                *line_height_ratio,
                fonts,
            );
        }
        AmLayerSpec::Null => {
            commands.entity(entity).insert(AmVisualSpawned);
        }
        AmLayerSpec::EmbedScene => {
            if embed_scene_size.is_none() {
                bevy::log::warn!(
                    "[SpawnVisuals] EmbedScene '{}' (id={}) has NO embed_scene_size!",
                    label,
                    id
                );
            }
            let _ = has_scale_animation;
            commands.entity(entity).insert(AmVisualSpawned);
        }
        AmLayerSpec::Camera { .. } => {
            commands.entity(entity).insert(AmVisualSpawned);
        }
    }
}

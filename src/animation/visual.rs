//! # visual.rs
//!
//! # 视觉组件模块
//!
//! Visual component creation for AM layers including add_visual_components,
//! spawn_sdf_visual, and extract_fill_color helper functions.
//!
//! AM 图层的视觉组件创建，包括 add_visual_components、spawn_sdf_visual
//! 以及 extract_fill_color 辅助函数。

use bevy::asset::Assets;
use bevy::prelude::*;
use std::collections::HashMap;

mod material;
mod mesh;

use crate::scene::{AmLayerMarker, AmLayerSpec, AmMaskInfo, AmPaletteMapParams, AmVisualSpawned};
use crate::sdf_material::SdfMaterial;

use self::material::create_unified_material;
use self::mesh::{create_anchored_rectangle, create_anchored_rectangle_with_blur};
use super::components::AmUnifiedUsesTransformScale;
use super::sdf_spawn::spawn_sdf_visual;
use super::visual_helpers::{
    create_stretch_bounds_mesh, extract_fill_color, trace_visual_path_once,
};

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
    initial_stretch_mesh_bounds: Option<(f32, f32, f32, f32)>, // (min_x, max_x, min_y, max_y)
    fit_scale: f32,                                            // Scale factor for mask coordinates
    is_embed_content: bool,    // True if this is content inside an embed
    has_scale_animation: bool, // True if embed has scale animation (needs bounds clipping)
    has_scale_assist: bool, // True if layer has scale_assist effect (needs UnifiedEffectMaterial for dynamic sizing)
    has_repeat: bool,       // True if layer has repeat effect (needs UnifiedEffectMaterial)
    has_threshold: bool,    // True if layer has threshold effect (needs UnifiedEffectMaterial)
    has_grid: bool,         // True if layer has grid effect (needs UnifiedEffectMaterial)
    has_pixelate: bool,     // True if layer has pixelate effect (needs UnifiedEffectMaterial)
    has_stretch2: bool,     // True if layer has stretch2 effect (needs UnifiedEffectMaterial)
    has_solidcolor: bool,   // True if layer has solidcolor effect (needs UnifiedEffectMaterial)
    has_wavewarp2: bool,    // True if layer has wavewarp2 effect (needs UnifiedEffectMaterial)
    has_mirror: bool,       // True if layer has mirror effect (needs UnifiedEffectMaterial)
    has_lift: bool, // True if layer has lift (copy background) effect (needs UnifiedEffectMaterial)
    has_rays: bool, // True if layer has rays effect (needs UnifiedEffectMaterial)
    has_rgb_split: bool, // True if layer has RGB split effect (needs UnifiedEffectMaterial)
    has_exposure: bool, // True if layer has exposure/gamma effect (needs UnifiedEffectMaterial)
    has_blend: bool, // True if layer has non-normal blend mode (needs UnifiedEffectMaterial)
    has_chromakey: bool, // True if layer has chromakey effect (needs UnifiedEffectMaterial)
    _has_parenthelper: bool, // Parenthelper is handled via Transform.scale on the fast path
    has_child_layers: bool, // True if layer has AM children that depend on its visual scale
    rgb_split_max_offset: f32, // Max RGB split offset in UV space (max_strength / 8.0) for mesh expansion
    pixelate_expansion: f32,   // Max pixelate expansion in display units (half max grid cell size)
    wavewarp2_max_m2: f32,     // Max wavewarp2 magnitude across keyframes (for mesh expansion)
    mirror_max_offset: f32,    // Max mirror offset across keyframes (for mesh expansion)
    global_time_ms: u64,       // Current playback time for mask initialization
    replace_color_params: Option<(Vec4, Vec4, Vec4, Vec4)>, // (flags, old_color, new_color, params)
    max_animated_scale: f32,   // Max scale from animation keyframes for SDF mesh sizing
) {
    use crate::masked_sprite::UnifiedEffectMarker;

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

    // Determine which effects are needed.
    // Embed content no longer forces the unified material path by itself:
    // Direct embeds can render their content normally, and isolated embeds only
    // need unified materials when a real effect or clip path requires it.
    let needs_stretch = stretch_params.is_some();
    let needs_wipe = wipe_params.is_some();
    let needs_mask = mask_info.is_some();
    let needs_blur = blur_params.is_some();
    let needs_palette = palette_params.is_some();
    let needs_replace_color = replace_color_params.is_some();
    // Parenthelper now corrects Transform.scale directly in apply_parenthelper_system.
    // Plain embed visuals can stay on the cheaper transform-scale path unless
    // some real effect requires mesh resizing; child AM layers still inherit the
    // parent's transform through the normal Bevy hierarchy.
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

    // Direct embed content bypasses the RTT/unified path, so it must still pick up the
    // project fit-scale that AM bakes into the rendered composite size.
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
            let base_width = *width * direct_embed_size_scale;
            let base_height = *height * direct_embed_size_scale;

            if *is_media && !image_uri.is_empty() {
                bevy::log::debug!(
                    "[add_visual_components] SpriteShape '{}': is_media=true, image_uri='{}', image_found={}",
                    label,
                    image_uri,
                    images.contains_key(image_uri)
                );
            }
            let blur_only = needs_blur && !needs_mask && !needs_wipe && !needs_stretch;
            let media_handle = (*is_media && !image_uri.is_empty())
                .then(|| images.get(image_uri))
                .flatten();

            if let Some(handle) = media_handle.filter(|_| blur_only) {
                // Use RTT-based Gaussian blur for best quality
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
            } else if let Some(handle) = media_handle.filter(|_| needs_any_effect) {
                // Use UnifiedEffectMaterial for combined effects (mask/wipe/stretch + optional blur)
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
                        // Expand mesh by max displacement magnitude to show content beyond original bounds
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
                        // Mirror offset pushes reflected content beyond layer bounds
                        mirror_max_offset * scaled_width.max(scaled_height)
                    } else {
                        0.0
                    }
                    + if has_rgb_split && rgb_split_max_offset > 0.001 {
                        // RGB split offset pushes color channels beyond layer bounds
                        rgb_split_max_offset * scaled_width.max(scaled_height)
                    } else {
                        0.0
                    };

                let stretch_mesh =
                    initial_stretch_mesh_bounds.map(|(min_x, max_x, min_y, max_y)| {
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

                let blur_params_with_expansion = blur_params
                    .map(|bp| Vec4::new(bp.x, material_width, material_height, blur_expansion));
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
                );

                commands.entity(entity).insert((
                    Mesh2d(mesh),
                    MeshMaterial2d(material),
                    UnifiedEffectMarker,
                    AmVisualSpawned,
                ));
                if unified_uses_transform_scale {
                    commands.entity(entity).insert(AmUnifiedUsesTransformScale);
                }
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
            } else if let Some(handle) = media_handle {
                // No effects - use normal sprite
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
            } else if let Some(wp) = white_pixel
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
                let stretch_mesh =
                    initial_stretch_mesh_bounds.map(|(min_x, max_x, min_y, max_y)| {
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
                );

                commands.entity(entity).insert((
                    Mesh2d(mesh),
                    MeshMaterial2d(material),
                    UnifiedEffectMarker,
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
            } else if let Some(wp) = white_pixel {
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
            } else {
                bevy::log::warn!(
                    "[Visual] Cannot spawn fill sprite '{}': white_pixel is None! is_media={}, image_uri='{}'",
                    label,
                    is_media,
                    image_uri
                );
            }
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
            let base_width = *width * direct_embed_size_scale;
            let base_height = *height * direct_embed_size_scale;

            if let Some(handle) = images.get(image_uri)
                && needs_any_effect
            {
                // Use initial stretch mesh bounds if provided (to prevent first frame jump)
                let stretch_mesh =
                    initial_stretch_mesh_bounds.map(|(min_x, max_x, min_y, max_y)| {
                        create_stretch_bounds_mesh(meshes, min_x, max_x, min_y, max_y)
                    });
                let mesh = stretch_mesh.unwrap_or_else(|| {
                    create_anchored_rectangle(meshes, base_width, base_height, anchor)
                });

                // Calculate mesh size for stretch bounds
                let mesh_size = initial_stretch_mesh_bounds
                    .map(|(min_x, max_x, min_y, max_y)| (max_x - min_x, max_y - min_y));

                let material = create_unified_material(
                    unified_materials,
                    handle.clone(),
                    LinearRgba::WHITE,
                    base_width,
                    base_height,
                    mask_info,
                    wipe_params,
                    stretch_params,
                    blur_params,
                    palette_params,
                    initial_mesh_offset,
                    mesh_size,
                    fit_scale,
                    global_time_ms,
                    replace_color_params,
                );

                // Transform.scale from scene.rs will handle the scaling
                commands.entity(entity).insert((
                    Mesh2d(mesh),
                    MeshMaterial2d(material),
                    UnifiedEffectMarker,
                    AmVisualSpawned,
                ));

                bevy::log::trace!(
                    "[Visual] Spawned image '{}' with unified effect: base_size=({:.1},{:.1}), has_stretch_bounds={}",
                    label,
                    base_width,
                    base_height,
                    initial_stretch_mesh_bounds.is_some()
                );
            } else if let Some(handle) = images.get(image_uri) {
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
            }
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
            use bevy::text::Justify;

            let color = extract_fill_color(fill_color, false);
            let justify = match align.as_str() {
                "center" => Justify::Center,
                "right" => Justify::Right,
                _ => Justify::Left,
            };

            let font = fonts
                .get(font_name)
                .cloned()
                .unwrap_or_else(Handle::default);

            // AM text element position is always the CENTER of the text box
            let anchor = bevy::sprite::Anchor::CENTER;

            // Use AM-matching line height computed from font hhea metrics.
            // AM's StaticLayout uses float ascent + descent for line spacing.
            let line_height = bevy::text::LineHeight::RelativeToFont(*line_height_ratio);

            commands.entity(entity).insert((
                Text2d::new(content.clone()),
                TextFont {
                    font,
                    font_size: *font_size,
                    ..default()
                },
                TextLayout::new_with_justify(justify),
                TextColor(color),
                bevy::text::TextBounds::new_horizontal(*wrap_width),
                line_height,
                anchor,
                AmVisualSpawned,
            ));
        }
        AmLayerSpec::Null => {
            commands.entity(entity).insert(AmVisualSpawned);
        }
        AmLayerSpec::EmbedScene => {
            // EmbedScene strategy metadata is attached during entity spawn, before visuals are
            // added. Re-inserting it here would overwrite flags such as requires_composite.
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
            // Camera layers have no visual — marker only
            commands.entity(entity).insert(AmVisualSpawned);
        }
    }
}

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

use crate::scene::{AmLayerMarker, AmLayerSpec, AmMaskInfo, AmPaletteMapParams, AmVisualSpawned};
use crate::sdf_material::SdfMaterial;

use super::sdf_spawn::spawn_sdf_visual;
use super::visual_helpers::{compute_initial_mask_params, create_stretch_bounds_mesh};

#[expect(clippy::too_many_arguments)] // reason: visual setup requires many GPU resource handles
pub(crate) fn add_visual_components(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    unified_materials: &mut Assets<crate::masked_sprite::UnifiedEffectMaterial>,
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
    rgb_split_max_offset: f32, // Max RGB split offset in UV space (max_strength / 8.0) for mesh expansion
    pixelate_expansion: f32,   // Max pixelate expansion in display units (half max grid cell size)
    wavewarp2_max_m2: f32,     // Max wavewarp2 magnitude across keyframes (for mesh expansion)
    mirror_max_offset: f32,    // Max mirror offset across keyframes (for mesh expansion)
    global_time_ms: u64,       // Current playback time for mask initialization
    replace_color_params: Option<(Vec4, Vec4, Vec4, Vec4)>, // (flags, old_color, new_color, params)
    max_animated_scale: f32,   // Max scale from animation keyframes for SDF mesh sizing
) {
    use crate::masked_sprite::{UnifiedEffectMarker, UnifiedEffectMaterial};

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

    // Determine which effects are needed
    // Embed content always needs effect material to support bounds clipping later
    // Scale_assist also needs effect material for dynamic sizing
    let needs_stretch = stretch_params.is_some();
    let needs_wipe = wipe_params.is_some();
    let needs_mask = mask_info.is_some();
    let needs_blur = blur_params.is_some();
    let needs_palette = palette_params.is_some();
    let needs_replace_color = replace_color_params.is_some();
    let needs_any_effect = needs_stretch
        || needs_wipe
        || needs_mask
        || needs_blur
        || needs_palette
        || needs_replace_color
        || is_embed_content
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

    // Helper function to create a rectangle mesh with anchor offset
    fn create_anchored_rectangle(
        meshes: &mut Assets<Mesh>,
        width: f32,
        height: f32,
        anchor: &bevy::sprite::Anchor,
    ) -> Handle<Mesh> {
        let anchor_vec = anchor.as_vec();
        let offset_x = -anchor_vec.x * width;
        let offset_y = -anchor_vec.y * height;
        bevy::log::debug!(
            "[MESH] create_anchored_rectangle: size=({:.1}, {:.1}), anchor=({:.3}, {:.3}), vertex_offset=({:.1}, {:.1})",
            width,
            height,
            anchor_vec.x,
            anchor_vec.y,
            offset_x,
            offset_y
        );
        create_anchored_rectangle_with_blur(meshes, width, height, anchor, 0.0)
    }

    // Helper function to create a rectangle mesh with anchor offset and blur expansion
    // blur_expansion: additional pixels to add on each side for blur overflow
    fn create_anchored_rectangle_with_blur(
        meshes: &mut Assets<Mesh>,
        width: f32,
        height: f32,
        anchor: &bevy::sprite::Anchor,
        blur_expansion: f32,
    ) -> Handle<Mesh> {
        if blur_expansion > 0.001 {
            bevy::log::warn!(
                "[MESH] create_anchored_rectangle_with_blur: size=({:.1},{:.1}) expansion={:.2}",
                width,
                height,
                blur_expansion
            );
        }
        let anchor_vec = anchor.as_vec();
        // Anchor offset based on original size (this positions the image center)
        let offset_x = -anchor_vec.x * width;
        let offset_y = -anchor_vec.y * height;

        // Original half-sizes
        let half_w = width / 2.0;
        let half_h = height / 2.0;

        // Vertices expand outward from original rectangle by blur_expansion
        // This keeps the image centered while expanding the mesh for blur overflow
        let vertices = vec![
            [
                offset_x - half_w - blur_expansion,
                offset_y - half_h - blur_expansion,
                0.0,
            ],
            [
                offset_x + half_w + blur_expansion,
                offset_y - half_h - blur_expansion,
                0.0,
            ],
            [
                offset_x + half_w + blur_expansion,
                offset_y + half_h + blur_expansion,
                0.0,
            ],
            [
                offset_x - half_w - blur_expansion,
                offset_y + half_h + blur_expansion,
                0.0,
            ],
        ];

        let normals = vec![
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ];

        // UV coordinates that map the expanded mesh to extended texture sampling
        // When blur_expansion > 0, UVs extend beyond 0-1 range
        // The shader's blur function handles out-of-bounds by treating them as transparent
        let uv_expand_x = blur_expansion / width;
        let uv_expand_y = blur_expansion / height;
        let uvs = vec![
            [-uv_expand_x, 1.0 + uv_expand_y],      // bottom-left
            [1.0 + uv_expand_x, 1.0 + uv_expand_y], // bottom-right
            [1.0 + uv_expand_x, -uv_expand_y],      // top-right
            [-uv_expand_x, -uv_expand_y],           // top-left
        ];

        let indices = vec![0, 1, 2, 0, 2, 3];

        let mut mesh = Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_indices(bevy::mesh::Indices::U32(indices));

        meshes.add(mesh)
    }

    // Helper to create unified material with effects
    fn create_unified_material(
        unified_materials: &mut Assets<UnifiedEffectMaterial>,
        texture: Handle<Image>,
        color: LinearRgba,
        width: f32,
        height: f32,
        mask_info: &Option<AmMaskInfo>,
        wipe_params: Option<Vec4>,
        stretch_params: Option<Vec4>,
        blur_params: Option<Vec4>,
        palette_params: Option<&AmPaletteMapParams>,
        mesh_offset: Option<Vec4>,
        mesh_size: Option<(f32, f32)>, // Optional mesh size for stretch bounds
        fit_scale: f32,                // Scale factor for mask coordinates
        global_time_ms: u64,           // Current playback time for mask initialization
        replace_color_params: Option<(Vec4, Vec4, Vec4, Vec4)>, // (flags, old_color, new_color, params)
    ) -> Handle<UnifiedEffectMaterial> {
        // Use mesh_size if provided (for stretch bounds), otherwise use original size
        let (mesh_width, mesh_height) = mesh_size.unwrap_or((width, height));

        // Pre-calculate mask params if mask is present, to ensure first frame renders correctly
        let (
            initial_effect_flags_x,
            initial_mask_params,
            initial_mask2_flags_x,
            initial_mask2_params,
        ) = compute_initial_mask_params(mask_info, fit_scale, global_time_ms);

        let mut material = UnifiedEffectMaterial {
            uniform_data: crate::masked_sprite::UnifiedEffectUniform {
                color: Vec4::new(color.red, color.green, color.blue, color.alpha),
                effect_flags: Vec4::new(initial_effect_flags_x, 0.0, 0.0, 0.0),
                mask_params: initial_mask_params,
                original_size: Vec4::new(width, height, mesh_width, mesh_height),
                mesh_offset: mesh_offset.unwrap_or(Vec4::ZERO),
                mask2_params: initial_mask2_params,
                mask2_flags: Vec4::new(initial_mask2_flags_x, 0.0, 0.0, 0.0),
                ..default()
            },
            texture: Some(texture),
            lift_comp_texture: None,
        };

        // Enable wipe if present
        if let Some(wp) = wipe_params {
            material.uniform_data.effect_flags.y = 1.0;
            material.uniform_data.wipe_params = wp;
        }

        // Enable stretch if present
        if let Some(sp) = stretch_params {
            material.uniform_data.effect_flags.z = 1.0;
            material.uniform_data.stretch_params = sp;
        }

        // Enable blur if present
        if let Some(bp) = blur_params {
            material.uniform_data.effect_flags.w = 1.0;
            material.uniform_data.blur_params = bp;
        }

        // Enable palette map if present
        if let Some(palette) = palette_params {
            material.uniform_data.palette_flags.x = 1.0; // enabled
            material.uniform_data.palette_flags.y = palette.count as f32;
            material.uniform_data.palette_flags.z = 0.0; // shades already resolved into colors
            material.uniform_data.palette_flags.w = palette.initial_alpha;
            material.uniform_data.palette_color1 = palette.colors[0];
            material.uniform_data.palette_color2 = palette.colors[1];
            material.uniform_data.palette_color3 = palette.colors[2];
            material.uniform_data.palette_color4 = palette.colors[3];
            material.uniform_data.palette_color5 = palette.colors[4];
            material.uniform_data.palette_color6 = palette.colors[5];
            material.uniform_data.palette_color7 = palette.colors[6];
            material.uniform_data.palette_color8 = palette.colors[7];
        }

        // Enable replace color if present
        if let Some((flags, old_color, new_color, params)) = replace_color_params {
            material.uniform_data.replace_color_flags = flags;
            material.uniform_data.replace_old_color = old_color;
            material.uniform_data.replace_new_color = new_color;
            material.uniform_data.replace_color_params = params;
        }

        unified_materials.add(material)
    }

    match spec {
        AmLayerSpec::SpriteShape {
            image_uri,
            is_media,
            fill_color,
            width,
            height,
            anchor,
        } => {
            // Apply size_scale for embed children to compensate for fit_scale
            let base_width = *width * size_scale;
            let base_height = *height * size_scale;

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
                        scaled_width,
                        scaled_height,
                        anchor,
                        blur_expansion,
                    )
                });

                let blur_params_with_expansion = blur_params
                    .map(|bp| Vec4::new(bp.x, scaled_width, scaled_height, blur_expansion));
                let mesh_size = initial_stretch_mesh_bounds
                    .map(|(min_x, max_x, min_y, max_y)| (max_x - min_x, max_y - min_y));

                let material = create_unified_material(
                    unified_materials,
                    handle.clone(),
                    LinearRgba::WHITE,
                    scaled_width,
                    scaled_height,
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
                        scaled_width,
                        scaled_height,
                        anchor,
                        blur_expansion,
                    )
                });

                let blur_params_with_expansion = blur_params
                    .map(|bp| Vec4::new(bp.x, scaled_width, scaled_height, blur_expansion));
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
            // Apply size_scale for embed children to compensate for fit_scale
            let base_width = *width * size_scale;
            let base_height = *height * size_scale;

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
            // Add render strategy evaluation marker if scene size is available
            // The evaluate_render_strategy_system will determine the appropriate strategy
            if let Some((width, height)) = embed_scene_size {
                bevy::log::trace!(
                    "[SpawnVisuals] EmbedScene '{}' (id={}) gets NeedsStrategyEvaluation: {}x{}, has_scale_anim={}",
                    label,
                    id,
                    width,
                    height,
                    has_scale_animation
                );
                commands.entity(entity).insert((
                    crate::effects::NeedsStrategyEvaluation {
                        scene_width: width,
                        scene_height: height,
                        has_scale_animation,
                    },
                    AmVisualSpawned,
                ));
            } else {
                bevy::log::warn!(
                    "[SpawnVisuals] EmbedScene '{}' (id={}) has NO embed_scene_size!",
                    label,
                    id
                );
                commands.entity(entity).insert(AmVisualSpawned);
            }
        }
        AmLayerSpec::Camera { .. } => {
            // Camera layers have no visual — marker only
            commands.entity(entity).insert(AmVisualSpawned);
        }
    }
}

/// Extract fill color from AmFillColor.
///
/// - `no_fill`: When true (fillType="none"), always returns transparent regardless of fill_color.
/// - When false and `fill_color` is None, returns white as default.
/// - Otherwise extracts color from fill_color value or keyframes.
pub(crate) fn extract_fill_color(
    fill_color: &Option<crate::schema::AmFillColor>,
    no_fill: bool,
) -> Color {
    // fillType="none" means transparent fill
    if no_fill {
        return Color::srgba(0.0, 0.0, 0.0, 0.0);
    }

    if let Some(fc) = fill_color {
        if !fc.value.is_empty() {
            if let Ok(c) = crate::schema::parse_color(&fc.value) {
                return Color::srgba(c[0], c[1], c[2], c[3]);
            }
        } else if !fc.keyframes.is_empty() {
            let mut sorted: Vec<_> = fc.keyframes.iter().collect();
            sorted.sort_by(|a, b| {
                a.time
                    .partial_cmp(&b.time)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            if let Ok(c) = crate::schema::parse_color(&sorted[0].value) {
                return Color::srgba(c[0], c[1], c[2], c[3]);
            }
        }
    }
    // Default to white when no fill color specified (fillType="color" without fillColor element)
    Color::WHITE
}

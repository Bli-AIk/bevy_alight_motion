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

#[allow(clippy::too_many_arguments)]
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
    pixelate_expansion: f32, // Max pixelate expansion in display units (half max grid cell size)
    global_time_ms: u64,    // Current playback time for mask initialization
    replace_color_params: Option<(Vec4, Vec4, Vec4, Vec4)>, // (flags, old_color, new_color, params)
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
        || has_solidcolor;

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
    #[allow(clippy::too_many_arguments)]
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
        // Support up to 2 masks for dual-mask effects
        let (
            initial_effect_flags_x,
            initial_mask_params,
            initial_mask2_flags_x,
            initial_mask2_params,
        ) = if let Some(mask_info) = mask_info {
            let active_masks = mask_info.get_active_masks(global_time_ms);

            if active_masks.is_empty() {
                bevy::log::trace!(
                    "[MaterialInit] No active mask at time {}, mask_info has {} masks",
                    global_time_ms,
                    mask_info.masks.len()
                );
                (
                    0.0,
                    Vec4::new(0.0, 0.0, 10000.0, 10000.0),
                    0.0,
                    Vec4::new(0.0, 0.0, 10000.0, 10000.0),
                )
            } else {
                // First mask
                let mask1 = active_masks[0];
                let base_type1 = if mask1.is_circle { 2.0 } else { 1.0 };
                let mask1_type = if mask1.is_exclude {
                    base_type1 + 2.0
                } else {
                    base_type1
                };
                // Apply fit_scale to center and half_size * scale for world coordinate space
                let mask1_params = Vec4::new(
                    mask1.center.x * fit_scale,
                    mask1.center.y * fit_scale,
                    mask1.half_size.x * fit_scale * mask1.scale.x,
                    mask1.half_size.y * fit_scale * mask1.scale.y,
                );

                // Second mask (if present)
                let (mask2_type, mask2_params) = if active_masks.len() >= 2 {
                    let mask2 = active_masks[1];
                    let base_type2 = if mask2.is_circle { 2.0 } else { 1.0 };
                    let m2_type = if mask2.is_exclude {
                        base_type2 + 2.0
                    } else {
                        base_type2
                    };
                    // Apply fit_scale to center and half_size * scale for world coordinate space
                    let m2_params = Vec4::new(
                        mask2.center.x * fit_scale,
                        mask2.center.y * fit_scale,
                        mask2.half_size.x * fit_scale * mask2.scale.x,
                        mask2.half_size.y * fit_scale * mask2.scale.y,
                    );
                    bevy::log::trace!(
                        "[MaterialInit] DUAL Mask init: mask1_type={}, mask2_type={}, fit_scale={:.4}",
                        mask1_type,
                        m2_type,
                        fit_scale
                    );
                    (m2_type, m2_params)
                } else {
                    bevy::log::trace!(
                        "[MaterialInit] Mask init: effect_flags.x={}, center=({:.1},{:.1}), half_size=({:.1},{:.1}), fit_scale={:.4}",
                        mask1_type,
                        mask1.center.x * fit_scale,
                        mask1.center.y * fit_scale,
                        mask1.half_size.x * fit_scale * mask1.scale.x,
                        mask1.half_size.y * fit_scale * mask1.scale.y,
                        fit_scale
                    );
                    (0.0, Vec4::new(0.0, 0.0, 10000.0, 10000.0))
                };

                (mask1_type, mask1_params, mask2_type, mask2_params)
            }
        } else {
            (
                0.0,
                Vec4::new(0.0, 0.0, 10000.0, 10000.0),
                0.0,
                Vec4::new(0.0, 0.0, 10000.0, 10000.0),
            )
        };

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
                if let Some(handle) = images.get(image_uri) {
                    // Check if ONLY blur is needed (no mask/wipe/stretch)
                    // In this case, use Sprite + RTT blur for best quality
                    let blur_only = needs_blur && !needs_mask && !needs_wipe && !needs_stretch;

                    if blur_only {
                        // Use RTT-based Gaussian blur for best quality
                        // Sprite will be replaced by RTT output in GaussianBlurPlugin
                        let scaled_width = base_width * initial_scale.0.abs();
                        let scaled_height = base_height * initial_scale.1.abs();

                        // Calculate blur radius from blur_params
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
                    } else if needs_any_effect {
                        // Use UnifiedEffectMaterial for combined effects (mask/wipe/stretch + optional blur)
                        // For effect layers, Transform.scale is reset to Vec3::ONE in spawn_layer_entity
                        // So we must bake the scale into the mesh dimensions
                        let scaled_width = base_width * initial_scale.0.abs();
                        let scaled_height = base_height * initial_scale.1.abs();

                        // Don't expand mesh statically for blur - blur will work within original bounds
                        // For pixelate, expand mesh so edge blocks aren't clipped at layer boundary
                        let blur_expansion = pixelate_expansion;

                        // Use initial stretch mesh bounds if provided (to prevent first frame jump)
                        let mesh = if let Some((min_x, max_x, min_y, max_y)) =
                            initial_stretch_mesh_bounds
                        {
                            // Create mesh with stretch-expanded bounds
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
                            meshes.add(new_mesh)
                        } else {
                            create_anchored_rectangle_with_blur(
                                meshes,
                                scaled_width,
                                scaled_height,
                                anchor,
                                blur_expansion,
                            )
                        };

                        // Pass blur expansion info to material via blur_params.w
                        // This allows shader to correctly map UVs for the expanded mesh
                        let blur_params_with_expansion = blur_params.map(|mut bp| {
                            bp.y = scaled_width;
                            bp.z = scaled_height;
                            bp.w = blur_expansion;
                            bp
                        });

                        // Calculate mesh size for stretch bounds
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

                        // Transform.scale is Vec3::ONE for effect layers, scale is baked into mesh
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
                    } else {
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
                    }
                }
            } else if let Some(wp) = white_pixel {
                bevy::log::trace!(
                    "[Visual] Spawning fill sprite '{}' with white_pixel, color fill",
                    label
                );
                let color = extract_fill_color(fill_color, false);
                if needs_any_effect {
                    // Use initial stretch mesh bounds if provided (to prevent first frame jump)
                    let mesh =
                        if let Some((min_x, max_x, min_y, max_y)) = initial_stretch_mesh_bounds {
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
                            meshes.add(new_mesh)
                        } else {
                            create_anchored_rectangle(meshes, base_width, base_height, anchor)
                        };

                    // Calculate mesh size for stretch bounds
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
                        "[Visual] Spawned fill sprite '{}' with unified effect: base_size=({:.1},{:.1}), has_stretch_bounds={}",
                        label,
                        base_width,
                        base_height,
                        initial_stretch_mesh_bounds.is_some()
                    );
                } else {
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
                }
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

            if let Some(handle) = images.get(image_uri) {
                if needs_any_effect {
                    // Use initial stretch mesh bounds if provided (to prevent first frame jump)
                    let mesh =
                        if let Some((min_x, max_x, min_y, max_y)) = initial_stretch_mesh_bounds {
                            // Create mesh with stretch-expanded bounds
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
                            meshes.add(new_mesh)
                        } else {
                            // Create mesh with BASE dimensions (not scaled)
                            // Transform.scale will handle the actual scaling
                            create_anchored_rectangle(meshes, base_width, base_height, anchor)
                        };

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
                } else {
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
        }
        AmLayerSpec::Text {
            content,
            font_name,
            font_size,
            align,
            fill_color,
            wrap_width,
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

            // Set anchor based on alignment
            let anchor = match align.as_str() {
                "right" => bevy::sprite::Anchor(Vec2::new(0.5, 0.0)),
                "center" => bevy::sprite::Anchor::CENTER,
                _ => bevy::sprite::Anchor(Vec2::new(-0.5, 0.0)),
            };

            commands.entity(entity).insert((
                Text2d::new(content.clone()),
                TextFont {
                    font,
                    font_size: *font_size,
                    ..default()
                },
                TextLayout::new_with_justify(justify),
                TextColor(color),
                bevy::text::TextBounds::new_horizontal(wrap_width * 3.0),
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

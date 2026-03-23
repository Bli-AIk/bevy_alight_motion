use bevy::asset::Assets;
use bevy::prelude::*;
use std::collections::HashMap;

use crate::scene::{AmMaskInfo, AmPaletteMapParams, AmVisualSpawned};

use super::material::create_unified_material;
use super::mesh::create_anchored_rectangle;

#[expect(clippy::too_many_arguments)] // reason: image visuals need mesh/material/effect fan-in
pub(super) fn handle_image_visual(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    unified_materials: &mut Assets<crate::masked_sprite::UnifiedEffectMaterial>,
    entity: Entity,
    image_uri: &str,
    width: f32,
    height: f32,
    anchor: &bevy::sprite::Anchor,
    images: &HashMap<String, Handle<Image>>,
    label: &str,
    mask_info: &Option<AmMaskInfo>,
    palette_params: Option<&AmPaletteMapParams>,
    wipe_params: Option<Vec4>,
    stretch_params: Option<Vec4>,
    blur_params: Option<Vec4>,
    size_scale: f32,
    initial_mesh_offset: Option<Vec4>,
    initial_stretch_mesh_bounds: Option<(f32, f32, f32, f32)>,
    fit_scale: f32,
    global_time_ms: u64,
    replace_color_params: Option<(Vec4, Vec4, Vec4, Vec4)>,
    needs_any_effect: bool,
) {
    use crate::masked_sprite::UnifiedEffectMarker;

    let base_width = width * size_scale;
    let base_height = height * size_scale;

    if let Some(handle) = images.get(image_uri)
        && needs_any_effect
    {
        let stretch_mesh = initial_stretch_mesh_bounds.map(|(min_x, max_x, min_y, max_y)| {
            super::super::visual_helpers::create_stretch_bounds_mesh(
                meshes, min_x, max_x, min_y, max_y,
            )
        });
        let mesh = stretch_mesh
            .unwrap_or_else(|| create_anchored_rectangle(meshes, base_width, base_height, anchor));

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

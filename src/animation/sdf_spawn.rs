//! # sdf_spawn.rs
//!
//! # SDF 形状生成模块
//!
//! SDF (Signed Distance Field) shape spawning for stroked shapes.
//! Contains spawn_sdf_visual function for creating SDF rendered shapes.
//!
//! SDF（有符号距离场）形状生成，用于带描边的形状。
//! 包含用于创建 SDF 渲染形状的 spawn_sdf_visual 函数。

use bevy::asset::Assets;
use bevy::prelude::*;

use crate::scene::{AmLayerMarker, AmMaskInfo, AmVisualSpawned};
use crate::sdf_material::{SdfMaterial, SdfShapeType, pack_color};

use super::components::{AmSdfParams, AmSdfShapeParent};
use super::visual::extract_fill_color;

#[allow(clippy::too_many_arguments)]
pub fn spawn_sdf_visual(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    sdf_materials: &mut Assets<SdfMaterial>,
    parent_entity: Entity,
    fill_color: &Option<crate::schema::AmFillColor>,
    stroke_color_value: &str,
    stroke_width: f32,
    stroke_join: &str,
    width: f32,
    height: f32,
    pivot_x: f32,
    pivot_y: f32,
    shape_type: &str,
    marker: &AmLayerMarker,
    initial_scale: (f32, f32),
    mask_info: &Option<AmMaskInfo>,
    global_time_ms: u64, // Current playback time for mask initialization
    fit_scale: f32,      // Scale factor for mask coordinates
) {
    let fill = extract_fill_color(fill_color);
    let stroke = if !stroke_color_value.is_empty() {
        crate::schema::parse_color(stroke_color_value)
            .map(|c| Color::srgba(c[0], c[1], c[2], c[3]))
            .unwrap_or(Color::WHITE)
    } else {
        Color::WHITE
    };

    // Target dimensions from shape properties (base size before animation scale)
    let target_half_width = width / 2.0;
    let target_half_height = height / 2.0;

    // Select shape type based on AM shape type and stroke join
    // .circle -> Circle/Ellipse
    // .rect -> Box variants based on join type
    let sdf_shape_type = if shape_type == ".circle" {
        SdfShapeType::Circle // or Ellipse if w != h
    } else {
        match stroke_join {
            "miter" => SdfShapeType::BoxMiter,
            "round" => SdfShapeType::BoxRound,
            "bevel" | "" => SdfShapeType::BoxBevel,
            _ => SdfShapeType::BoxRound,
        }
    };

    bevy::log::trace!("[SDF] Spawning {} with join='{}'", shape_type, stroke_join);

    // Get base stroke alpha for animation
    let base_stroke_alpha = stroke.to_srgba().alpha;
    // Pack stroke color into u32 bits stored as f32
    let packed_stroke = pack_color(stroke);

    // Frame size for rendering - must be large enough for the largest expected shape.
    // Since we scale via params, the frame needs to accommodate the max size + stroke.
    // We use a conservative estimate based on the target size * reasonable max scale factor.
    // AM animations typically don't exceed 10x scale, so use that as a safety margin.
    let max_scale_factor = 10.0;
    let frame_half =
        (target_half_width.max(target_half_height) * max_scale_factor) + stroke_width * 2.0;
    let frame_size = frame_half * 2.0;

    // Calculate initial translation based on pivot and initial scale (with Y-flip for Bevy)
    // Pivot (px, py) in AM means Center is at (-px, -py) relative to Pivot.
    // Bevy Y is flipped, so Center Y is -(-py) = py.
    // Apply initial scale to the pivot offset so the child is correctly positioned from the start.
    let initial_translation = Vec3::new(-pivot_x * initial_scale.0, pivot_y * initial_scale.1, 0.0);

    // Create quad mesh for SDF rendering
    let mesh = meshes.add(Rectangle::new(frame_size, frame_size));

    // Convert fill color to LinearRgba for the material
    let fill_linear = fill.to_linear();

    // Convert shape type to f32 for the shader
    let shape_type_f32 = match sdf_shape_type {
        SdfShapeType::BoxRound => 0.0,
        SdfShapeType::BoxMiter => 1.0,
        SdfShapeType::BoxBevel => 2.0,
        SdfShapeType::Circle => 3.0,
    };

    // Create SDF material - with or without mask
    // Use first active mask at current playback time
    // Apply fit_scale to mask coordinates to convert from canvas space to world space
    let active_mask = mask_info
        .as_ref()
        .and_then(|m| m.get_active_mask(global_time_ms));
    let material = if let Some(mask) = active_mask {
        // Scale mask center and half_size by fit_scale for world coordinate space
        let scaled_center = mask.center * fit_scale;
        let scaled_half_size = mask.half_size * fit_scale * mask.scale;
        sdf_materials.add(SdfMaterial::new_with_mask_and_frame_half(
            sdf_shape_type,
            target_half_width,
            target_half_height,
            fill,
            stroke_width,
            stroke,
            scaled_center,
            scaled_half_size,
            mask.is_circle,
            mask.is_exclude,
            frame_half,
        ))
    } else {
        sdf_materials.add(SdfMaterial::from_linear(
            fill_linear,
            Vec4::new(
                target_half_width,
                target_half_height,
                stroke_width,
                packed_stroke,
            ),
            shape_type_f32,
            frame_half,
        ))
    };

    // Spawn SDF entity with Material2d components
    let sdf_entity = commands
        .spawn((
            Name::new(format!("SdfShape[{}]: {}", marker.id, marker.label)),
            Transform::from_translation(initial_translation),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            Mesh2d(mesh),
            MeshMaterial2d(material),
            // Store base params for animation
            AmSdfParams {
                base_half_width: target_half_width,
                base_half_height: target_half_height,
                stroke_width,
                packed_stroke,
                base_stroke_alpha,
                base_pivot_x: pivot_x,
                base_pivot_y: pivot_y,
            },
        ))
        .id();

    // Add as child and mark parent
    commands
        .entity(parent_entity)
        .add_child(sdf_entity)
        .insert((AmVisualSpawned, AmSdfShapeParent));

    bevy::log::trace!(
        "[SDF] Created shape for '{}': size={}x{}, stroke_width={}, frame={}, pivot=({:.1},{:.1}), initial_scale=({:.2},{:.2}), initial_translation=({:.1},{:.1},{:.1})",
        marker.label,
        width,
        height,
        stroke_width,
        frame_size,
        pivot_x,
        pivot_y,
        initial_scale.0,
        initial_scale.1,
        initial_translation.x,
        initial_translation.y,
        initial_translation.z
    );
}

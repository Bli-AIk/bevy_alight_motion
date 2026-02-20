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
    stroke_direction: &str,
    border2_color_value: &str,
    border2_width: f32,
    border2_direction: &str,
    width: f32,
    height: f32,
    pivot_x: f32,
    pivot_y: f32,
    shape_type: &str,
    marker: &AmLayerMarker,
    initial_scale: (f32, f32),
    mask_info: &Option<AmMaskInfo>,
    global_time_ms: u64,
    fit_scale: f32,
    no_fill: bool,
    shape_extra: Vec4,
    shape_extra2: Vec4,
    shape_extra3: Vec4,
    shape_extra4: Vec4,
    shape_extra5: Vec4,
    shape_extra6: Vec4,
    shape_extra7: Vec4,
    gradient_type: u8,
    gradient_start_color: Vec4,
    gradient_end_color: Vec4,
    gradient_points: Vec4,
) {
    let fill = extract_fill_color(fill_color, no_fill);
    let stroke = if !stroke_color_value.is_empty() {
        crate::schema::parse_color(stroke_color_value)
            .map(|c| Color::srgba(c[0], c[1], c[2], c[3]))
            .unwrap_or(Color::WHITE)
    } else {
        Color::WHITE
    };

    // Border direction mode: 0.0=centered, 1.0=inside, -1.0=outside
    let border_mode = match stroke_direction {
        "inside" => 1.0_f32,
        "outside" => -1.0_f32,
        _ => 0.0_f32, // "centered" or empty
    };

    // Border 2 direction
    let border2_mode = match border2_direction {
        "inside" => 1.0_f32,
        "outside" => -1.0_f32,
        _ => 0.0_f32,
    };
    let border2_color = if !border2_color_value.is_empty() {
        crate::schema::parse_color(border2_color_value)
            .map(|c| Color::srgba(c[0], c[1], c[2], c[3]))
            .unwrap_or(Color::WHITE)
    } else {
        Color::WHITE
    };
    let packed_border2 = if border2_width > 0.0 {
        pack_color(border2_color)
    } else {
        0.0
    };

    // Target dimensions from shape properties (base size before animation scale)
    let target_half_width = width / 2.0;
    let target_half_height = height / 2.0;

    // Select shape type based on AM shape type and stroke join
    let sdf_shape_type = match shape_type {
        ".circle" => SdfShapeType::Circle,
        ".roundrect" => SdfShapeType::RoundRect,
        ".poly" => SdfShapeType::Polygon,
        ".star" => SdfShapeType::Star,
        ".pie" => SdfShapeType::Pie,
        ".plus" => SdfShapeType::Plus,
        ".multifoil" => SdfShapeType::Multifoil,
        ".line" => SdfShapeType::Line,
        ".arc" => SdfShapeType::Arc,
        ".triangle" => SdfShapeType::Triangle,
        ".quad" => SdfShapeType::Quad,
        ".penta" => SdfShapeType::Penta,
        _ if shape_type.is_empty() || shape_type == ".path" => SdfShapeType::Path,
        _ => {
            // Default: rect variants based on join type
            match stroke_join {
                "miter" => SdfShapeType::BoxMiter,
                "round" => SdfShapeType::BoxRound,
                "bevel" | "" => SdfShapeType::BoxBevel,
                _ => SdfShapeType::BoxRound,
            }
        }
    };

    bevy::log::trace!("[SDF] Spawning {} with join='{}'", shape_type, stroke_join);
    bevy::log::debug!(
        "[SDF] '{}': fill={:?}, stroke={:?}, stroke_width={}, no_fill={}",
        marker.label,
        fill,
        stroke,
        stroke_width,
        no_fill
    );

    // Get base stroke alpha for animation
    let base_stroke_alpha = stroke.to_srgba().alpha;
    // Pack stroke color into u32 bits stored as f32
    let packed_stroke = pack_color(stroke);

    // Frame size for rendering - must be large enough for the largest expected shape.
    // For shapes with radius-based sizing (poly, star, pie, etc.), use shape_extra params.
    let shape_extent = match sdf_shape_type {
        SdfShapeType::Polygon | SdfShapeType::Pie | SdfShapeType::Arc => shape_extra.y, // radius
        SdfShapeType::Star | SdfShapeType::Multifoil => shape_extra.y.max(shape_extra.z), // max(outer, inner)
        SdfShapeType::Line => {
            let dx = shape_extra.z - shape_extra.x;
            let dy = shape_extra.w - shape_extra.y;
            (dx * dx + dy * dy).sqrt() * 0.5 + 50.0
        }
        SdfShapeType::Triangle | SdfShapeType::Quad | SdfShapeType::Penta | SdfShapeType::Path => {
            // Max extent from all points
            let mut max_r: f32 = 0.0;
            let pts = [
                shape_extra.x,
                shape_extra.y,
                shape_extra.z,
                shape_extra.w,
                shape_extra2.x,
                shape_extra2.y,
                shape_extra2.z,
                shape_extra2.w,
                shape_extra3.x,
                shape_extra3.y,
                shape_extra3.z,
                shape_extra3.w,
                shape_extra4.x,
                shape_extra4.y,
                shape_extra4.z,
                shape_extra4.w,
                shape_extra5.x,
                shape_extra5.y,
                shape_extra5.z,
                shape_extra5.w,
                shape_extra6.x,
                shape_extra6.y,
                shape_extra6.z,
                shape_extra6.w,
                shape_extra7.x,
                shape_extra7.y,
            ];
            let mut i = 0;
            while i < 26 {
                let r = (pts[i] * pts[i] + pts[i + 1] * pts[i + 1]).sqrt();
                if r > max_r {
                    max_r = r;
                }
                i += 2;
            }
            max_r + 10.0
        }
        _ => target_half_width.max(target_half_height),
    };
    let max_scale_factor = 100.0;
    let frame_half = (shape_extent.max(target_half_width.max(target_half_height))
        * max_scale_factor)
        + stroke_width * 2.0;
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

    let shape_type_f32 = sdf_shape_type.to_f32();

    bevy::log::warn!(
        "[SDF_SPAWN] '{}': shape_type={:?}({}), half=({:.1},{:.1}), frame_half={:.1}, extra=({:.1},{:.1},{:.1},{:.1}), fill=({:.3},{:.3},{:.3},{:.3})",
        marker.label,
        sdf_shape_type,
        shape_type_f32,
        target_half_width,
        target_half_height,
        frame_half,
        shape_extra.x,
        shape_extra.y,
        shape_extra.z,
        shape_extra.w,
        fill_linear.red,
        fill_linear.green,
        fill_linear.blue,
        fill_linear.alpha
    );

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
        bevy::log::info!(
            "[SDF_SPAWN] '{}': Creating material with mask center=({:.1},{:.1}), half_size=({:.1},{:.1}), fit_scale={:.2}, original_center=({:.1},{:.1})",
            marker.label,
            scaled_center.x,
            scaled_center.y,
            scaled_half_size.x,
            scaled_half_size.y,
            fit_scale,
            mask.center.x,
            mask.center.y
        );
        let mut mat = SdfMaterial::new_with_mask_and_frame_half(
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
        );
        mat.uniform_data.border_mode = border_mode;
        mat.uniform_data.border2_width = border2_width;
        mat.uniform_data.border2_packed_color = packed_border2;
        mat.uniform_data.border2_mode = border2_mode;
        mat.uniform_data.shape_extra = shape_extra;
        mat.uniform_data.shape_extra2 = shape_extra2;
        mat.uniform_data.shape_extra3 = shape_extra3;
        mat.uniform_data.shape_extra4 = shape_extra4;
        mat.uniform_data.shape_extra5 = shape_extra5;
        mat.uniform_data.shape_extra6 = shape_extra6;
        mat.uniform_data.shape_extra7 = shape_extra7;
        mat.uniform_data.base_half_width = target_half_width;
        mat.uniform_data.gradient_config = Vec4::new(gradient_type as f32, 0.0, 0.0, 0.0);
        mat.uniform_data.gradient_start_color = gradient_start_color;
        mat.uniform_data.gradient_end_color = gradient_end_color;
        mat.uniform_data.gradient_points = gradient_points;
        sdf_materials.add(mat)
    } else {
        let mut mat = SdfMaterial::from_linear(
            fill_linear,
            Vec4::new(
                target_half_width,
                target_half_height,
                stroke_width,
                packed_stroke,
            ),
            shape_type_f32,
            frame_half,
        );
        mat.uniform_data.border_mode = border_mode;
        mat.uniform_data.border2_width = border2_width;
        mat.uniform_data.border2_packed_color = packed_border2;
        mat.uniform_data.border2_mode = border2_mode;
        mat.uniform_data.shape_extra = shape_extra;
        mat.uniform_data.shape_extra2 = shape_extra2;
        mat.uniform_data.shape_extra3 = shape_extra3;
        mat.uniform_data.shape_extra4 = shape_extra4;
        mat.uniform_data.shape_extra5 = shape_extra5;
        mat.uniform_data.shape_extra6 = shape_extra6;
        mat.uniform_data.shape_extra7 = shape_extra7;
        mat.uniform_data.base_half_width = target_half_width;
        mat.uniform_data.gradient_config = Vec4::new(gradient_type as f32, 0.0, 0.0, 0.0);
        mat.uniform_data.gradient_start_color = gradient_start_color;
        mat.uniform_data.gradient_end_color = gradient_end_color;
        mat.uniform_data.gradient_points = gradient_points;
        sdf_materials.add(mat)
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
                border2_width,
                border2_packed_color: packed_border2,
                border2_mode,
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

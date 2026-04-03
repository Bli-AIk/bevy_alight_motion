//! Contains geometry helpers shared by RTT synchronization code.
//! It models visible rectangles, rectangle intersection, viewport transforms, and
//! texture resizing helpers that the RTT camera systems use to compute effective
//! embed bounds.
//!
//! 存放 RTT 同步代码共用的几何辅助函数。它描述可见矩形、矩形相交、
//! 视口变换以及纹理缩放等逻辑，供 RTT 相机系统计算嵌套场景的有效显示边界。

use bevy::math::Affine3A;
use bevy::prelude::*;
use bevy::render::render_resource::Extent3d;
use bevy::sprite::Anchor;

use crate::effects::EmbedSceneRtt;

#[derive(Clone, Copy, Debug)]
pub(super) struct EmbedVisibleRect {
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
}

impl EmbedVisibleRect {
    pub(super) fn width(self) -> f32 {
        (self.right - self.left).max(0.0)
    }

    pub(super) fn height(self) -> f32 {
        (self.top - self.bottom).max(0.0)
    }

    pub(super) fn center(self) -> Vec2 {
        Vec2::new(
            (self.left + self.right) * 0.5,
            (self.bottom + self.top) * 0.5,
        )
    }
}

pub(super) fn scene_local_rect(width: f32, height: f32) -> EmbedVisibleRect {
    EmbedVisibleRect {
        left: -width * 0.5,
        right: width * 0.5,
        bottom: -height * 0.5,
        top: height * 0.5,
    }
}

fn viewport_world_rect(width: f32, height: f32) -> EmbedVisibleRect {
    scene_local_rect(width, height)
}

fn transform_rect_enclosing(rect: EmbedVisibleRect, affine: Affine3A) -> EmbedVisibleRect {
    let corners = [
        affine.transform_point3(Vec3::new(rect.left, rect.top, 0.0)),
        affine.transform_point3(Vec3::new(rect.right, rect.top, 0.0)),
        affine.transform_point3(Vec3::new(rect.right, rect.bottom, 0.0)),
        affine.transform_point3(Vec3::new(rect.left, rect.bottom, 0.0)),
    ];

    let mut min = corners[0].truncate();
    let mut max = min;
    for point in corners.iter().skip(1) {
        let point = point.truncate();
        min = min.min(point);
        max = max.max(point);
    }

    EmbedVisibleRect {
        left: min.x,
        right: max.x,
        bottom: min.y,
        top: max.y,
    }
}

fn intersect_rect(a: EmbedVisibleRect, b: EmbedVisibleRect) -> Option<EmbedVisibleRect> {
    let left = a.left.max(b.left);
    let right = a.right.min(b.right);
    let bottom = a.bottom.max(b.bottom);
    let top = a.top.min(b.top);

    (left < right && bottom < top).then_some(EmbedVisibleRect {
        left,
        right,
        bottom,
        top,
    })
}

fn clamp_rect(rect: EmbedVisibleRect, bounds: EmbedVisibleRect) -> EmbedVisibleRect {
    EmbedVisibleRect {
        left: rect.left.clamp(bounds.left, bounds.right),
        right: rect.right.clamp(bounds.left, bounds.right),
        bottom: rect.bottom.clamp(bounds.bottom, bounds.top),
        top: rect.top.clamp(bounds.bottom, bounds.top),
    }
}

pub(super) fn transformed_rect_edge_lengths(rect: EmbedVisibleRect, affine: Affine3A) -> Vec2 {
    let top_left = affine.transform_point3(Vec3::new(rect.left, rect.top, 0.0));
    let top_right = affine.transform_point3(Vec3::new(rect.right, rect.top, 0.0));
    let bottom_right = affine.transform_point3(Vec3::new(rect.right, rect.bottom, 0.0));

    Vec2::new(
        top_right.distance(top_left).max(1.0),
        bottom_right.distance(top_right).max(1.0),
    )
}

pub(super) fn compute_scene_visible_rect(
    scene_width: f32,
    scene_height: f32,
    dynamic_resolution: bool,
    embed_global: &GlobalTransform,
    animated: &crate::animation::AmAnimated,
) -> EmbedVisibleRect {
    let full_rect = scene_local_rect(scene_width, scene_height);
    if std::env::var_os("AM_DISABLE_DYNAMIC_RESOLUTION_CROP").is_some() {
        return full_rect;
    }

    if !dynamic_resolution {
        return full_rect;
    }

    let viewport = viewport_world_rect(animated.canvas_width, animated.canvas_height);
    let affine = embed_global.affine();
    let visible_world = intersect_rect(viewport, transform_rect_enclosing(full_rect, affine));
    let Some(visible_world) = visible_world else {
        return full_rect;
    };

    let visible_local = clamp_rect(
        transform_rect_enclosing(visible_world, affine.inverse()),
        full_rect,
    );

    if visible_local.width() <= 1e-3 || visible_local.height() <= 1e-3 {
        full_rect
    } else {
        visible_local
    }
}

pub(super) fn compute_embed_visible_rect(
    rtt: &EmbedSceneRtt,
    embed_global: &GlobalTransform,
    animated: &crate::animation::AmAnimated,
) -> EmbedVisibleRect {
    compute_scene_visible_rect(
        rtt.scene_width,
        rtt.scene_height,
        rtt.dynamic_resolution,
        embed_global,
        animated,
    )
}

fn write_rect_mesh(mesh: &mut Mesh, rect: EmbedVisibleRect) {
    write_rect_mesh_with_uv(mesh, rect, [0.0, 1.0, 0.0, 1.0]);
}

fn rect_uv_bounds(rect: EmbedVisibleRect, full_rect: EmbedVisibleRect) -> [f32; 4] {
    let width = full_rect.width().max(1.0);
    let height = full_rect.height().max(1.0);
    let uv_left = ((rect.left - full_rect.left) / width).clamp(0.0, 1.0);
    let uv_right = ((rect.right - full_rect.left) / width).clamp(0.0, 1.0);
    // Bevy mesh UVs use v=0 at the top of the texture. Match the sprite rect path so
    // dynamicResolution crops map to the same top-origin texture coordinates.
    let uv_top = ((full_rect.top - rect.top) / height).clamp(0.0, 1.0);
    let uv_bottom = ((full_rect.top - rect.bottom) / height).clamp(0.0, 1.0);
    [uv_left, uv_right, uv_top, uv_bottom]
}

fn rect_pixel_bounds(
    rect: EmbedVisibleRect,
    full_rect: EmbedVisibleRect,
    texture_size: Vec2,
) -> Rect {
    let [uv_left, uv_right, uv_top, uv_bottom] = rect_uv_bounds(rect, full_rect);
    // Round to integer pixel boundaries so that nested RTT sprites sample at
    // exact pixel positions.  Without rounding, fractional sprite-rect coords
    // cause sub-pixel interpolation that compounds across nesting levels
    // (≈3 px shift per level in 4-level revenge embeds).
    Rect {
        // Sprite::rect expects top-origin pixel coordinates. rect_uv_bounds() already
        // returns top-origin V values, so applying `1.0 - uv` here would flip Y twice.
        min: Vec2::new(
            (uv_left * texture_size.x).round(),
            (uv_top * texture_size.y).round(),
        ),
        max: Vec2::new(
            (uv_right * texture_size.x).round(),
            (uv_bottom * texture_size.y).round(),
        ),
    }
}

fn write_rect_mesh_with_uv(mesh: &mut Mesh, rect: EmbedVisibleRect, uv_rect: [f32; 4]) {
    let vertices = vec![
        [rect.left, rect.bottom, 0.0],
        [rect.right, rect.bottom, 0.0],
        [rect.right, rect.top, 0.0],
        [rect.left, rect.top, 0.0],
    ];
    let normals = vec![
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
    ];
    let [uv_left, uv_right, uv_top, uv_bottom] = uv_rect;
    let uvs = vec![
        [uv_left, uv_bottom],
        [uv_right, uv_bottom],
        [uv_right, uv_top],
        [uv_left, uv_top],
    ];
    let indices = vec![0u32, 1, 2, 0, 2, 3];

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));
}

pub(super) fn sync_dynamic_resolution_sprite(
    commands: &mut Commands,
    embed_entity: Entity,
    sprite: &mut Sprite,
    anchor: Option<Mut<Anchor>>,
    rtt: &EmbedSceneRtt,
    visible_rect: EmbedVisibleRect,
    full_rect: EmbedVisibleRect,
    visible_size: Vec2,
    local_center: Vec2,
    texture_size: Vec2,
) {
    sprite.custom_size = Some(visible_size);
    let texture_matches_visible = (texture_size.x - visible_size.x).abs() <= 0.5
        && (texture_size.y - visible_size.y).abs() <= 0.5;
    let sprite_rect = if rtt.dynamic_resolution && !texture_matches_visible {
        Some(rect_pixel_bounds(visible_rect, full_rect, texture_size))
    } else {
        None
    };
    sprite.rect = sprite_rect;

    let custom_anchor = Anchor(Vec2::new(
        -local_center.x / visible_size.x,
        -local_center.y / visible_size.y,
    ));
    if let Some(mut anchor) = anchor {
        *anchor = custom_anchor;
    } else {
        commands.entity(embed_entity).insert(custom_anchor);
    }
}

pub(super) fn sync_dynamic_resolution_mesh(
    meshes: &mut Assets<Mesh>,
    mesh2d: &Mesh2d,
    rtt: &EmbedSceneRtt,
    visible_rect: EmbedVisibleRect,
    full_rect: EmbedVisibleRect,
    texture_matches_visible: bool,
) {
    let Some(mesh) = meshes.get_mut(&mesh2d.0) else {
        return;
    };

    if rtt.dynamic_resolution && !texture_matches_visible {
        write_rect_mesh_with_uv(mesh, visible_rect, rect_uv_bounds(visible_rect, full_rect));
    } else {
        write_rect_mesh(mesh, visible_rect);
    }
}

pub(super) fn resize_render_texture(
    images: &mut Assets<Image>,
    texture: &Handle<Image>,
    new_extent: Extent3d,
) {
    // Check size via immutable access first to avoid emitting a spurious
    // AssetEvent::Modified from get_mut(). Without this guard every active
    // embed texture is re-extracted by the render pipeline every frame, even
    // when the size hasn't changed.
    let needs_resize = images
        .get(texture)
        .is_some_and(|img| img.texture_descriptor.size != new_extent);
    if !needs_resize {
        return;
    }
    if let Some(image) = images.get_mut(texture) {
        image.resize(new_extent);
    }
}

pub(super) fn propagate_to_descendants(
    commands: &mut Commands,
    embed_entity: Entity,
    children: &Children,
    target_layer: &bevy::camera::visibility::RenderLayers,
    children_query: &Query<&Children>,
    render_layers_query: &Query<&bevy::camera::visibility::RenderLayers>,
    visibility_query: &Query<&Visibility>,
    force_hidden_query: &Query<(), With<crate::scene::AmForceHidden>>,
    non_embed_query: &Query<
        Entity,
        (
            Without<EmbedSceneRtt>,
            Without<super::types::RenderStrategy>,
        ),
    >,
) -> u32 {
    let mut updates = 0;

    for child_entity in children.iter() {
        let layer_needs_update = match render_layers_query.get(child_entity) {
            Ok(current) => current != target_layer,
            Err(_) => true,
        };
        let vis_needs_update = match visibility_query.get(child_entity) {
            Ok(Visibility::Hidden) => true,
            Err(_) => false,
            _ => false,
        } && force_hidden_query.get(child_entity).is_err();

        if layer_needs_update || vis_needs_update {
            let mut entity_commands = commands.entity(child_entity);
            if layer_needs_update {
                entity_commands.insert(target_layer.clone());
            }
            if vis_needs_update {
                entity_commands.insert(Visibility::Inherited);
            }
            updates += 1;
            bevy::log::trace!(
                "[PropagateChildren] Updated child {:?} of embed {:?}",
                child_entity,
                embed_entity
            );
        }

        let mut to_process: Vec<Entity> = Vec::new();
        if non_embed_query.get(child_entity).is_ok()
            && let Ok(grandchildren) = children_query.get(child_entity)
        {
            to_process.extend(grandchildren.to_vec());
        }

        while let Some(entity) = to_process.pop() {
            if non_embed_query.get(entity).is_err() {
                continue;
            }

            let layer_needs_update = match render_layers_query.get(entity) {
                Ok(current) => current != target_layer,
                Err(_) => true,
            };
            let vis_needs_update = match visibility_query.get(entity) {
                Ok(Visibility::Hidden) => true,
                Err(_) => false,
                _ => false,
            } && force_hidden_query.get(entity).is_err();

            if layer_needs_update {
                commands.entity(entity).insert(target_layer.clone());
            }
            if vis_needs_update {
                commands.entity(entity).insert(Visibility::Inherited);
            }
            if layer_needs_update || vis_needs_update {
                updates += 1;
            }

            if let Ok(grandchildren) = children_query.get(entity) {
                to_process.extend(grandchildren.to_vec());
            }
        }
    }

    updates
}

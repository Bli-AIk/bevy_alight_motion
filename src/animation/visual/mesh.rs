//! Creates anchored rectangle meshes for runtime visuals.
//! 为运行时可视对象创建带锚点的矩形网格。
//!
//! Imported layers often use sprite-style quads, but their authored anchor and blur expansion rules
//! do not line up with Bevy's defaults. This file centralizes mesh creation so every image, media,
//! and sprite-shape visual uses the same anchor math and optional blur padding.
//! 导入的图层大多会落成 sprite 风格的四边形，但作者设置的锚点和模糊外扩规则并不等同于 Bevy 默认行为。
//! 这个文件把 mesh 创建逻辑统一起来，让所有图片、媒体和 SpriteShape 可视对象都使用同一套锚点计算与
//! 可选 blur padding。

use bevy::asset::Assets;
use bevy::prelude::*;

pub(super) fn create_anchored_rectangle(
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

pub(super) fn create_anchored_rectangle_with_blur(
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
    let offset_x = -anchor_vec.x * width;
    let offset_y = -anchor_vec.y * height;

    let half_w = width / 2.0;
    let half_h = height / 2.0;

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

    let uv_expand_x = blur_expansion / width;
    let uv_expand_y = blur_expansion / height;
    let uvs = vec![
        [-uv_expand_x, 1.0 + uv_expand_y],
        [1.0 + uv_expand_x, 1.0 + uv_expand_y],
        [1.0 + uv_expand_x, -uv_expand_y],
        [-uv_expand_x, -uv_expand_y],
    ];

    let indices = vec![0, 1, 2, 0, 2, 3];

    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::RENDER_WORLD | bevy::asset::RenderAssetUsages::MAIN_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));

    meshes.add(mesh)
}

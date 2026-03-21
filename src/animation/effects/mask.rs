//! Mask-related effect systems: mesh blur helper and unified mask system.

use bevy::prelude::*;

mod compute;
mod embed;
mod repeat;
mod system;
mod trace;

pub use system::update_unified_mask_system;

/// Helper function to update mesh vertices and UVs for dynamic blur expansion.
/// This allows the blur glow/halo effect to extend beyond original image boundaries.
/// Note: This assumes CENTER anchor since anchor info is not stored in AmAnimated.
#[allow(dead_code)]
fn update_mesh_for_blur(
    mesh: &mut Mesh,
    width: f32,
    height: f32,
    _anchor: &bevy::sprite::Anchor, // Reserved for future use
    blur_expansion: f32,
) {
    let offset_x = 0.0;
    let offset_y = 0.0;

    let half_w = width / 2.0;
    let half_h = height / 2.0;

    let vertices: Vec<[f32; 3]> = vec![
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

    let uv_expand_x = if width > 0.0 {
        blur_expansion / width
    } else {
        0.0
    };
    let uv_expand_y = if height > 0.0 {
        blur_expansion / height
    } else {
        0.0
    };
    let uvs: Vec<[f32; 2]> = vec![
        [-uv_expand_x, 1.0 + uv_expand_y],
        [1.0 + uv_expand_x, 1.0 + uv_expand_y],
        [1.0 + uv_expand_x, -uv_expand_y],
        [-uv_expand_x, -uv_expand_y],
    ];

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
}

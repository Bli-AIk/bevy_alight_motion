//! Repeat, linear repeat, and radial repeat effect processing helpers.

use bevy::prelude::*;

mod java_random;
mod linear;
mod radial;
mod standard;

pub(crate) use java_random::compute_java_random_state_packed;
pub(crate) use linear::compute_sdf_linear_repeat_displacement;
pub(super) use linear::process_linear_repeat_effect;
pub(super) use radial::process_radial_repeat_effect;
pub(super) use standard::process_repeat_effect;

pub(super) fn overwrite_repeat_mesh(
    meshes: &mut Assets<Mesh>,
    mesh2d: &bevy::mesh::Mesh2d,
    vertices: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
) {
    let Some(mesh) = meshes.get_mut(&mesh2d.0) else {
        return;
    };

    *mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::RENDER_WORLD | bevy::asset::RenderAssetUsages::MAIN_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));
}

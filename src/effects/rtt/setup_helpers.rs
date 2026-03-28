use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;

use super::{AmGroupFill, GroupFillType};

#[derive(Component)]
pub(crate) struct PendingGroupFillTextureRefresh(pub(crate) u8);

pub(super) fn trace_rtt_setup_enabled(layer_id: u64) -> bool {
    std::env::var_os("AM_TRACE_RTT_SETUP_IDS")
        .and_then(|value| value.into_string().ok())
        .is_some_and(|ids| {
            ids.split(',')
                .filter_map(|value| value.trim().parse::<u64>().ok())
                .any(|id| id == layer_id)
        })
}

pub(super) fn composite_camera_order(
    embed_entity: Entity,
    render_layer: usize,
    parent_query: &Query<&ChildOf>,
    layer_spec_query: &Query<&crate::scene::AmLayerSpec>,
) -> isize {
    let mut embed_depth = 0_isize;
    let mut current = embed_entity;

    while let Ok(child_of) = parent_query.get(current) {
        let parent = child_of.parent();
        if layer_spec_query
            .get(parent)
            .is_ok_and(|spec| matches!(spec, crate::scene::AmLayerSpec::EmbedScene))
        {
            embed_depth += 1;
        }
        current = parent;
    }

    if embed_depth == 0 {
        -(render_layer as isize)
    } else {
        -((embed_depth + 1) * 100 + render_layer as isize)
    }
}

pub(super) fn dynamic_render_layer(layer: usize) -> RenderLayers {
    RenderLayers::from_layers(&[layer])
}

pub(super) fn selected_embed_rtt_format() -> TextureFormat {
    match std::env::var("AM_EMBED_RTT_FORMAT").ok().as_deref() {
        Some("rgba8unorm") => TextureFormat::Rgba8Unorm,
        Some("bgra8unormsrgb") => TextureFormat::Bgra8UnormSrgb,
        Some("bgra8unorm") => TextureFormat::Bgra8Unorm,
        _ => TextureFormat::Rgba8UnormSrgb,
    }
}

pub(super) fn parented_camera_uses_local_projection() -> bool {
    std::env::var_os("AM_PARENT_RTT_LOCAL_PROJECTION").is_some()
}

pub(super) fn unparented_camera_uses_full_scale() -> bool {
    std::env::var_os("AM_RTT_CAMERA_FULL_SCALE").is_some()
}

pub(super) fn mirrored_capture_root_enabled() -> bool {
    std::env::var_os("AM_DISABLE_MIRRORED_RTT_CAPTURE_ROOT").is_none()
}

pub(super) fn flatten_parented_rtt_to_world_enabled() -> bool {
    std::env::var_os("AM_FLATTEN_PARENTED_RTT_TO_WORLD").is_some()
}

pub(super) fn plain_rtt_uses_straight_alpha() -> bool {
    std::env::var_os("AM_PLAIN_RTT_STRAIGHT_ALPHA").is_some()
}

pub(super) fn sign_axis(value: f32) -> f32 {
    if value.is_sign_negative() { -1.0 } else { 1.0 }
}

pub(super) fn insert_group_fill_debug_sprite(
    commands: &mut Commands,
    entity: Entity,
    render_texture_handle: Handle<Image>,
    scene_width: f32,
    scene_height: f32,
) {
    commands.entity(entity).insert(Sprite {
        image: render_texture_handle,
        custom_size: Some(Vec2::new(scene_width, scene_height)),
        ..default()
    });
}

pub(super) fn insert_group_fill_mesh(
    commands: &mut Commands,
    fill: &AmGroupFill,
    entity: Entity,
    render_texture_handle: Handle<Image>,
    scene_width: f32,
    scene_height: f32,
    fill_materials: &mut Assets<crate::group_fill::GroupFillMaterial>,
    meshes: &mut Assets<Mesh>,
) {
    use crate::group_fill::{GroupFillMaterial, GroupFillUniform};

    let uniform = match &fill.fill_type {
        GroupFillType::Color => GroupFillUniform {
            fill_color: fill.fill_color,
            gradient_config: Vec4::ZERO,
            ..default()
        },
        GroupFillType::Gradient {
            gradient_type,
            start_color,
            end_color,
            points,
        } => GroupFillUniform {
            fill_color: Vec4::ONE,
            gradient_config: Vec4::new(*gradient_type as f32, 0.0, 0.0, 0.0),
            gradient_start_color: *start_color,
            gradient_end_color: *end_color,
            gradient_points: *points,
        },
        GroupFillType::None => unreachable!(),
    };
    let material = fill_materials.add(GroupFillMaterial {
        uniform_data: uniform,
        texture: Some(render_texture_handle),
    });
    let mesh = meshes.add(Rectangle::new(scene_width, scene_height));
    commands.entity(entity).insert((
        Mesh2d(mesh),
        MeshMaterial2d(material),
        PendingGroupFillTextureRefresh(8),
    ));
}

pub(super) fn trace_group_fill_mode(
    trace_renderlayers: bool,
    entity: Entity,
    render_layer: usize,
    mode: &str,
    fill_type: &GroupFillType,
) {
    if trace_renderlayers {
        bevy::log::warn!(
            "[RTT-GroupFill] embed={:?} render_layer={} mode={} fill_type={:?}",
            entity,
            render_layer,
            mode,
            fill_type,
        );
    }
}

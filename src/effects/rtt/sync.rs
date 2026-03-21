use bevy::prelude::*;
use bevy::render::render_resource::Extent3d;
use bevy::sprite::Anchor;

use super::{
    EmbedSceneRtt, EmbedSceneRttCamera, compute_embed_visible_rect, resize_render_texture,
    scene_local_rect, sync_dynamic_resolution_mesh, sync_dynamic_resolution_sprite,
    transformed_rect_edge_lengths,
};

pub fn sync_rtt_camera_position_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut embed_query: Query<(
        Entity,
        &EmbedSceneRtt,
        &GlobalTransform,
        &crate::animation::AmAnimated,
        Option<&mut Sprite>,
        Option<&mut Anchor>,
        Option<&Mesh2d>,
    )>,
    mut camera_query: Query<(&EmbedSceneRttCamera, &mut Transform, &mut Projection)>,
) {
    for (camera_marker, mut camera_transform, mut projection) in camera_query.iter_mut() {
        if let Ok((embed_entity, rtt, embed_global, animated, sprite, anchor, mesh2d)) =
            embed_query.get_mut(camera_marker.embed_entity)
        {
            let full_rect = scene_local_rect(rtt.scene_width, rtt.scene_height);
            let visible_rect = compute_embed_visible_rect(rtt, embed_global, animated);
            let visible_size = Vec2::new(
                visible_rect.width().max(1.0),
                visible_rect.height().max(1.0),
            );
            let local_center = visible_rect.center();

            let (global_scale, embed_rotation, embed_translation) =
                embed_global.to_scale_rotation_translation();
            camera_transform.translation =
                Vec3::new(embed_translation.x, embed_translation.y, 1000.0);
            camera_transform.rotation = embed_rotation;
            camera_transform.scale =
                Vec3::new(global_scale.x.signum(), global_scale.y.signum(), 1.0);

            let effective_size = transformed_rect_edge_lengths(full_rect, embed_global.affine());
            if let Projection::Orthographic(ref mut ortho) = *projection {
                ortho.scaling_mode = bevy::camera::ScalingMode::Fixed {
                    width: effective_size.x,
                    height: effective_size.y,
                };
            }

            let new_extent = Extent3d {
                width: effective_size.x.ceil().max(1.0) as u32,
                height: effective_size.y.ceil().max(1.0) as u32,
                depth_or_array_layers: 1,
            };
            resize_render_texture(&mut images, &rtt.render_texture, new_extent);

            if let Some(mut sprite) = sprite {
                sync_dynamic_resolution_sprite(
                    &mut commands,
                    embed_entity,
                    &mut sprite,
                    anchor,
                    rtt,
                    visible_rect,
                    full_rect,
                    visible_size,
                    local_center,
                );
            } else if let Some(mesh2d) = mesh2d {
                sync_dynamic_resolution_mesh(&mut meshes, mesh2d, rtt, visible_rect, full_rect);
            }
        }
    }
}

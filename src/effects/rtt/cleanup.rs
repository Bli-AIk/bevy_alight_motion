//! Cleans up render-to-texture resources associated with embed scenes.
//! It despawns orphaned embed content and RTT cameras, and returns no-longer-used
//! render layers to the pool so composite rendering can keep reusing scarce slots.
//!
//! 负责清理嵌套场景相关的 RTT 资源。它会销毁已经失去宿主的 embed 内容和
//! RTT 相机，并把不再使用的 render layer 归还给池子，让复合渲染继续复用稀缺槽位。

use bevy::prelude::*;

use super::{
    EmbedSceneRenderLayerPool, EmbedSceneRtt, EmbedSceneRttCamera, EmbedSceneRttCaptureRoot,
    RttTextureCache,
};

pub fn cleanup_embed_content_system(
    mut commands: Commands,
    content_query: Query<(Entity, &crate::scene::AmEmbedContentMarker)>,
    embed_exists_query: Query<Entity>,
) {
    for (content_entity, marker) in content_query.iter() {
        if embed_exists_query.get(marker.embed_entity).is_err() {
            bevy::log::debug!(
                "Despawning orphaned embed content {:?} (embed entity {:?} no longer exists)",
                content_entity,
                marker.embed_entity
            );
            commands.entity(content_entity).despawn();
        }
    }
}

pub fn cleanup_embed_scene_rtt_system(
    mut commands: Commands,
    mut layer_pool: ResMut<EmbedSceneRenderLayerPool>,
    mut rtt_cache: ResMut<RttTextureCache>,
    mut removed: RemovedComponents<EmbedSceneRtt>,
    rtt_query: Query<&EmbedSceneRtt>,
    camera_query: Query<(Entity, &EmbedSceneRttCamera)>,
    capture_root_query: Query<(Entity, &EmbedSceneRttCaptureRoot)>,
    images: Res<Assets<Image>>,
) {
    for entity in removed.read() {
        bevy::log::debug!("EmbedSceneRtt removed from {:?}", entity);
    }

    for (camera_entity, camera_marker) in camera_query.iter() {
        let should_cleanup = rtt_query.get(camera_marker.embed_entity).is_err();
        if should_cleanup {
            // Cache the texture handle for reuse instead of dropping it.
            if let Some(img) = images.get(&camera_marker.render_texture) {
                let size = img.size();
                rtt_cache.push(size.x, size.y, camera_marker.render_texture.clone());
                bevy::log::trace!(
                    "Cached RTT texture ({}x{}) from layer {}",
                    size.x,
                    size.y,
                    camera_marker.render_layer,
                );
            }

            layer_pool.release(camera_marker.render_layer);
            bevy::log::debug!(
                "Released render layer {} from orphaned RTT camera {:?} for embed {:?}",
                camera_marker.render_layer,
                camera_entity,
                camera_marker.embed_entity
            );
            commands.entity(camera_entity).despawn();
        }
    }

    for (capture_root_entity, capture_root) in capture_root_query.iter() {
        if rtt_query.get(capture_root.embed_entity).is_ok() {
            continue;
        }

        bevy::log::debug!(
            "Despawning orphaned RTT capture root {:?} for embed {:?}",
            capture_root_entity,
            capture_root.embed_entity
        );
        commands.entity(capture_root_entity).despawn();
    }
}

use bevy::prelude::*;

use super::{EmbedSceneBounds, RenderStrategy};

pub fn apply_embed_bounds_clipping_system(
    embed_query: Query<(&EmbedSceneBounds, &GlobalTransform, Option<&RenderStrategy>)>,
    content_query: Query<(
        Entity,
        &crate::scene::AmEmbedContentMarker,
        &MeshMaterial2d<crate::masked_sprite::UnifiedEffectMaterial>,
        Option<&crate::scene::AmMaskInfo>,
    )>,
    playback: Res<crate::animation::AmPlayback>,
    mut materials: ResMut<Assets<crate::masked_sprite::UnifiedEffectMaterial>>,
) {
    let global_time = playback.current_time_ms as u64;

    for (entity, marker, material_handle, mask_info) in content_query.iter() {
        let Ok((bounds, embed_gt, strategy)) = embed_query.get(marker.embed_entity) else {
            continue;
        };

        if strategy.is_some_and(|s| *s == RenderStrategy::Direct) {
            continue;
        }

        let Some(material) = materials.get_mut(&material_handle.0) else {
            continue;
        };

        let has_active_mask = mask_info
            .map(|info| !info.get_active_masks(global_time).is_empty())
            .unwrap_or(false);

        if has_active_mask {
            continue;
        }

        let (embed_scale, embed_rotation, embed_pos) = embed_gt.to_scale_rotation_translation();
        let half_width = bounds.width * 0.5 * embed_scale.x.abs();
        let half_height = bounds.height * 0.5 * embed_scale.y.abs();
        let center_x = embed_pos.x;
        let center_y = embed_pos.y;
        let rotation_z = embed_rotation.to_euler(bevy::math::EulerRot::XYZ).2;

        material.uniform_data.effect_flags.x = 1.0;
        material.uniform_data.mask_params = Vec4::new(center_x, center_y, half_width, half_height);
        material.uniform_data.mask_blend = Vec4::new(1.0, 1.0, 0.0, 0.0);
        material.uniform_data.mask2_flags.y = rotation_z;

        bevy::log::trace!(
            "[EmbedClip] Content {:?} clipped to embed bounds: center=({:.1},{:.1}), half=({:.1},{:.1}), rot={:.3}, embed_scale=({:.3},{:.3})",
            entity,
            center_x,
            center_y,
            half_width,
            half_height,
            rotation_z,
            embed_scale.x,
            embed_scale.y
        );
    }
}

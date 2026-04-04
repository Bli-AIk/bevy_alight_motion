//! Applies embed-scene bounds clipping for RTT content.
//! When an embed scene is rendered through a texture rather than directly, the
//! systems here write a clip rectangle into unified materials so child content
//! stays inside the visible embed bounds unless a stronger mask is already active.
//!
//! 负责给 RTT 路径下的嵌套场景内容施加边界裁剪。当 embed scene 通过纹理而非
//! 直接路径渲染时，这里的系统会把裁剪矩形写进统一材质，确保子内容保持在可见的嵌套场景
//! 边界内；如果已有更强的遮罩在生效，则这里会让位。

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
    if std::env::var_os("AM_DISABLE_EMBED_BOUNDS_CLIP").is_some() {
        return;
    }

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

        let (embed_scale, embed_rotation, embed_pos) = embed_gt.to_scale_rotation_translation();
        let half_width = bounds.width * 0.5 * embed_scale.x.abs();
        let half_height = bounds.height * 0.5 * embed_scale.y.abs();
        let center_x = embed_pos.x;
        let center_y = embed_pos.y;
        let rotation_z = embed_rotation.to_euler(bevy::math::EulerRot::XYZ).2;

        if has_active_mask {
            // Entity has a real mask — write embed bounds into the dedicated
            // embed_clip uniform so both mask AND embed clipping apply.
            material.uniform_data.embed_clip_params =
                Vec4::new(center_x, center_y, half_width, half_height);
            material.uniform_data.embed_clip_rotation = Vec4::new(rotation_z, 0.0, 0.0, 0.0);
        } else {
            // No real mask — use the original mask_params path for embed clipping.
            material.uniform_data.effect_flags.x = 1.0;
            material.uniform_data.mask_params =
                Vec4::new(center_x, center_y, half_width, half_height);
            material.uniform_data.mask_blend = Vec4::new(1.0, 1.0, 0.0, 0.0);
            material.uniform_data.mask2_flags.y = rotation_z;
        }

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

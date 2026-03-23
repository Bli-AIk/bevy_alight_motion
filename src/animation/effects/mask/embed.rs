use bevy::prelude::*;

use crate::animation::components::AmAnimated;

pub(super) fn apply_embed_mask_uv(
    mask_entity: Entity,
    mask1: &crate::scene::AmMaskEntry,
    mask_layer_query: &Query<(&GlobalTransform, &AmAnimated, &crate::scene::AmLayerSpec)>,
    fit_scale: f32,
    material: &mut crate::masked_sprite::UnifiedEffectMaterial,
) {
    let Ok((mask_gt, _, _)) = mask_layer_query.get(mask_entity) else {
        return;
    };
    let (mask_scale, mask_rot, mask_pos) = mask_gt.to_scale_rotation_translation();
    let mask_rotation = mask_rot.to_euler(bevy::math::EulerRot::ZYX).0;
    let (scene_w, scene_h) = mask1.embed_scene_size.unwrap_or((1280.0, 960.0));
    let half_w = scene_w / 2.0 * fit_scale * mask_scale.x;
    let half_h = scene_h / 2.0 * fit_scale * mask_scale.y;

    bevy::log::debug!(
        "[MASK-DBG] mask pos=({:.1},{:.1}), half=({:.1},{:.1}), rot={:.3}",
        mask_pos.x,
        mask_pos.y,
        half_w,
        half_h,
        mask_rotation
    );

    material.uniform_data.mask_params = Vec4::new(mask_pos.x, mask_pos.y, half_w, half_h);
    material.uniform_data.mask2_flags.y = mask_rotation;
}

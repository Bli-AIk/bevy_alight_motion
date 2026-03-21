use bevy::prelude::*;

use crate::animation::components::{AmAnimated, AmPlayback};
use crate::scene::{AmLayerMarker, AmMaskInfo};

use super::compute::{compute_mask_params, mask_type_flag};
use super::embed::apply_embed_mask_uv;
use super::repeat::set_mask_repeat_uniforms;
use super::trace::trace_mask_once;

pub fn update_unified_mask_system(
    playback: Res<AmPlayback>,
    query: Query<(
        &AmMaskInfo,
        &MeshMaterial2d<crate::masked_sprite::UnifiedEffectMaterial>,
        &AmLayerMarker,
        &GlobalTransform,
    )>,
    pending_query: Query<&crate::scene::AmPendingLayers>,
    mask_layer_query: Query<(&GlobalTransform, &AmAnimated, &crate::scene::AmLayerSpec)>,
    embed_rtt_marker_query: Query<(Entity, &AmLayerMarker, &crate::effects::EmbedSceneRtt)>,
    mut materials: ResMut<Assets<crate::masked_sprite::UnifiedEffectMaterial>>,
) {
    if playback.force_stopped {
        return;
    }

    let Some(pending) = pending_query.iter().next() else {
        return;
    };
    let fit_scale = 1.0 / pending.inv_fit_scale;

    let global_time = playback.current_time_ms as u64;
    for (mask_info, material_handle, marker, _entity_global_transform) in query.iter() {
        let active_masks = mask_info.get_active_masks(global_time);
        let Some(material) = materials.get_mut(&material_handle.0) else {
            continue;
        };

        trace_mask_once(format!("active-layer:{}", marker.id), || {
            let masks = active_masks
                .iter()
                .map(|mask| {
                    format!(
                        "{}(parent={},embed={},exclude={})",
                        mask.mask_layer_id,
                        mask.mask_parent_layer_id,
                        mask.is_embed_mask,
                        mask.is_exclude
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "[MASK-ACTIVE] layer_id={} label='{}' masks=[{}]",
                marker.id, marker.label, masks
            )
        });

        if active_masks.is_empty() {
            material.uniform_data.effect_flags.x = 0.0;
            material.uniform_data.mask2_flags.x = 0.0;
            material.uniform_data.mask2_flags.y = 0.0;
            material.uniform_data.mask2_flags.z = 0.0;
            material.uniform_data.mask1_lr_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
            material.uniform_data.mask1_lr2_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
            material.uniform_data.mask1_repeat_params1 = Vec4::ZERO;
            material.uniform_data.mask1_rr_params1 = Vec4::ZERO;
            continue;
        }

        let mask1 = active_masks[0];

        if mask1.is_embed_mask {
            let mask_type = if mask1.is_exclude { 6.0 } else { 5.0 };
            material.uniform_data.effect_flags.x = mask_type;

            let rtt_match = embed_rtt_marker_query
                .iter()
                .find(|(_, m, _)| m.id == mask1.mask_layer_id);

            bevy::log::debug!(
                "[MASK-DBG] embed mask: layer_id={}, rtt_match={}",
                mask1.mask_layer_id,
                rtt_match.is_some()
            );

            if let Some((mask_entity, _, rtt)) = rtt_match {
                material.mask_texture = Some(rtt.render_texture.clone());
                bevy::log::debug!("[MASK-DBG] RTT found for mask entity {:?}", mask_entity);
                apply_embed_mask_uv(mask_entity, mask1, &mask_layer_query, fit_scale, material);
            } else {
                #[expect(clippy::excessive_nesting)]
                trace_mask_once(format!("embed-rtt-missing:{}", mask1.mask_layer_id), || {
                    format!(
                        "[MASK-DBG] RTT NOT found for mask layer_id={}",
                        mask1.mask_layer_id
                    )
                });
                material.uniform_data.effect_flags.x = 0.0;
                material.mask_texture = None;
            }

            set_mask_repeat_uniforms(
                mask1,
                pending,
                &mask_layer_query,
                playback.current_time_ms,
                fit_scale,
                material,
            );
        } else {
            let m1 = compute_mask_params(
                mask1,
                pending,
                &mask_layer_query,
                playback.current_time_ms,
                fit_scale,
            );

            material.uniform_data.effect_flags.x =
                mask_type_flag(mask1.is_circle, mask1.is_exclude);
            material.uniform_data.mask_params =
                Vec4::new(m1.center.x, m1.center.y, m1.half_size.x, m1.half_size.y);
            material.uniform_data.mask_blend =
                Vec4::new(m1.blend.x, m1.blend.y, m1.blend.z, m1.sign_code);
            material.uniform_data.mask2_flags.y = m1.rotation;
            material.uniform_data.mask1_stretch1_params = m1.stretch1;
            material.uniform_data.mask1_stretch2_params = m1.stretch2;
            material.uniform_data.mask1_stretch_info = m1.stretch_info;

            set_mask_repeat_uniforms(
                mask1,
                pending,
                &mask_layer_query,
                playback.current_time_ms,
                fit_scale,
                material,
            );
        }

        if active_masks.len() >= 2 {
            let mask2 = active_masks[1];
            let m2 = compute_mask_params(
                mask2,
                pending,
                &mask_layer_query,
                playback.current_time_ms,
                fit_scale,
            );

            material.uniform_data.mask2_flags.x = mask_type_flag(mask2.is_circle, mask2.is_exclude);
            material.uniform_data.mask2_params =
                Vec4::new(m2.center.x, m2.center.y, m2.half_size.x, m2.half_size.y);
            material.uniform_data.mask2_blend =
                Vec4::new(m2.blend.x, m2.blend.y, m2.blend.z, m2.sign_code);
            material.uniform_data.mask2_flags.z = m2.rotation;
        } else {
            material.uniform_data.mask2_flags.x = 0.0;
            material.uniform_data.mask2_flags.z = 0.0;
            material.uniform_data.mask2_blend = Vec4::ZERO;
        }
    }
}

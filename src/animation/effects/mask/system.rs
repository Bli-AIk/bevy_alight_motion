//! Applies active mask state to unified materials each frame.
//! It looks up the currently active masks for a layer, chooses the correct mask
//! mode, and then combines direct, embed-scene, and repeat-aware mask data into
//! the final uniform payload used for rendering.
//!
//! 负责在每一帧把活动中的遮罩状态应用到统一材质上。它会查找当前图层正在
//! 生效的遮罩，选择正确的 mask 模式，并把直接遮罩、嵌套场景遮罩和带重复信息的遮罩
//! 合并成最终用于渲染的 uniform 数据。

use bevy::prelude::*;

use crate::animation::components::{AmAnimated, AmPlayback};
use crate::scene::{AmLayerMarker, AmMaskInfo};

use super::compute::{compute_mask_params, mask_type_flag};
use super::embed::apply_embed_mask_uv;
use super::repeat::set_mask_repeat_uniforms;
use super::trace::trace_mask_once;

fn parse_disabled_mask_ids() -> Option<Vec<u64>> {
    std::env::var_os("AM_DISABLE_MASK_IDS")
        .and_then(|value| value.into_string().ok())
        .map(|ids| {
            ids.split(',')
                .filter_map(|value| value.trim().parse::<u64>().ok())
                .collect()
        })
}

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

    // Cache env var once per frame instead of per entity
    let disabled_mask_ids = parse_disabled_mask_ids();

    let global_time = playback.current_time_ms as u64;
    for (mask_info, material_handle, marker, _entity_global_transform) in query.iter() {
        let active_masks = mask_info.get_active_masks(global_time);
        let Some(old_mat) = materials.get(&material_handle.0) else {
            continue;
        };

        let mut new_uniform = old_mat.uniform_data;
        let mut new_mask_texture = old_mat.mask_texture.clone();

        if disabled_mask_ids
            .as_ref()
            .is_some_and(|ids| ids.contains(&marker.id))
        {
            new_uniform.effect_flags.x = 0.0;
            new_uniform.mask2_flags.x = 0.0;
            new_uniform.mask2_flags.y = 0.0;
            new_uniform.mask2_flags.z = 0.0;
            new_uniform.mask1_lr_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
            new_uniform.mask1_lr2_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
            new_uniform.mask1_repeat_params1 = Vec4::ZERO;
            new_uniform.mask1_rr_params1 = Vec4::ZERO;
            new_mask_texture = None;
            bevy::log::warn!(
                "[MaskTrace] layer_id={} mask disabled by AM_DISABLE_MASK_IDS",
                marker.id
            );

            if new_uniform != old_mat.uniform_data || new_mask_texture != old_mat.mask_texture {
                let material = materials.get_mut(&material_handle.0).unwrap();
                material.uniform_data = new_uniform;
                material.mask_texture = new_mask_texture;
            }
            continue;
        }

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
            new_uniform.effect_flags.x = 0.0;
            new_uniform.mask2_flags.x = 0.0;
            new_uniform.mask2_flags.y = 0.0;
            new_uniform.mask2_flags.z = 0.0;
            new_uniform.mask1_lr_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
            new_uniform.mask1_lr2_params1 = Vec4::new(-1.0, 0.0, 0.0, 0.0);
            new_uniform.mask1_repeat_params1 = Vec4::ZERO;
            new_uniform.mask1_rr_params1 = Vec4::ZERO;

            if new_uniform != old_mat.uniform_data {
                let material = materials.get_mut(&material_handle.0).unwrap();
                material.uniform_data = new_uniform;
            }
            continue;
        }

        let mask1 = active_masks[0];

        if mask1.is_embed_mask {
            let mask_type = if mask1.is_exclude { 6.0 } else { 5.0 };
            new_uniform.effect_flags.x = mask_type;

            let rtt_match = embed_rtt_marker_query
                .iter()
                .find(|(_, m, _)| m.id == mask1.mask_layer_id);

            bevy::log::debug!(
                "[MASK-DBG] embed mask: layer_id={}, rtt_match={}",
                mask1.mask_layer_id,
                rtt_match.is_some()
            );

            if let Some((mask_entity, _, rtt)) = rtt_match {
                new_mask_texture = Some(rtt.render_texture.clone());
                bevy::log::debug!("[MASK-DBG] RTT found for mask entity {:?}", mask_entity);
                apply_embed_mask_uv(
                    mask_entity,
                    mask1,
                    &mask_layer_query,
                    fit_scale,
                    &mut new_uniform,
                );
            } else {
                #[expect(clippy::excessive_nesting)]
                // reason: keep the missing-RTT trace at the exact fallback branch
                trace_mask_once(format!("embed-rtt-missing:{}", mask1.mask_layer_id), || {
                    format!(
                        "[MASK-DBG] RTT NOT found for mask layer_id={}",
                        mask1.mask_layer_id
                    )
                });
                new_uniform.effect_flags.x = 0.0;
                new_mask_texture = None;
            }

            set_mask_repeat_uniforms(
                mask1,
                pending,
                &mask_layer_query,
                playback.current_time_ms,
                fit_scale,
                &mut new_uniform,
            );
        } else {
            let m1 = compute_mask_params(
                mask1,
                pending,
                &mask_layer_query,
                playback.current_time_ms,
                fit_scale,
            );

            new_uniform.effect_flags.x = mask_type_flag(mask1.is_circle, mask1.is_exclude);
            new_uniform.mask_params =
                Vec4::new(m1.center.x, m1.center.y, m1.half_size.x, m1.half_size.y);
            new_uniform.mask_blend = Vec4::new(m1.blend.x, m1.blend.y, m1.blend.z, m1.sign_code);
            new_uniform.mask2_flags.y = m1.rotation;
            new_uniform.mask1_stretch1_params = m1.stretch1;
            new_uniform.mask1_stretch2_params = m1.stretch2;
            new_uniform.mask1_stretch_info = m1.stretch_info;

            set_mask_repeat_uniforms(
                mask1,
                pending,
                &mask_layer_query,
                playback.current_time_ms,
                fit_scale,
                &mut new_uniform,
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

            new_uniform.mask2_flags.x = mask_type_flag(mask2.is_circle, mask2.is_exclude);
            new_uniform.mask2_params =
                Vec4::new(m2.center.x, m2.center.y, m2.half_size.x, m2.half_size.y);
            new_uniform.mask2_blend = Vec4::new(m2.blend.x, m2.blend.y, m2.blend.z, m2.sign_code);
            new_uniform.mask2_flags.z = m2.rotation;
        } else {
            new_uniform.mask2_flags.x = 0.0;
            new_uniform.mask2_flags.z = 0.0;
            new_uniform.mask2_blend = Vec4::ZERO;
        }

        let changed =
            new_uniform != old_mat.uniform_data || new_mask_texture != old_mat.mask_texture;
        if changed {
            let material = materials.get_mut(&material_handle.0).unwrap();
            material.uniform_data = new_uniform;
            material.mask_texture = new_mask_texture;
        }
    }
}

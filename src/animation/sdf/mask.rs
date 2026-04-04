//! Pushes animated mask parameters into SDF materials.
//! 把动画遮罩参数写入 SDF 材质。
//!
//! Parent SDF layers collect mask metadata during scene import, but the actual mask transform,
//! repeat mode, and fit-scale compensation all depend on runtime playback time. This file reads the
//! current parent/mask transforms and updates each child SDF material so shader-based masking stays
//! aligned with the animated scene.
//! scene 导入阶段只会记录父级 SDF 图层的遮罩元数据，真正的遮罩变换、重复模式和 fit-scale
//! 补偿都依赖运行时播放时间。这个文件读取当前父级和遮罩图层的变换，把结果写回每个子级
//! SDF 材质，确保 shader 侧遮罩始终和动画场景保持对齐。

use bevy::prelude::*;

use crate::scene::{AmLayerMarker, AmMaskInfo};
use crate::sdf_material::SdfMaterial;

use super::super::components::{AmAnimated, AmPlayback, AmSdfShapeParent};
use super::super::sdf_mask::{
    apply_sdf_mask_linear_repeat, apply_sdf_mask_radial_repeat, compute_sdf_mask_params,
};

pub fn update_sdf_mask_system(
    playback: Res<AmPlayback>,
    parent_query: Query<
        (
            &AmAnimated,
            &Children,
            &AmMaskInfo,
            &AmLayerMarker,
            &GlobalTransform,
        ),
        With<AmSdfShapeParent>,
    >,
    pending_query: Query<&crate::scene::AmPendingLayers>,
    mask_layer_query: Query<(&GlobalTransform, &AmAnimated, &crate::scene::AmLayerSpec)>,
    mut sdf_query: Query<(&MeshMaterial2d<SdfMaterial>, &GlobalTransform)>,
    mut materials: ResMut<Assets<SdfMaterial>>,
) {
    if playback.force_stopped {
        return;
    }

    let pending = match pending_query.iter().next() {
        Some(p) => p,
        None => return,
    };
    let fit_scale = 1.0 / pending.inv_fit_scale;

    static mut LOGGED_SCALE: bool = false;
    unsafe {
        if !LOGGED_SCALE {
            bevy::log::debug!(
                "[MASK_SYSTEM] fit_scale={}, inv_fit_scale={}",
                fit_scale,
                pending.inv_fit_scale
            );
            LOGGED_SCALE = true;
        }
    }

    let global_time = playback.current_time_ms;

    for (_animated, children, mask_info, marker, parent_global_transform) in parent_query.iter() {
        let parent_scale = parent_global_transform.to_scale_rotation_translation().0;
        bevy::log::debug!(
            "[MaskParent] '{}' parent_global_scale=({:.2},{:.2})",
            marker.label,
            parent_scale.x,
            parent_scale.y,
        );

        let active_masks = mask_info.get_active_masks(global_time as u64);

        for child in children.iter() {
            let Ok((material_handle, child_global_transform)) = sdf_query.get_mut(child) else {
                continue;
            };
            let Some(material) = materials.get_mut(&material_handle.0) else {
                continue;
            };
            let _child_translation = child_global_transform.translation();
            let _child_scale = child_global_transform.to_scale_rotation_translation().0;
            let _frame_half = material.uniform_data.frame_half;

            if active_masks.is_empty() {
                material.uniform_data.mask_type = 0.0;
                material.uniform_data.mask2_type = 0.0;
                material.uniform_data.mask1_rr_params1 = Vec4::ZERO;
                continue;
            }

            let mask1 = active_masks[0];
            let (mask1_center, mask1_half_size, mask1_rotation, mask1_blend) =
                compute_sdf_mask_params(
                    mask1,
                    pending,
                    &mask_layer_query,
                    playback.current_time_ms,
                    fit_scale,
                );

            material.uniform_data.mask_params = Vec4::new(
                mask1_center.x,
                mask1_center.y,
                mask1_half_size.x,
                mask1_half_size.y,
            );
            material.uniform_data.mask_blend =
                Vec4::new(mask1_blend.x, mask1_blend.y, mask1_blend.z, 0.0);

            let base_type1 = if mask1.is_circle { 2.0 } else { 1.0 };
            material.uniform_data.mask_type = if mask1.is_exclude {
                base_type1 + 2.0
            } else {
                base_type1
            };
            material.uniform_data.mask_rotation = mask1_rotation;

            apply_sdf_mask_radial_repeat(
                mask1,
                pending,
                &mask_layer_query,
                playback.current_time_ms,
                fit_scale,
                material,
            );

            apply_sdf_mask_linear_repeat(
                mask1,
                pending,
                &mask_layer_query,
                playback.current_time_ms,
                fit_scale,
                material,
            );

            if active_masks.len() >= 2 {
                let mask2 = active_masks[1];
                let (mask2_center, mask2_half_size, mask2_rotation, mask2_blend) =
                    compute_sdf_mask_params(
                        mask2,
                        pending,
                        &mask_layer_query,
                        playback.current_time_ms,
                        fit_scale,
                    );

                material.uniform_data.mask2_params = Vec4::new(
                    mask2_center.x,
                    mask2_center.y,
                    mask2_half_size.x,
                    mask2_half_size.y,
                );
                material.uniform_data.mask2_blend =
                    Vec4::new(mask2_blend.x, mask2_blend.y, mask2_blend.z, 0.0);
                let base_type2 = 1.0 + mask2.is_circle as u8 as f32;
                material.uniform_data.mask2_type = base_type2 + mask2.is_exclude as u8 as f32 * 2.0;
                material.uniform_data.mask2_rotation = mask2_rotation;
            } else {
                material.uniform_data.mask2_type = 0.0;
                material.uniform_data.mask2_rotation = 0.0;
                material.uniform_data.mask2_blend = Vec4::ZERO;
            }
        }
    }
}

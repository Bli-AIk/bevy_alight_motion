//! # lifecycle.rs
//!
//! # 生命周期管理模块
//!
//! Layer lifecycle management system for lazy spawning and despawning of AM layers.
//! Handles entity creation and destruction based on playback time.
//!
//! AM 图层的生命周期管理系统，实现懒惰生成和销毁。
//! 根据播放时间处理实体的创建和销毁。

use bevy::asset::Assets;
use bevy::prelude::*;
use std::collections::HashMap;

use crate::loader::AmProject;
use crate::plugin::AmWhitePixel;
use crate::scene::{AmPendingLayers, AmProjectRoot, PendingLayer};
use crate::sdf_material::SdfMaterial;

use super::components::AmPlayback;
use super::spawn::{process_pending_layers, count_total_layers};

/// System to manage layer lifecycle based on playback time.
/// - Creates entities when layers enter their time range
/// - Destroys entities when layers exit their time range
/// - Implements true lazy spawning where no entities exist until needed
///
/// 基于播放时间管理图层生命周期的系统。
/// - 当图层进入时间范围时创建实体
/// - 当图层退出时间范围时销毁实体
/// - 实现真正的懒惰生成，实体在需要时才存在
#[allow(clippy::too_many_arguments)]
pub fn manage_layer_lifecycle_system(
    mut commands: Commands,
    playback: Res<AmPlayback>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut unified_materials: ResMut<Assets<crate::masked_sprite::UnifiedEffectMaterial>>,
    mut sdf_materials: ResMut<Assets<SdfMaterial>>,
    white_pixel: Option<Res<AmWhitePixel>>,
    projects: Res<Assets<AmProject>>,
    mut project_query: Query<(Entity, &AmProjectRoot, &mut AmPendingLayers)>,
) {
    // Skip if force stopped
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    // Debug logging
    static mut FRAME_COUNT: u32 = 0;
    unsafe {
        FRAME_COUNT += 1;
    }

    for (project_entity, root, mut pending) in project_query.iter_mut() {
        let Some(project) = projects.get(&root.handle) else {
            continue;
        };

        let white_pixel_handle = white_pixel.as_ref().map(|wp| wp.0.clone());

        // Use layers_container as parent for top-level layers, fall back to project_entity
        let parent_for_layers = pending.layers_container.unwrap_or(project_entity);

        // Process all pending layers (including nested ones)
        process_pending_layers(
            &mut commands,
            &mut meshes,
            &mut unified_materials,
            &mut sdf_materials,
            &mut pending,
            &project.images,
            &project.fonts,
            white_pixel_handle.as_ref(),
            global_time,
            parent_for_layers,
            0, // root time offset
        );

        // Log stats occasionally
        unsafe {
            if FRAME_COUNT % 300 == 1 {
                let spawned_count = pending.spawned_entities.len();
                let total_layers = count_total_layers(&pending.layers);
                bevy::log::trace!(
                    "[Lifecycle] time={:.0}ms | spawned={}/{} entities",
                    global_time,
                    spawned_count,
                    total_layers
                );
            }
        }
    }
}

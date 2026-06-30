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
use std::sync::atomic::{AtomicU32, Ordering};

use crate::loader::AmProject;
use crate::plugin::AmWhitePixel;
use crate::scene::{AmPendingLayers, AmProjectRoot};
use crate::sdf_material::SdfMaterial;

use super::components::AmPlayback;
use super::spawn::process_pending_layers;

/// Lightweight per-frame diagnostic: tracks wall-clock inter-frame time,
/// logs when a frame exceeds the configurable threshold (default 16 ms).
/// Enable with env `AM_FRAME_DIAG=1`. The system runs at the very top
/// of the Update schedule so it captures the entire previous frame.
///
/// 轻量帧诊断：跟踪帧间 wall-clock 时间，当帧耗时超过阈值时输出日志。
/// 通过环境变量 `AM_FRAME_DIAG=1` 启用。
pub fn frame_diagnostics_system(playback: Res<AmPlayback>, query: Query<&AmPendingLayers>) {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let enabled = *ENABLED.get_or_init(|| std::env::var_os("AM_FRAME_DIAG").is_some());
    if !enabled {
        return;
    }

    use std::sync::Mutex;
    static PREV_INSTANT: Mutex<Option<std::time::Instant>> = Mutex::new(None);
    static FRAME: AtomicU32 = AtomicU32::new(0);
    static PREV_TIME_MS: Mutex<Option<f32>> = Mutex::new(None);

    let now = std::time::Instant::now();
    let frame = FRAME.fetch_add(1, Ordering::Relaxed);

    let mut prev_guard = PREV_INSTANT.lock().unwrap();
    if let Some(prev) = *prev_guard {
        let dt_ms = now.duration_since(prev).as_secs_f64() * 1000.0;

        // Detect loop transition
        let mut prev_time_guard = PREV_TIME_MS.lock().unwrap();
        let looped = prev_time_guard
            .map(|pt| playback.current_time_ms < pt - 100.0)
            .unwrap_or(false);
        *prev_time_guard = Some(playback.current_time_ms);

        // Count spawned entities
        let spawned_count: usize = query.iter().map(|p| p.spawned_entities.len()).sum();

        if dt_ms > 16.0 || looped {
            let loop_marker = if looped { " [LOOP]" } else { "" };
            bevy::log::warn!(
                "[FRAME-DIAG] frame={} dt={:.2}ms time={:.1}ms spawned={}{loop_marker}",
                frame,
                dt_ms,
                playback.current_time_ms,
                spawned_count,
            );
        }
    }
    *prev_guard = Some(now);
}

/// System to manage layer lifecycle based on playback time.
/// - Creates entities when layers enter their time range
/// - Destroys entities when layers exit their time range
/// - Implements true lazy spawning where no entities exist until needed
///
/// 基于播放时间管理图层生命周期的系统。
/// - 当图层进入时间范围时创建实体
/// - 当图层退出时间范围时销毁实体
/// - 实现真正的懒惰生成，实体在需要时才存在
pub fn manage_layer_lifecycle_system(
    mut commands: Commands,
    playback: Res<AmPlayback>,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut unified_materials: ResMut<Assets<crate::masked_sprite::UnifiedEffectMaterial>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    mut sdf_materials: ResMut<Assets<SdfMaterial>>,
    white_pixel: Option<Res<AmWhitePixel>>,
    projects: Res<Assets<AmProject>>,
    mut project_query: Query<(
        Entity,
        &AmProjectRoot,
        &mut AmPendingLayers,
        Option<&crate::scene::AmSpawnSettings>,
    )>,
) {
    let global_time = playback.current_time_ms;
    let lifecycle_start = std::time::Instant::now();

    // Skip if force stopped
    if playback.force_stopped {
        bevy::log::trace!("[Lifecycle] Skipped: force_stopped=true");
        return;
    }

    for (project_entity, root, mut pending, spawn_settings) in project_query.iter_mut() {
        let Some(project) = projects.get(&root.handle) else {
            continue;
        };

        let white_pixel_handle = white_pixel.as_ref().map(|wp| wp.0.clone());

        // Use layers_container as parent for top-level layers, fall back to project_entity
        let parent_for_layers = pending.layers_container.unwrap_or(project_entity);

        // 获取过滤器 (Get the filter)
        let filter = spawn_settings
            .map(|s| &s.filter)
            .unwrap_or(&crate::scene::LayerFilter::None);

        let before_spawned = pending.spawned_entities.len();

        // Process all pending layers (including nested ones)
        process_pending_layers(
            &mut commands,
            &mut meshes,
            &mut unified_materials,
            &mut color_materials,
            &mut sdf_materials,
            &mut pending,
            &project.images,
            &project.fonts,
            white_pixel_handle.as_ref(),
            global_time,
            parent_for_layers,
            0.0, // root time offset
            filter,
            time.delta_secs(),
        );

        let after_spawned = pending.spawned_entities.len();
        let lifecycle_ms = lifecycle_start.elapsed().as_secs_f64() * 1000.0;
        let delta = after_spawned as i64 - before_spawned as i64;

        // Log when lifecycle takes significant time or entities changed
        if lifecycle_ms > 1.0 || delta != 0 {
            bevy::log::info!(
                "[Lifecycle] {:.2}ms, time={:.1}ms, entities: {} → {} (Δ{:+})",
                lifecycle_ms,
                global_time,
                before_spawned,
                after_spawned,
                delta,
            );
        }
    }
}

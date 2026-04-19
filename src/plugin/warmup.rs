//! Pipeline warmup: pre-compiles Material2d shader pipelines during loading
//! to eliminate first-frame stutter.
//!
//! After a project finishes loading, this module spawns one tiny visible entity
//! per material type, waits a few frames for async GPU pipeline compilation,
//! then despawns them and unpauses playback.
//!
//! 管线预热：在加载阶段预编译 Material2d 着色器管线，消除首帧卡顿。
//! 项目加载完成后，本模块为每种材质类型生成一个微小但可见的实体，等待数帧让
//! GPU 异步编译管线，然后清理并恢复播放。

use bevy::prelude::*;

use crate::animation::AmPlayback;
use crate::gaussian_blur::{GaussianBlurHMaterial, GaussianBlurVMaterial};
use crate::group_fill::GroupFillMaterial;
use crate::masked_sprite::UnifiedEffectMaterial;
use crate::scene::AmProjectRoot;
use crate::sdf_material::SdfMaterial;

/// Number of render frames to keep warmup entities alive.
/// Pipeline compilation is async; 3 frames is enough for most drivers.
const WARMUP_FRAMES: u32 = 3;

/// Tracks ongoing pipeline warmup. Inserted by `start_warmup_system`,
/// removed by `tick_warmup_system` when complete.
#[derive(Resource)]
pub(crate) struct PipelineWarmup {
    frames_remaining: u32,
    was_playing: bool,
}

/// Marker for warmup entities to be despawned after warmup.
#[derive(Component)]
pub(crate) struct WarmupEntity;

/// Detects project load completion and kicks off warmup.
pub(super) fn start_warmup_system(
    mut commands: Commands,
    roots: Query<&AmProjectRoot, Changed<AmProjectRoot>>,
    warmup: Option<Res<PipelineWarmup>>,
    mut playback: ResMut<AmPlayback>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut sdf_mats: ResMut<Assets<SdfMaterial>>,
    mut unified_mats: ResMut<Assets<UnifiedEffectMaterial>>,
    mut group_fill_mats: ResMut<Assets<GroupFillMaterial>>,
    mut blur_h_mats: ResMut<Assets<GaussianBlurHMaterial>>,
    mut blur_v_mats: ResMut<Assets<GaussianBlurVMaterial>>,
) {
    if warmup.is_some() {
        return;
    }
    let just_loaded = roots.iter().any(|r| r.spawned);
    if !just_loaded {
        return;
    }

    bevy::log::info!("Pipeline warmup: spawning warmup entities");

    // 1×1 quad, placed far behind the scene so it's rendered but invisible.
    let mesh = meshes.add(Rectangle::new(1.0, 1.0));
    let z = -9999.0;

    commands.spawn((
        WarmupEntity,
        Mesh2d(mesh.clone()),
        MeshMaterial2d(sdf_mats.add(SdfMaterial::default())),
        Transform::from_translation(Vec3::new(0.0, 0.0, z)),
    ));
    commands.spawn((
        WarmupEntity,
        Mesh2d(mesh.clone()),
        MeshMaterial2d(unified_mats.add(UnifiedEffectMaterial::default())),
        Transform::from_translation(Vec3::new(1.0, 0.0, z)),
    ));
    commands.spawn((
        WarmupEntity,
        Mesh2d(mesh.clone()),
        MeshMaterial2d(group_fill_mats.add(GroupFillMaterial::default())),
        Transform::from_translation(Vec3::new(2.0, 0.0, z)),
    ));
    commands.spawn((
        WarmupEntity,
        Mesh2d(mesh.clone()),
        MeshMaterial2d(blur_h_mats.add(GaussianBlurHMaterial::default())),
        Transform::from_translation(Vec3::new(3.0, 0.0, z)),
    ));
    commands.spawn((
        WarmupEntity,
        Mesh2d(mesh),
        MeshMaterial2d(blur_v_mats.add(GaussianBlurVMaterial::default())),
        Transform::from_translation(Vec3::new(4.0, 0.0, z)),
    ));

    let was_playing = playback.playing;
    playback.playing = false;

    commands.insert_resource(PipelineWarmup {
        frames_remaining: WARMUP_FRAMES,
        was_playing,
    });
}

/// Counts down warmup frames, then cleans up and restores playback.
pub(super) fn tick_warmup_system(
    mut commands: Commands,
    warmup: Option<ResMut<PipelineWarmup>>,
    entities: Query<Entity, With<WarmupEntity>>,
    mut playback: ResMut<AmPlayback>,
) {
    let Some(mut warmup) = warmup else { return };

    warmup.frames_remaining = warmup.frames_remaining.saturating_sub(1);
    if warmup.frames_remaining > 0 {
        return;
    }

    for entity in entities.iter() {
        commands.entity(entity).despawn();
    }

    bevy::log::info!("Pipeline warmup complete — resuming playback");
    playback.playing = warmup.was_playing;
    commands.remove_resource::<PipelineWarmup>();
}

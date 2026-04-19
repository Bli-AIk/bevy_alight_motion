//! Pipeline warmup: pre-compiles Material2d shader pipelines and triggers
//! Bevy's internal render-target infrastructure during loading to eliminate
//! first-frame stutter.
//!
//! After a project finishes loading, this module:
//! 1. Spawns one tiny visible entity per Material2d type → triggers pipeline compilation.
//! 2. Spawns a dummy off-screen Camera2d + render texture → triggers
//!    `prepare_view_upscaling_pipelines`, `prepare_view_targets`, depth-texture
//!    allocation, and other per-camera-view GPU setup that would otherwise spike
//!    on the first embed-scene render.
//! 3. Waits several frames for async GPU compilation, then despawns everything
//!    and resumes playback.
//!
//! 管线预热：在加载阶段预编译 Material2d 着色器管线并触发 Bevy 内部渲染目标
//! 基础设施，消除首帧卡顿。

use bevy::camera::{ClearColorConfig, RenderTarget, ScalingMode};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;

use crate::animation::AmPlayback;
use crate::effects::create_rtt_image;
use crate::gaussian_blur::{GaussianBlurHMaterial, GaussianBlurVMaterial};
use crate::group_fill::GroupFillMaterial;
use crate::masked_sprite::UnifiedEffectMaterial;
use crate::scene::AmProjectRoot;
use crate::sdf_material::SdfMaterial;

/// Number of render frames to keep warmup entities alive.
/// Pipeline compilation is async; 5 frames gives enough headroom for drivers
/// that queue compilation work across multiple present cycles.
const WARMUP_FRAMES: u32 = 5;

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
    mut images: ResMut<Assets<Image>>,
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

    bevy::log::info!("Pipeline warmup: spawning material + RTT warmup entities");

    let mesh = meshes.add(Rectangle::new(1.0, 1.0));

    // --- Material warmup entities (z=-999, within default orthographic frustum) ---
    let z = -999.0;
    spawn_material_entities(
        &mut commands,
        &mesh,
        z,
        &mut sdf_mats,
        &mut unified_mats,
        &mut group_fill_mats,
        &mut blur_h_mats,
        &mut blur_v_mats,
    );

    // --- RTT camera warmup: triggers prepare_view_upscaling_pipelines, ---
    // --- prepare_view_targets, depth-texture allocation, etc.          ---
    spawn_rtt_warmup_camera(&mut commands, &mesh, &mut images, &mut unified_mats);

    let was_playing = playback.playing;
    playback.playing = false;

    commands.insert_resource(PipelineWarmup {
        frames_remaining: WARMUP_FRAMES,
        was_playing,
    });
}

fn spawn_material_entities(
    commands: &mut Commands,
    mesh: &Handle<Mesh>,
    z: f32,
    sdf_mats: &mut Assets<SdfMaterial>,
    unified_mats: &mut Assets<UnifiedEffectMaterial>,
    group_fill_mats: &mut Assets<GroupFillMaterial>,
    blur_h_mats: &mut Assets<GaussianBlurHMaterial>,
    blur_v_mats: &mut Assets<GaussianBlurVMaterial>,
) {
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
        Mesh2d(mesh.clone()),
        MeshMaterial2d(blur_v_mats.add(GaussianBlurVMaterial::default())),
        Transform::from_translation(Vec3::new(4.0, 0.0, z)),
    ));
}

/// Spawns a tiny off-screen Camera2d + render texture so Bevy's internal
/// per-view render infrastructure (upscaling pipeline, view targets, depth
/// textures, etc.) is compiled before the real embed-scene cameras appear.
fn spawn_rtt_warmup_camera(
    commands: &mut Commands,
    mesh: &Handle<Mesh>,
    images: &mut Assets<Image>,
    unified_mats: &mut Assets<UnifiedEffectMaterial>,
) {
    let rtt_image = create_rtt_image(2, 2, TextureFormat::Rgba8UnormSrgb);
    let rtt_handle = images.add(rtt_image);

    // A tiny scene object on the RTT camera's render layer so the draw pass
    // actually executes (empty cameras may skip pipeline compilation).
    commands.spawn((
        WarmupEntity,
        Mesh2d(mesh.clone()),
        MeshMaterial2d(unified_mats.add(UnifiedEffectMaterial::default())),
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
    ));

    commands.spawn((
        WarmupEntity,
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::NONE),
            order: -999,
            ..default()
        },
        RenderTarget::Image(rtt_handle.into()),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed {
                width: 2.0,
                height: 2.0,
            },
            near: -1000.0,
            far: 2000.0,
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_translation(Vec3::new(0.0, 0.0, 1000.0)),
    ));
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

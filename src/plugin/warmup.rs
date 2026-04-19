//! Pipeline warmup: pre-compiles Material2d shader pipelines, pre-uploads
//! project textures to GPU, and triggers Bevy's render-target infrastructure
//! during loading to eliminate first-loop stutter.
//!
//! After a project finishes loading, this module:
//! 1. Spawns one tiny entity per Material2d type → triggers pipeline compilation.
//! 2. Spawns dummy sprites referencing ALL project images → forces GPU texture upload.
//! 3. Spawns N RTT cameras (matching embed scene count) → pre-allocates render targets.
//! 4. Waits several frames for async GPU work, then despawns everything and
//!    resumes playback.
//!
//! 管线预热：在加载阶段预编译着色器、预上传所有纹理、预分配渲染目标，
//! 消除首次循环卡顿。

use bevy::camera::{ClearColorConfig, RenderTarget, ScalingMode};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;

use crate::animation::AmPlayback;
use crate::effects::create_rtt_image;
use crate::gaussian_blur::{GaussianBlurHMaterial, GaussianBlurVMaterial};
use crate::group_fill::GroupFillMaterial;
use crate::loader::AmProject;
use crate::masked_sprite::UnifiedEffectMaterial;
use crate::scene::{AmPendingLayers, AmProjectRoot};
use crate::sdf_material::SdfMaterial;

/// Number of render frames to keep warmup entities alive.
/// Must be long enough for GPU to process all pre-uploaded textures and
/// render targets. Scaled up when many images/embeds are present.
const BASE_WARMUP_FRAMES: u32 = 5;

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
    roots: Query<(&AmProjectRoot, &AmPendingLayers), Changed<AmProjectRoot>>,
    warmup: Option<Res<PipelineWarmup>>,
    projects: Res<Assets<AmProject>>,
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
    let Some((root, pending)) = roots.iter().find(|(r, _)| r.spawned) else {
        return;
    };

    let mesh = meshes.add(Rectangle::new(1.0, 1.0));
    let z = -999.0;

    // --- Phase 1: Material pipeline warmup ---
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

    // --- Phase 2: Pre-upload ALL project textures to GPU ---
    let image_count = if let Some(project) = projects.get(&root.handle) {
        spawn_texture_warmup_entities(&mut commands, &mesh, z, &mut unified_mats, &project.images)
    } else {
        0
    };

    // --- Phase 3: Pre-allocate RTT cameras matching embed scene count ---
    let embed_count = count_embed_scenes(&pending.layers);
    // reason: at least 1 RTT camera for pipeline warmup, plus one per embed scene
    let rtt_camera_count = embed_count.max(1);
    for i in 0..rtt_camera_count {
        spawn_rtt_warmup_camera(
            &mut commands,
            &mesh,
            &mut images,
            &mut unified_mats,
            -(999 + i as i32),
        );
    }

    // Scale warmup frames based on content volume.
    // GPU needs time to process texture uploads and render-target allocations.
    let extra_frames = ((image_count + rtt_camera_count) / 4) as u32;
    let warmup_frames = BASE_WARMUP_FRAMES + extra_frames;

    bevy::log::info!(
        "Pipeline warmup: {} material + {} texture + {} RTT entities ({} frames)",
        5,
        image_count,
        rtt_camera_count,
        warmup_frames
    );

    let was_playing = playback.playing;
    playback.playing = false;

    commands.insert_resource(PipelineWarmup {
        frames_remaining: warmup_frames,
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

/// Pre-upload all project textures by creating tiny off-screen sprites.
/// Returns the number of texture warmup entities created.
fn spawn_texture_warmup_entities(
    commands: &mut Commands,
    mesh: &Handle<Mesh>,
    z: f32,
    unified_mats: &mut Assets<UnifiedEffectMaterial>,
    project_images: &std::collections::HashMap<String, Handle<Image>>,
) -> usize {
    let mut count = 0;
    for (i, (_uri, image_handle)) in project_images.iter().enumerate() {
        let mat = UnifiedEffectMaterial {
            texture: Some(image_handle.clone()),
            ..Default::default()
        };
        let x_offset = 10.0 + i as f32;
        commands.spawn((
            WarmupEntity,
            Mesh2d(mesh.clone()),
            MeshMaterial2d(unified_mats.add(mat)),
            Transform::from_translation(Vec3::new(x_offset, 0.0, z)),
        ));
        count += 1;
    }
    count
}

/// Counts the number of EmbedScene layers in the pending layer list.
fn count_embed_scenes(layers: &[crate::scene::PendingLayer]) -> usize {
    layers
        .iter()
        .filter(|l| matches!(l.spec, crate::scene::AmLayerSpec::EmbedScene))
        .count()
}

/// Spawns a dummy RTT camera + render texture to pre-allocate GPU render
/// targets (`prepare_view_targets`, depth textures, upscaling pipelines).
fn spawn_rtt_warmup_camera(
    commands: &mut Commands,
    mesh: &Handle<Mesh>,
    images: &mut Assets<Image>,
    unified_mats: &mut Assets<UnifiedEffectMaterial>,
    camera_order: i32,
) {
    let rtt_image = create_rtt_image(2, 2, TextureFormat::Rgba8UnormSrgb);
    let rtt_handle = images.add(rtt_image);

    // A tiny scene object so the draw pass actually executes.
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
            order: camera_order as isize,
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

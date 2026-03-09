//! # plugin.rs
//!
//! # plugin.rs 文件
//!
//! ## Module Overview
//!
//! ## 模块概述
//!
//! Bevy plugin for Alight Motion support.
//!
//! 用于支持 Alight Motion 的 Bevy 插件。
//!
//! ## Source File Overview
//!
//! ## 源文件概述
//!
//! This file defines `AlightMotionPlugin` which registers all necessary systems,
//! assets, and resources for loading and playing Alight Motion projects.
//!
//! 本文件定义了 `AlightMotionPlugin`，用于注册加载和播放 Alight Motion 项目所需的所有系统、资源和资源加载器。

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;

use crate::animation::{
    AmPlayback, advance_playback_system, animate_am_camera_system, animate_opacity_system,
    animate_path_repeat_system, animate_rtt_blur_system, animate_sdf_opacity_system,
    animate_sdf_scale_system, animate_size_system, animate_text_opacity_system,
    animate_text_progress_system, animate_text_spacing_system, animate_transform_system,
    animate_unified_effect_system, apply_mask_clipping_system, fix_rtl_line_alignment_system,
    manage_layer_lifecycle_system, update_echo_runtime_system, update_sdf_mask_system,
    update_unified_mask_system,
};
use crate::effects::EffectRenderPlugin;
use crate::gaussian_blur::{GaussianBlurHMaterial, GaussianBlurPlugin, GaussianBlurVMaterial};
use crate::group_fill::GroupFillMaterial;
use crate::loader::{AlightMotionLoader, AmProject};
use crate::masked_sprite::UnifiedEffectMaterial;
use crate::scene::{AmProjectBundle, AmProjectRoot, AmSceneConfig};
use crate::sdf::hot_reload_shader_system;
use crate::sdf_material::SdfMaterial;

/// Resource to configure how the AM project is scaled relative to the window.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq)]
pub enum AmProjectResolution {
    /// No scaling (1:1 pixel mapping).
    #[default]
    None,
    /// Scale the project to fit within the window, preserving aspect ratio.
    FitWindow,
    /// Scale the project to cover the window, preserving aspect ratio.
    CoverWindow,
    /// Scale the project to a fixed width, preserving aspect ratio.
    FixedWidth(f32),
    /// Scale the project to a fixed height, preserving aspect ratio.
    FixedHeight(f32),
}

/// Resource holding the white pixel texture used for solid color sprites.
///
/// 保存用于纯色精灵的白色像素纹理的资源。
#[derive(Resource)]
pub struct AmWhitePixel(pub Handle<Image>);

/// Plugin providing Alight Motion support for Bevy.
///
/// 为 Bevy 提供 Alight Motion 支持的插件。
pub struct AlightMotionPlugin;

impl Plugin for AlightMotionPlugin {
    fn build(&self, app: &mut App) {
        use bevy::ecs::schedule::ApplyDeferred;

        app.add_plugins(Material2dPlugin::<SdfMaterial>::default())
            .add_plugins(Material2dPlugin::<UnifiedEffectMaterial>::default())
            .add_plugins(Material2dPlugin::<GaussianBlurHMaterial>::default())
            .add_plugins(Material2dPlugin::<GaussianBlurVMaterial>::default())
            .add_plugins(Material2dPlugin::<GroupFillMaterial>::default())
            .add_plugins(EffectRenderPlugin)
            .add_plugins(GaussianBlurPlugin)
            .init_asset::<AmProject>()
            .init_asset_loader::<AlightMotionLoader>()
            .init_resource::<AmPlayback>()
            .init_resource::<AmProjectResolution>()
            // 注意: AmEntitySpawned 使用 EntityEvent derive，不需要 add_event 注册
            // Note: AmEntitySpawned uses EntityEvent derive, no need for add_event registration
            // 用户可以通过 commands.add_observer() 或 app.add_observer() 来监听事件
            .add_systems(Startup, setup_white_pixel_system)
            .add_systems(Startup, load_system_fonts_for_fallback)
            .add_systems(
                Update,
                (
                    spawn_loaded_projects_system,
                    advance_playback_system,
                    manage_layer_lifecycle_system, // Spawn/despawn visuals based on time
                    // Flush commands so newly spawned entities are available for animation systems
                    ApplyDeferred,
                    // RTT setup must happen before animation systems to ensure proper rendering
                    crate::effects::setup_embed_scene_rtt_system,
                    ApplyDeferred,
                    // Fix RenderLayers for nested embeds (must run after RTT setup and ApplyDeferred)
                    crate::effects::fix_nested_embed_render_layers_system,
                    // Propagate RenderLayers immediately after RTT setup
                    crate::effects::propagate_render_layers_system,
                    // TODO: propagate_render_layers_to_children_system disabled - causes transform issues
                    // crate::effects::propagate_render_layers_to_children_system,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    // Update echokf runtime before animation systems
                    update_echo_runtime_system,
                    animate_transform_system,
                    animate_am_camera_system, // Animate Bevy camera from AM camera layer
                    animate_size_system,      // Update size from size property animation
                    animate_sdf_scale_system, // Update SDF dimensions based on scale animation
                    animate_opacity_system,
                    animate_sdf_opacity_system,
                    animate_text_opacity_system,
                    fix_rtl_line_alignment_system, // Fix RTL line alignment before spacing
                    animate_text_spacing_system,   // Text spacing post-layout modification
                    animate_text_progress_system,  // Text progress visibility
                    animate_unified_effect_system, // Unified effect system (RTT-ready)
                    animate_path_repeat_system,    // Path repeat copy positioning
                    animate_rtt_blur_system,       // RTT Gaussian blur animation
                    apply_mask_clipping_system,    // Apply mask clipping to masked layers
                    hot_reload_shader_system,      // Hot-reload shader when 'R' is pressed
                    crate::effects::sync_rtt_camera_position_system,
                )
                    .chain()
                    .after(crate::effects::fix_nested_embed_render_layers_system),
            )
            // Run mask systems in PostUpdate after TransformPropagate
            // This ensures GlobalTransform is correctly propagated before mask calculations
            .add_systems(
                PostUpdate,
                (update_sdf_mask_system, update_unified_mask_system)
                    .chain()
                    .after(bevy::transform::TransformSystems::Propagate),
            );
    }
}

/// Create a 1x1 white pixel texture for solid color sprites.
///
/// 创建 1x1 白色像素纹理用于纯色精灵。
fn setup_white_pixel_system(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    let white_pixel = Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255, 255, 255, 255], // RGBA white pixel
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );

    let handle = images.add(white_pixel);
    commands.insert_resource(AmWhitePixel(handle));
}

/// Load system fonts into the CosmicFontSystem for font fallback.
/// This enables rendering of CJK, Arabic, Hindi, and other scripts
/// even when the primary font doesn't have those glyphs.
#[cfg(not(target_arch = "wasm32"))]
fn load_system_fonts_for_fallback(mut font_system: ResMut<bevy::text::CosmicFontSystem>) {
    // Load system fonts from common directories
    font_system.0.db_mut().load_system_fonts();
    let count = font_system.0.db().faces().count();
    bevy::log::info!("Loaded {} system font faces for fallback", count);
}

#[cfg(target_arch = "wasm32")]
fn load_system_fonts_for_fallback() {
    // No system fonts on WASM
}

/// System to collect pending layers when a project finishes loading.
/// Note: This doesn't spawn entities immediately - the lifecycle system handles that.
///
/// 在项目加载完成时收集待处理图层的系统。
/// 注意：这不会立即生成实体 - 生命周期系统会处理这个。
fn spawn_loaded_projects_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut AmProjectRoot, &mut Transform)>,
    projects: Res<Assets<AmProject>>,
    mut playback: ResMut<AmPlayback>,
    resolution_config: Res<AmProjectResolution>,
    window_query: Query<&Window>,
) {
    for (entity, mut root, mut transform) in query.iter_mut() {
        if root.spawned {
            continue;
        }

        let Some(project) = projects.get(&root.handle) else {
            continue;
        };
        bevy::log::info!(
            "Loading AM project: {} ({}x{}, {}ms)",
            project.scene.title,
            project.scene.width,
            project.scene.height,
            project.scene.total_time
        );
        bevy::log::debug!("  Media count: {}", project.scene.media.len());
        bevy::log::debug!("  Images loaded: {}", project.images.len());
        for uri in project.images.keys() {
            bevy::log::trace!("    - {}", uri);
        }
        bevy::log::debug!("  Layers count: {}", project.scene.layers.len());

        // Update playback duration
        playback.total_time_ms = project.scene.total_time as f32;

        // Apply resolution scaling and compute inverse scale for embed children
        let mut fit_scale = 1.0f32;
        match *resolution_config {
            AmProjectResolution::None => {
                // Default scale 1.0
            }
            AmProjectResolution::FitWindow => {
                if let Some(window) = window_query.iter().next() {
                    let s_x = window.width() / (project.scene.width as f32);
                    let s_y = window.height() / (project.scene.height as f32);
                    fit_scale = s_x.min(s_y);
                    transform.scale = Vec3::splat(fit_scale);
                    bevy::log::info!(
                        "Scaled project to fit window: scale={:.4} (win={}x{})",
                        fit_scale,
                        window.width(),
                        window.height()
                    );
                }
            }
            AmProjectResolution::CoverWindow => {
                if let Some(window) = window_query.iter().next() {
                    let s_x = window.width() / (project.scene.width as f32);
                    let s_y = window.height() / (project.scene.height as f32);
                    fit_scale = s_x.max(s_y);
                    transform.scale = Vec3::splat(fit_scale);
                    bevy::log::info!(
                        "Scaled project to cover window: scale={:.4} (win={}x{})",
                        fit_scale,
                        window.width(),
                        window.height()
                    );
                }
            }
            AmProjectResolution::FixedWidth(target_width) => {
                fit_scale = target_width / (project.scene.width as f32);
                transform.scale = Vec3::splat(fit_scale);
                bevy::log::info!(
                    "Scaled project to fixed width {}: scale={:.4}",
                    target_width,
                    fit_scale
                );
            }
            AmProjectResolution::FixedHeight(target_height) => {
                fit_scale = target_height / (project.scene.height as f32);
                transform.scale = Vec3::splat(fit_scale);
                bevy::log::info!(
                    "Scaled project to fixed height {}: scale={:.4}",
                    target_height,
                    fit_scale
                );
            }
        }

        // Build scene configuration
        let config = AmSceneConfig {
            canvas_width: project.scene.width as f32,
            canvas_height: project.scene.height as f32,
            scene_fps: project.scene.fps as f32,
            scene_total_time: project.scene.total_time as f32,
            ..Default::default()
        };

        // Collect pending layers instead of spawning immediately
        let pending_layers = crate::scene::collect_pending_layers(
            &project.scene,
            &project.fonts,
            &project.font_metrics,
            &config,
        );

        bevy::log::info!(
            "Prepared {} pending layers for lazy spawning",
            pending_layers.len()
        );

        // Create layers container as child of project root
        let layers_container = commands
            .spawn((
                Name::new("AmLayersContainer"),
                crate::scene::AmLayersContainer,
                Transform::default(),
                GlobalTransform::default(),
                Visibility::Inherited,
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ))
            .id();

        commands.entity(entity).add_child(layers_container);

        // Create embed contents container as SIBLING of project root (not child!)
        // This container has identity Transform, so embed content coordinates remain unchanged
        // Embed content uses internal canvas coordinates and renders to RTT camera
        // It must NOT inherit the project's fit_scale Transform
        let embed_contents_container = commands
            .spawn((
                Name::new("AmEmbedContentsContainer"),
                crate::scene::AmEmbedContentsContainer,
                Transform::default(),
                GlobalTransform::default(),
                Visibility::Inherited,
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ))
            .id();

        // Create RTT cameras container as SIBLING of project root
        // This container organizes all EmbedSceneRttCamera entities
        let rtt_cameras_container = commands
            .spawn((
                Name::new("AmRttCamerasContainer"),
                crate::scene::AmRttCamerasContainer,
                Transform::default(),
                GlobalTransform::default(),
                Visibility::Inherited,
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ))
            .id();

        // Add the pending layers component to the project root
        // Store inverse fit scale for embed children coordinate adjustment
        // Include layers_container entity for spawning layers as its children
        commands
            .entity(entity)
            .insert(crate::scene::AmPendingLayers {
                layers: pending_layers,
                spawned_entities: std::collections::HashMap::new(),
                inv_fit_scale: 1.0 / fit_scale,
                layers_container: Some(layers_container),
                embed_contents_container: Some(embed_contents_container),
                rtt_cameras_container: Some(rtt_cameras_container),
            });

        root.spawned = true;
        bevy::log::info!("Project ready for playback");
    }
}

/// Helper function to load and spawn an AM project.
pub fn load_am_project(
    commands: &mut Commands,
    asset_server: &AssetServer,
    path: impl Into<String>,
) -> Entity {
    let path_string: String = path.into();
    let handle: Handle<AmProject> = asset_server.load(path_string.clone());

    // Extract project name from path for entity naming
    let project_name = path_string
        .rsplit('/')
        .next()
        .unwrap_or(&path_string)
        .trim_end_matches(".amproj")
        .trim_end_matches(".xml");

    commands
        .spawn((
            Name::new(format!("AmProject: {}", project_name)),
            AmProjectBundle {
                transform: Transform::default(),
                global_transform: GlobalTransform::default(),
                visibility: Visibility::default(),
                inherited_visibility: InheritedVisibility::default(),
                view_visibility: ViewVisibility::default(),
                marker: AmProjectRoot {
                    handle,
                    spawned: false,
                },
            },
        ))
        .id()
}

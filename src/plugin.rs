//! Bevy plugin for Alight Motion support.

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;
use bevy_smud::SmudPlugin;

use crate::animation::{
    AmPlayback, advance_playback, animate_opacity, animate_sdf_opacity, animate_sdf_scale,
    animate_size, animate_text_opacity, animate_transform, apply_mask_clipping,
    manage_layer_lifecycle,
};
use crate::loader::{AlightMotionLoader, AmProject};
use crate::masked_sprite::MaskedSpriteMaterial;
use crate::scene::{AmProjectBundle, AmProjectRoot, AmSceneConfig};
use crate::sdf::{hot_reload_shader, setup_sdf_shaders};

/// Resource holding the white pixel texture used for solid color sprites.
#[derive(Resource)]
pub struct AmWhitePixel(pub Handle<Image>);

/// Plugin providing Alight Motion support for Bevy.
pub struct AlightMotionPlugin;

impl Plugin for AlightMotionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SmudPlugin)
            .add_plugins(Material2dPlugin::<MaskedSpriteMaterial>::default())
            .init_asset::<AmProject>()
            .init_asset_loader::<AlightMotionLoader>()
            .init_resource::<AmPlayback>()
            .add_systems(Startup, (setup_white_pixel, setup_sdf_shaders))
            .add_systems(
                Update,
                (
                    spawn_loaded_projects,
                    advance_playback,
                    manage_layer_lifecycle, // Spawn/despawn visuals based on time
                    animate_transform,
                    animate_size, // Update size from size property animation (runs before scale)
                    animate_sdf_scale, // Update SDF dimensions based on scale animation
                    animate_opacity,
                    animate_sdf_opacity,
                    animate_text_opacity,
                    apply_mask_clipping, // Apply mask clipping to masked layers
                    hot_reload_shader,   // Hot-reload shader when 'R' is pressed
                )
                    .chain(),
            );
    }
}

/// Create a 1x1 white pixel texture for solid color sprites.
fn setup_white_pixel(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
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

/// System to collect pending layers when a project finishes loading.
/// Note: This doesn't spawn entities immediately - the lifecycle system handles that.
fn spawn_loaded_projects(
    mut commands: Commands,
    mut query: Query<(Entity, &mut AmProjectRoot)>,
    projects: Res<Assets<AmProject>>,
    mut playback: ResMut<AmPlayback>,
) {
    for (entity, mut root) in query.iter_mut() {
        if root.spawned {
            continue;
        }

        if let Some(project) = projects.get(&root.handle) {
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

            // Build scene configuration
            let config = AmSceneConfig {
                canvas_width: project.scene.width as f32,
                canvas_height: project.scene.height as f32,
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

            // Add the pending layers component to the project root
            commands
                .entity(entity)
                .insert(crate::scene::AmPendingLayers {
                    layers: pending_layers,
                    spawned_entities: std::collections::HashMap::new(),
                });

            root.spawned = true;
            bevy::log::info!("Project ready for playback");
        }
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

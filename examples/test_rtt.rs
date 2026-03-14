use bevy::prelude::*;
use bevy::camera::RenderTarget;
use bevy::camera::visibility::RenderLayers;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "RTT Test".into(),
                resolution: bevy::window::WindowResolution::new(640, 480),
                visible: false,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(FrameCount(0))
        .insert_resource(Phase(0))
        .add_systems(Startup, setup_main)
        .add_systems(Update, (phase_system, check_and_exit).chain())
        .run();
}

#[derive(Resource)]
struct FrameCount(u32);
#[derive(Resource)]
struct Phase(u32);
#[derive(Component)]
struct ContentMarker;

fn setup_main(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn phase_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut phase: ResMut<Phase>,
    frame_count: Res<FrameCount>,
    content_query: Query<Entity, With<ContentMarker>>,
) {
    match (frame_count.0, phase.0) {
        (1, 0) => {
            phase.0 = 1;
            // Phase 1: Spawn parent entity (embed) on layer 0
            let parent = commands.spawn((
                Name::new("Parent"),
                Transform::default(),
                Visibility::Inherited,
                RenderLayers::layer(0),
            )).id();

            // Spawn content as CHILD of parent (like real embed content)
            // Content starts WITHOUT RenderLayers (will be set later)
            let child = commands.spawn((
                Name::new("Content"),
                ContentMarker,
                Sprite {
                    color: Color::linear_rgb(1.0, 0.0, 0.0),
                    custom_size: Some(Vec2::new(100.0, 100.0)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, 0.0),
            )).id();
            commands.entity(parent).add_child(child);
            eprintln!("Phase 1: Spawned parent {:?} and content child {:?}", parent, child);
        }
        (2, 1) => {
            phase.0 = 2;
            // Phase 2: Create RTT camera and set content's RenderLayers
            let render_texture = Image::new_target_texture(
                256, 256, TextureFormat::Rgba8Unorm, Some(TextureFormat::Rgba8UnormSrgb),
            );
            let handle = images.add(render_texture);

            commands.spawn((
                Camera2d,
                Camera { clear_color: ClearColorConfig::Custom(Color::NONE), order: -1, ..default() },
                RenderTarget::Image(handle.clone().into()),
                RenderLayers::layer(1),
                Transform::from_xyz(0.0, 0.0, 1000.0),
            ));

            // Set content's RenderLayers to layer 1 (deferred, like our system)
            for entity in content_query.iter() {
                commands.entity(entity).insert(RenderLayers::layer(1));
                eprintln!("Phase 2: Set content {:?} to RenderLayers::layer(1)", entity);
            }

            commands.spawn((
                Sprite { image: handle.clone(), custom_size: Some(Vec2::new(400.0, 400.0)), ..default() },
                Transform::from_xyz(0.0, 0.0, 0.0),
            ));
        }
        _ => {}
    }
}

fn check_and_exit(mut frame_count: ResMut<FrameCount>, mut commands: Commands) {
    frame_count.0 += 1;
    if frame_count.0 == 10 {
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk("rtt_test_screenshot.png"));
    }
    if frame_count.0 == 15 { std::process::exit(0); }
}

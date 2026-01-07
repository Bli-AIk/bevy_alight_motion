//! Example player for Alight Motion projects.
//!
//! Usage:
//!   cargo run -p bevy_alight_motion --example player -- <project_name>
//!
//! Available projects:
//!   - simple_gb (default)
//!   - complex_1
//!   - complex_2
//!   - complex_3
//!
//! Controls:
//! - Space: Play/Pause toggle
//! - R: Reset to beginning (keeps current play state)
//! - P: Replay from beginning (resets and plays)
//! - Left/Right: Seek backward/forward by 50ms
//! - Up/Down: Speed up/slow down playback
//! - L: Toggle loop mode

use bevy::prelude::*;
use bevy_alight_motion::prelude::*;

/// Get the project file based on CLI argument.
fn get_project_file() -> String {
    let args: Vec<String> = std::env::args().collect();
    let project_name = args.get(1).map(|s| s.as_str()).unwrap_or("simple_gb");

    let path = match project_name {
        "simple_gb" => "am/simple_gb.amproj",
        "complex_1" => "am/complex_examples_1.amproj",
        "complex_2" => "am/complex_examples_2.amproj",
        "complex_3" => "am/complex_examples_3.amproj",
        other => {
            // Try to use the argument directly as a path
            return format!("am/{}.amproj", other);
        }
    };

    path.to_string()
}

fn main() {
    let project_file = get_project_file();
    println!("Loading project: {}", project_file);

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("Alight Motion Player - {}", project_file),
                resolution: (1280, 960).into(),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        // Black background matching AM project
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(ProjectFile(project_file))
        .init_resource::<DebugOverlaySettings>()
        .add_plugins(AlightMotionPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_input, update_ui, debug_sprites, toggle_debug_overlay))
        .run();
}

/// Resource to store the project file path.
#[derive(Resource)]
struct ProjectFile(String);

/// UI text component for status display.
#[derive(Component)]
struct StatusText;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, project_file: Res<ProjectFile>) {
    // Spawn camera
    commands.spawn(Camera2d);

    // Load the AM project from assets folder
    load_am_project(&mut commands, &asset_server, &project_file.0);

    // Spawn UI for status display
    commands.spawn((
        Text::new("Loading..."),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        StatusText,
    ));

    // Instructions - clear English key descriptions
    commands.spawn((
        Text::new("[Space] Play/Pause | [R] Reset | [P] Replay | [Left/Right] Seek | [Up/Down] Speed | [L] Loop"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

/// Debug system to print sprite info once
fn debug_sprites(query: Query<(&AmLayerMarker, &Transform, &Sprite), Added<Sprite>>) {
    for (marker, transform, sprite) in query.iter() {
        println!(
            "Sprite added: '{}' at ({:.1},{:.1},{:.1}) scale=({:.2},{:.2}) alpha={:.2} size={:?}",
            marker.label,
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
            transform.scale.x,
            transform.scale.y,
            sprite.color.alpha(),
            sprite.custom_size
        );
    }
}

fn handle_input(keyboard: Res<ButtonInput<KeyCode>>, mut playback: ResMut<AmPlayback>) {
    // Play/Pause toggle
    if keyboard.just_pressed(KeyCode::Space) {
        playback.toggle();
    }

    // Reset (keeps current play/pause state)
    if keyboard.just_pressed(KeyCode::KeyR) {
        playback.reset();
    }

    // Replay (reset and start playing)
    if keyboard.just_pressed(KeyCode::KeyP) {
        playback.reset();
        playback.playing = true;
    }

    // Seek backward/forward by 50ms
    if keyboard.pressed(KeyCode::ArrowLeft) {
        playback.current_time_ms = (playback.current_time_ms - 50.0).max(0.0);
    }
    if keyboard.pressed(KeyCode::ArrowRight) {
        playback.current_time_ms = (playback.current_time_ms + 50.0).min(playback.total_time_ms);
    }

    // Speed control (up = faster, down = slower)
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        playback.speed = (playback.speed + 0.1).min(4.0);
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        playback.speed = (playback.speed - 0.1).max(0.1);
    }

    // Loop mode toggle
    if keyboard.just_pressed(KeyCode::KeyL) {
        playback.looping = !playback.looping;
    }
}

fn update_ui(playback: Res<AmPlayback>, mut query: Query<&mut Text, With<StatusText>>) {
    for mut text in query.iter_mut() {
        let status = if playback.playing {
            "Playing"
        } else {
            "Paused"
        };
        let loop_status = if playback.looping { "Loop" } else { "Once" };

        **text = format!(
            "{} | Time: {:.0}/{:.0}ms | Speed: {:.2}x | {}",
            status, playback.current_time_ms, playback.total_time_ms, playback.speed, loop_status
        );
    }
}

/// Resource to control debug overlay visibility.
#[derive(Resource, Default)]
struct DebugOverlaySettings {
    show_overlay: bool,
}

/// Component for the debug overlay image entity.
#[derive(Component)]
struct DebugOverlay;

/// Toggle the debug overlay with the F4 key.
fn toggle_debug_overlay(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<DebugOverlaySettings>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    overlay_query: Query<Entity, With<DebugOverlay>>,
    window_query: Query<&Window>,
) {
    if keyboard.just_pressed(KeyCode::F4) {
        settings.show_overlay = !settings.show_overlay;

        if settings.show_overlay {
            // Remove any existing overlay entity before spawning a new one.
            for entity in overlay_query.iter() {
                commands.entity(entity).despawn();
            }

            // Look up the most recently modified image in the debug folder.
            if let Some(latest_image_path) = find_latest_debug_image() {
                println!("Loading debug overlay image: {}", latest_image_path);

                // Load the selected image asset.
                let image_handle: Handle<Image> = asset_server.load(&latest_image_path);

                // Query the current window size for correct scaling.
                if let Ok(window) = window_query.single() {
                    let window_width = window.width();
                    let window_height = window.height();

                    // Spawn the overlay node with a semi-transparent background.
                    commands
                        .spawn((
                            Name::new("DebugOverlay"),
                            DebugOverlay,
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                top: Val::Px(0.0),
                                left: Val::Px(0.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            ZIndex(1000),
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                ImageNode {
                                    image: image_handle,
                                    // Render the image as semi-transparent.
                                    color: Color::srgba(1.0, 1.0, 1.0, 0.5),
                                    ..default()
                                },
                                Node {
                                    // Scale the image to fit the window while preserving aspect ratio.
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    max_width: Val::Px(window_width),
                                    max_height: Val::Px(window_height),
                                    ..default()
                                },
                            ));
                        });

                    println!("Debug image overlay: ON");
                }
            }
        } else {
            // Remove the overlay entity.
            for entity in overlay_query.iter() {
                commands.entity(entity).despawn();
            }
            println!("Debug image overlay: OFF");
        }
    }
}

/// Find the most recently modified image in the debug folder.
fn find_latest_debug_image() -> Option<String> {
    use std::fs;
    use std::path::Path;
    use std::time::SystemTime;

    // Check multiple potential debug folder locations.
    let possible_paths = [
        "crates/bevy_alight_motion/assets/debug",
        "assets/debug",
        "../souprune/assets/debug",
        "crates/souprune/assets/debug",
    ];
    let extensions = ["png", "jpg", "jpeg", "gif", "bmp", "tiff"];

    let mut latest_file: Option<(String, SystemTime)> = None;
    let mut found_debug_folder = false;

    for debug_path in &possible_paths {
        if !Path::new(debug_path).exists() {
            continue;
        }

        found_debug_folder = true;

        if let Ok(entries) = fs::read_dir(debug_path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        if let Some(file_name) = entry.file_name().to_str() {
                            // Check whether the file uses a supported image extension.
                            if let Some(extension) = file_name.split('.').next_back() {
                                if extensions.contains(&extension.to_lowercase().as_str()) {
                                    if let Ok(metadata) = entry.metadata() {
                                        if let Ok(modified) = metadata.modified() {
                                            let relative_path = format!("debug/{}", file_name);

                                            if latest_file.is_none() || latest_file.as_ref().unwrap().1 < modified {
                                                latest_file = Some((relative_path, modified));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Once files are found in this path we can stop probing others.
            if latest_file.is_some() {
                break;
            }
        }
    }

    if !found_debug_folder {
        println!("Debug folder not found in any of the expected locations");
        return None;
    }

    if let Some((path, _)) = latest_file {
        println!("Selected latest debug image: {}", path);
        Some(path)
    } else {
        println!("No image files found in debug folder");
        None
    }
}

#![allow(clippy::collapsible_if, deprecated)]
//! Example player for Alight Motion projects.
//! 用以播放 Alight Motion 工程的示例播放器。
//!
//! # Usage / 用法
//!
//! ### Interactive Playback / 交互式播放
//! ```bash
//! cargo run -p bevy_alight_motion --example player -- <project_name>
//! ```
//!
//! ### With Debug Inspector / 启用 Debug 面板
//! ```bash
//! cargo run -p bevy_alight_motion --example player --features debug -- <project_name>
//! ```
//!
//! ### With Video Overlay / 启用视频覆盖层
//! ```bash
//! cargo run -p bevy_alight_motion --example player --features video-debug -- <project_name>
//! ```
//!
#![allow(clippy::collapsible_else_if, clippy::manual_is_multiple_of)]
//! ### Run Video Comparison Test / 运行视频比对测试
//! ```bash
//! cargo run -p bevy_alight_motion --example player --features video-comparison -- <project_name>
//! ```
//! This runs a non-interactive test that compares rendered frames against a reference video
//! and generates a report in the `reports/` directory.
//!
//! （此命令会运行一个非交互式测试，将渲染结果与参考视频逐帧比对，并在 `reports/` 目录下生成报告。）
//!
//! ### Run Frame Test (FPS Benchmark) / 运行帧测试（FPS 基准）
//! ```bash
//! cargo run -p bevy_alight_motion --example player --features frame-test -- <project_name>
//! ```
//! This plays the project and measures FPS performance.
//! - ≥144 FPS: PASS ✅  |  60-144 FPS: WARNING ⚠️  |  <60 FPS: FAIL ❌
//!
//! （此命令会播放工程并测量 FPS 性能。）
//!
//! # Available projects / 可用工程
//!   - `simple_gb` (default)
//!   - `basic_shape`
//!   - `basic_pivot`
//!   - `complex_1`
//!   - `complex_2`
//!   - `complex_3`
//!
//! # Controls (Interactive Mode) / 交互模式下的按键操作
//! - **Space**: Play/Pause toggle
//! - **R**: Reset to beginning (keeps current play state)
//! - **P**: Replay from beginning (resets and plays)
//! - **Left/Right**: Step backward/forward by one frame (pauses playback)
//! - **Up/Down**: Speed up/slow down playback
//! - **L**: Toggle loop mode
//! - **F1**: Toggle inspector window (requires `--features debug`)
//! - **F4**: Toggle debug image overlay
//! - **F6**: Toggle video debug overlay (requires `--features video-debug`)

#[path = "video_utils.rs"]
mod video_utils;

use bevy::prelude::MeshMaterial2d;
use bevy::prelude::*;
use bevy_alight_motion::prelude::*;

#[cfg(feature = "debug")]
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};

/// Get the project file based on CLI argument.
fn get_project_file() -> String {
    let args: Vec<String> = std::env::args().collect();
    let project_name = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("complex/misc/simple_gb");

    // Shortcuts for common projects
    let path = match project_name {
        "simple_gb" => "projects/complex/misc/simple_gb.amproj",
        "complex_1" => "projects/complex/examples/1.amproj",
        "complex_2" => "projects/complex/examples/2.amproj",
        "complex_3" => "projects/complex/examples/3.amproj",
        other => {
            // Use the argument directly as a path under projects/
            // e.g., "basic/shape/shape" -> "projects/basic/shape/shape.amproj"
            return format!("projects/{}.amproj", other);
        }
    };

    path.to_string()
}

fn main() {
    let project_file = get_project_file();
    println!("Loading project: {}", project_file);

    // Default resolution
    let mut resolution = Vec2::new(1280.0, 960.0);

    // In comparison mode, try to match video resolution
    #[cfg(feature = "video-comparison")]
    {
        if let Some(video_path) = video_utils::find_debug_video(Some(&project_file)) {
            if let Some((width, height)) = video_utils::get_video_resolution(&video_path) {
                resolution = Vec2::new(width as f32, height as f32);
                println!(
                    "Comparison mode: Using video resolution {}x{}",
                    width, height
                );
            } else {
                println!("Comparison mode: Could not read video resolution, using default.");
            }
        }
    }

    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: format!("Alight Motion Player - {}", project_file),
            resolution: bevy::window::WindowResolution::new(
                resolution.x as u32,
                resolution.y as u32,
            ),
            resizable: false,
            // Disable VSync in frame-test mode for accurate FPS measurement
            #[cfg(feature = "frame-test")]
            present_mode: bevy::window::PresentMode::AutoNoVsync,
            // In comparison mode, we might want to hide the window or keep it for debugging
            ..default()
        }),
        ..default()
    }))
    // Black background matching AM project
    .insert_resource(ClearColor(Color::BLACK))
    .insert_resource(ProjectFile(project_file.clone()))
    .insert_resource(AmProjectResolution::FitWindow)
    .init_resource::<DebugOverlaySettings>()
    .init_resource::<MaskDebugSettings>()
    .add_plugins(AlightMotionPlugin)
    .add_systems(Startup, setup);

    // Only add interactive UI systems when NOT in frame-test mode
    #[cfg(not(feature = "frame-test"))]
    app.add_systems(
        Update,
        (
            handle_input,
            update_ui,
            debug_sprites,
            debug_unified_effects,
            debug_position_changes,
            debug_sdf_shapes,
            toggle_debug_overlay,
            toggle_mask_debug,
        ),
    );

    // In frame-test mode, only add the FPS counter UI (no interaction, no "Playing" text)
    #[cfg(feature = "frame-test")]
    app.add_systems(Update, frame_test_systems::update_fps_display);

    // Add video debug systems when video-debug feature is enabled
    #[cfg(feature = "video-debug")]
    {
        app.init_resource::<VideoDebugState>()
            .add_systems(Startup, setup_video_debug)
            .add_systems(Update, (load_video_frames, update_video_debug_overlay));
        println!("Video debug mode enabled: Press F6 to toggle video overlay");
    }

    // Add video comparison systems
    #[cfg(feature = "video-comparison")]
    {
        app.init_resource::<video_comparison_systems::ComparisonState>()
            .add_systems(Startup, video_comparison_systems::setup_comparison)
            // Pause playback at the very beginning of each frame to prevent time advancing
            .add_systems(First, video_comparison_systems::ensure_paused_during_load)
            .add_systems(Update, video_comparison_systems::comparison_loop);
        println!("Video comparison mode enabled: Running automated test...");
    }

    // Add frame test (FPS benchmark) systems
    #[cfg(feature = "frame-test")]
    {
        app.init_resource::<frame_test_systems::FrameTestState>()
            .add_systems(Startup, frame_test_systems::setup_frame_test)
            .add_systems(Update, frame_test_systems::frame_test_loop);
        println!("Frame test mode enabled: Running FPS benchmark...");
    }

    // Add inspector plugin when debug feature is enabled
    #[cfg(feature = "debug")]
    {
        app.add_plugins(EguiPlugin::default());
        app.add_plugins(WorldInspectorPlugin::default());
        println!("Debug mode enabled: Inspector will be shown in the window");
    }

    app.run();
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

    // Only spawn UI if NOT in comparison or frame-test mode
    #[cfg(not(any(feature = "video-comparison", feature = "frame-test")))]
    {
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

        // Instructions
        commands.spawn((
            Text::new("[Space] Play/Pause | [R] Reset | [P] Replay | [F5] Force Stop | [LEFT/RIGHT] Frame Step | [UP/DOWN] Speed | [L] Loop"),
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
}

/// Debug system to print sprite info once
#[allow(unused_variables)]
fn debug_sprites(
    query: Query<(&AmLayerMarker, &Transform, &GlobalTransform, &Sprite), Added<Sprite>>,
) {
    #[cfg(not(feature = "video-comparison"))]
    for (marker, transform, global_transform, _sprite) in query.iter() {
        let global_z = global_transform.translation().z;
        trace!(
            "[SpawnDebug] Sprite added: '{}' at local=({:.1},{:.1},{:.4}) global=({:.1},{:.1},{:.4}) scale=({:.2},{:.2})",
            marker.label,
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
            global_transform.translation().x,
            global_transform.translation().y,
            global_z,
            transform.scale.x,
            transform.scale.y,
        );
    }
}

/// Debug system to print UnifiedEffect sprite info once
#[allow(unused_variables)]
fn debug_unified_effects(
    query: Query<
        (
            &AmLayerMarker,
            &Transform,
            &GlobalTransform,
            &MeshMaterial2d<bevy_alight_motion::masked_sprite::UnifiedEffectMaterial>,
        ),
        Added<bevy_alight_motion::masked_sprite::UnifiedEffectMarker>,
    >,
    materials: Res<Assets<bevy_alight_motion::masked_sprite::UnifiedEffectMaterial>>,
) {
    #[cfg(not(feature = "video-comparison"))]
    for (marker, transform, global_transform, material_handle) in query.iter() {
        let mesh_offset = if let Some(material) = materials.get(&material_handle.0) {
            (material.mesh_offset().x, material.mesh_offset().y)
        } else {
            (0.0, 0.0)
        };
        trace!(
            "[SpawnDebug] UnifiedEffect added: '{}' at local=({:.1},{:.1},{:.4}) global=({:.1},{:.1},{:.4}) scale=({:.2},{:.2}) mesh_offset=({:.2},{:.2})",
            marker.label,
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
            global_transform.translation().x,
            global_transform.translation().y,
            global_transform.translation().z,
            transform.scale.x,
            transform.scale.y,
            mesh_offset.0,
            mesh_offset.1,
        );
    }
}

/// Debug system to track position changes per frame
#[allow(unused_variables)]
fn debug_position_changes(
    playback: Res<AmPlayback>,
    query: Query<
        (
            &AmLayerMarker,
            &Transform,
            &GlobalTransform,
            &MeshMaterial2d<bevy_alight_motion::masked_sprite::UnifiedEffectMaterial>,
        ),
        With<bevy_alight_motion::masked_sprite::UnifiedEffectMarker>,
    >,
    _materials: Res<Assets<bevy_alight_motion::masked_sprite::UnifiedEffectMaterial>>,
) {
    // Debug output for 长方形 2
    #[cfg(not(feature = "video-comparison"))]
    for (marker, transform, global_transform, _material_handle) in query.iter() {
        if marker.label == "长方形 2"
            && playback.current_time_ms > 2000.0
            && playback.current_time_ms < 2200.0
        {
            let gt = global_transform.translation();
            println!(
                "[PosDebug] time={:.1}ms '{}' local=({:.1},{:.1}) global=({:.1},{:.1}) scale=({:.3},{:.3})",
                playback.current_time_ms,
                marker.label,
                transform.translation.x,
                transform.translation.y,
                gt.x,
                gt.y,
                transform.scale.x,
                transform.scale.y,
            );
        }
    }
}

/// Debug system to print SDF shape info once
fn debug_sdf_shapes(
    query: Query<
        (&Name, &Transform, &GlobalTransform),
        Added<MeshMaterial2d<bevy_alight_motion::sdf_material::SdfMaterial>>,
    >,
) {
    #[allow(unused_variables)]
    let _ = &query;
}

#[allow(unused_variables, unused_mut)]
fn handle_input(keyboard: Res<ButtonInput<KeyCode>>, mut playback: ResMut<AmPlayback>) {
    // Disable manual input in comparison mode
    #[cfg(not(feature = "video-comparison"))]
    {
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

        // Force stop toggle (F5) - freezes all animation updates for inspector editing
        if keyboard.just_pressed(KeyCode::F5) {
            playback.toggle_force_stop();
            let status = if playback.force_stopped { "ON" } else { "OFF" };
            println!(
                "Force stop: {} (animation updates frozen for inspector editing)",
                status
            );
        }

        // Frame-by-frame stepping (Left/Right arrows)
        // Pauses playback and moves one frame at a time
        let frame_duration_ms = 1000.0 / 30.0; // 30 fps
        if keyboard.just_pressed(KeyCode::ArrowLeft) {
            playback.playing = false;
            playback.current_time_ms = (playback.current_time_ms - frame_duration_ms).max(0.0);
            println!("[FrameStep] time={:.1}ms", playback.current_time_ms);
        }
        if keyboard.just_pressed(KeyCode::ArrowRight) {
            playback.playing = false;
            playback.current_time_ms =
                (playback.current_time_ms + frame_duration_ms).min(playback.total_time_ms);
            println!("[FrameStep] time={:.1}ms", playback.current_time_ms);
        }

        // Speed control (up = faster, down = slower)
        if keyboard.just_pressed(KeyCode::ArrowUp) {
            playback.speed = (playback.speed + 0.1).min(4.0);
            println!("[Speed] {:.1}x", playback.speed);
        }
        if keyboard.just_pressed(KeyCode::ArrowDown) {
            playback.speed = (playback.speed - 0.1).max(0.1);
            println!("[Speed] {:.1}x", playback.speed);
        }

        // Loop mode toggle
        if keyboard.just_pressed(KeyCode::KeyL) {
            playback.looping = !playback.looping;
        }
    }
}

fn update_ui(playback: Res<AmPlayback>, mut query: Query<&mut Text, With<StatusText>>) {
    for mut text in query.iter_mut() {
        let status = if playback.force_stopped {
            "FORCE STOPPED"
        } else if playback.playing {
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
                if let Ok(file_type) = entry.file_type()
                    && file_type.is_file()
                    && let Some(file_name) = entry.file_name().to_str()
                {
                    // Check whether the file uses a supported image extension.
                    if let Some(extension) = file_name.split('.').next_back()
                        && extensions.contains(&extension.to_lowercase().as_str())
                        && let Ok(metadata) = entry.metadata()
                        && let Ok(modified) = metadata.modified()
                    {
                        let relative_path = format!("debug/{}", file_name);

                        if latest_file.is_none() || latest_file.as_ref().unwrap().1 < modified {
                            latest_file = Some((relative_path, modified));
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
        // println!("Debug folder not found in any of the expected locations");
        return None;
    }

    if let Some((path, _)) = latest_file {
        // println!("Selected latest debug image: {}", path);
        Some(path)
    } else {
        // println!("No image files found in debug folder");
        None
    }
}

// ============================================================================
// Mask Debug Visualization
// ============================================================================

/// Resource to control mask debug visualization
#[derive(Resource, Default)]
struct MaskDebugSettings {
    #[allow(dead_code)]
    show_masks: bool,
}

/// Component for mask debug visualization entities
#[derive(Component)]
struct MaskDebugVisual;

/// Toggle mask debug visualization with the M key
#[allow(unused_variables, unused_mut)]
fn toggle_mask_debug(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<MaskDebugSettings>,
    mut commands: Commands,
    mask_query: Query<&AmMaskInfo, Without<MaskDebugVisual>>,
    debug_visual_query: Query<Entity, With<MaskDebugVisual>>,
) {
    #[cfg(not(feature = "video-comparison"))]
    {
        if keyboard.just_pressed(KeyCode::KeyM) {
            settings.show_masks = !settings.show_masks;

            if settings.show_masks {
                // Spawn mask visualization entities for each mask
                // First, find unique mask centers (masks may be shared across many entities)
                let mut seen_masks: std::collections::HashSet<(i32, i32, i32, i32)> =
                    std::collections::HashSet::new();

                for mask_info in mask_query.iter() {
                    for mask in &mask_info.masks {
                        // Create a key based on mask position and size (rounded to int for comparison)
                        let key = (
                            (mask.center.x * 10.0) as i32,
                            (mask.center.y * 10.0) as i32,
                            (mask.half_size.x * 10.0) as i32,
                            (mask.half_size.y * 10.0) as i32,
                        );

                        if seen_masks.contains(&key) {
                            continue;
                        }
                        seen_masks.insert(key);

                        // Spawn a semi-transparent rectangle to visualize the mask
                        println!(
                            "[MASK DEBUG] Visualizing mask at ({:.1},{:.1}) size ({:.1},{:.1})",
                            mask.center.x,
                            mask.center.y,
                            mask.half_size.x * 2.0,
                            mask.half_size.y * 2.0
                        );

                        // Create a sprite to show the mask region
                        commands.spawn((
                            Name::new("MaskDebugVisual"),
                            MaskDebugVisual,
                            Sprite {
                                color: Color::srgba(1.0, 0.0, 0.0, 0.3), // Semi-transparent red
                                custom_size: Some(Vec2::new(
                                    mask.half_size.x * 2.0,
                                    mask.half_size.y * 2.0,
                                )),
                                ..default()
                            },
                            Transform::from_translation(Vec3::new(
                                mask.center.x,
                                mask.center.y,
                                100.0, // High z to render on top
                            )),
                        ));
                    }
                }

                if seen_masks.is_empty() {
                    println!("[MASK DEBUG] No masks found to visualize");
                } else {
                    println!("[MASK DEBUG] Showing {} mask region(s)", seen_masks.len());
                }
            } else {
                // Remove debug visualizations
                let count = debug_visual_query.iter().count();
                for entity in debug_visual_query.iter() {
                    commands.entity(entity).despawn();
                }
                println!("[MASK DEBUG] Hidden {} mask visualization(s)", count);
            }
        }
    }
}

// ============================================================================
// Video Debug Overlay (requires --features video-debug)
// ============================================================================

#[cfg(feature = "video-debug")]
mod video_debug_systems {
    use super::*;
    use crate::video_utils;
    use std::path::PathBuf;

    /// Resource to control video debug overlay state
    #[derive(Resource)]
    pub struct VideoDebugState {
        /// Whether the video overlay is enabled
        pub enabled: bool,
        /// Extracted frame paths (sorted by frame number)
        pub frame_paths: Vec<PathBuf>,
        /// Frame handles loaded into Bevy
        pub frame_handles: Vec<Handle<Image>>,
        /// Current frame index
        pub current_frame: usize,
        /// Video frame rate
        pub fps: f32,
        /// Time of last frame update
        #[allow(dead_code)]
        pub last_frame_time: f32,
        /// Total duration in seconds
        pub duration: f32,
        /// Temp directory for extracted frames
        pub temp_dir: Option<PathBuf>,
        /// Whether frames have been loaded into Bevy
        pub frames_loaded: bool,
        /// Whether all frames are ready (fully loaded)
        #[allow(dead_code)]
        pub frames_ready: bool,
    }

    impl Default for VideoDebugState {
        fn default() -> Self {
            Self {
                enabled: false,
                frame_paths: Vec::new(),
                frame_handles: Vec::new(),
                current_frame: 0,
                fps: 12.0,
                last_frame_time: 0.0,
                duration: 0.0,
                temp_dir: None,
                frames_loaded: false,
                frames_ready: false,
            }
        }
    }

    /// Component for the video debug overlay entity
    #[derive(Component)]
    pub struct VideoDebugOverlay;

    /// Component for the image node that displays frames
    #[derive(Component)]
    pub struct VideoDebugImageNode;

    /// Setup video debug overlay on startup
    pub fn setup_video_debug(
        mut state: ResMut<VideoDebugState>,
        project_file: Res<super::ProjectFile>,
    ) {
        // Find video file
        let Some(video_path) = video_utils::find_debug_video(Some(&project_file.0)) else {
            println!("[VIDEO DEBUG] No video file found in debug folder");
            return;
        };

        println!("[VIDEO DEBUG] Found video: {:?}", video_path);

        // Get video info
        let Some((fps, duration)) = video_utils::get_video_info(&video_path) else {
            println!("[VIDEO DEBUG] Failed to get video info");
            return;
        };

        println!(
            "[VIDEO DEBUG] Video info: {:.2} FPS, {:.2}s duration",
            fps, duration
        );

        // Extract frames
        let Some(temp_dir) = video_utils::extract_frames(&video_path, fps) else {
            println!("[VIDEO DEBUG] Failed to extract frames");
            return;
        };

        // Collect frame paths
        let mut frame_paths: Vec<PathBuf> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&temp_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "png").unwrap_or(false) {
                    frame_paths.push(path);
                }
            }
        }

        // Sort by filename (frame_000001.png, frame_000002.png, etc.)
        frame_paths.sort();

        let frame_count = frame_paths.len();
        println!("[VIDEO DEBUG] Extracted {} frames", frame_count);

        if frame_count == 0 {
            println!("[VIDEO DEBUG] No frames extracted!");
            return;
        }

        state.fps = fps;
        state.duration = duration;
        state.frame_paths = frame_paths;
        state.temp_dir = Some(temp_dir);
        state.enabled = true; // Auto-enable
    }

    /// Load frame images into Bevy asset system
    pub fn load_video_frames(mut state: ResMut<VideoDebugState>, asset_server: Res<AssetServer>) {
        if state.frames_loaded || state.frame_paths.is_empty() {
            return;
        }

        // Load all frames as images using relative asset paths
        // frame_paths contain paths like:
        // - Absolute: /path/to/crates/bevy_alight_motion/assets/debug/_video_frames/video_name/frame_000001.png
        // - Relative: assets/projects/xxx/_video_frames/video_name/frame_000001.png
        // We need to extract the asset-relative path (everything after "assets/")
        state.frame_handles = state
            .frame_paths
            .iter()
            .filter_map(|path| {
                let path_str = path.to_string_lossy();
                // Find "assets/" and extract everything after it
                if let Some(idx) = path_str.find("assets/") {
                    let asset_path = &path_str[idx + "assets/".len()..];
                    Some(asset_server.load(asset_path.to_string()))
                } else {
                    println!("[VIDEO DEBUG] Could not find asset path in: {:?}", path);
                    None
                }
            })
            .collect();

        state.frames_loaded = true;
        println!(
            "[VIDEO DEBUG] Loading {} frame handles...",
            state.frame_handles.len()
        );
    }

    /// Update video debug overlay each frame
    pub fn update_video_debug_overlay(
        mut commands: Commands,
        mut state: ResMut<VideoDebugState>,
        keyboard: Res<ButtonInput<KeyCode>>,
        playback: Res<AmPlayback>,
        overlay_query: Query<Entity, With<VideoDebugOverlay>>,
        mut image_node_query: Query<&mut ImageNode, With<VideoDebugImageNode>>,
        window_query: Query<&Window>,
    ) {
        // Toggle with F6
        if keyboard.just_pressed(KeyCode::F6) {
            state.enabled = !state.enabled;
            println!(
                "[VIDEO DEBUG] Overlay {}",
                if state.enabled { "ON" } else { "OFF" }
            );

            if !state.enabled {
                // Remove overlay entities
                for entity in overlay_query.iter() {
                    commands.entity(entity).despawn();
                }
                return;
            }
        }

        if !state.enabled || state.frame_handles.is_empty() {
            return;
        }

        // Calculate which frame to show based on playback time
        let current_time = playback.current_time_ms / 1000.0; // Convert to seconds
        let frame_duration = 1.0 / state.fps;
        let total_frames = state.frame_handles.len();

        // Calculate frame index (with looping)
        let frame_index = ((current_time / frame_duration) as usize) % total_frames;

        // Spawn overlay if it doesn't exist
        if overlay_query.is_empty() {
            if let Ok(window) = window_query.single() {
                let window_width = window.width();
                let window_height = window.height();

                let initial_handle = state.frame_handles[frame_index].clone();

                commands
                    .spawn((
                        Name::new("VideoDebugOverlay"),
                        VideoDebugOverlay,
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
                        ZIndex(999),
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            VideoDebugImageNode,
                            ImageNode {
                                image: initial_handle,
                                color: Color::srgba(1.0, 1.0, 1.0, 0.5), // Semi-transparent
                                ..default()
                            },
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                max_width: Val::Px(window_width),
                                max_height: Val::Px(window_height),
                                ..default()
                            },
                        ));
                    });

                state.current_frame = frame_index;
            }
        } else if frame_index != state.current_frame {
            // Update the displayed frame
            for mut image_node in image_node_query.iter_mut() {
                image_node.image = state.frame_handles[frame_index].clone();
            }
            state.current_frame = frame_index;
        }
    }

    /// Cleanup temp files on exit
    impl Drop for VideoDebugState {
        fn drop(&mut self) {
            if let Some(temp_dir) = &self.temp_dir {
                if let Err(e) = std::fs::remove_dir_all(temp_dir) {
                    eprintln!("[VIDEO DEBUG] Failed to cleanup temp dir: {:?}", e);
                }
            }
        }
    }
}

#[cfg(feature = "video-debug")]
use video_debug_systems::*;

// ============================================================================
// Video Comparison (requires --features video-comparison)
// ============================================================================

#[cfg(feature = "video-comparison")]
mod video_comparison_systems {
    use super::*;
    use crate::video_utils;
    use bevy::app::AppExit;
    use bevy::ecs::message::MessageWriter;
    use bevy::render::view::screenshot::{Screenshot, save_to_disk};
    use bevy::window::PrimaryWindow;
    use owo_colors::OwoColorize;
    use serde::Deserialize;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[derive(Resource)]
    pub struct ComparisonState {
        pub frame_paths: Vec<PathBuf>,
        pub current_frame: usize,
        pub fps: f32,
        pub temp_dir: Option<PathBuf>,
        pub stage: TestStage,
        pub wait_frames: u32, // Wait frame count instead of timer for stability
        #[allow(dead_code)]
        pub total_diff: f64,
        pub frame_scores: Vec<f32>,
        pub report_dir: PathBuf,
        // Config thresholds
        pub avg_threshold: f32,
        pub frame_threshold: f32,
        pub frame_offset: f32,         // Frame time offset for alignment
        pub min_frame_similarity: f32, // Minimum similarity for any frame
        pub max_failed_rate: f32,      // Maximum ratio of failed frames allowed
        pub max_critical_rate: f32, // Maximum ratio of critical frames (below min_frame_similarity) allowed
        pub project_name: String,
        pub skipped: bool,
        pub pending_time_ms: Option<f32>, // Time to set in next First schedule
        pub render_wait_frames: u32, // Frames to wait after applying time before allowing screenshot
    }

    #[derive(PartialEq, Debug)]
    pub enum TestStage {
        Initializing,
        WaitingForProjectLoad, // Wait for project to load and first frame to render
        SettingTime,
        WaitingForRender,
        Capturing,
        WaitingForScreenshot, // Wait one frame for screenshot to be executed
        Comparing,
        Finished,
        #[allow(dead_code)]
        Cancelled, // User closed the window
    }

    impl Default for ComparisonState {
        fn default() -> Self {
            Self {
                frame_paths: Vec::new(),
                current_frame: 0,
                fps: 12.0,
                temp_dir: None,
                stage: TestStage::Initializing,
                wait_frames: 0, // Frame counter for stable waiting
                total_diff: 0.0,
                frame_scores: Vec::new(),
                report_dir: PathBuf::from("comparison_report"),
                avg_threshold: 0.98,
                frame_threshold: 0.98,
                frame_offset: 0.0,
                min_frame_similarity: 0.75,
                max_failed_rate: 0.05,
                max_critical_rate: 0.02,
                project_name: String::new(),
                skipped: false,
                pending_time_ms: None,
                render_wait_frames: 0,
            }
        }
    }
    #[derive(Deserialize, Debug)]
    struct ComparisonConfig {
        default: ProjectConfig,
        #[serde(default)]
        overrides: HashMap<String, OverrideConfig>,
    }

    fn default_avg_threshold() -> f32 {
        0.95
    }
    fn default_frame_threshold() -> f32 {
        0.95
    }
    #[derive(Deserialize, Debug, Clone, Copy)]
    struct ProjectConfig {
        #[serde(default)]
        skip: bool,
        #[serde(default = "default_avg_threshold")]
        avg_threshold: f32,
        #[serde(default = "default_frame_threshold")]
        frame_threshold: f32,
        #[serde(default)]
        frame_offset: f32,
        #[serde(default = "default_min_frame_similarity")]
        min_frame_similarity: f32,
        #[serde(default = "default_max_failed_rate")]
        max_failed_rate: f32,
        #[serde(default = "default_max_critical_rate")]
        max_critical_rate: f32,
    }

    // Override config with optional fields for proper inheritance from [default]
    #[derive(Deserialize, Debug, Clone, Copy)]
    struct OverrideConfig {
        #[serde(default)]
        skip: Option<bool>,
        avg_threshold: Option<f32>,
        frame_threshold: Option<f32>,
        frame_offset: Option<f32>,
        min_frame_similarity: Option<f32>,
        max_failed_rate: Option<f32>,
        max_critical_rate: Option<f32>,
    }

    impl OverrideConfig {
        fn merge_with(&self, base: &ProjectConfig) -> ProjectConfig {
            ProjectConfig {
                skip: self.skip.unwrap_or(base.skip),
                avg_threshold: self.avg_threshold.unwrap_or(base.avg_threshold),
                frame_threshold: self.frame_threshold.unwrap_or(base.frame_threshold),
                frame_offset: self.frame_offset.unwrap_or(base.frame_offset),
                min_frame_similarity: self
                    .min_frame_similarity
                    .unwrap_or(base.min_frame_similarity),
                max_failed_rate: self.max_failed_rate.unwrap_or(base.max_failed_rate),
                max_critical_rate: self.max_critical_rate.unwrap_or(base.max_critical_rate),
            }
        }
    }

    fn default_min_frame_similarity() -> f32 {
        0.75
    }

    fn default_max_failed_rate() -> f32 {
        0.05
    }

    fn default_max_critical_rate() -> f32 {
        0.02
    }

    pub fn setup_comparison(mut state: ResMut<ComparisonState>, project_file: Res<ProjectFile>) {
        // Prepare report dir
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let report_dir = PathBuf::from("reports").join(format!("run_{}", timestamp));
        std::fs::create_dir_all(&report_dir).expect("Failed to create report dir");
        state.report_dir = report_dir;

        // Extract project name from path - use relative path for config lookup
        // e.g., "projects/effects/repeat/basic.amproj" -> "effects/repeat/basic"
        let project_name = std::path::Path::new(&project_file.0)
            .strip_prefix("projects/")
            .ok()
            .and_then(|p| p.to_str())
            .map(|s| s.strip_suffix(".amproj").unwrap_or(s))
            .unwrap_or("unknown")
            .to_string();
        state.project_name = project_name.clone();

        // Load configuration
        let config_path = "crates/bevy_alight_motion/comparison_config.toml";
        let config = if let Ok(content) = std::fs::read_to_string(config_path) {
            match toml::from_str::<ComparisonConfig>(&content) {
                Ok(cfg) => Some(cfg),
                Err(e) => {
                    println!("[COMPARISON] Error parsing config: {}", e);
                    None
                }
            }
        } else {
            // Try simpler path if running from crate root
            if let Ok(content) = std::fs::read_to_string("comparison_config.toml") {
                match toml::from_str::<ComparisonConfig>(&content) {
                    Ok(cfg) => Some(cfg),
                    Err(e) => {
                        println!("[COMPARISON] Error parsing config: {}", e);
                        None
                    }
                }
            } else {
                println!("[COMPARISON] Config file not found, using defaults");
                None
            }
        };

        // Apply configuration
        if let Some(cfg) = config {
            let settings = if let Some(override_cfg) = cfg.overrides.get(&project_name) {
                override_cfg.merge_with(&cfg.default)
            } else {
                cfg.default
            };

            // Check if this test should be skipped
            if settings.skip {
                println!(
                    "{} {} (configured to skip)",
                    "[COMPARISON] SKIP:".yellow().bold(),
                    project_name.yellow()
                );
                state.skipped = true;
                state.stage = TestStage::Finished;
                return;
            }

            state.avg_threshold = settings.avg_threshold;
            state.frame_threshold = settings.frame_threshold;
            state.frame_offset = settings.frame_offset;
            state.min_frame_similarity = settings.min_frame_similarity;
            state.max_failed_rate = settings.max_failed_rate;
            state.max_critical_rate = settings.max_critical_rate;
            println!(
                "[COMPARISON] Config for '{}': avg_thresh={:.2}, frame_thresh={:.2}, frame_offset={:.2}, min_frame={:.2}, max_failed={:.1}%, max_critical={:.1}%",
                project_name,
                state.avg_threshold,
                state.frame_threshold,
                state.frame_offset,
                state.min_frame_similarity,
                state.max_failed_rate * 100.0,
                state.max_critical_rate * 100.0
            );
        }

        // Find and extract video
        let Some(video_path) = video_utils::find_debug_video(Some(&project_file.0)) else {
            println!(
                "{} {}",
                "[COMPARISON] SKIP:".yellow().bold(),
                "No video found for comparison!".yellow()
            );
            state.skipped = true;
            state.stage = TestStage::Finished;
            return;
        };

        println!("[COMPARISON] Using video: {:?}", video_path);

        let Some((fps, _)) = video_utils::get_video_info(&video_path) else {
            println!("[COMPARISON] Failed to get video info");
            state.stage = TestStage::Finished;
            return;
        };

        state.fps = fps;

        let Some(temp_dir) = video_utils::extract_frames(&video_path, fps) else {
            println!("[COMPARISON] Failed to extract frames");
            state.stage = TestStage::Finished;
            return;
        };

        state.temp_dir = Some(temp_dir.clone());

        // Collect paths
        let mut frame_paths: Vec<PathBuf> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&temp_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "png").unwrap_or(false) {
                    frame_paths.push(path);
                }
            }
        }
        frame_paths.sort();

        // Skip the first frame of reference video
        // AM video export's frame 1 is actually at t≈29ms, not t=0ms
        // Our shot_000000 at t=0ms doesn't match ref frame_000001
        // So we skip frame_000001 and start from frame_000002
        if !frame_paths.is_empty() {
            frame_paths.remove(0);
            println!(
                "[COMPARISON] Skipped first reference frame (AM video export timing mismatch)"
            );
        }

        state.frame_paths = frame_paths;

        println!(
            "[COMPARISON] Starting comparison of {} frames...",
            state.frame_paths.len()
        );
        state.stage = TestStage::WaitingForProjectLoad;
        state.wait_frames = 0;
    }

    /// Runs at the very beginning of each frame (First schedule) to prevent
    /// playback from advancing during load. This ensures animation doesn't
    /// "run through" the first few frames before comparison starts.
    /// Also handles force_stopped for screenshot capture timing and applies pending time changes.
    pub fn ensure_paused_during_load(
        mut state: ResMut<ComparisonState>,
        mut playback: ResMut<AmPlayback>,
    ) {
        // Apply pending time change (set in previous frame's SettingTime stage)
        // This ensures time is set in First schedule, BEFORE lifecycle_system runs in Update
        if let Some(time_ms) = state.pending_time_ms.take() {
            playback.current_time_ms = time_ms;
            // NOTE: Keep force_stopped=true for this frame!
            // Start a wait period - lifecycle needs to run with the new time first,
            // then we need to wait for the scene to be rendered.
            playback.force_stopped = true;
            state.render_wait_frames = 2; // Wait for mesh commands to be applied
            debug!(
                "[PAUSED] Applied pending time: {:.1}ms, render_wait_frames=2",
                time_ms
            );
            return; // Don't override force_stopped below
        }

        // Handle render_wait_frames countdown
        if state.render_wait_frames > 0 {
            state.render_wait_frames -= 1;
            if state.render_wait_frames > 0 {
                // Still waiting - allow lifecycle to run (force_stopped=false)
                // but don't proceed to screenshot yet
                playback.force_stopped = false;
                debug!(
                    "[PAUSED] render_wait_frames={} (allowing lifecycle)",
                    state.render_wait_frames
                );
            } else {
                // Wait complete - keep force_stopped=true for screenshot capture
                playback.force_stopped = true;
                debug!("[PAUSED] render_wait_frames=0 (ready for screenshot)");
            }
            return;
        }

        // Keep playback paused and at time 0 until we're in SettingTime stage
        match state.stage {
            TestStage::Initializing | TestStage::WaitingForProjectLoad => {
                if playback.current_time_ms != 0.0 || playback.playing {
                    println!(
                        "[COMPARISON] Resetting playback: was time={:.1}ms playing={}",
                        playback.current_time_ms, playback.playing
                    );
                }
                playback.playing = false;
                playback.current_time_ms = 0.0;
            }
            // During these stages, freeze lifecycle to prevent spawning
            // SettingTime: comparison_loop will set pending_time_ms but lifecycle shouldn't run yet
            // Capturing/WaitingForScreenshot/Comparing: waiting for screenshot to complete
            TestStage::SettingTime
            | TestStage::Capturing
            | TestStage::WaitingForScreenshot
            | TestStage::Comparing => {
                playback.force_stopped = true;
                debug!("[PAUSED] stage={:?} force_stopped=true", state.stage);
            }
            // In WaitingForRender, lifecycle should be managed by render_wait_frames
            // If we get here with render_wait_frames=0, it means we're waiting for comparison_loop
            // to advance to Capturing
            TestStage::WaitingForRender => {
                // force_stopped should already be set by render_wait_frames logic above
                // or this is after screenshot capture
            }
            _ => {}
        }
    }

    pub fn comparison_loop(
        mut state: ResMut<ComparisonState>,
        mut playback: ResMut<AmPlayback>,
        mut commands: Commands,
        _window_query: Query<Entity, With<PrimaryWindow>>,
        _time: Res<Time>,
        mut exit: MessageWriter<AppExit>,
        // Query to check if project is loaded
        project_query: Query<&AmProjectRoot>,
    ) {
        // Use frame-based waiting instead of time-based for determinism
        // Wait at least 3 frames to ensure:
        // 1. Animation system processes new time
        // 2. Transform updates propagate
        // 3. Render pipeline is flushed
        // 4. Material GPU buffers are updated (critical for first frame)
        const WAIT_FRAMES: u32 = 5;
        // Wait more frames for initial load to ensure textures are uploaded to GPU
        // Increased to 30 to ensure all GPU resources are fully uploaded before first comparison
        const INITIAL_WAIT_FRAMES: u32 = 30;

        match state.stage {
            TestStage::Initializing => {} // Handled in setup

            TestStage::WaitingForProjectLoad => {
                // Pause is handled by ensure_paused_during_load in First schedule

                // Check if project is loaded by looking for a spawned AmProjectRoot
                let project_loaded = project_query.iter().any(|root| root.spawned);

                if project_loaded {
                    state.wait_frames += 1;
                    // Wait additional frames for GPU texture upload and first render
                    if state.wait_frames >= INITIAL_WAIT_FRAMES {
                        println!("[COMPARISON] Project loaded, starting comparison...");
                        state.wait_frames = 0;
                        state.stage = TestStage::SettingTime;
                    }
                }
                // If not loaded yet, just wait
            }

            TestStage::SettingTime => {
                // Check for max frame limit (useful for quick debugging)
                let max_frames = std::env::var("MAX_FRAMES")
                    .ok()
                    .and_then(|s| s.parse::<usize>().ok());
                let frame_limit = max_frames.unwrap_or(state.frame_paths.len());

                if state.current_frame >= state.frame_paths.len()
                    || state.current_frame >= frame_limit
                {
                    state.stage = TestStage::Finished;
                    return;
                }

                // Calculate time for this frame
                // Add half-frame offset to match AM video export timing
                // Use config frame_offset, or env var FRAME_OFFSET as override
                // Note: We add 1 to current_frame because we skipped the first reference frame
                // So current_frame=0 now corresponds to frame_000002.png which is at t = 1/fps
                let frame_offset: f32 = std::env::var("FRAME_OFFSET")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(state.frame_offset);
                let time_sec = (state.current_frame as f32 + 1.0 + frame_offset) / state.fps;
                playback.playing = false; // Ensure paused

                // DON'T set time immediately - store it as pending to be applied in next frame's First schedule
                // This ensures lifecycle_system won't run with new time in this frame's Update schedule
                let time_ms = time_sec * 1000.0;
                state.pending_time_ms = Some(time_ms);

                // Debug: log time setting for frame 30
                if state.current_frame == 29 || state.current_frame == 30 {
                    println!(
                        "[COMPARISON DEBUG] Frame {}: SETTING PENDING TIME: current_frame={}, frame_offset={}, fps={}, time_sec={}, time_ms={}",
                        state.current_frame + 1,
                        state.current_frame,
                        frame_offset,
                        state.fps,
                        time_sec,
                        time_ms
                    );
                }

                // Start frame counter
                state.wait_frames = 0;
                state.stage = TestStage::WaitingForRender;
            }

            TestStage::WaitingForRender => {
                // Wait until render_wait_frames countdown is complete
                // This is decremented in ensure_paused_during_load (First schedule)
                // and controls when lifecycle is allowed to run
                if state.render_wait_frames == 0 {
                    state.wait_frames += 1;
                    if state.wait_frames >= WAIT_FRAMES {
                        state.stage = TestStage::Capturing;
                    }
                }
                // If render_wait_frames > 0, just wait for it to count down
            }

            TestStage::Capturing => {
                let frame_idx = state.current_frame;
                let report_dir = state.report_dir.clone();
                let shot_path = report_dir.join(format!("shot_{:06}.png", frame_idx));

                // Note: force_stopped is set in ensure_paused_during_load (First schedule)
                // to guarantee it's set BEFORE lifecycle_system runs

                // Trigger screenshot
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(shot_path));

                state.stage = TestStage::WaitingForScreenshot;
            }

            TestStage::WaitingForScreenshot => {
                // Wait one frame for screenshot to be executed (in Render phase)
                // This ensures the screenshot captures the frozen state
                state.stage = TestStage::Comparing;
            }

            TestStage::Comparing => {
                let frame_idx = state.current_frame;
                let shot_path = state.report_dir.join(format!("shot_{:06}.png", frame_idx));

                if !shot_path.exists() {
                    // Still saving...
                    return;
                }

                // Give it a tiny moment to flush? Filesystem race is rare but possible.
                // Load images
                let shot_img = match image::open(&shot_path) {
                    Ok(img) => img.to_rgba8(),
                    Err(_) => return, // Wait more?
                };

                let ref_path = &state.frame_paths[frame_idx];

                // Debug: log actual paths being compared for frame 30 and copy ref frame
                if frame_idx % 5 == 0 {
                    println!(
                        "[COMPARE DEBUG] Frame {}: shot={:?}, ref={:?}",
                        frame_idx,
                        shot_path.file_name(),
                        ref_path.file_name()
                    );
                    // Copy reference frame to report dir for inspection
                    let ref_copy_path = state.report_dir.join(format!("ref_{:06}.png", frame_idx));
                    let _ = std::fs::copy(ref_path, &ref_copy_path);
                }

                let ref_img = image::open(ref_path)
                    .expect("Failed to open ref image")
                    .to_rgba8();

                // Compare
                let (result, diff_img) = video_utils::compare_images(&shot_img, &ref_img);

                // Use content similarity for scoring to avoid dilution by empty background
                let similarity = result.content_similarity;
                state.frame_scores.push(similarity);

                // Check against configured frame threshold
                let threshold = state.frame_threshold;

                // Save diff if similarity < threshold
                if similarity < threshold {
                    let diff_path = state.report_dir.join(format!("diff_{:06}.png", frame_idx));
                    diff_img.save(diff_path).unwrap();
                    println!(
                        "[FRAME {:03}] Similarity: {:.4} ({} < {:.2}) | Content: {:.4}, Match: {:.1}%",
                        frame_idx,
                        similarity,
                        "FAIL".red().bold(),
                        threshold,
                        result.content_similarity,
                        result.pixel_match_rate * 100.0
                    );
                } else {
                    if frame_idx % 10 == 0 {
                        println!(
                            "[FRAME {:03}] Similarity: {:.4} ({}) | Content: {:.4}, Match: {:.1}%",
                            frame_idx,
                            similarity,
                            "OK".green(),
                            result.content_similarity,
                            result.pixel_match_rate * 100.0
                        );
                    }
                }

                // Clean up shot to save space? Keep it for now.

                state.current_frame += 1;
                state.stage = TestStage::SettingTime;
            }

            TestStage::Finished | TestStage::Cancelled => {
                // Check if this was cancelled (not all frames captured)
                let total_expected_frames = state.frame_paths.len();
                let total_captured = state.frame_scores.len();
                let was_cancelled = total_captured < total_expected_frames && !state.skipped;

                // Generate Report
                let total_frames = state.frame_scores.len();
                let avg_score: f32 = if total_frames == 0 {
                    0.0
                } else {
                    state.frame_scores.iter().sum::<f32>() / total_frames as f32
                };

                // Calculate per-frame pass rate
                let failed_frames: Vec<(usize, f32)> = state
                    .frame_scores
                    .iter()
                    .enumerate()
                    .filter(|(_, score)| **score < state.frame_threshold)
                    .map(|(i, score)| (i, *score))
                    .collect();

                let critical_failed_frames: Vec<(usize, f32)> = failed_frames
                    .iter()
                    .filter(|(_, score)| *score < state.min_frame_similarity)
                    .cloned()
                    .collect();

                let failed_count = failed_frames.len();
                let critical_failed_count = critical_failed_frames.len();
                let max_allowed_failed =
                    (total_frames as f32 * state.max_failed_rate).ceil() as usize;
                let max_allowed_critical =
                    (total_frames as f32 * state.max_critical_rate).ceil() as usize;

                println!("========================================");
                println!("COMPARISON FINISHED: {}", state.project_name);

                if was_cancelled {
                    println!(
                        "Captured {} of {} frames before cancellation",
                        total_captured, total_expected_frames
                    );
                    println!("{}", "RESULT: CANCELLED ⛔".yellow().bold());
                } else if state.skipped {
                    println!("{}", "RESULT: SKIP ⚠️".yellow().bold());
                } else {
                    println!("Total Frames: {}", total_frames);

                    // Average pass rate check
                    let avg_passed = avg_score >= state.avg_threshold;
                    let avg_status = if avg_passed {
                        "✓".green().to_string()
                    } else {
                        "✗".red().to_string()
                    };
                    println!(
                        "Average Similarity: {:.4} (Threshold: {:.2}) {}",
                        avg_score, state.avg_threshold, avg_status
                    );

                    // Per-frame pass rate check
                    let frame_rate_passed = critical_failed_count <= max_allowed_critical
                        && failed_count <= max_allowed_failed;
                    let frame_status = if frame_rate_passed {
                        "✓".green().to_string()
                    } else {
                        "✗".red().to_string()
                    };
                    println!(
                        "Per-Frame Pass Rate: {} failed/{} total (max allowed: {}, critical: {}/{}, min similarity: {:.2}) {}",
                        failed_count,
                        total_frames,
                        max_allowed_failed,
                        critical_failed_count,
                        max_allowed_critical,
                        state.min_frame_similarity,
                        frame_status
                    );

                    // List critical failures if any
                    if !critical_failed_frames.is_empty() {
                        println!(
                            "  {} frames below min threshold ({:.2}):",
                            "Critical:".red().bold(),
                            state.min_frame_similarity
                        );
                        for (idx, score) in critical_failed_frames.iter().take(5) {
                            println!("    Frame {}: {:.4}", idx, score);
                        }
                        if critical_failed_frames.len() > 5 {
                            println!("    ... and {} more", critical_failed_frames.len() - 5);
                        }
                    }

                    // Final result
                    let overall_passed = avg_passed && frame_rate_passed;
                    if overall_passed {
                        println!("{}", "RESULT: PASS ✅".green().bold());
                    } else {
                        println!("{}", "RESULT: FAIL ❌".red().bold());
                    }
                }

                println!("Report saved to: {:?}", state.report_dir);
                println!("========================================");

                // Cleanup temp dir
                if let Some(temp_dir) = &state.temp_dir {
                    let _ = std::fs::remove_dir_all(temp_dir);
                }

                // Exit with appropriate code
                if state.skipped {
                    exit.write(AppExit::Success); // Or maybe a specific code for skip?
                } else {
                    let avg_passed = avg_score >= state.avg_threshold;
                    let frame_rate_passed = critical_failed_count <= max_allowed_critical
                        && failed_count <= max_allowed_failed;
                    let overall_passed = avg_passed && frame_rate_passed;
                    if overall_passed {
                        exit.write(AppExit::Success);
                    } else {
                        exit.write(AppExit::Error(std::num::NonZero::new(1).unwrap()));
                    }
                }
            }
        }
    }
}

// ============================================================================
// Frame Test / FPS Benchmark (requires --features frame-test)
// ============================================================================

#[cfg(feature = "frame-test")]
mod frame_test_systems {
    use super::*;
    use bevy::app::AppExit;
    use bevy::ecs::message::MessageWriter;
    use bevy_alight_motion::scene::AmProjectRoot;
    use owo_colors::OwoColorize;
    use serde::Deserialize;

    #[derive(PartialEq, Debug)]
    pub enum FrameTestStage {
        WaitingForLoad,
        Warmup,
        Running,
        Finished,
    }

    #[derive(Resource)]
    pub struct FrameTestState {
        pub stage: FrameTestStage,
        pub warmup_frames_remaining: u32,
        pub frame_times: Vec<f64>,
        pub measurement_elapsed: f64,
        pub project_name: String,
        // Config
        pub pass_fps: f32,
        pub fail_fps: f32,
        pub max_below_fail_rate: f32,
        pub max_below_pass_rate: f32,
        pub min_sample_frames: u32,
        pub warmup_frames: u32,
        pub measure_duration_secs: f32,
    }

    impl Default for FrameTestState {
        fn default() -> Self {
            Self {
                stage: FrameTestStage::WaitingForLoad,
                warmup_frames_remaining: 60,
                frame_times: Vec::new(),
                measurement_elapsed: 0.0,
                project_name: String::new(),
                pass_fps: 120.0,
                fail_fps: 60.0,
                max_below_fail_rate: 0.05,
                max_below_pass_rate: 0.20,
                min_sample_frames: 30,
                warmup_frames: 60,
                measure_duration_secs: 15.0,
            }
        }
    }

    #[derive(Deserialize, Debug)]
    struct FrameTestConfig {
        #[serde(default = "default_pass_fps")]
        pass_fps: f32,
        #[serde(default = "default_fail_fps")]
        fail_fps: f32,
        #[serde(default = "default_max_below_fail_rate")]
        max_below_fail_rate: f32,
        #[serde(default = "default_max_below_pass_rate")]
        max_below_pass_rate: f32,
        #[serde(default = "default_min_sample_frames")]
        min_sample_frames: u32,
        #[serde(default = "default_warmup_frames")]
        warmup_frames: u32,
        #[serde(default = "default_measure_duration")]
        measure_duration_secs: f32,
    }

    fn default_pass_fps() -> f32 {
        120.0
    }
    fn default_fail_fps() -> f32 {
        60.0
    }
    fn default_max_below_fail_rate() -> f32 {
        0.05
    }
    fn default_max_below_pass_rate() -> f32 {
        0.20
    }
    fn default_min_sample_frames() -> u32 {
        30
    }
    fn default_warmup_frames() -> u32 {
        60
    }
    fn default_measure_duration() -> f32 {
        15.0
    }

    #[derive(Deserialize, Debug)]
    struct ConfigFile {
        #[serde(default)]
        frame_test: Option<FrameTestConfig>,
    }

    pub fn setup_frame_test(mut state: ResMut<FrameTestState>, project_file: Res<ProjectFile>) {
        let project_name = std::path::Path::new(&project_file.0)
            .strip_prefix("projects/")
            .ok()
            .and_then(|p| p.to_str())
            .map(|s| s.strip_suffix(".amproj").unwrap_or(s))
            .unwrap_or("unknown")
            .to_string();
        state.project_name = project_name.clone();

        // Load config
        let config_content = std::fs::read_to_string(
            "crates/bevy_alight_motion/comparison_config.toml",
        )
        .or_else(|_| std::fs::read_to_string("comparison_config.toml"))
        .ok();

        if let Some(content) = config_content {
            if let Ok(cfg) = toml::from_str::<ConfigFile>(&content) {
                if let Some(ft) = cfg.frame_test {
                    state.pass_fps = ft.pass_fps;
                    state.fail_fps = ft.fail_fps;
                    state.max_below_fail_rate = ft.max_below_fail_rate;
                    state.max_below_pass_rate = ft.max_below_pass_rate;
                    state.min_sample_frames = ft.min_sample_frames;
                    state.warmup_frames = ft.warmup_frames;
                    state.measure_duration_secs = ft.measure_duration_secs;
                }
            }
        }

        state.warmup_frames_remaining = state.warmup_frames;

        println!(
            "[FRAME-TEST] Config for '{}': pass_fps={:.0}, fail_fps={:.0}, warmup={}, measure={:.0}s",
            project_name, state.pass_fps, state.fail_fps, state.warmup_frames, state.measure_duration_secs
        );
    }

    pub fn frame_test_loop(
        mut state: ResMut<FrameTestState>,
        mut playback: ResMut<AmPlayback>,
        time: Res<Time>,
        project_query: Query<&AmProjectRoot>,
        mut exit: MessageWriter<AppExit>,
    ) {
        match state.stage {
            FrameTestStage::WaitingForLoad => {
                // Wait until project is actually loaded (AmProjectRoot.spawned == true)
                let project_loaded = project_query.iter().any(|root| root.spawned);
                if project_loaded {
                    playback.looping = true;
                    state.stage = FrameTestStage::Warmup;
                    println!(
                        "[FRAME-TEST] Project loaded (duration={:.1}ms), warming up {} frames...",
                        playback.total_time_ms, state.warmup_frames_remaining
                    );
                }
            }

            FrameTestStage::Warmup => {
                if state.warmup_frames_remaining > 0 {
                    state.warmup_frames_remaining -= 1;
                } else {
                    state.stage = FrameTestStage::Running;
                    println!(
                        "[FRAME-TEST] Warmup complete, measuring FPS for {:.0}s...",
                        state.measure_duration_secs
                    );
                }
            }

            FrameTestStage::Running => {
                let dt = time.delta_secs_f64();
                if dt > 0.0 {
                    state.frame_times.push(dt);
                    state.measurement_elapsed += dt;
                }

                // Stop after configured measurement duration
                if state.measurement_elapsed >= state.measure_duration_secs as f64 {
                    state.stage = FrameTestStage::Finished;
                }
            }

            FrameTestStage::Finished => {
                playback.playing = false;
                report_results(&state, &mut exit);
            }
        }
    }

    fn report_results(state: &FrameTestState, exit: &mut MessageWriter<AppExit>) {
        let total_frames = state.frame_times.len();
        println!();
        println!("========================================");
        println!("FRAME TEST RESULTS: {}", state.project_name);
        println!("========================================");

        // Frame time histogram for diagnostics
        let buckets = [
            (0.0, 1.0, ">1000 FPS"),
            (1.0, 2.0, "500-1000 FPS"),
            (2.0, 4.0, "250-500 FPS"),
            (4.0, 6.94, "144-250 FPS"),
            (6.94, 16.67, "60-144 FPS"),
            (16.67, 33.33, "30-60 FPS"),
            (33.33, 1000.0, "<30 FPS"),
        ];
        println!("Frame time distribution:");
        for (lo_ms, hi_ms, label) in &buckets {
            let count = state
                .frame_times
                .iter()
                .filter(|&&dt| {
                    let ms = dt * 1000.0;
                    ms >= *lo_ms as f64 && ms < *hi_ms as f64
                })
                .count();
            if count > 0 {
                println!(
                    "  {:>12}: {:>5} frames ({:.1}%)",
                    label,
                    count,
                    count as f64 / total_frames as f64 * 100.0
                );
            }
        }

        if (total_frames as u32) < state.min_sample_frames {
            println!(
                "{}",
                format!(
                    "RESULT: FAIL ❌ (insufficient frames: {} < {})",
                    total_frames, state.min_sample_frames
                )
                .red()
                .bold()
            );
            exit.write(AppExit::Error(std::num::NonZero::new(1).unwrap()));
            return;
        }

        // Compute FPS stats
        let avg_dt: f64 = state.frame_times.iter().sum::<f64>() / total_frames as f64;
        let avg_fps = 1.0 / avg_dt;

        let min_dt = state
            .frame_times
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let max_dt = state
            .frame_times
            .iter()
            .copied()
            .fold(0.0_f64, f64::max);
        let max_fps = 1.0 / min_dt;
        let min_fps = 1.0 / max_dt;

        // 1% low FPS
        let mut sorted = state.frame_times.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p99_idx = ((total_frames as f64) * 0.99).ceil() as usize;
        let p99_dt = sorted[p99_idx.min(total_frames - 1)];
        let p1_fps = 1.0 / p99_dt;

        let below_fail = state
            .frame_times
            .iter()
            .filter(|&&dt| 1.0 / dt < state.fail_fps as f64)
            .count();
        let below_pass = state
            .frame_times
            .iter()
            .filter(|&&dt| 1.0 / dt < state.pass_fps as f64)
            .count();

        let below_fail_rate = below_fail as f64 / total_frames as f64;
        let below_pass_rate = below_pass as f64 / total_frames as f64;

        println!("Total frames sampled: {}", total_frames);
        println!("Average FPS: {:.1}", avg_fps);
        println!("Min FPS: {:.1} | Max FPS: {:.1}", min_fps, max_fps);
        println!("1% Low FPS: {:.1}", p1_fps);
        println!(
            "Frames below {:.0} FPS (fail threshold): {} ({:.1}%) [max allowed: {:.1}%]",
            state.fail_fps,
            below_fail,
            below_fail_rate * 100.0,
            state.max_below_fail_rate * 100.0
        );
        println!(
            "Frames below {:.0} FPS (pass threshold): {} ({:.1}%) [max allowed: {:.1}%]",
            state.pass_fps,
            below_pass,
            below_pass_rate * 100.0,
            state.max_below_pass_rate * 100.0
        );
        println!("----------------------------------------");

        // Determine result:
        // FAIL if avg < fail_fps OR too many frames below fail_fps
        // PASS if avg >= pass_fps AND few frames below pass_fps
        // WARNING otherwise (between fail and pass)
        let fail_rate_exceeded = below_fail_rate > state.max_below_fail_rate as f64;

        if avg_fps < state.fail_fps as f64 || fail_rate_exceeded {
            let reason = if fail_rate_exceeded {
                format!(
                    "avg {:.1} FPS, {:.1}% frames below {:.0} FPS (max {:.1}%)",
                    avg_fps,
                    below_fail_rate * 100.0,
                    state.fail_fps,
                    state.max_below_fail_rate * 100.0
                )
            } else {
                format!("avg {:.1} FPS < {:.0}", avg_fps, state.fail_fps)
            };
            println!(
                "{}",
                format!("RESULT: FAIL ❌ ({})", reason).red().bold()
            );
            exit.write(AppExit::Error(std::num::NonZero::new(1).unwrap()));
        } else if avg_fps >= state.pass_fps as f64
            && below_pass_rate <= state.max_below_pass_rate as f64
        {
            println!(
                "{}",
                format!(
                    "RESULT: PASS ✅ (avg {:.1} FPS >= {:.0})",
                    avg_fps, state.pass_fps
                )
                .green()
                .bold()
            );
            exit.write(AppExit::Success);
        } else {
            let reason = if avg_fps < state.pass_fps as f64 {
                format!(
                    "avg {:.1} FPS: {:.0} <= fps < {:.0}",
                    avg_fps, state.fail_fps, state.pass_fps
                )
            } else {
                format!(
                    "avg {:.1} FPS but {:.1}% frames below {:.0} FPS (max {:.1}%)",
                    avg_fps,
                    below_pass_rate * 100.0,
                    state.pass_fps,
                    state.max_below_pass_rate * 100.0
                )
            };
            println!(
                "{}",
                format!("RESULT: WARNING ⚠️ ({})", reason).yellow().bold()
            );
            // Warning is still a pass (exit 0)
            exit.write(AppExit::Success);
        }
        println!("========================================");
    }

    /// Display FPS in window title (no UI text overlay to minimize rendering overhead)
    pub fn update_fps_display(
        time: Res<Time>,
        state: Res<FrameTestState>,
        mut windows: Query<&mut Window>,
    ) {
        if let Ok(mut window) = windows.single_mut() {
            let fps = 1.0 / time.delta_secs_f64();
            let stage = match state.stage {
                FrameTestStage::WaitingForLoad => "Loading",
                FrameTestStage::Warmup => "Warmup",
                FrameTestStage::Running => "Measuring",
                FrameTestStage::Finished => "Done",
            };
            window.title = format!(
                "[Frame Test] {} | FPS: {:.0} | {} ({} samples)",
                stage,
                fps,
                state.project_name,
                state.frame_times.len()
            );
        }
    }
}

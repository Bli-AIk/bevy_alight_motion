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
//! ### Run Video Comparison Test / 运行视频比对测试
//! ```bash
//! cargo run -p bevy_alight_motion --example player --features video-comparison -- <project_name>
//! ```
//! This runs a non-interactive test that compares rendered frames against a reference video
//! and generates a report in the `reports/` directory.
//!
//! （此命令会运行一个非交互式测试，将渲染结果与参考视频逐帧比对，并在 `reports/` 目录下生成报告。）
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

    // Default resolution
    let mut resolution = Vec2::new(1280.0, 960.0);

    // In comparison mode, try to match video resolution
    #[cfg(feature = "video-comparison")]
    {
        if let Some(video_path) = video_utils::find_debug_video(Some(&project_file)) {
            // ...
            println!("Comparison mode: Using default resolution for now. Ensure video matches.");
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
    .add_systems(Startup, setup)
    .add_systems(
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

    // Only spawn UI if NOT in comparison mode
    #[cfg(not(feature = "video-comparison"))]
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
fn debug_sprites(
    query: Query<(&AmLayerMarker, &Transform, &GlobalTransform, &Sprite), Added<Sprite>>,
) {
    for (marker, transform, global_transform, sprite) in query.iter() {
        let global_z = global_transform.translation().z;
        println!(
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
    for (marker, transform, global_transform, material_handle) in query.iter() {
        let mesh_offset = if let Some(material) = materials.get(&material_handle.0) {
            (material.mesh_offset.x, material.mesh_offset.y)
        } else {
            (0.0, 0.0)
        };
        println!(
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
fn debug_position_changes(
    _playback: Res<AmPlayback>,
    _query: Query<
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
    // Debug output disabled to reduce log spam
    // Enable by uncommenting the following:
    /*
    for (marker, transform, global_transform, material_handle) in query.iter() {
        if marker.label.contains("骨头") {
            let gt = global_transform.translation();
            if let Some(material) = materials.get(&material_handle.0) {
                println!(
                    "[PosDebug] frame_time={:.1}ms '{}' local=({:.1},{:.1}) global=({:.1},{:.1}) mesh_offset=({:.1},{:.1})",
                    playback.current_time_ms,
                    marker.label,
                    transform.translation.x,
                    transform.translation.y,
                    gt.x,
                    gt.y,
                    material.mesh_offset.x,
                    material.mesh_offset.y,
                );
            }
        }
    }
    */
}

/// Debug system to print SDF shape info once
fn debug_sdf_shapes(
    query: Query<(&Name, &Transform, &GlobalTransform), Added<bevy::prelude::MeshMaterial2d<bevy_alight_motion::sdf_material::SdfMaterial>>>,
) {
    #[cfg(not(feature = "video-comparison"))]
    for (name, transform, global_transform) in query.iter() {
        let local_z = transform.translation.z;
        let global_z = global_transform.translation().z;
        println!(
            "[SDF Z-DEBUG] '{}': local_z={:.6} global_z={:.6}",
            name, local_z, global_z,
        );
    }
}

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
    show_masks: bool,
}

/// Component for mask debug visualization entities
#[derive(Component)]
struct MaskDebugVisual;

/// Toggle mask debug visualization with the M key
fn toggle_mask_debug(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<MaskDebugSettings>,
    mut commands: Commands,
    mask_query: Query<&bevy_alight_motion::scene::AmMaskInfo, Without<MaskDebugVisual>>,
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
                    // Create a key based on mask position and size (rounded to int for comparison)
                    let key = (
                        (mask_info.center.x * 10.0) as i32,
                        (mask_info.center.y * 10.0) as i32,
                        (mask_info.half_size.x * 10.0) as i32,
                        (mask_info.half_size.y * 10.0) as i32,
                    );

                    if seen_masks.contains(&key) {
                        continue;
                    }
                    seen_masks.insert(key);

                    // Spawn a semi-transparent rectangle to visualize the mask
                    println!(
                        "[MASK DEBUG] Visualizing mask at ({:.1},{:.1}) size ({:.1},{:.1})",
                        mask_info.center.x,
                        mask_info.center.y,
                        mask_info.half_size.x * 2.0,
                        mask_info.half_size.y * 2.0
                    );

                    // Create a sprite to show the mask region
                    commands.spawn((
                        Name::new("MaskDebugVisual"),
                        MaskDebugVisual,
                        Sprite {
                            color: Color::srgba(1.0, 0.0, 0.0, 0.3), // Semi-transparent red
                            custom_size: Some(Vec2::new(
                                mask_info.half_size.x * 2.0,
                                mask_info.half_size.y * 2.0,
                            )),
                            ..default()
                        },
                        Transform::from_translation(Vec3::new(
                            mask_info.center.x,
                            mask_info.center.y,
                            100.0, // High z to render on top
                        )),
                    ));
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
        pub last_frame_time: f32,
        /// Total duration in seconds
        pub duration: f32,
        /// Temp directory for extracted frames
        pub temp_dir: Option<PathBuf>,
        /// Whether frames have been loaded into Bevy
        pub frames_loaded: bool,
        /// Whether all frames are ready (fully loaded)
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
        // frame_paths contain absolute paths like:
        // /path/to/crates/bevy_alight_motion/assets/debug/_video_frames/video_name/frame_000001.png
        // We need to extract the asset-relative path: debug/_video_frames/video_name/frame_000001.png
        state.frame_handles = state
            .frame_paths
            .iter()
            .filter_map(|path| {
                // Find "debug/_video_frames" in the path and extract everything after "assets/"
                let path_str = path.to_string_lossy();
                if let Some(idx) = path_str.find("debug/_video_frames") {
                    let asset_path = &path_str[idx..];
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
        pub total_diff: f64,
        pub frame_scores: Vec<f32>,
        pub report_dir: PathBuf,
        // Config thresholds
        pub avg_threshold: f32,
        pub frame_threshold: f32,
        pub project_name: String,
        pub skipped: bool,
    }

    #[derive(PartialEq, Debug)]
    pub enum TestStage {
        Initializing,
        WaitingForProjectLoad, // Wait for project to load and first frame to render
        SettingTime,
        WaitingForRender,
        Capturing,
        Comparing,
        Finished,
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
                project_name: String::new(),
                skipped: false,
            }
        }
    }
    #[derive(Deserialize, Debug)]
    struct ComparisonConfig {
        default: ProjectConfig,
        #[serde(default)]
        overrides: HashMap<String, ProjectConfig>,
    }

    #[derive(Deserialize, Debug, Clone, Copy)]
    struct ProjectConfig {
        avg_threshold: f32,
        frame_threshold: f32,
    }

    pub fn setup_comparison(
        mut state: ResMut<ComparisonState>,
        project_file: Res<super::ProjectFile>,
    ) {
        // Prepare report dir
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let report_dir = PathBuf::from("reports").join(format!("run_{}", timestamp));
        std::fs::create_dir_all(&report_dir).expect("Failed to create report dir");
        state.report_dir = report_dir;

        // Extract project name from path
        let project_name = std::path::Path::new(&project_file.0)
            .file_stem()
            .and_then(|s| s.to_str())
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
            let settings = cfg.overrides.get(&project_name).unwrap_or(&cfg.default);
            state.avg_threshold = settings.avg_threshold;
            state.frame_threshold = settings.frame_threshold;
            println!(
                "[COMPARISON] Config for '{}': avg_thresh={:.2}, frame_thresh={:.2}",
                project_name, state.avg_threshold, state.frame_threshold
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
    pub fn ensure_paused_during_load(
        state: Res<ComparisonState>,
        mut playback: ResMut<AmPlayback>,
    ) {
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
            _ => {}
        }
    }

    pub fn comparison_loop(
        mut state: ResMut<ComparisonState>,
        mut playback: ResMut<AmPlayback>,
        mut commands: Commands,
        _window_query: Query<Entity, With<PrimaryWindow>>,
        _time: Res<Time>,
        mut exit: EventWriter<AppExit>,
        // Query to check if project is loaded
        project_query: Query<&bevy_alight_motion::scene::AmProjectRoot>,
    ) {
        // Use frame-based waiting instead of time-based for determinism
        // Wait at least 3 frames to ensure:
        // 1. Animation system processes new time
        // 2. Transform updates propagate
        // 3. Render pipeline is flushed
        const WAIT_FRAMES: u32 = 3;
        // Wait more frames for initial load to ensure textures are uploaded to GPU
        const INITIAL_WAIT_FRAMES: u32 = 10;

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

                // Set precise time for this frame
                let time_sec = state.current_frame as f32 / state.fps;
                playback.playing = false; // Ensure paused
                playback.current_time_ms = time_sec * 1000.0;
                playback.force_stopped = false; // Allow update

                // Start frame counter
                state.wait_frames = 0;
                state.stage = TestStage::WaitingForRender;
            }

            TestStage::WaitingForRender => {
                state.wait_frames += 1;
                if state.wait_frames >= WAIT_FRAMES {
                    state.stage = TestStage::Capturing;
                }
            }

            TestStage::Capturing => {
                let frame_idx = state.current_frame;
                let report_dir = state.report_dir.clone();
                let shot_path = report_dir.join(format!("shot_{:06}.png", frame_idx));

                // Trigger screenshot
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(shot_path));

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

            TestStage::Finished => {
                // Generate Report
                let avg_score: f32 = if state.frame_scores.is_empty() {
                    0.0
                } else {
                    state.frame_scores.iter().sum::<f32>() / state.frame_scores.len() as f32
                };

                println!("========================================");
                println!("COMPARISON FINISHED: {}", state.project_name);

                if state.skipped {
                    println!("{}", "RESULT: SKIP ⚠️".yellow().bold());
                } else {
                    println!("Total Frames: {}", state.frame_paths.len());
                    println!(
                        "Average Similarity: {:.4} (Threshold: {:.2})",
                        avg_score, state.avg_threshold
                    );

                    let passed = avg_score >= state.avg_threshold;
                    if passed {
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
                    let passed = avg_score >= state.avg_threshold;
                    if passed {
                        exit.write(AppExit::Success);
                    } else {
                        exit.write(AppExit::Error(std::num::NonZero::new(1).unwrap()));
                    }
                }
            }
        }
    }
}

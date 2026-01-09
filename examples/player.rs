//! Example player for Alight Motion projects.
//!
//! Usage:
//!   cargo run -p bevy_alight_motion --example player -- <project_name>
//!   cargo run -p bevy_alight_motion --example player --features debug -- <project_name>
//!   cargo run -p bevy_alight_motion --example player --features video-debug -- <project_name>
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
//! - F1: Toggle inspector window (requires --features debug)
//! - F4: Toggle debug image overlay
//! - F6: Toggle video debug overlay (requires --features video-debug)

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

    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
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
    .init_resource::<MaskDebugSettings>()
    .add_plugins(AlightMotionPlugin)
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            handle_input,
            update_ui,
            debug_sprites,
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
        Text::new("[Space] Play/Pause | [R] Reset | [P] Replay | [F5] Force Stop | [Left/Right] Seek | [Up/Down] Speed | [L] Loop"),
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
fn debug_sprites(
    query: Query<(&AmLayerMarker, &Transform, &GlobalTransform, &Sprite), Added<Sprite>>,
) {
    for (marker, transform, global_transform, sprite) in query.iter() {
        let global_z = global_transform.translation().z;
        println!(
            "Sprite added: '{}' at ({:.1},{:.1}) local_z={:.2} global_z={:.2} scale=({:.2},{:.2}) alpha={:.2}",
            marker.label,
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
            global_z,
            transform.scale.x,
            transform.scale.y,
            sprite.color.alpha(),
        );
    }
}

/// Debug system to print SDF shape info once (to verify GlobalTransform z propagation)
/// Run in PostUpdate to ensure GlobalTransform is propagated
fn debug_sdf_shapes(
    query: Query<(&Name, &Transform, &GlobalTransform), Added<bevy_smud::SmudShape>>,
) {
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

// ============================================================================
// Video Debug Overlay (requires --features video-debug)
// Uses ffmpeg to extract frames at startup, then plays them as an overlay
// ============================================================================

#[cfg(feature = "video-debug")]
mod video_debug {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;

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

    /// Find the latest video file in the debug folder
    fn find_latest_debug_video() -> Option<PathBuf> {
        use std::fs;
        use std::time::SystemTime;

        let possible_paths = ["crates/bevy_alight_motion/assets/debug", "assets/debug"];
        let extensions = ["mp4", "mov", "avi", "webm", "mkv"];

        let mut latest_file: Option<(PathBuf, SystemTime)> = None;

        for debug_path in &possible_paths {
            let path = std::path::Path::new(debug_path);
            if !path.exists() {
                continue;
            }

            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_file() {
                            if let Some(file_name) = entry.file_name().to_str() {
                                if let Some(extension) = file_name.split('.').next_back() {
                                    if extensions.contains(&extension.to_lowercase().as_str()) {
                                        if let Ok(metadata) = entry.metadata() {
                                            if let Ok(modified) = metadata.modified() {
                                                if latest_file.is_none()
                                                    || latest_file.as_ref().unwrap().1 < modified
                                                {
                                                    latest_file = Some((entry.path(), modified));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if latest_file.is_some() {
                    break;
                }
            }
        }

        latest_file.map(|(path, _)| path)
    }

    /// Get video info using ffprobe
    fn get_video_info(video_path: &PathBuf) -> Option<(f32, f32)> {
        // Get frame rate
        let fps_output = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=r_frame_rate",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
            ])
            .arg(video_path)
            .output()
            .ok()?;

        let fps_str = String::from_utf8_lossy(&fps_output.stdout);
        let fps = parse_fps(&fps_str.trim()).unwrap_or(12.0);

        // Get duration
        let duration_output = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
            ])
            .arg(video_path)
            .output()
            .ok()?;

        let duration_str = String::from_utf8_lossy(&duration_output.stdout);
        let duration: f32 = duration_str.trim().parse().unwrap_or(0.0);

        Some((fps, duration))
    }

    /// Parse FPS from ffprobe output (handles formats like "12/1" or "29.97")
    fn parse_fps(s: &str) -> Option<f32> {
        if s.contains('/') {
            let parts: Vec<&str> = s.split('/').collect();
            if parts.len() == 2 {
                let num: f32 = parts[0].parse().ok()?;
                let den: f32 = parts[1].parse().ok()?;
                if den > 0.0 {
                    return Some(num / den);
                }
            }
        }
        s.parse().ok()
    }

    /// Extract frames from video using ffmpeg
    fn extract_frames(video_path: &PathBuf, fps: f32) -> Option<PathBuf> {
        use std::fs;

        // Create frames directory inside assets/debug
        let possible_assets_dirs = [
            "crates/bevy_alight_motion/assets/debug/_video_frames",
            "assets/debug/_video_frames",
        ];

        let mut frames_dir = None;
        for dir_path in &possible_assets_dirs {
            let parent = std::path::Path::new(dir_path).parent()?;
            if parent.exists() {
                frames_dir = Some(PathBuf::from(dir_path));
                break;
            }
        }

        let frames_dir = frames_dir?;

        // Clean up existing frames
        if frames_dir.exists() {
            let _ = fs::remove_dir_all(&frames_dir);
        }
        fs::create_dir_all(&frames_dir).ok()?;

        println!("[VIDEO DEBUG] Extracting frames to {:?}", frames_dir);

        // Extract frames using ffmpeg
        let output_pattern = frames_dir.join("frame_%06d.png");
        let status = Command::new("ffmpeg")
            .args(["-i"])
            .arg(video_path)
            .args([
                "-vf",
                &format!("fps={}", fps),
                "-y", // Overwrite existing files
            ])
            .arg(&output_pattern)
            .output();

        match status {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    println!("[VIDEO DEBUG] ffmpeg error: {}", stderr);
                    return None;
                }
                Some(frames_dir)
            }
            Err(e) => {
                println!("[VIDEO DEBUG] Failed to run ffmpeg: {:?}", e);
                None
            }
        }
    }

    /// Setup video debug overlay on startup
    pub fn setup_video_debug(mut state: ResMut<VideoDebugState>) {
        // Find the latest video file
        let Some(video_path) = find_latest_debug_video() else {
            println!("[VIDEO DEBUG] No video file found in debug folder");
            return;
        };

        println!("[VIDEO DEBUG] Found video: {:?}", video_path);

        // Get video info
        let Some((fps, duration)) = get_video_info(&video_path) else {
            println!("[VIDEO DEBUG] Failed to get video info");
            return;
        };

        println!(
            "[VIDEO DEBUG] Video info: {:.2} FPS, {:.2}s duration",
            fps, duration
        );

        // Extract frames
        let Some(temp_dir) = extract_frames(&video_path, fps) else {
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
        // Convert absolute paths to relative paths under assets/
        state.frame_handles = state
            .frame_paths
            .iter()
            .map(|path| {
                // Extract just the filename and construct asset-relative path
                let filename = path.file_name().unwrap().to_string_lossy();
                let asset_path = format!("debug/_video_frames/{}", filename);
                asset_server.load(asset_path)
            })
            .collect();

        state.frames_loaded = true;
        println!(
            "[VIDEO DEBUG] Loading {} frame handles...",
            state.frame_handles.len()
        );
    }

    /// Check if all video frames are loaded and pause playback until ready
    pub fn check_video_frames_ready(
        mut state: ResMut<VideoDebugState>,
        mut playback: ResMut<AmPlayback>,
        images: Res<Assets<Image>>,
    ) {
        if state.frames_ready || state.frame_handles.is_empty() {
            return;
        }

        // Check if all frame images are loaded
        let loaded_count = state
            .frame_handles
            .iter()
            .filter(|handle| images.get(*handle).is_some())
            .count();

        let total = state.frame_handles.len();

        if loaded_count < total {
            // Still loading - ensure playback is paused and at beginning
            if playback.playing {
                playback.playing = false;
                playback.current_time_ms = 0.0;
                println!(
                    "[VIDEO DEBUG] Waiting for frames: {}/{}",
                    loaded_count, total
                );
            }
        } else {
            // All frames loaded!
            state.frames_ready = true;
            playback.playing = true;
            playback.current_time_ms = 0.0;
            println!(
                "[VIDEO DEBUG] All {} frames ready, starting playback!",
                total
            );
        }
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
pub use video_debug::*;

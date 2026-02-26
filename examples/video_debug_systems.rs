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

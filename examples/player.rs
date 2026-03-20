#![allow(deprecated, dead_code)]
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
//! ### Run Frame Test (FPS Benchmark) / 运行帧测试（FPS 基准）
//! ```bash
//! cargo run -p bevy_alight_motion --example player --features frame-test -- <project_name>
//! ```
//! This plays the project and measures FPS performance.
//! - ≥144 FPS: PASS ✅  |  60-144 FPS: WARNING ⚠️  |  <60 FPS: FAIL ❌
//!
//! （此命令会播放工程并测量 FPS 性能。）
//!
//! ### Enable BRP/MCP Control For Player / 为 player 启用 BRP/MCP 控制
//! ```bash
//! BRP_EXTRAS_PORT=15702 cargo run -p bevy_alight_motion --example player --features player-brp -- <project_name>
//! ```
//! This enables `bevy_brp_extras` for the example player only, so MCP tools can inspect
//! and control the running app without changing library defaults.
//!
//! （只为示例 player 启用 `bevy_brp_extras`，便于 MCP 连接，不改变库默认行为。）
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
#[cfg(feature = "player-brp")]
use bevy_brp_extras::BrpExtrasPlugin;

#[cfg(feature = "debug")]
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};

/// Resource storing the offscreen render target image handle for headless rendering.
/// 无头渲染模式下的离屏渲染目标 Image handle。
#[cfg(feature = "headless-render")]
#[derive(Resource, Clone)]
struct HeadlessRenderTarget(Handle<Image>);

/// Resource storing the target resolution for headless rendering.
/// 无头渲染模式下的目标分辨率。
#[cfg(feature = "headless-render")]
#[derive(Resource)]
struct HeadlessResolution(Vec2);

/// Parsed CLI arguments for the player.
struct CliArgs {
    project_file: String,
    headless: bool,
}

/// Parse CLI arguments: `[--headless] <project_name>`
fn parse_cli_args() -> CliArgs {
    let args: Vec<String> = std::env::args().collect();
    let mut headless = false;
    let mut project_name = None;

    for arg in args.iter().skip(1) {
        if arg == "--headless" {
            headless = true;
        } else if project_name.is_none() {
            project_name = Some(arg.as_str());
        }
    }

    let name = project_name.unwrap_or("complex/misc/simple_gb");
    let project_file = match name {
        "simple_gb" => "projects/complex/misc/simple_gb.amproj".to_string(),
        "complex_1" => "projects/complex/examples/1.amproj".to_string(),
        "complex_2" => "projects/complex/examples/2.amproj".to_string(),
        "complex_3" => "projects/complex/examples/3.amproj".to_string(),
        other => format!("projects/{}.amproj", other),
    };

    CliArgs {
        project_file,
        headless,
    }
}

fn main() {
    let cli = parse_cli_args();
    let project_file = cli.project_file;
    println!("Loading project: {}", project_file);

    #[allow(unused_mut)]
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

    // Headless mode: no window, offscreen render target, schedule runner loop
    #[cfg(feature = "headless-render")]
    {
        app.add_plugins(
            DefaultPlugins
                .build()
                .disable::<bevy::winit::WinitPlugin>()
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: bevy::window::ExitCondition::DontExit,
                    ..default()
                }),
        )
        .add_plugins(bevy::app::ScheduleRunnerPlugin::run_loop(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ))
        .insert_resource(HeadlessResolution(resolution));
    }

    // Normal mode: windowed rendering via WinitPlugin
    #[cfg(not(feature = "headless-render"))]
    {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("Alight Motion Player - {}", project_file),
                resolution: bevy::window::WindowResolution::new(
                    resolution.x as u32,
                    resolution.y as u32,
                ),
                resizable: false,
                visible: !cli.headless,
                // Disable VSync in frame-test mode for accurate FPS measurement
                #[cfg(feature = "frame-test")]
                present_mode: bevy::window::PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        }));
    }

    #[cfg(feature = "player-brp")]
    app.add_plugins(BrpExtrasPlugin::default());

    app
        // Black background matching AM project
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(ProjectFile(project_file.clone()))
        .init_resource::<DebugOverlaySettings>()
        .init_resource::<MaskDebugSettings>()
        .add_plugins(AlightMotionPlugin)
        .add_systems(Startup, setup);

    // Headless: use FixedSize since there's no window to query
    #[cfg(feature = "headless-render")]
    app.insert_resource(AmProjectResolution::FixedSize(resolution.x, resolution.y));
    #[cfg(not(feature = "headless-render"))]
    app.insert_resource(AmProjectResolution::FitWindow);

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

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    project_file: Res<ProjectFile>,
    #[cfg(feature = "headless-render")] mut images: ResMut<Assets<Image>>,
    #[cfg(feature = "headless-render")] headless_res: Res<HeadlessResolution>,
) {
    // Headless mode: create offscreen render target and camera
    #[cfg(feature = "headless-render")]
    {
        let render_image = Image::new_target_texture(
            headless_res.0.x as u32,
            headless_res.0.y as u32,
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            None,
        );
        let render_handle = images.add(render_image);
        commands.insert_resource(HeadlessRenderTarget(render_handle.clone()));
        commands.spawn((
            Camera2d,
            Camera {
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            bevy::camera::RenderTarget::Image(render_handle.into()),
        ));
        println!(
            "Headless render target: {}x{}",
            headless_res.0.x as u32, headless_res.0.y as u32
        );
    }

    // Normal mode: spawn default camera
    #[cfg(not(feature = "headless-render"))]
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
    if !keyboard.just_pressed(KeyCode::F4) {
        return;
    }

    settings.show_overlay = !settings.show_overlay;

    if !settings.show_overlay {
        for entity in overlay_query.iter() {
            commands.entity(entity).despawn();
        }
        println!("Debug image overlay: OFF");
        return;
    }

    // Remove any existing overlay entity before spawning a new one.
    for entity in overlay_query.iter() {
        commands.entity(entity).despawn();
    }

    // Look up the most recently modified image in the debug folder.
    let Some(latest_image_path) = find_latest_debug_image() else {
        return;
    };
    println!("Loading debug overlay image: {}", latest_image_path);

    // Load the selected image asset.
    let image_handle: Handle<Image> = asset_server.load(&latest_image_path);

    // Query the current window size for correct scaling.
    let Ok(window) = window_query.single() else {
        return;
    };
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

        let Ok(entries) = fs::read_dir(debug_path) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_file() {
                continue;
            }
            let file_name_os = entry.file_name();
            let Some(file_name) = file_name_os.to_str() else {
                continue;
            };
            let Some(extension) = file_name.split('.').next_back() else {
                continue;
            };
            if !extensions.contains(&extension.to_lowercase().as_str()) {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            let Ok(modified) = metadata.modified() else {
                continue;
            };

            let relative_path = format!("debug/{}", file_name);
            if latest_file.is_none() || latest_file.as_ref().unwrap().1 < modified {
                latest_file = Some((relative_path, modified));
            }
        }
        // Once files are found in this path we can stop probing others.
        if latest_file.is_some() {
            break;
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
        if !keyboard.just_pressed(KeyCode::KeyM) {
            return;
        }

        settings.show_masks = !settings.show_masks;

        if !settings.show_masks {
            // Remove debug visualizations
            let count = debug_visual_query.iter().count();
            for entity in debug_visual_query.iter() {
                commands.entity(entity).despawn();
            }
            println!("[MASK DEBUG] Hidden {} mask visualization(s)", count);
            return;
        }

        // Spawn mask visualization entities for each mask
        // First, find unique mask centers (masks may be shared across many entities)
        let mut seen_masks: std::collections::HashSet<(i32, i32, i32, i32)> =
            std::collections::HashSet::new();

        let all_masks: Vec<_> = mask_query
            .iter()
            .flat_map(|info| info.masks.iter())
            .collect();

        for mask in &all_masks {
            // Create a key based on mask position and size (rounded to int for comparison)
            let key = (
                (mask.center.x * 10.0) as i32,
                (mask.center.y * 10.0) as i32,
                (mask.half_size.x * 10.0) as i32,
                (mask.half_size.y * 10.0) as i32,
            );

            if !seen_masks.insert(key) {
                continue;
            }

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
                    custom_size: Some(Vec2::new(mask.half_size.x * 2.0, mask.half_size.y * 2.0)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(
                    mask.center.x,
                    mask.center.y,
                    100.0, // High z to render on top
                )),
            ));
        }

        if seen_masks.is_empty() {
            println!("[MASK DEBUG] No masks found to visualize");
        } else {
            println!("[MASK DEBUG] Showing {} mask region(s)", seen_masks.len());
        }
    }
}

// ============================================================================
// Video Debug Overlay (requires --features video-debug)
// ============================================================================

#[cfg(feature = "video-debug")]
#[path = "video_debug_systems.rs"]
mod video_debug_systems;

#[cfg(feature = "video-debug")]
use video_debug_systems::*;

// ============================================================================
// Video Comparison (requires --features video-comparison)
// ============================================================================

#[cfg(feature = "video-comparison")]
#[path = "video_comparison_systems.rs"]
mod video_comparison_systems;

// ============================================================================
// Frame Test / FPS Benchmark (requires --features frame-test)
// ============================================================================

#[cfg(feature = "frame-test")]
#[path = "frame_test_systems.rs"]
mod frame_test_systems;

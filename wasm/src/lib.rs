//! # bevy_alight_motion_wasm
//!
//! WASM entry point for bevy_alight_motion player.
//! 用于浏览器的 bevy_alight_motion 播放器 WASM 入口。
//!
//! 使用 Bevy 原生的 MemoryAssetReader 实现动态资源加载。

use bevy::asset::io::memory::{Dir, MemoryAssetReader};
use bevy::asset::io::{AssetSourceBuilder, AssetSourceId};
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use bevy::text::{TextColor, TextFont};
use bevy::ui::Node;
use bevy::window::WindowPlugin;
use bevy_alight_motion::prelude::*;
use bevy_embedded_assets::{EmbeddedAssetPlugin, PluginMode};
use serde::Serialize;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use wasm_bindgen::prelude::*;

/// Global state for JavaScript interop
/// 用于与 JavaScript 交互的全局状态
static APP_STATE: Mutex<Option<AppState>> = Mutex::new(None);

/// Log buffer for collecting runtime logs
/// 运行时日志缓冲区
static LOG_BUFFER: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Global memory directory for uploaded assets
/// 用于上传资源的全局内存目录
static UPLOAD_DIR: OnceLock<Dir> = OnceLock::new();

/// Application state shared with JavaScript
#[derive(Default, Clone, Serialize)]
struct AppState {
    is_playing: bool,
    current_frame: u32,
    total_frames: u32,
    fps: f32,
    project_loaded: bool,
}

/// Resource to signal that a new project should be loaded
#[derive(Resource, Default)]
struct PendingProjectLoad {
    should_load: bool,
}

/// UI text component for status display
#[derive(Component)]
struct StatusText;

/// UI text component for instructions display
#[derive(Component)]
struct InstructionsText;

/// Plugin to register the "uploaded://" asset source
struct UploadedAssetSourcePlugin;

impl Plugin for UploadedAssetSourcePlugin {
    fn build(&self, app: &mut App) {
        // Get or create the upload directory
        let dir = UPLOAD_DIR.get_or_init(Dir::default).clone();

        // Register "uploaded://" as an asset source using MemoryAssetReader
        app.register_asset_source(
            AssetSourceId::from("uploaded"),
            AssetSourceBuilder::new(move || Box::new(MemoryAssetReader { root: dir.clone() })),
        );

        info!("[WASM] Registered 'uploaded://' asset source");
    }
}

/// WASM module initialization — lightweight, no Bevy.
/// Bevy app is started separately via `start_app()` after the canvas is visible.
#[wasm_bindgen(start)]
pub fn wasm_init() {
    console_error_panic_hook::set_once();
    add_log("WASM module initialized");

    *APP_STATE.lock().unwrap() = Some(AppState::default());
    add_log("App state initialized, call start_app() when canvas is ready");
}

/// Start the Bevy application.
/// Must be called AFTER `<canvas id="bevy-canvas">` is visible and has non-zero dimensions.
/// On high-DPI mobile devices, cap `window.devicePixelRatio` from JS before calling this.
#[wasm_bindgen]
pub fn start_app() {
    add_log("start_app() called, launching Bevy...");

    App::new()
        .add_plugins(UploadedAssetSourcePlugin)
        .add_plugins((
            EmbeddedAssetPlugin {
                mode: PluginMode::ReplaceDefault,
            },
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        canvas: Some("#bevy-canvas".into()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(bevy::log::LogPlugin {
                    level: bevy::log::Level::INFO,
                    filter: "wgpu=warn,bevy_render=warn".to_string(),
                    ..default()
                }),
        ))
        .add_plugins(AlightMotionPlugin)
        .insert_resource(AmProjectResolution::FitWindow)
        .init_resource::<PendingProjectLoad>()
        .add_systems(Startup, setup_camera)
        .add_systems(
            Update,
            (
                check_pending_load,
                handle_input,
                update_ui,
                sync_state_to_js,
                handle_pending_load,
            )
                .chain(),
        )
        .run();
}

/// Setup the 2D camera and UI text
fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
    info!("[WASM] Camera2d spawned");

    // Status text (top-left)
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

    // Instructions text (bottom-left)
    commands.spawn((
        Text::new("[Space] Play/Pause | [R] Reset | [P] Replay | [←/→] Frame Step | [↑/↓] Speed | [L] Loop"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgba(0.8, 0.8, 0.8, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        InstructionsText,
    ));
}

/// Handle keyboard input for playback control
/// 处理键盘输入以控制播放
fn handle_input(keyboard: Res<ButtonInput<KeyCode>>, mut playback: Option<ResMut<AmPlayback>>) {
    let Some(ref mut playback) = playback else {
        return;
    };

    // Play/Pause toggle (Space)
    if keyboard.just_pressed(KeyCode::Space) {
        playback.toggle();
        info!(
            "[WASM] Toggle playback: {}",
            if playback.playing {
                "playing"
            } else {
                "paused"
            }
        );
    }

    // Reset (R)
    if keyboard.just_pressed(KeyCode::KeyR) {
        playback.reset();
        info!("[WASM] Reset playback");
    }

    // Replay (P) - reset and play
    if keyboard.just_pressed(KeyCode::KeyP) {
        playback.reset();
        playback.playing = true;
        info!("[WASM] Replay");
    }

    // Frame-by-frame stepping (Left/Right arrows)
    let frame_duration_ms = 1000.0 / 30.0; // 30 fps
    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        playback.playing = false;
        playback.current_time_ms = (playback.current_time_ms - frame_duration_ms).max(0.0);
        info!("[WASM] Frame step back: {:.1}ms", playback.current_time_ms);
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) {
        playback.playing = false;
        playback.current_time_ms =
            (playback.current_time_ms + frame_duration_ms).min(playback.total_time_ms);
        info!(
            "[WASM] Frame step forward: {:.1}ms",
            playback.current_time_ms
        );
    }

    // Speed control (Up = faster, Down = slower)
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        playback.speed = (playback.speed + 0.1).min(4.0);
        info!("[WASM] Speed: {:.1}x", playback.speed);
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        playback.speed = (playback.speed - 0.1).max(0.1);
        info!("[WASM] Speed: {:.1}x", playback.speed);
    }

    // Loop toggle (L)
    if keyboard.just_pressed(KeyCode::KeyL) {
        playback.looping = !playback.looping;
        info!("[WASM] Loop: {}", playback.looping);
    }
}

/// Update UI text with playback status
fn update_ui(playback: Option<Res<AmPlayback>>, mut query: Query<&mut Text, With<StatusText>>) {
    let Some(playback) = playback else {
        // 项目尚未加载
        for mut text in query.iter_mut() {
            **text = "Upload a .amproj file to start".to_string();
        }
        return;
    };

    for mut text in query.iter_mut() {
        let status = if playback.force_stopped {
            "STOPPED"
        } else if playback.playing {
            "Playing"
        } else {
            "Paused"
        };
        let loop_status = if playback.looping { "Loop" } else { "Once" };

        **text = format!(
            "{} | {:.0}/{:.0}ms | {:.1}x | {}",
            status, playback.current_time_ms, playback.total_time_ms, playback.speed, loop_status
        );
    }
}

/// Handle pending project load requests
fn handle_pending_load(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut pending: ResMut<PendingProjectLoad>,
) {
    if pending.should_load {
        pending.should_load = false;

        add_log("Starting project load from uploaded://project.amproj");
        info!("[WASM] Loading project from uploaded://project.amproj");

        let entity = load_am_project(&mut commands, &asset_server, "uploaded://project.amproj");
        add_log(&format!("Project loaded, entity: {:?}", entity));
    }
}

/// Sync Bevy state to JavaScript-accessible global state
fn sync_state_to_js(playback: Option<Res<AmPlayback>>) {
    if let Some(playback) = playback {
        if let Ok(mut state) = APP_STATE.lock() {
            if let Some(ref mut s) = *state {
                let was_loaded = s.project_loaded;
                s.is_playing = playback.playing;
                s.current_frame = (playback.current_time_ms / 16.67) as u32;
                s.total_frames = (playback.total_time_ms / 16.67) as u32;
                s.fps = 60.0;

                if !was_loaded && playback.total_time_ms > 0.0 {
                    s.project_loaded = true;
                    add_log("Project loaded successfully!");
                }
            }
        }
    }
}

/// Load a project from JavaScript (receives ArrayBuffer bytes)
/// 从 JavaScript 加载项目 (接收 ArrayBuffer 字节)
///
/// This function:
/// 1. Inserts the project bytes into the memory asset source
/// 2. Triggers the Bevy asset system to load it
#[wasm_bindgen]
pub fn load_project_from_bytes(data: &[u8]) -> bool {
    let msg = format!("Received project data: {} bytes", data.len());
    add_log(&msg);
    info!("[WASM] {}", msg);

    // Get the upload directory
    let Some(dir) = UPLOAD_DIR.get() else {
        add_log("ERROR: Upload directory not initialized");
        web_sys::console::error_1(&"[WASM] Upload directory not initialized".into());
        return false;
    };

    // Insert the project bytes into the memory directory
    // 将项目字节插入内存目录
    dir.insert_asset(Path::new("project.amproj"), data.to_vec());
    add_log("Project bytes inserted into uploaded:// source");
    info!("[WASM] Project bytes inserted into uploaded:// source");

    // We can't directly access Bevy's World from here, so we use a static flag
    // The handle_pending_load system will pick this up
    // 我们无法从这里直接访问 Bevy 的 World，所以使用静态标志
    // handle_pending_load 系统会处理这个

    // Set the pending load flag via APP_STATE
    if let Ok(mut state) = APP_STATE.lock() {
        if let Some(ref mut s) = *state {
            s.project_loaded = false; // Reset until actually loaded
        }
    }

    // Signal that we need to load - we'll use a different mechanism
    // For now, we need to trigger the load from Bevy's Update loop
    // This is a limitation of WASM - we can't call Bevy systems directly
    add_log("Project ready for loading. Triggering load...");
    web_sys::console::log_1(&"[WASM] Project ready for loading. Triggering load...".into());

    // Use atomic flag to signal pending load (thread-safe)
    PENDING_LOAD.store(true, Ordering::SeqCst);

    true
}

/// Atomic flag for pending load
static PENDING_LOAD: AtomicBool = AtomicBool::new(false);

/// Check and handle pending load flag
fn check_pending_load(mut pending: ResMut<PendingProjectLoad>) {
    if PENDING_LOAD.swap(false, Ordering::SeqCst) {
        pending.should_load = true;
        add_log("Detected pending load, will load project");
        info!("[WASM] Detected pending load, will load project");
    }
}

/// Get current player state as JSON
/// 获取当前播放器状态 (JSON 格式)
#[wasm_bindgen]
pub fn get_state() -> JsValue {
    if let Ok(state) = APP_STATE.lock() {
        if let Some(ref s) = *state {
            return serde_wasm_bindgen::to_value(s).unwrap_or(JsValue::NULL);
        }
    }
    JsValue::NULL
}

/// Play the animation
/// 播放动画
#[wasm_bindgen]
pub fn play() {
    web_sys::console::log_1(&"[WASM] Play requested".into());
    // TODO: Send event to Bevy to start playing
}

/// Pause the animation
/// 暂停动画
#[wasm_bindgen]
pub fn pause() {
    web_sys::console::log_1(&"[WASM] Pause requested".into());
    // TODO: Send event to Bevy to pause
}

/// Seek to a specific frame
/// 跳转到指定帧
#[wasm_bindgen]
pub fn seek(frame: u32) {
    web_sys::console::log_1(&format!("[WASM] Seek to frame {}", frame).into());
    // TODO: Send event to Bevy to seek
}

/// Reset to the beginning
/// 重置到开头
#[wasm_bindgen]
pub fn reset() {
    web_sys::console::log_1(&"[WASM] Reset requested".into());
    seek(0);
}

/// Get current frame pixels for video comparison
/// 获取当前帧像素数据用于视频对比
#[wasm_bindgen]
pub fn get_current_frame_pixels() -> Vec<u8> {
    // TODO: Implement frame capture from Bevy's render target
    // This requires access to the GPU texture which is complex in WASM
    vec![]
}

/// Download runtime logs as a text file
/// 下载运行时日志为文本文件 (兼容移动端)
#[wasm_bindgen]
pub fn download_logs() {
    let logs = {
        let buffer = LOG_BUFFER.lock().unwrap();
        buffer.join("\n")
    };

    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");

    // 移动端兼容：使用 data URL 方案
    // 将日志内容编码为 base64
    let encoded = js_sys::encode_uri_component(&logs);
    let data_url = format!("data:text/plain;charset=utf-8,{}", encoded);

    // 创建隐藏的 a 标签
    if let Ok(a) = document.create_element("a") {
        let _ = a.set_attribute("href", &data_url);
        let _ = a.set_attribute("download", "bevy_alight_motion_logs.txt");

        // 尝试使用 PointerEvent (更兼容移动端)
        if let Ok(pe) = web_sys::PointerEvent::new("click") {
            let _ = a.dispatch_event(&pe);
        } else if let Ok(me) = web_sys::MouseEvent::new("click") {
            let _ = a.dispatch_event(&me);
        }
    }

    web_sys::console::log_1(&"Logs downloaded".into());
}

/// Add a log entry to the buffer
/// 向缓冲区添加日志条目
pub fn add_log(message: &str) {
    let timestamp = js_sys::Date::new_0().to_iso_string();
    let entry = format!("[{}] {}", timestamp, message);
    if let Ok(mut buffer) = LOG_BUFFER.lock() {
        buffer.push(entry);
    }
}

/// Get logs as string for JavaScript
/// 获取日志字符串供 JavaScript 使用
#[wasm_bindgen]
pub fn get_logs() -> String {
    let buffer = LOG_BUFFER.lock().unwrap();
    buffer.join("\n")
}

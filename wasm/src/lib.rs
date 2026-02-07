//! # bevy_alight_motion_wasm
//!
//! WASM entry point for bevy_alight_motion player.
//! 用于浏览器的 bevy_alight_motion 播放器 WASM 入口。
//!
//! 使用 Bevy 原生的 MemoryAssetReader 实现动态资产加载。

use bevy::asset::io::memory::{Dir, MemoryAssetReader};
use bevy::asset::io::{AssetSource, AssetSourceId};
use bevy::prelude::*;
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

/// Global memory directory for uploaded assets
/// 用于上传资产的全局内存目录
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

/// Plugin to register the "uploaded://" asset source
struct UploadedAssetSourcePlugin;

impl Plugin for UploadedAssetSourcePlugin {
    fn build(&self, app: &mut App) {
        // Get or create the upload directory
        let dir = UPLOAD_DIR.get_or_init(Dir::default).clone();
        
        // Register "uploaded://" as an asset source using MemoryAssetReader
        app.register_asset_source(
            AssetSourceId::from("uploaded"),
            AssetSource::build().with_reader(move || {
                Box::new(MemoryAssetReader { root: dir.clone() })
            }),
        );
        
        info!("[WASM] Registered 'uploaded://' asset source");
    }
}

/// Main entry point for WASM
#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    // Set up panic hook for better error messages in browser console
    console_error_panic_hook::set_once();

    // Initialize app state
    *APP_STATE.lock().unwrap() = Some(AppState::default());

    App::new()
        // Register uploaded asset source BEFORE other plugins
        // 在其他插件之前注册上传资产源
        .add_plugins(UploadedAssetSourcePlugin)
        // 使用嵌入式资产插件，在编译时将 assets 目录嵌入 WASM
        // 必须在 DefaultPlugins 之前添加
        // ReplaceDefault 模式会替换默认资产源，使 "shaders/xxx.wgsl" 路径可正常工作
        .add_plugins((
            EmbeddedAssetPlugin { mode: PluginMode::ReplaceDefault },
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        canvas: Some("#bevy-canvas".into()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: false,
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
        .insert_resource(AmProjectResolution::FitWindow) // 适应窗口大小
        .init_resource::<PendingProjectLoad>()
        .add_systems(Startup, setup_camera)
        .add_systems(Update, (check_pending_load, sync_state_to_js, handle_pending_load).chain())
        .run();

    Ok(())
}

/// Setup the 2D camera
fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
    info!("[WASM] Camera2d spawned");
}

/// Handle pending project load requests
fn handle_pending_load(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut pending: ResMut<PendingProjectLoad>,
) {
    if pending.should_load {
        pending.should_load = false;
        
        // Load the project from the uploaded asset source
        info!("[WASM] Loading project from uploaded://project.amproj");
        load_am_project(&mut commands, &asset_server, "uploaded://project.amproj");
    }
}

/// Sync Bevy state to JavaScript-accessible global state
fn sync_state_to_js(playback: Option<Res<AmPlayback>>) {
    if let Some(playback) = playback {
        if let Ok(mut state) = APP_STATE.lock() {
            if let Some(ref mut s) = *state {
                s.is_playing = playback.playing;
                s.current_frame = (playback.current_time_ms / 16.67) as u32;
                s.total_frames = (playback.total_time_ms / 16.67) as u32;
                s.fps = 60.0;
                s.project_loaded = true;
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
    info!("[WASM] Received project data: {} bytes", data.len());
    
    // Get the upload directory
    let Some(dir) = UPLOAD_DIR.get() else {
        web_sys::console::error_1(&"[WASM] Upload directory not initialized".into());
        return false;
    };
    
    // Insert the project bytes into the memory directory
    // 将项目字节插入内存目录
    dir.insert_asset(Path::new("project.amproj"), data.to_vec());
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

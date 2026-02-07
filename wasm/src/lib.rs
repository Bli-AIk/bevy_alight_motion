//! # bevy_alight_motion_wasm
//!
//! WASM entry point for bevy_alight_motion player.
//! 用于浏览器的 bevy_alight_motion 播放器 WASM 入口。

use bevy::prelude::*;
use bevy::window::WindowPlugin;
use bevy_alight_motion::prelude::*;
use serde::Serialize;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;

/// Global state for JavaScript interop
/// 用于与 JavaScript 交互的全局状态
static APP_STATE: Mutex<Option<AppState>> = Mutex::new(None);

/// Application state shared with JavaScript
#[derive(Default, Clone, Serialize)]
struct AppState {
    is_playing: bool,
    current_frame: u32,
    total_frames: u32,
    fps: f32,
    project_loaded: bool,
}

/// Main entry point for WASM
#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    // Set up panic hook for better error messages in browser console
    console_error_panic_hook::set_once();

    // Initialize app state
    *APP_STATE.lock().unwrap() = Some(AppState::default());

    App::new()
        .add_plugins(
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
        )
        .add_plugins(AlightMotionPlugin)
        .add_systems(Update, sync_state_to_js)
        .run();

    Ok(())
}

/// Sync Bevy state to JavaScript-accessible global state
fn sync_state_to_js(
    playback: Option<Res<AmPlayback>>,
) {
    if let Some(playback) = playback {
        if let Ok(mut state) = APP_STATE.lock() {
            if let Some(ref mut s) = *state {
                s.is_playing = playback.playing;
                s.current_frame = (playback.current_time_ms / 16.67) as u32; // Approximate frame at 60fps
                s.total_frames = (playback.total_time_ms / 16.67) as u32;
                s.fps = 60.0;
                s.project_loaded = true;
            }
        }
    }
}

/// Load a project from JavaScript (receives ArrayBuffer bytes)
/// 从 JavaScript 加载项目 (接收 ArrayBuffer 字节)
#[wasm_bindgen]
pub fn load_project_from_bytes(data: &[u8]) -> bool {
    web_sys::console::log_1(&format!("[WASM] Received project data: {} bytes", data.len()).into());

    // Store in pending project resource
    // Note: In a full implementation, we'd use an event or channel
    // For now, we log and return success
    // The actual loading happens through Bevy's asset system

    true
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

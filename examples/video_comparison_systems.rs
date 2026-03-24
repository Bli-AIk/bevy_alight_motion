//! Systems that compare rendered frames against reference video.
//! 负责将渲染结果与参考视频逐帧对比的系统集合。
//!
//! This module is the execution core behind the example player's video-comparison mode. It drives
//! capture timing, loads extracted reference frames, computes similarity scores, and writes the
//! final report that decides whether the render matches the expected video.
//! 这个模块是示例播放器视频对比模式的执行核心。它负责推进截图时机、加载提取出的参考帧、计算相似度，
//! 并最终产出用来判定渲染是否符合预期视频的报告。

use super::*;
use crate::video_utils;
use bevy::app::AppExit;
use bevy::ecs::message::MessageWriter;
#[cfg(not(feature = "headless-render"))]
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
    pub settle_signature: Option<(usize, usize, usize)>,
    pub settle_stable_frames: u32,
    pub prime_capture_requests_remaining: u32,
}

#[derive(PartialEq, Debug)]
pub enum TestStage {
    Initializing,
    WaitingForProjectLoad, // Wait for project to load and first frame to render
    SettingTime,
    WaitingForRender,
    PrimingCapture,
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
            settle_signature: None,
            settle_stable_frames: 0,
            prime_capture_requests_remaining: 0,
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

    // Skip the first frame of reference video (when we have enough frames).
    // AM video export's frame 1 is actually at t≈29ms, not t=0ms
    // Our shot_000000 at t=0ms doesn't match ref frame_000001
    // So we skip frame_000001 and start from frame_000002.
    // For ultra-short videos with only 1 frame, keep it to have something to compare.
    if frame_paths.len() > 1 {
        frame_paths.remove(0);
        println!("[COMPARISON] Skipped first reference frame (AM video export timing mismatch)");
    } else if frame_paths.len() == 1 {
        println!("[COMPARISON] Only 1 reference frame available, keeping it (ultra-short video)");
    }

    state.frame_paths = frame_paths;

    if let Some(start_frame) = std::env::var("COMPARISON_START_FRAME")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
    {
        let max_start = state.frame_paths.len().saturating_sub(1);
        state.current_frame = start_frame.min(max_start);
        println!(
            "[COMPARISON] Starting from frame {} via COMPARISON_START_FRAME",
            state.current_frame
        );
    }

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
    let trace_time = std::env::var_os("COMPARISON_TRACE_TIME").is_some();

    // Apply pending time change (set in previous frame's SettingTime stage)
    // This ensures time is set in First schedule, BEFORE lifecycle_system runs in Update
    if let Some(time_ms) = state.pending_time_ms.take() {
        playback.current_time_ms = time_ms;
        // Keep animation systems running at the paused time so composite RTT chains
        // can fully settle before capture. `playing=false` already prevents time from advancing.
        playback.force_stopped = false;
        state.render_wait_frames = std::env::var("COMPARISON_RENDER_WAIT_FRAMES")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(4);
        state.settle_signature = None;
        state.settle_stable_frames = 0;
        state.prime_capture_requests_remaining = 0;
        if trace_time {
            println!(
                "[COMPARISON TIME] applied current_frame={} time_ms={:.1} render_wait_frames={}",
                state.current_frame, time_ms, state.render_wait_frames
            );
        }
        debug!(
            "[PAUSED] Applied pending time: {:.1}ms, render_wait_frames={}",
            time_ms, state.render_wait_frames
        );
        return; // Don't override force_stopped below
    }

    // Handle render_wait_frames countdown
    if state.render_wait_frames > 0 {
        state.render_wait_frames -= 1;
        playback.force_stopped = false;
        if trace_time {
            println!(
                "[COMPARISON TIME] waiting current_frame={} time_ms={:.1} render_wait_frames={}",
                state.current_frame, playback.current_time_ms, state.render_wait_frames
            );
        }
        debug!(
            "[PAUSED] render_wait_frames={} (keeping systems live at fixed time)",
            state.render_wait_frames
        );
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
            playback.force_stopped = false;
            debug!("[PAUSED] stage={:?} force_stopped=false", state.stage);
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

fn print_critical_failures(critical_failed_frames: &[(usize, f32)], min_frame_similarity: f32) {
    if critical_failed_frames.is_empty() {
        return;
    }
    println!(
        "  {} frames below min threshold ({:.2}):",
        "Critical:".red().bold(),
        min_frame_similarity
    );
    for (idx, score) in critical_failed_frames.iter().take(5) {
        println!("    Frame {}: {:.4}", idx, score);
    }
    if critical_failed_frames.len() > 5 {
        println!("    ... and {} more", critical_failed_frames.len() - 5);
    }
}

pub fn comparison_loop(
    mut state: ResMut<ComparisonState>,
    mut playback: ResMut<AmPlayback>,
    #[cfg(not(feature = "headless-render"))] mut commands: Commands,
    _window_query: Query<Entity, With<PrimaryWindow>>,
    _time: Res<Time>,
    mut exit: MessageWriter<AppExit>,
    // Query to check if project is loaded
    project_query: Query<&AmProjectRoot>,
    all_entities: Query<Entity>,
    am_visuals: Query<Entity, With<AmVisualSpawned>>,
    pending_layers_query: Query<&bevy_alight_motion::scene::AmPendingLayers>,
    pending_strategy_query: Query<
        Entity,
        With<bevy_alight_motion::effects::NeedsStrategyEvaluation>,
    >,
    pending_rtt_query: Query<Entity, With<bevy_alight_motion::effects::NeedsEmbedSceneRtt>>,
    #[cfg(feature = "headless-render")] headless_capture_query: Query<
        &headless_capture::HeadlessImageCopier,
    >,
    #[cfg(feature = "headless-render")] mut headless_capture_state: ResMut<
        headless_capture::HeadlessCaptureState,
    >,
) {
    // Use frame-based waiting instead of time-based for determinism.
    // The defaults remain conservative for real comparison runs, but allowing
    // env overrides makes VPS-side debugging practical without changing repo defaults.
    let default_wait_frames = if cfg!(feature = "headless-render") {
        3
    } else {
        5
    };
    let wait_frames = std::env::var("COMPARISON_WAIT_FRAMES")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(default_wait_frames);
    let default_initial_wait_frames = if cfg!(feature = "headless-render") {
        10
    } else {
        30
    };
    let initial_wait_frames = std::env::var("COMPARISON_INITIAL_WAIT_FRAMES")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(default_initial_wait_frames);

    match state.stage {
        TestStage::Initializing => {} // Handled in setup

        TestStage::WaitingForProjectLoad => {
            // Pause is handled by ensure_paused_during_load in First schedule

            // Check if project is loaded by looking for a spawned AmProjectRoot
            let project_loaded = project_query.iter().any(|root| root.spawned);

            if project_loaded {
                state.wait_frames += 1;
                // Wait additional frames for GPU texture upload and first render
                if state.wait_frames >= initial_wait_frames {
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

            if state.current_frame >= state.frame_paths.len() || state.current_frame >= frame_limit
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
            // AM uses integer millisecond times via frameStartTimeFromFrameNumber which does
            // integer division: (frame * 100000) / fphs. Floor to match AM's truncation.
            let time_ms = (time_sec * 1000.0).floor();
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
                let pending_setup =
                    pending_strategy_query.iter().len() + pending_rtt_query.iter().len();
                let tracked_spawned_layers: usize = pending_layers_query
                    .iter()
                    .map(|pending| pending.spawned_entities.len())
                    .sum();
                let settle_signature = (
                    all_entities.iter().len(),
                    am_visuals.iter().len(),
                    tracked_spawned_layers,
                );

                if pending_setup > 0 {
                    state.settle_signature = None;
                    state.settle_stable_frames = 0;
                    return;
                }

                if state.settle_signature == Some(settle_signature) {
                    state.settle_stable_frames += 1;
                } else {
                    state.settle_signature = Some(settle_signature);
                    state.settle_stable_frames = 1;
                }

                state.wait_frames = state.settle_stable_frames;
                if state.settle_stable_frames >= wait_frames {
                    state.settle_signature = None;
                    state.settle_stable_frames = 0;
                    state.prime_capture_requests_remaining =
                        std::env::var("COMPARISON_PRIME_CAPTURES")
                            .ok()
                            .and_then(|s| s.parse::<u32>().ok())
                            .unwrap_or(1);
                    state.stage = TestStage::PrimingCapture;
                }
            }
            // If render_wait_frames > 0, just wait for it to count down
        }

        TestStage::PrimingCapture => {
            #[cfg(feature = "headless-render")]
            {
                if state.prime_capture_requests_remaining > 0 {
                    if headless_capture_state.pending_path.is_none()
                        && headless_capture_state.discard_captures == 0
                        && let Ok(image_copier) = headless_capture_query.single()
                    {
                        let serial = headless_capture_state.next_serial;
                        headless_capture_state.next_serial += 1;
                        headless_capture_state.discard_captures =
                            headless_capture_state.discard_captures.saturating_add(1);
                        state.prime_capture_requests_remaining -= 1;
                        image_copier.request(serial);
                    }
                    return;
                }

                if headless_capture_state.discard_captures > 0
                    || headless_capture_state.pending_path.is_some()
                {
                    return;
                }
            }

            state.stage = TestStage::Capturing;
        }

        TestStage::Capturing => {
            let frame_idx = state.current_frame;
            let report_dir = state.report_dir.clone();
            let shot_path = report_dir.join(format!("shot_{:06}.png", frame_idx));

            // Note: force_stopped is set in ensure_paused_during_load (First schedule)
            // to guarantee it's set BEFORE lifecycle_system runs

            // Trigger screenshot
            #[cfg(feature = "headless-render")]
            {
                if headless_capture_state.pending_path.is_none()
                    && let Ok(image_copier) = headless_capture_query.single()
                {
                    let serial = headless_capture_state.next_serial;
                    headless_capture_state.next_serial += 1;
                    image_copier.request(serial);
                    headless_capture_state.pending_serial = Some(serial);
                    headless_capture_state.pending_path = Some(shot_path);
                }
            }
            #[cfg(not(feature = "headless-render"))]
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(shot_path));

            state.wait_frames = 0;
            state.stage = TestStage::WaitingForScreenshot;
        }

        TestStage::WaitingForScreenshot => {
            state.wait_frames += 1;
            if state.wait_frames < 1 {
                return;
            }

            state.stage = TestStage::Comparing;
        }

        TestStage::Comparing => {
            let frame_idx = state.current_frame;
            let shot_path = state.report_dir.join(format!("shot_{:06}.png", frame_idx));

            if !shot_path.exists() {
                return;
            }

            let shot_img = match image::open(&shot_path) {
                Ok(img) => img.to_rgba8(),
                Err(_) => return,
            };

            let ref_path = &state.frame_paths[frame_idx];

            // Debug: log actual paths being compared for frame 30 and copy ref frame
            if frame_idx.is_multiple_of(5) {
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
            } else if frame_idx.is_multiple_of(10) {
                println!(
                    "[FRAME {:03}] Similarity: {:.4} ({}) | Content: {:.4}, Match: {:.1}%",
                    frame_idx,
                    similarity,
                    "OK".green(),
                    result.content_similarity,
                    result.pixel_match_rate * 100.0
                );
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
            let max_allowed_failed = (total_frames as f32 * state.max_failed_rate).ceil() as usize;
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
                print_critical_failures(&critical_failed_frames, state.min_frame_similarity);

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

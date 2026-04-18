//! Systems that implement the frame-rate benchmark mode for the example player.
//!
//! 为示例播放器实现帧率基准模式的系统集合。
//!
//! The example player can run in a non-interactive benchmark mode that warms up the scene, samples
//! frame times, and decides pass/warn/fail thresholds. That state machine and the
//! reporting logic around it.
//!
//! 示例播放器支持一种非交互式基准模式：先预热场景，再采样帧耗时，并给出通过 / 警告 / 失败结论。
//! 负责这套状态机以及围绕它的报告输出逻辑。

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
    pub animation_completed: bool,
    pub prev_time_ms: f64,
    /// Two-pass measurement: pass 1 warms shaders (discarded), pass 2 is the real measurement.
    pub measurement_pass: u32,
    /// Wall-clock instant of the previous frame, used for uncapped delta measurement.
    pub last_instant: Option<std::time::Instant>,
    // Config
    pub pass_fps: f32,
    pub fail_fps: f32,
    pub max_below_fail_rate: f32,
    pub max_below_pass_rate: f32,
    pub min_sample_frames: u32,
    pub warmup_frames: u32,
    pub measure_duration_secs: f32,
    pub play_once: bool,
    pub stutter_threshold_multiplier: f32,
    pub max_stutter_rate: f32,
}

impl Default for FrameTestState {
    fn default() -> Self {
        Self {
            stage: FrameTestStage::WaitingForLoad,
            warmup_frames_remaining: 60,
            frame_times: Vec::new(),
            measurement_elapsed: 0.0,
            project_name: String::new(),
            animation_completed: false,
            prev_time_ms: 0.0,
            measurement_pass: 1,
            last_instant: None,
            pass_fps: 120.0,
            fail_fps: 60.0,
            max_below_fail_rate: 0.20,
            max_below_pass_rate: 0.20,
            min_sample_frames: 30,
            warmup_frames: 60,
            measure_duration_secs: 15.0,
            play_once: false,
            stutter_threshold_multiplier: 2.0,
            max_stutter_rate: 0.05,
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
    #[serde(default)]
    play_once: bool,
    #[serde(default = "default_stutter_threshold_multiplier")]
    stutter_threshold_multiplier: f32,
    #[serde(default = "default_max_stutter_rate")]
    max_stutter_rate: f32,
}

fn default_pass_fps() -> f32 {
    120.0
}
fn default_fail_fps() -> f32 {
    60.0
}
fn default_max_below_fail_rate() -> f32 {
    0.20
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
fn default_stutter_threshold_multiplier() -> f32 {
    2.0
}
fn default_max_stutter_rate() -> f32 {
    0.05
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

    // Environment variable overrides play_once
    let env_play_once = std::env::var("AM_FRAME_TEST_PLAY_ONCE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    // Load config
    let config_content =
        std::fs::read_to_string("crates/bevy_alight_motion/comparison_config.toml")
            .or_else(|_| std::fs::read_to_string("comparison_config.toml"))
            .ok();

    if let Some(content) = config_content
        && let Ok(cfg) = toml::from_str::<ConfigFile>(&content)
        && let Some(ft) = cfg.frame_test
    {
        state.pass_fps = ft.pass_fps;
        state.fail_fps = ft.fail_fps;
        state.max_below_fail_rate = ft.max_below_fail_rate;
        state.max_below_pass_rate = ft.max_below_pass_rate;
        state.min_sample_frames = ft.min_sample_frames;
        state.warmup_frames = ft.warmup_frames;
        state.measure_duration_secs = ft.measure_duration_secs;
        state.play_once = ft.play_once;
        state.stutter_threshold_multiplier = ft.stutter_threshold_multiplier;
        state.max_stutter_rate = ft.max_stutter_rate;
    }

    // Env var overrides config
    if env_play_once {
        state.play_once = true;
    }

    state.warmup_frames_remaining = state.warmup_frames;

    let mode = if state.play_once { "play-once" } else { "loop" };
    println!(
        "[FRAME-TEST] Config for '{}': pass_fps={:.0}, fail_fps={:.0}, warmup={}, \
         measure={:.0}s, mode={}, stutter_mult={:.1}x",
        project_name,
        state.pass_fps,
        state.fail_fps,
        state.warmup_frames,
        state.measure_duration_secs,
        mode,
        state.stutter_threshold_multiplier,
    );
}

fn handle_warmup_completion(state: &mut FrameTestState, playback: &mut AmPlayback) {
    state.stage = FrameTestStage::Running;
    if state.measurement_pass == 1 {
        println!(
            "[FRAME-TEST] Warmup complete, pass 1/2: shader warmup ({:.0}s, discarded)...",
            state.measure_duration_secs
        );
    } else {
        // Inter-pass warmup done → start real measurement
        if state.play_once {
            playback.looping = false;
            playback.current_time_ms = 0.0;
            playback.playing = true;
            state.prev_time_ms = 0.0;
        }
        let mode_msg = if state.play_once {
            format!("one full animation ({:.0}ms)", playback.total_time_ms)
        } else {
            format!("{:.0}s", state.measure_duration_secs)
        };
        println!(
            "[FRAME-TEST] Stabilized, pass 2/2: measuring FPS for {}...",
            mode_msg
        );
    }
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
            let project_loaded = project_query.iter().any(|root| root.spawned);
            if project_loaded {
                // Always loop during warmup + first measurement pass so all shaders
                // and pipelines compile before the real measurement begins.
                playback.looping = true;
                state.stage = FrameTestStage::Warmup;
                println!(
                    "[FRAME-TEST] Project loaded (duration={:.1}ms), warming up ({} frames)...",
                    playback.total_time_ms, state.warmup_frames_remaining
                );
            }
        }

        FrameTestStage::Warmup => {
            if state.warmup_frames_remaining > 0 {
                state.warmup_frames_remaining -= 1;
            }

            if state.warmup_frames_remaining == 0 {
                handle_warmup_completion(&mut state, &mut playback);
            }
        }

        FrameTestStage::Running => {
            let now = std::time::Instant::now();
            let dt = state.last_instant.map_or(time.delta_secs_f64(), |prev| {
                now.duration_since(prev).as_secs_f64()
            });
            state.last_instant = Some(now);
            if dt > 0.0 {
                state.frame_times.push(dt);
                state.measurement_elapsed += dt;
            }

            // Check end condition
            let should_finish = if state.play_once && state.measurement_pass == 2 {
                // Detect animation completion: current_time wrapped back or reached end
                let current = playback.current_time_ms as f64;
                let total = playback.total_time_ms as f64;
                let completed = if total > 0.0 {
                    // Animation finished when time reaches total or wraps around
                    current >= total - 1.0
                        || (current < state.prev_time_ms && state.prev_time_ms > total * 0.5)
                        || !playback.playing
                } else {
                    false
                };
                state.prev_time_ms = current;
                if completed {
                    state.animation_completed = true;
                }
                state.animation_completed
            } else {
                state.measurement_elapsed >= state.measure_duration_secs as f64
            };

            if should_finish {
                if state.measurement_pass == 1 {
                    // First pass done: all shaders compiled, pipelines cached.
                    // Go back to warmup for inter-pass stabilization before real measurement.
                    let discarded = state.frame_times.len();
                    state.frame_times.clear();
                    state.measurement_elapsed = 0.0;
                    state.animation_completed = false;
                    state.measurement_pass = 2;
                    state.last_instant = None;
                    state.warmup_frames_remaining = state.warmup_frames;
                    state.stage = FrameTestStage::Warmup;
                    println!(
                        "[FRAME-TEST] Pass 1 complete ({} frames discarded). \
                         Stabilizing ({} frames)...",
                        discarded, state.warmup_frames
                    );
                } else {
                    state.stage = FrameTestStage::Finished;
                }
            }
        }

        FrameTestStage::Finished => {
            playback.playing = false;
            report_results(&state, &mut exit);
        }
    }
}

fn report_results(state: &FrameTestState, exit: &mut MessageWriter<AppExit>) {
    let raw_frames = state.frame_times.len();
    println!();
    println!("========================================");
    println!("FRAME TEST RESULTS: {}", state.project_name);
    println!("========================================");

    if (raw_frames as u32) < state.min_sample_frames {
        println!(
            "{}",
            format!(
                "RESULT: FAIL ❌ (insufficient frames: {} < {})",
                raw_frames, state.min_sample_frames
            )
            .red()
            .bold()
        );
        println!(
            "[FRAME-TEST-JSON] {{\"project\":\"{}\",\"status\":\"fail\",\"reason\":\"insufficient_frames\",\
             \"total_frames\":{}}}",
            state.project_name, raw_frames
        );
        exit.write(AppExit::Error(std::num::NonZero::new(1).unwrap()));
        return;
    }

    // Exclude environmental outliers (container scheduling, GPU driver maintenance, etc.)
    // These are frames so slow they cannot be caused by rendering code.
    const OUTLIER_THRESHOLD_SECS: f64 = 0.5; // 500ms
    let outlier_count = state
        .frame_times
        .iter()
        .filter(|&&dt| dt > OUTLIER_THRESHOLD_SECS)
        .count();
    if outlier_count > 0 {
        println!(
            "⚠️  Excluding {} environmental outlier frame(s) (>{:.0}ms) from stats",
            outlier_count,
            OUTLIER_THRESHOLD_SECS * 1000.0
        );
        for (i, &dt) in state.frame_times.iter().enumerate() {
            if dt > OUTLIER_THRESHOLD_SECS {
                println!("   Frame {}: {:.1}ms", i, dt * 1000.0);
            }
        }
    }
    let frame_times: Vec<f64> = state
        .frame_times
        .iter()
        .copied()
        .filter(|&dt| dt <= OUTLIER_THRESHOLD_SECS)
        .collect();
    let total_frames = frame_times.len();

    // Frame time histogram for diagnostics
    let buckets = [
        (0.0, 1.0, ">1000 FPS"),
        (1.0, 2.0, "500-1000 FPS"),
        (2.0, 4.0, "250-500 FPS"),
        (4.0, 6.94, "144-250 FPS"),
        (6.94, 16.67, "60-144 FPS"),
        (16.67, 33.33, "30-60 FPS"),
        (33.33, 500.0, "<30 FPS"),
    ];
    println!("Frame time distribution:");
    for (lo_ms, hi_ms, label) in &buckets {
        let count = frame_times
            .iter()
            .filter(|&&dt| {
                let ms = dt * 1000.0;
                ms >= *lo_ms && ms < *hi_ms
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

    // Compute FPS stats
    let avg_dt: f64 = frame_times.iter().sum::<f64>() / total_frames as f64;
    let avg_fps = 1.0 / avg_dt;

    let min_dt = frame_times.iter().copied().fold(f64::INFINITY, f64::min);
    let max_dt = frame_times.iter().copied().fold(0.0_f64, f64::max);
    let max_fps = 1.0 / min_dt;
    let min_fps = 1.0 / max_dt;
    let max_frame_time_ms = max_dt * 1000.0;

    // Percentile FPS: sort frame times ascending (shortest first → highest FPS)
    let mut sorted = frame_times.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let percentile = |p: f64| -> f64 {
        let idx = ((total_frames as f64) * p).ceil() as usize;
        1.0 / sorted[idx.min(total_frames - 1)]
    };

    let p95_fps = percentile(0.95);
    let p99_fps = percentile(0.99);
    let p1_fps = p99_fps; // 1% low = p99 of frame times

    // Median frame time for stutter detection
    let median_dt = sorted[total_frames / 2];
    let stutter_threshold = median_dt * state.stutter_threshold_multiplier as f64;
    let stutter_count = frame_times
        .iter()
        .filter(|&&dt| dt > stutter_threshold)
        .count();
    let stutter_rate = stutter_count as f64 / total_frames as f64;

    // Find top stutter spikes (for diagnostics)
    let mut spike_indices: Vec<(usize, f64)> = frame_times
        .iter()
        .enumerate()
        .filter(|(_, dt)| **dt > stutter_threshold)
        .map(|(i, dt)| (i, *dt * 1000.0))
        .collect();
    spike_indices.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    spike_indices.truncate(10);

    let below_fail = frame_times
        .iter()
        .filter(|&&dt| 1.0 / dt < state.fail_fps as f64)
        .count();
    let below_pass = frame_times
        .iter()
        .filter(|&&dt| 1.0 / dt < state.pass_fps as f64)
        .count();

    let below_fail_rate = below_fail as f64 / total_frames as f64;
    let below_pass_rate = below_pass as f64 / total_frames as f64;

    println!("Total frames sampled: {}", total_frames);
    println!(
        "Mode: {}",
        if state.play_once { "play-once" } else { "loop" }
    );
    println!("Average FPS: {:.1}", avg_fps);
    println!("Min FPS: {:.1} | Max FPS: {:.1}", min_fps, max_fps);
    println!("1% Low FPS (p99): {:.1}", p1_fps);
    println!("5% Low FPS (p95): {:.1}", p95_fps);
    println!("Max frame time: {:.2}ms", max_frame_time_ms);
    println!(
        "Median frame time: {:.2}ms ({:.0} FPS)",
        median_dt * 1000.0,
        1.0 / median_dt
    );
    println!(
        "Stutter frames (>{:.1}x median = >{:.2}ms): {} ({:.1}%) [max allowed: {:.1}%]",
        state.stutter_threshold_multiplier,
        stutter_threshold * 1000.0,
        stutter_count,
        stutter_rate * 100.0,
        state.max_stutter_rate * 100.0,
    );
    if !spike_indices.is_empty() {
        println!("Top stutter spikes:");
        for (i, (frame_idx, ms)) in spike_indices.iter().enumerate() {
            println!(
                "  #{}: frame {} = {:.2}ms ({:.1} FPS)",
                i + 1,
                frame_idx,
                ms,
                1000.0 / ms
            );
        }
    }
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

    // Determine result (two-tier system):
    //
    // FAIL:    avg < fail_fps OR too many frames below fail_fps
    // GREAT:   avg >= fail_fps (stable 60 FPS tier)
    // PERFECT: avg >= pass_fps AND few frames below pass_fps AND stutter OK
    let fail_rate_exceeded = below_fail_rate > state.max_below_fail_rate as f64;
    let stutter_exceeded = stutter_rate > state.max_stutter_rate as f64;

    let status;
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
        println!("{}", format!("RESULT: FAIL ❌ ({})", reason).red().bold());
        if stutter_exceeded {
            println!(
                "{}",
                format!(
                    "  + STUTTER ❌ ({:.1}% > {:.1}%)",
                    stutter_rate * 100.0,
                    state.max_stutter_rate * 100.0
                )
                .red()
            );
        }
        status = "fail";
        exit.write(AppExit::Error(std::num::NonZero::new(1).unwrap()));
    } else if avg_fps >= state.pass_fps as f64
        && below_pass_rate <= state.max_below_pass_rate as f64
        && !stutter_exceeded
    {
        println!(
            "{}",
            format!(
                "RESULT: PERFECT ✨ (avg {:.1} FPS >= {:.0}, stutter {:.1}%)",
                avg_fps,
                state.pass_fps,
                stutter_rate * 100.0,
            )
            .green()
            .bold()
        );
        status = "perfect";
        exit.write(AppExit::Success);
    } else {
        // GREAT tier: stable 60+ FPS, not yet perfect
        let mut notes = Vec::new();
        if avg_fps < state.pass_fps as f64 {
            notes.push(format!(
                "avg {:.1} FPS (target {:.0})",
                avg_fps, state.pass_fps
            ));
        }
        if below_pass_rate > state.max_below_pass_rate as f64 {
            notes.push(format!(
                "{:.1}% below {:.0} FPS",
                below_pass_rate * 100.0,
                state.pass_fps,
            ));
        }
        if stutter_exceeded {
            notes.push(format!("stutter {:.1}%", stutter_rate * 100.0,));
        }
        println!(
            "{}",
            format!(
                "RESULT: GREAT ✅ (avg {:.1} FPS >= {:.0}; {})",
                avg_fps,
                state.fail_fps,
                notes.join("; ")
            )
            .yellow()
            .bold()
        );
        status = "great";
        exit.write(AppExit::Success);
    }
    println!("========================================");

    // Emit machine-readable JSON line for script parsing
    let spikes_json: Vec<String> = spike_indices
        .iter()
        .map(|(idx, ms)| format!("{{\"frame\":{},\"ms\":{:.2}}}", idx, ms))
        .collect();
    println!(
        "[FRAME-TEST-JSON] {{\"project\":\"{}\",\"status\":\"{}\",\"total_frames\":{},\
         \"avg_fps\":{:.1},\"min_fps\":{:.1},\"max_fps\":{:.1},\"p95_fps\":{:.1},\
         \"p99_fps\":{:.1},\"max_frame_time_ms\":{:.2},\"median_frame_time_ms\":{:.2},\
         \"stutter_count\":{},\"stutter_rate\":{:.4},\"below_fail_count\":{},\
         \"below_fail_rate\":{:.4},\"mode\":\"{}\",\"top_spikes\":[{}]}}",
        state.project_name,
        status,
        total_frames,
        avg_fps,
        min_fps,
        max_fps,
        p95_fps,
        p99_fps,
        max_frame_time_ms,
        median_dt * 1000.0,
        stutter_count,
        stutter_rate,
        below_fail,
        below_fail_rate,
        if state.play_once { "play_once" } else { "loop" },
        spikes_json.join(","),
    );
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

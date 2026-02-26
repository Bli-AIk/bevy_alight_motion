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
    let config_content =
        std::fs::read_to_string("crates/bevy_alight_motion/comparison_config.toml")
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
        project_name,
        state.pass_fps,
        state.fail_fps,
        state.warmup_frames,
        state.measure_duration_secs
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
    let max_dt = state.frame_times.iter().copied().fold(0.0_f64, f64::max);
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
        println!("{}", format!("RESULT: FAIL ❌ ({})", reason).red().bold());
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

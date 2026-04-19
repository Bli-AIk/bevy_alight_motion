//! Shared helper functions for the example player's video workflows.
//! 为示例播放器的视频流程提供共享辅助函数。
//!
//! The example modes that debug or compare against reference video all need the same filesystem and
//! ffmpeg-facing utilities: locating candidate videos, extracting frames, and managing temporary
//! output. This file centralizes those helpers so the example systems stay focused on orchestration.
//! 示例里的视频调试和视频对比模式都依赖同一批文件系统与 ffmpeg 相关工具：查找候选视频、抽帧以及管理
//! 临时输出目录。这个文件把这些辅助统一起来，让示例系统只关注流程编排本身。
#![allow(dead_code, unused_imports)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Find video file for debug/comparison
/// First try to find a video with the same name as the project, then fall back to latest
pub fn find_debug_video(project_path: Option<&str>) -> Option<PathBuf> {
    use std::time::SystemTime;

    // First, try to find a video matching the project path (same directory, .mp4 extension)
    if let Some(path) = project_path {
        // project_path is like "projects/basic/shape/shape.amproj"
        // video should be at "projects/basic/shape/shape.mp4"
        let base_path = Path::new(path).with_extension("mp4");

        // Try both possible asset root directories
        let possible_roots = ["crates/bevy_alight_motion/assets", "assets"];
        for root in &possible_roots {
            let video_file = Path::new(root).join(&base_path);
            if video_file.exists() {
                println!("[VIDEO UTILS] Found matching video: {:?}", video_file);
                return Some(video_file);
            }
        }

        println!(
            "[VIDEO UTILS] No matching video for '{}', falling back to latest",
            path
        );
    }

    // Fall back to finding the latest video file in the projects directory
    let mut latest_file: Option<(PathBuf, SystemTime)> = None;

    let possible_paths = [
        "crates/bevy_alight_motion/assets/projects",
        "assets/projects",
    ];
    let extensions = ["mp4", "mov", "avi", "webm", "mkv"];

    fn search_videos(dir: &Path, extensions: &[&str], latest: &mut Option<(PathBuf, SystemTime)>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                search_videos(&entry.path(), extensions, latest);
                continue;
            }
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
            if latest.is_none() || latest.as_ref().unwrap().1 < modified {
                *latest = Some((entry.path(), modified));
            }
        }
    }

    for projects_path in &possible_paths {
        let base_path = Path::new(projects_path);
        if !base_path.exists() {
            continue;
        }

        search_videos(base_path, &extensions, &mut latest_file);
        if latest_file.is_some() {
            break;
        }
    }

    latest_file.map(|(path, _)| path)
}

/// Get video info using ffprobe.
/// Returns `(fps, duration)`. When `r_frame_rate` is invalid (e.g. `1/0`),
/// tries `avg_frame_rate`, then falls back to `project_fps` if supplied,
/// and finally to 30.0.
pub fn get_video_info(video_path: &PathBuf) -> Option<(f32, f32)> {
    // Try r_frame_rate first, then avg_frame_rate
    let fps = try_ffprobe_fps(video_path, "r_frame_rate")
        .or_else(|| try_ffprobe_fps(video_path, "avg_frame_rate"));

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

    // If we couldn't get fps from ffprobe, try extracting from the matching .amproj
    let fps = fps.unwrap_or_else(|| {
        let project_fps = extract_fps_from_amproj(video_path);
        if let Some(pf) = project_fps {
            println!(
                "[VIDEO UTILS] Using project fps ({}) for {:?} (video fps unavailable)",
                pf, video_path
            );
            pf
        } else {
            println!(
                "[VIDEO UTILS] WARNING: Could not determine fps for {:?}, falling back to 30.0",
                video_path
            );
            30.0
        }
    });

    Some((fps, duration))
}

/// Try to get fps from ffprobe using a specific stream field.
fn try_ffprobe_fps(video_path: &PathBuf, field: &str) -> Option<f32> {
    let entry = format!("stream={}", field);
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            &entry,
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(video_path)
        .output()
        .ok()?;

    let fps_str = String::from_utf8_lossy(&output.stdout);
    parse_fps(fps_str.trim())
}

/// Extract fps from the .amproj project file corresponding to the given video path.
/// Looks for a file with the same base name but `.amproj` extension.
fn extract_fps_from_amproj(video_path: &Path) -> Option<f32> {
    let amproj_path = video_path.with_extension("amproj");
    if !amproj_path.exists() {
        return None;
    }

    let file = std::fs::File::open(&amproj_path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;

    // Find the XML file in the archive
    let xml_name = (0..archive.len()).find_map(|i| {
        let entry = archive.by_index(i).ok()?;
        let name = entry.name().to_string();
        if name.ends_with(".xml") {
            Some(name)
        } else {
            None
        }
    })?;

    let mut xml_file = archive.by_name(&xml_name).ok()?;
    let mut xml_content = String::new();
    std::io::Read::read_to_string(&mut xml_file, &mut xml_content).ok()?;

    // Extract fps from the first scene element: fps="60"
    for line in xml_content.lines() {
        if let Some(pos) = line.find("fps=\"") {
            let rest = &line[pos + 5..];
            if let Some(end) = rest.find('"') {
                return rest[..end].parse().ok();
            }
        }
    }

    None
}

/// Get video resolution (width, height) from video file using ffprobe
pub fn get_video_resolution(video_path: &PathBuf) -> Option<(u32, u32)> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0:s=x",
        ])
        .arg(video_path)
        .output()
        .ok()?;

    let resolution_str = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = resolution_str.trim().split('x').collect();
    if parts.len() == 2 {
        let width: u32 = parts[0].parse().ok()?;
        let height: u32 = parts[1].parse().ok()?;
        return Some((width, height));
    }
    None
}

/// Parse FPS from ffprobe output
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
/// Returns the directory where frames are stored
/// Each video gets its own subdirectory based on the video filename to allow parallel extraction
pub fn extract_frames(video_path: &PathBuf, fps: f32) -> Option<PathBuf> {
    // Get video name for unique subdirectory
    let video_name = video_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    // Create frames directory inside assets/debug, with unique subdirectory per video
    let possible_assets_dirs = [
        "crates/bevy_alight_motion/assets/debug/_video_frames",
        "assets/debug/_video_frames",
    ];

    let mut base_frames_dir = None;
    for dir_path in &possible_assets_dirs {
        let parent = Path::new(dir_path).parent()?;
        if parent.exists() {
            base_frames_dir = Some(PathBuf::from(dir_path));
            break;
        }
    }

    // If no existing parent dir found (e.g. running from wrong CWD), try to create one relative to video
    if base_frames_dir.is_none()
        && let Some(parent) = video_path.parent()
    {
        base_frames_dir = Some(parent.join("_video_frames"));
    }

    let base_frames_dir = base_frames_dir?;
    // Use video-specific subdirectory to avoid race conditions in parallel tests
    let frames_dir = base_frames_dir.join(video_name);

    // Clean up existing frames for this video
    if frames_dir.exists() {
        let _ = fs::remove_dir_all(&frames_dir);
    }
    fs::create_dir_all(&frames_dir).ok()?;

    println!("[VIDEO UTILS] Extracting frames to {:?}", frames_dir);

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
                println!("[VIDEO UTILS] ffmpeg error: {}", stderr);
                return None;
            }

            // Check if any frames were extracted
            let frame_count = fs::read_dir(&frames_dir)
                .ok()
                .map(|entries| {
                    entries
                        .flatten()
                        .filter(|e| {
                            e.path()
                                .extension()
                                .map(|ext| ext == "png")
                                .unwrap_or(false)
                        })
                        .count()
                })
                .unwrap_or(0);

            // For ultra-short videos (0 duration), fps-based extraction may yield 0 frames.
            // Fall back to extracting all frames without fps filter.
            if frame_count == 0 {
                println!(
                    "[VIDEO UTILS] fps-based extraction yielded 0 frames, trying raw extraction..."
                );
                let fallback = Command::new("ffmpeg")
                    .args(["-i"])
                    .arg(video_path)
                    .args(["-y"])
                    .arg(&output_pattern)
                    .output();

                if let Ok(fb_output) = fallback
                    && !fb_output.status.success()
                {
                    let stderr = String::from_utf8_lossy(&fb_output.stderr);
                    println!("[VIDEO UTILS] ffmpeg fallback error: {}", stderr);
                    return None;
                }
            }

            Some(frames_dir)
        }
        Err(e) => {
            println!("[VIDEO UTILS] Failed to run ffmpeg: {:?}", e);
            None
        }
    }
}

#[cfg(feature = "video-comparison")]
pub use bevy_alight_motion::image_comparison::{ImageComparisonResult, compare_images};

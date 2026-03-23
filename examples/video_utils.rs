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

/// Result of an image comparison operation
#[cfg(feature = "video-comparison")]
#[derive(Debug, Clone, Copy)]
pub struct ComparisonResult {
    /// Similarity over the entire image area (0.0 - 1.0)
    pub global_similarity: f32,
    /// Similarity over only the non-empty content area (0.0 - 1.0)
    /// This metric ignores pixels that are background/empty in both images.
    /// Used to prevent empty space from diluting errors in small objects.
    pub content_similarity: f32,
    /// Percentage of pixels that are considered a "match" (diff < threshold)
    pub pixel_match_rate: f32,
    /// Count of pixels that differ
    pub differing_pixels: u64,
}

/// Check if a pixel is "empty" (transparent or black).
#[cfg(feature = "video-comparison")]
fn is_pixel_empty(p: &image::Rgba<u8>) -> bool {
    // Encoded black backgrounds often come back as (1,1,1) instead of exact zero.
    p[3] == 0 || (p[0] <= 1 && p[1] <= 1 && p[2] <= 1)
}

/// Check if a pixel lies on the edge of content within a 3-pixel Manhattan radius.
#[cfg(feature = "video-comparison")]
fn is_edge_pixel(img: &image::RgbaImage, x: u32, y: u32) -> bool {
    if is_pixel_empty(img.get_pixel(x, y)) {
        return false;
    }
    let w = img.width();
    let h = img.height();
    for dx in -3i32..=3 {
        for dy in -3i32..=3 {
            if dx == 0 && dy == 0 || dx.abs() + dy.abs() > 3 {
                continue;
            }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx >= 0
                && nx < w as i32
                && ny >= 0
                && ny < h as i32
                && is_pixel_empty(img.get_pixel(nx as u32, ny as u32))
            {
                return true;
            }
        }
    }
    false
}

/// Compression tolerance threshold for a given perceptual luminance.
#[cfg(feature = "video-comparison")]
fn compression_tolerance(max_lum: f64) -> f64 {
    const BASE: f64 = 10.0;
    const DARK_LUM_CUTOFF: f64 = 40.0;
    const MAX_DARK_BONUS: f64 = 60.0;
    let dark_bonus = if max_lum < DARK_LUM_CUTOFF {
        MAX_DARK_BONUS * (1.0 - max_lum / DARK_LUM_CUTOFF)
    } else {
        0.0
    };
    BASE + dark_bonus
}

/// Build the diff-image pixel for a given raw per-channel difference.
#[cfg(feature = "video-comparison")]
fn diff_pixel(pixel_diff: u64, is_edge: bool) -> image::Rgba<u8> {
    let intensity = (pixel_diff.min(255) as u8).max(50);
    if is_edge {
        image::Rgba([intensity / 2, 0, 0, 128])
    } else {
        image::Rgba([intensity, 0, 0, 255])
    }
}

/// Compare two images and return detailed similarity metrics and a diff image.
///
/// `img1` = rendered shot (sRGB), `img2` = reference from video.
///
/// **Dark-pixel compression tolerance**: H.264/H.265 quantization is very
/// aggressive at low luminance (especially for blue, which has only 7.2%
/// weight in the BT.709 luma formula, and chroma is subsampled 4:2:0).
/// A per-pixel tolerance proportional to darkness is subtracted from the
/// diff before accumulation, preventing compression artifacts in dark areas
/// from penalising the content similarity score.
#[cfg(feature = "video-comparison")]
pub fn compare_images(
    img1: &image::RgbaImage,
    img2: &image::RgbaImage,
) -> (ComparisonResult, image::RgbaImage) {
    let width = img1.width().min(img2.width());
    let height = img1.height().min(img2.height());

    let mut diff_image = image::RgbaImage::new(width, height);

    let mut total_diff_global: u64 = 0;
    let mut total_max_global: u64 = 0;

    // Use f64 accumulators for content similarity (tolerance is fractional)
    let mut total_diff_content: f64 = 0.0;
    let mut total_max_content: f64 = 0.0;

    let mut matching_pixels: u64 = 0;
    let mut differing_pixels: u64 = 0;

    // Threshold for considering a pixel a "match" (out of 255*4 = 1020)
    const MATCH_THRESHOLD: u64 = 10;

    for y in 0..height {
        for x in 0..width {
            let p1 = img1.get_pixel(x, y);
            let p2 = img2.get_pixel(x, y);

            let r_diff = (p1[0] as i32 - p2[0] as i32).unsigned_abs() as u64;
            let g_diff = (p1[1] as i32 - p2[1] as i32).unsigned_abs() as u64;
            let b_diff = (p1[2] as i32 - p2[2] as i32).unsigned_abs() as u64;
            let a_diff = (p1[3] as i32 - p2[3] as i32).unsigned_abs() as u64;

            let pixel_diff = r_diff + g_diff + b_diff + a_diff;

            // Update global stats
            total_diff_global += pixel_diff;
            total_max_global += 255 * 4;

            let is_edge = is_edge_pixel(img1, x, y) || is_edge_pixel(img2, x, y);
            let is_content = !is_pixel_empty(p1) || !is_pixel_empty(p2);

            // Content similarity with dark-pixel compression tolerance
            if is_content && !is_edge {
                // Perceptual luminance (BT.709/sRGB coefficients)
                let lum1 = 0.2126 * p1[0] as f64 + 0.7152 * p1[1] as f64 + 0.0722 * p1[2] as f64;
                let lum2 = 0.2126 * p2[0] as f64 + 0.7152 * p2[1] as f64 + 0.0722 * p2[2] as f64;
                let max_lum = lum1.max(lum2);

                let effective_diff = (pixel_diff as f64 - compression_tolerance(max_lum)).max(0.0);
                total_diff_content += effective_diff;
                total_max_content += 1020.0;
            }

            // Match rate (still count all pixels)
            if pixel_diff <= MATCH_THRESHOLD {
                matching_pixels += 1;
            } else {
                differing_pixels += 1;
            }

            // Generate diff pixel (emphasize difference)
            if pixel_diff > MATCH_THRESHOLD {
                diff_image.put_pixel(x, y, diff_pixel(pixel_diff, is_edge));
            } else {
                diff_image.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
            }
        }
    }

    let global_similarity = if total_max_global > 0 {
        1.0 - (total_diff_global as f32 / total_max_global as f32)
    } else {
        1.0
    };

    let content_similarity = if total_max_content > 0.0 {
        1.0 - (total_diff_content as f32 / total_max_content as f32)
    } else {
        1.0
    };

    let pixel_match_rate = matching_pixels as f32 / (width * height) as f32;

    (
        ComparisonResult {
            global_similarity,
            content_similarity,
            pixel_match_rate,
            differing_pixels,
        },
        diff_image,
    )
}

#![allow(
    dead_code,
    clippy::collapsible_if,
    clippy::cast_abs_to_unsigned,
    unused_imports
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Find video file for debug/comparison
/// First try to find a video with the same name as the project, then fall back to latest
pub fn find_debug_video(project_path: Option<&str>) -> Option<PathBuf> {
    use std::time::SystemTime;

    let possible_paths = ["crates/bevy_alight_motion/assets/debug", "assets/debug"];
    let extensions = ["mp4", "mov", "avi", "webm", "mkv"];

    // First, try to find a video matching the project name
    if let Some(path) = project_path {
        // Extract just the filename without directory and extension
        let base_name = Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(path);

        println!(
            "[VIDEO UTILS] Looking for video matching project: {}",
            base_name
        );

        for debug_path in &possible_paths {
            let path = Path::new(debug_path);
            if !path.exists() {
                continue;
            }

            for ext in &extensions {
                let video_file = path.join(format!("{}.{}", base_name, ext));
                if video_file.exists() {
                    println!("[VIDEO UTILS] Found matching video: {:?}", video_file);
                    return Some(video_file);
                }
            }
        }
        println!(
            "[VIDEO UTILS] No matching video for '{}', falling back to latest",
            base_name
        );
    }

    // Fall back to finding the latest video file
    let mut latest_file: Option<(PathBuf, SystemTime)> = None;

    for debug_path in &possible_paths {
        let path = Path::new(debug_path);
        if !path.exists() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type()
                    && file_type.is_file()
                    && let Some(file_name) = entry.file_name().to_str()
                    && let Some(extension) = file_name.split('.').next_back()
                    && extensions.contains(&extension.to_lowercase().as_str())
                    && let Ok(metadata) = entry.metadata()
                    && let Ok(modified) = metadata.modified()
                    && (latest_file.is_none() || latest_file.as_ref().unwrap().1 < modified)
                {
                    latest_file = Some((entry.path(), modified));
                }
            }
            if latest_file.is_some() {
                break;
            }
        }
    }

    latest_file.map(|(path, _)| path)
}

/// Get video info using ffprobe
pub fn get_video_info(video_path: &PathBuf) -> Option<(f32, f32)> {
    // Get frame rate
    let fps_output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=r_frame_rate",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(video_path)
        .output()
        .ok()?;

    let fps_str = String::from_utf8_lossy(&fps_output.stdout);
    let fps = parse_fps(fps_str.trim()).unwrap_or(12.0);

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

    Some((fps, duration))
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

/// Compare two images and return detailed similarity metrics and a diff image.
#[cfg(feature = "video-comparison")]
pub fn compare_images(
    img1: &image::RgbaImage,
    img2: &image::RgbaImage,
) -> (ComparisonResult, image::RgbaImage) {
    use image::Pixel;

    let width = img1.width().min(img2.width());
    let height = img1.height().min(img2.height());

    let mut diff_image = image::RgbaImage::new(width, height);

    let mut total_diff_global: u64 = 0;
    let mut total_max_global: u64 = 0;

    let mut total_diff_content: u64 = 0;
    let mut total_max_content: u64 = 0;

    let mut matching_pixels: u64 = 0;
    let mut differing_pixels: u64 = 0;

    // Threshold for considering a pixel a "match" (out of 255*4 = 1020)
    // Small noise tolerance (e.g., compression artifacts)
    const MATCH_THRESHOLD: u64 = 10;

    // Helper function to check if a pixel is "empty" (transparent or black)
    let is_empty =
        |p: &image::Rgba<u8>| -> bool { p[3] == 0 || (p[0] == 0 && p[1] == 0 && p[2] == 0) };

    // Helper function to check if a pixel is on the edge of content
    // (has at least one empty neighbor within 2-pixel radius)
    // Extended radius helps handle small position offsets between images
    let is_edge_pixel = |img: &image::RgbaImage, x: u32, y: u32| -> bool {
        let p = img.get_pixel(x, y);
        if is_empty(p) {
            return false; // Empty pixels are not edges
        }
        // Check neighbors within 2-pixel radius (Manhattan distance)
        let w = img.width();
        let h = img.height();
        for dx in -2i32..=2 {
            for dy in -2i32..=2 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                // Only check within Manhattan distance 2
                if dx.abs() + dy.abs() > 2 {
                    continue;
                }
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0 && nx < w as i32 && ny >= 0 && ny < h as i32 {
                    if is_empty(img.get_pixel(nx as u32, ny as u32)) {
                        return true;
                    }
                }
            }
        }
        false
    };

    for y in 0..height {
        for x in 0..width {
            let p1 = img1.get_pixel(x, y);
            let p2 = img2.get_pixel(x, y);

            let r_diff = (p1[0] as i32 - p2[0] as i32).abs() as u64;
            let g_diff = (p1[1] as i32 - p2[1] as i32).abs() as u64;
            let b_diff = (p1[2] as i32 - p2[2] as i32).abs() as u64;
            let a_diff = (p1[3] as i32 - p2[3] as i32).abs() as u64;

            let pixel_diff = r_diff + g_diff + b_diff + a_diff;

            // Update global stats
            total_diff_global += pixel_diff;
            total_max_global += 255 * 4;

            // Check if pixel is on edge in either image (ignore edge pixels for content comparison)
            let p1_edge = is_edge_pixel(img1, x, y);
            let p2_edge = is_edge_pixel(img2, x, y);
            let is_edge = p1_edge || p2_edge;

            // Check if pixel is "empty"
            let p1_empty = is_empty(p1);
            let p2_empty = is_empty(p2);

            let is_content = !p1_empty || !p2_empty;

            // Only count non-edge content pixels for content similarity
            if is_content && !is_edge {
                total_diff_content += pixel_diff;
                total_max_content += 255 * 4;
            }

            // Match rate (still count all pixels)
            if pixel_diff <= MATCH_THRESHOLD {
                matching_pixels += 1;
            } else {
                differing_pixels += 1;
            }

            // Generate diff pixel (emphasize difference)
            if pixel_diff > MATCH_THRESHOLD {
                // Red scale based on diff (lighter for edge pixels)
                let intensity = (pixel_diff.min(255) as u8).max(50);
                if is_edge {
                    // Lighter red for edge pixels
                    diff_image.put_pixel(x, y, image::Rgba([intensity / 2, 0, 0, 128]));
                } else {
                    diff_image.put_pixel(x, y, image::Rgba([intensity, 0, 0, 255]));
                }
            } else {
                // Transparent
                diff_image.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
            }
        }
    }

    let global_similarity = if total_max_global > 0 {
        1.0 - (total_diff_global as f32 / total_max_global as f32)
    } else {
        1.0
    };

    let content_similarity = if total_max_content > 0 {
        1.0 - (total_diff_content as f32 / total_max_content as f32)
    } else {
        // If both images are completely empty, they are identical
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

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
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        if let Some(file_name) = entry.file_name().to_str() {
                            if let Some(extension) = file_name.split('.').next_back() {
                                if extensions.contains(&extension.to_lowercase().as_str()) {
                                    if let Ok(metadata) = entry.metadata() {
                                        if let Ok(modified) = metadata.modified() {
                                            if latest_file.is_none()
                                                || latest_file.as_ref().unwrap().1 < modified
                                            {
                                                latest_file = Some((entry.path(), modified));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
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
    let fps = parse_fps(&fps_str.trim()).unwrap_or(12.0);

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
pub fn extract_frames(video_path: &PathBuf, fps: f32) -> Option<PathBuf> {
    // Create frames directory inside assets/debug
    let possible_assets_dirs = [
        "crates/bevy_alight_motion/assets/debug/_video_frames",
        "assets/debug/_video_frames",
    ];

    let mut frames_dir = None;
    for dir_path in &possible_assets_dirs {
        let parent = Path::new(dir_path).parent()?;
        if parent.exists() {
            frames_dir = Some(PathBuf::from(dir_path));
            break;
        }
    }

    // If no existing parent dir found (e.g. running from wrong CWD), try to create one relative to video
    if frames_dir.is_none() {
        if let Some(parent) = video_path.parent() {
            frames_dir = Some(parent.join("_video_frames"));
        }
    }

    let frames_dir = frames_dir?;

    // Clean up existing frames
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

/// Compare two images and return a similarity score (0.0 to 1.0) and a diff image
/// Similarity 1.0 means identical.
#[cfg(feature = "video-comparison")]
pub fn compare_images(img1: &image::RgbaImage, img2: &image::RgbaImage) -> (f32, image::RgbaImage) {
    use image::Pixel;

    let width = img1.width().min(img2.width());
    let height = img1.height().min(img2.height());

    let mut diff_image = image::RgbaImage::new(width, height);
    let mut total_diff: u64 = 0;

    for y in 0..height {
        for x in 0..width {
            let p1 = img1.get_pixel(x, y);
            let p2 = img2.get_pixel(x, y);

            let r_diff = (p1[0] as i32 - p2[0] as i32).abs() as u64;
            let g_diff = (p1[1] as i32 - p2[1] as i32).abs() as u64;
            let b_diff = (p1[2] as i32 - p2[2] as i32).abs() as u64;
            let a_diff = (p1[3] as i32 - p2[3] as i32).abs() as u64; // Optionally ignore alpha if background is black

            let pixel_diff = r_diff + g_diff + b_diff + a_diff;
            total_diff += pixel_diff;

            // Generate diff pixel (emphasize difference)
            if pixel_diff > 0 {
                // Red scale based on diff
                let intensity = (pixel_diff.min(255) as u8).max(50);
                diff_image.put_pixel(x, y, image::Rgba([intensity, 0, 0, 255]));
            } else {
                // Transparent or faint copy of original
                diff_image.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
            }
        }
    }

    let max_diff = (width as u64) * (height as u64) * 255 * 4;
    let similarity = 1.0 - (total_diff as f64 / max_diff as f64) as f32;

    (similarity, diff_image)
}

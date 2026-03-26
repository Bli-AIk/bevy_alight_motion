//! Shared image-comparison utilities for render validation workflows.
//!
//! 为渲染校验流程提供共享图像对比工具。
//!
//! This module centralizes the pixel similarity and diff-image logic that was
//! originally embedded in the example-only video comparison workflow. Keeping it
//! in the library lets downstream tools reuse the exact same scoring behavior
//! instead of forking their own slightly different comparison implementations.
//!
//! 这个模块把原本只存在于示例视频比对流程里的像素相似度与 diff 图算法收敛到库内。
//! 这样下游工具可以复用同一套评分行为，而不是各自维护一份略有偏差的实现。

/// Result of an image comparison operation.
#[derive(Debug, Clone, Copy)]
pub struct ImageComparisonResult {
    /// Similarity over the entire image area (0.0 - 1.0).
    pub global_similarity: f32,
    /// Similarity over only the non-empty content area (0.0 - 1.0).
    ///
    /// This metric ignores pixels that are background/empty in both images so
    /// sparse content is not artificially boosted by large transparent regions.
    pub content_similarity: f32,
    /// Percentage of pixels that are considered a "match" (diff < threshold).
    pub pixel_match_rate: f32,
    /// Count of pixels that differ.
    pub differing_pixels: u64,
}

/// Compare two RGBA images and return similarity metrics plus a diff image.
///
/// `rendered_image` is usually the freshly rendered output, and
/// `reference_image` is the captured source frame that the renderer should
/// match.
pub fn compare_images(
    rendered_image: &image::RgbaImage,
    reference_image: &image::RgbaImage,
) -> (ImageComparisonResult, image::RgbaImage) {
    let width = rendered_image.width().min(reference_image.width());
    let height = rendered_image.height().min(reference_image.height());

    let mut diff_image = image::RgbaImage::new(width, height);

    let mut total_diff_global: u64 = 0;
    let mut total_max_global: u64 = 0;
    let mut total_diff_content: f64 = 0.0;
    let mut total_max_content: f64 = 0.0;
    let mut matching_pixels: u64 = 0;
    let mut differing_pixels: u64 = 0;

    const MATCH_THRESHOLD: u64 = 10;

    for y in 0..height {
        for x in 0..width {
            let rendered_pixel = rendered_image.get_pixel(x, y);
            let reference_pixel = reference_image.get_pixel(x, y);

            let red_diff =
                (rendered_pixel[0] as i32 - reference_pixel[0] as i32).unsigned_abs() as u64;
            let green_diff =
                (rendered_pixel[1] as i32 - reference_pixel[1] as i32).unsigned_abs() as u64;
            let blue_diff =
                (rendered_pixel[2] as i32 - reference_pixel[2] as i32).unsigned_abs() as u64;
            let alpha_diff =
                (rendered_pixel[3] as i32 - reference_pixel[3] as i32).unsigned_abs() as u64;

            let pixel_diff = red_diff + green_diff + blue_diff + alpha_diff;

            total_diff_global += pixel_diff;
            total_max_global += 255 * 4;

            let is_edge =
                is_edge_pixel(rendered_image, x, y) || is_edge_pixel(reference_image, x, y);
            let is_content = !is_pixel_empty(rendered_pixel) || !is_pixel_empty(reference_pixel);

            if is_content && !is_edge {
                let rendered_luminance = 0.2126 * rendered_pixel[0] as f64
                    + 0.7152 * rendered_pixel[1] as f64
                    + 0.0722 * rendered_pixel[2] as f64;
                let reference_luminance = 0.2126 * reference_pixel[0] as f64
                    + 0.7152 * reference_pixel[1] as f64
                    + 0.0722 * reference_pixel[2] as f64;
                let max_luminance = rendered_luminance.max(reference_luminance);

                let effective_diff =
                    (pixel_diff as f64 - compression_tolerance(max_luminance)).max(0.0);
                total_diff_content += effective_diff;
                total_max_content += 1020.0;
            }

            if pixel_diff <= MATCH_THRESHOLD {
                matching_pixels += 1;
            } else {
                differing_pixels += 1;
            }

            if pixel_diff > MATCH_THRESHOLD {
                diff_image.put_pixel(x, y, build_diff_pixel(pixel_diff, is_edge));
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
        ImageComparisonResult {
            global_similarity,
            content_similarity,
            pixel_match_rate,
            differing_pixels,
        },
        diff_image,
    )
}

fn is_pixel_empty(pixel: &image::Rgba<u8>) -> bool {
    pixel[3] == 0 || (pixel[0] <= 1 && pixel[1] <= 1 && pixel[2] <= 1)
}

fn is_edge_pixel(image: &image::RgbaImage, x: u32, y: u32) -> bool {
    if is_pixel_empty(image.get_pixel(x, y)) {
        return false;
    }

    let width = image.width();
    let height = image.height();

    for delta_x in -3i32..=3 {
        for delta_y in -3i32..=3 {
            if delta_x == 0 && delta_y == 0 || delta_x.abs() + delta_y.abs() > 3 {
                continue;
            }

            let neighbor_x = x as i32 + delta_x;
            let neighbor_y = y as i32 + delta_y;
            if neighbor_x >= 0
                && neighbor_x < width as i32
                && neighbor_y >= 0
                && neighbor_y < height as i32
                && is_pixel_empty(image.get_pixel(neighbor_x as u32, neighbor_y as u32))
            {
                return true;
            }
        }
    }

    false
}

fn compression_tolerance(max_luminance: f64) -> f64 {
    const BASE_TOLERANCE: f64 = 10.0;
    const DARK_LUMINANCE_CUTOFF: f64 = 40.0;
    const MAX_DARK_BONUS: f64 = 60.0;

    let dark_bonus = if max_luminance < DARK_LUMINANCE_CUTOFF {
        MAX_DARK_BONUS * (1.0 - max_luminance / DARK_LUMINANCE_CUTOFF)
    } else {
        0.0
    };

    BASE_TOLERANCE + dark_bonus
}

fn build_diff_pixel(pixel_diff: u64, is_edge: bool) -> image::Rgba<u8> {
    let intensity = (pixel_diff.min(255) as u8).max(50);
    if is_edge {
        image::Rgba([intensity / 2, 0, 0, 128])
    } else {
        image::Rgba([intensity, 0, 0, 255])
    }
}

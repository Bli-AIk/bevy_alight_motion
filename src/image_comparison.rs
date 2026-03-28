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
    /// F1 score over the binary non-empty content masks.
    pub content_mask_f1: f32,
    /// IoU between rendered/reference content bounding boxes.
    pub content_bbox_iou: f32,
    /// Similarity of bounding-box size, averaged over width and height.
    pub content_size_similarity: f32,
    /// Similarity of bounding-box centers after normalizing by reference size.
    pub content_center_similarity: f32,
    /// Count of pixels that differ.
    pub differing_pixels: u64,
}

#[derive(Debug, Clone, Copy)]
struct ContentBounds {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

impl ContentBounds {
    fn new(x: u32, y: u32) -> Self {
        Self {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        }
    }

    fn include(&mut self, x: u32, y: u32) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    fn width(self) -> u32 {
        self.max_x - self.min_x + 1
    }

    fn height(self) -> u32 {
        self.max_y - self.min_y + 1
    }

    fn area(self) -> u64 {
        self.width() as u64 * self.height() as u64
    }

    fn center_x(self) -> f32 {
        (self.min_x + self.max_x) as f32 * 0.5
    }

    fn center_y(self) -> f32 {
        (self.min_y + self.max_y) as f32 * 0.5
    }
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
    let dimensions_match = rendered_image.dimensions() == reference_image.dimensions();
    let raw_match = dimensions_match && rendered_image.as_raw() == reference_image.as_raw();
    let trace_raw = std::env::var_os("AM_COMPARE_RAW_TRACE").is_some();

    if trace_raw {
        let differing_bytes = if dimensions_match {
            rendered_image
                .as_raw()
                .iter()
                .zip(reference_image.as_raw().iter())
                .filter(|(left, right)| left != right)
                .count()
        } else {
            usize::MAX
        };
        println!(
            "[COMPARE RAW] size_rendered={:?} size_reference={:?} raw_match={} differing_bytes={}",
            rendered_image.dimensions(),
            reference_image.dimensions(),
            raw_match,
            differing_bytes
        );
    }

    if raw_match {
        let (width, height) = rendered_image.dimensions();
        return (
            ImageComparisonResult {
                global_similarity: 1.0,
                content_similarity: 1.0,
                pixel_match_rate: 1.0,
                content_mask_f1: 1.0,
                content_bbox_iou: 1.0,
                content_size_similarity: 1.0,
                content_center_similarity: 1.0,
                differing_pixels: 0,
            },
            image::RgbaImage::new(width, height),
        );
    }

    let width = rendered_image.width().min(reference_image.width());
    let height = rendered_image.height().min(reference_image.height());

    let mut diff_image = image::RgbaImage::new(width, height);

    let mut total_diff_global: u64 = 0;
    let mut total_max_global: u64 = 0;
    let mut total_diff_content: f64 = 0.0;
    let mut total_max_content: f64 = 0.0;
    let mut matching_pixels: u64 = 0;
    let mut differing_pixels: u64 = 0;
    let mut rendered_content_pixels: u64 = 0;
    let mut reference_content_pixels: u64 = 0;
    let mut overlapping_content_pixels: u64 = 0;
    let mut rendered_bounds: Option<ContentBounds> = None;
    let mut reference_bounds: Option<ContentBounds> = None;
    let mut differing_alpha_pixels: u64 = 0;
    let mut differing_same_alpha_pixels: u64 = 0;
    let mut differing_visible_pixels: u64 = 0;
    let mut differing_fully_transparent_pixels: u64 = 0;
    let mut diff_samples: Vec<(u32, u32, [u8; 4], [u8; 4])> = Vec::new();

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
            let rendered_is_content = !is_pixel_empty(rendered_pixel);
            let reference_is_content = !is_pixel_empty(reference_pixel);

            total_diff_global += pixel_diff;
            total_max_global += 255 * 4;

            let is_edge =
                is_edge_pixel(rendered_image, x, y) || is_edge_pixel(reference_image, x, y);
            let is_content = rendered_is_content || reference_is_content;

            if rendered_is_content {
                rendered_content_pixels += 1;
                match rendered_bounds.as_mut() {
                    Some(bounds) => bounds.include(x, y),
                    None => rendered_bounds = Some(ContentBounds::new(x, y)),
                }
            }
            if reference_is_content {
                reference_content_pixels += 1;
                match reference_bounds.as_mut() {
                    Some(bounds) => bounds.include(x, y),
                    None => reference_bounds = Some(ContentBounds::new(x, y)),
                }
            }
            if rendered_is_content && reference_is_content {
                overlapping_content_pixels += 1;
            }

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
                if alpha_diff > 0 {
                    differing_alpha_pixels += 1;
                } else {
                    differing_same_alpha_pixels += 1;
                }
                if rendered_pixel[3] == 0 && reference_pixel[3] == 0 {
                    differing_fully_transparent_pixels += 1;
                } else {
                    differing_visible_pixels += 1;
                }
                if trace_raw && diff_samples.len() < 5 {
                    diff_samples.push((x, y, rendered_pixel.0, reference_pixel.0));
                }
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
    let content_mask_f1 = compute_content_mask_f1(
        rendered_content_pixels,
        reference_content_pixels,
        overlapping_content_pixels,
    );
    let content_bbox_iou = compute_bbox_iou(rendered_bounds, reference_bounds);
    let content_size_similarity = compute_size_similarity(rendered_bounds, reference_bounds);
    let content_center_similarity = compute_center_similarity(rendered_bounds, reference_bounds);

    if trace_raw && differing_pixels > 0 {
        println!(
            "[COMPARE RAW] differing_pixels={} alpha_diff_pixels={} same_alpha_rgb_diff_pixels={} visible_diff_pixels={} transparent_diff_pixels={}",
            differing_pixels,
            differing_alpha_pixels,
            differing_same_alpha_pixels,
            differing_visible_pixels,
            differing_fully_transparent_pixels
        );
        for (x, y, rendered, reference) in diff_samples {
            println!(
                "[COMPARE RAW] sample x={} y={} rendered={:?} reference={:?}",
                x, y, rendered, reference
            );
        }
    }

    (
        ImageComparisonResult {
            global_similarity,
            content_similarity,
            pixel_match_rate,
            content_mask_f1,
            content_bbox_iou,
            content_size_similarity,
            content_center_similarity,
            differing_pixels,
        },
        diff_image,
    )
}

fn compute_content_mask_f1(
    rendered_content_pixels: u64,
    reference_content_pixels: u64,
    overlapping_content_pixels: u64,
) -> f32 {
    if rendered_content_pixels == 0 && reference_content_pixels == 0 {
        return 1.0;
    }
    if overlapping_content_pixels == 0 {
        return 0.0;
    }

    let precision = overlapping_content_pixels as f32 / rendered_content_pixels.max(1) as f32;
    let recall = overlapping_content_pixels as f32 / reference_content_pixels.max(1) as f32;
    let sum = precision + recall;
    if sum <= f32::EPSILON {
        0.0
    } else {
        2.0 * precision * recall / sum
    }
}

fn compute_bbox_iou(
    rendered_bounds: Option<ContentBounds>,
    reference_bounds: Option<ContentBounds>,
) -> f32 {
    match (rendered_bounds, reference_bounds) {
        (None, None) => 1.0,
        (Some(_), None) | (None, Some(_)) => 0.0,
        (Some(rendered), Some(reference)) => {
            let intersection_min_x = rendered.min_x.max(reference.min_x);
            let intersection_min_y = rendered.min_y.max(reference.min_y);
            let intersection_max_x = rendered.max_x.min(reference.max_x);
            let intersection_max_y = rendered.max_y.min(reference.max_y);

            let intersection_area = if intersection_min_x <= intersection_max_x
                && intersection_min_y <= intersection_max_y
            {
                (intersection_max_x - intersection_min_x + 1) as u64
                    * (intersection_max_y - intersection_min_y + 1) as u64
            } else {
                0
            };
            let union_area = rendered.area() + reference.area() - intersection_area;
            if union_area == 0 {
                1.0
            } else {
                intersection_area as f32 / union_area as f32
            }
        }
    }
}

fn compute_size_similarity(
    rendered_bounds: Option<ContentBounds>,
    reference_bounds: Option<ContentBounds>,
) -> f32 {
    match (rendered_bounds, reference_bounds) {
        (None, None) => 1.0,
        (Some(_), None) | (None, Some(_)) => 0.0,
        (Some(rendered), Some(reference)) => {
            let width_similarity = rendered.width().min(reference.width()) as f32
                / rendered.width().max(reference.width()) as f32;
            let height_similarity = rendered.height().min(reference.height()) as f32
                / rendered.height().max(reference.height()) as f32;
            (width_similarity + height_similarity) * 0.5
        }
    }
}

fn compute_center_similarity(
    rendered_bounds: Option<ContentBounds>,
    reference_bounds: Option<ContentBounds>,
) -> f32 {
    match (rendered_bounds, reference_bounds) {
        (None, None) => 1.0,
        (Some(_), None) | (None, Some(_)) => 0.0,
        (Some(rendered), Some(reference)) => {
            let dx = (rendered.center_x() - reference.center_x()).abs();
            let dy = (rendered.center_y() - reference.center_y()).abs();
            let normalized_dx = dx / reference.width().max(1) as f32;
            let normalized_dy = dy / reference.height().max(1) as f32;
            1.0 - normalized_dx.max(normalized_dy).min(1.0)
        }
    }
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

//! Loads embedded bitmap media from an Alight Motion project.
//! It detects a sensible image format for each blob, creates Bevy `Image`
//! assets, and registers both original and normalized URI labels so later scene
//! code can resolve textures without caring how the media was packaged.
//!
//! 负责加载 Alight Motion 项目里嵌入的位图媒体。它会为每个字节块判断
//! 合适的图片格式，创建 Bevy `Image` 资源，并同时注册原始 URI 与规范化标签，
//! 让后续场景代码无需关心媒体最初是如何打包的。

use std::collections::HashMap;

use bevy::asset::{LoadContext, RenderAssetUsages};
use bevy::prelude::*;

pub(super) fn load_embedded_images(
    embedded_images: &HashMap<String, Vec<u8>>,
    load_context: &mut LoadContext<'_>,
) -> HashMap<String, Handle<Image>> {
    let mut images = HashMap::new();

    for (uri, data) in embedded_images {
        let label = uri.trim_start_matches("amproj:");
        let format = detect_image_format(data, label);

        if let Ok(image) = Image::from_buffer(
            data,
            bevy::image::ImageType::Extension(format),
            bevy::image::CompressedImageFormats::NONE,
            true,
            bevy::image::ImageSampler::nearest(),
            RenderAssetUsages::all(),
        ) {
            let handle = load_context.add_labeled_asset(label.to_string(), image);
            images.insert(uri.clone(), handle.clone());
            images.insert(format!("am:{}", label), handle);
            debug!("Loaded image: {} (detected format: {})", uri, format);
        } else {
            warn!("Failed to load image: {} (tried format: {})", uri, format);
        }
    }

    images
}

fn detect_image_format<'a>(data: &[u8], label: &'a str) -> &'a str {
    if data.len() >= 8 && data[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
        "png"
    } else if data.len() >= 2 && data[0..2] == [0xFF, 0xD8] {
        "jpeg"
    } else if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        "webp"
    } else {
        match label
            .rsplit('.')
            .next()
            .unwrap_or("png")
            .to_lowercase()
            .as_str()
        {
            "jpg" | "jpeg" => "jpeg",
            "webp" => "webp",
            _ => "png",
        }
    }
}

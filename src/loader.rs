//! Asset loader for Alight Motion project files.

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext, RenderAssetUsages};
use bevy::prelude::*;
use std::collections::HashMap;
use std::io::{Cursor, Read};

use crate::error::AmError;
use crate::schema::AmScene;

/// Font metrics extracted from TTF/OTF files.
#[derive(Debug, Clone, Default)]
pub struct FontMetrics {
    /// Ascender height normalized to 1.0 = em height (from OS/2 usWinAscent).
    pub win_ascent: f32,
    /// Descender depth normalized to 1.0 = em height (from OS/2 usWinDescent).
    pub win_descent: f32,
    /// Units per em for normalization.
    pub units_per_em: u16,
}

impl FontMetrics {
    /// Calculate the vertical center offset relative to baseline.
    /// This is (win_ascent - win_descent) / 2.
    /// Note: win_descent is stored as positive value.
    pub fn win_center(&self) -> f32 {
        (self.win_ascent - self.win_descent) / 2.0
    }
}

/// Asset representing a loaded Alight Motion project.
#[derive(Asset, TypePath, Debug, Clone)]
pub struct AmProject {
    /// The parsed scene data.
    pub scene: AmScene,
    /// Mapping from amproj URIs to image handles.
    pub images: HashMap<String, Handle<Image>>,
    /// Mapping from font names to font handles.
    pub fonts: HashMap<String, Handle<Font>>,
    /// Mapping from font names to font metrics.
    pub font_metrics: HashMap<String, FontMetrics>,
    /// Raw image data for embedded images (before loading).
    pub embedded_images: HashMap<String, Vec<u8>>,
    /// Validation report about supported/unsupported features.
    pub validation_report: crate::validation::ValidationReport,
}

/// Loader for .amproj and .xml AM files.
#[derive(Default, bevy::prelude::TypePath)]
pub struct AlightMotionLoader;

impl AssetLoader for AlightMotionLoader {
    type Asset = AmProject;
    type Settings = ();
    type Error = AmError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let path = load_context.path().clone();
        let path_ref = path.path();
        let extension = path_ref
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("");

        match extension.to_lowercase().as_str() {
            "amproj" => load_amproj(&bytes, load_context).await,
            "xml" => load_xml(&bytes, load_context).await,
            _ => Err(AmError::InvalidFormat(format!(
                "Unknown file extension: {}",
                extension
            ))),
        }
    }

    fn extensions(&self) -> &[&str] {
        &["amproj", "xml"]
    }
}

/// Load from .amproj ZIP archive.
async fn load_amproj(
    bytes: &[u8],
    load_context: &mut LoadContext<'_>,
) -> Result<AmProject, AmError> {
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;

    // Find the XML file in the archive
    let mut xml_content = None;
    let mut embedded_images = HashMap::new();
    let mut embedded_fonts: HashMap<String, Vec<u8>> = HashMap::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        // Get the filename, preferring raw bytes decoded as UTF-8
        // The zip library sometimes doesn't correctly handle the UTF-8 flag
        let name = match std::str::from_utf8(file.name_raw()) {
            Ok(utf8_name) => utf8_name.to_string(),
            Err(_) => file.name().to_string(),
        };

        if name.ends_with(".xml") {
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            xml_content = Some(content);
        } else if name.ends_with(".png")
            || name.ends_with(".jpg")
            || name.ends_with(".jpeg")
            || name.ends_with(".webp")
        {
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;
            // Store with amproj: prefix for lookup
            let uri = format!("amproj:{}", name);
            bevy::log::debug!("Loaded embedded image: {}", uri);
            embedded_images.insert(uri, data);
        } else if name.ends_with(".ttf") || name.ends_with(".otf") {
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;
            // Store font by filename
            embedded_fonts.insert(name, data);
        }
    }

    let xml_content = xml_content
        .ok_or_else(|| AmError::InvalidFormat("No XML file found in amproj archive".to_string()))?;

    // Parse the XML
    let scene: AmScene = quick_xml::de::from_str(&xml_content)?;

    // Load embedded images as labeled assets
    let mut images = HashMap::new();
    for (uri, data) in &embedded_images {
        let label = uri.trim_start_matches("amproj:");

        // Detect image format from magic bytes (more reliable than extension)
        let format: &str =
            if data.len() >= 8 && data[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
                "png"
            } else if data.len() >= 2 && data[0..2] == [0xFF, 0xD8] {
                "jpeg"
            } else if data.len() >= 4
                && &data[0..4] == b"RIFF"
                && data.len() >= 12
                && &data[8..12] == b"WEBP"
            {
                "webp"
            } else {
                // Fall back to extension
                let extension = label.rsplit('.').next().unwrap_or("png").to_lowercase();
                if extension == "jpg" || extension == "jpeg" {
                    "jpeg"
                } else if extension == "webp" {
                    "webp"
                } else {
                    "png"
                }
            };

        // Try to load the image from raw bytes with detected format
        if let Ok(image) = Image::from_buffer(
            data,
            bevy::image::ImageType::Extension(format),
            bevy::image::CompressedImageFormats::NONE,
            true,
            bevy::image::ImageSampler::Default,
            RenderAssetUsages::all(),
        ) {
            let handle = load_context.add_labeled_asset(label.to_string(), image);
            images.insert(uri.clone(), handle);
            bevy::log::debug!("Loaded image: {} (detected format: {})", uri, format);
        } else {
            bevy::log::warn!("Failed to load image: {} (tried format: {})", uri, format);
        }
    }

    // Load embedded fonts as labeled assets and extract metrics
    let mut fonts = HashMap::new();
    let mut font_metrics = HashMap::new();
    for (name, data) in embedded_fonts {
        // Try loading font with fontdb first to check if it's valid
        // fontdb is what Bevy's text pipeline uses internally
        // 先用 fontdb 测试字体是否有效，fontdb 是 Bevy 文本管线内部使用的
        let mut test_db = fontdb::Database::new();
        test_db.load_font_data(data.clone());
        if test_db.faces().count() == 0 {
            bevy::log::warn!(
                "Font '{}' failed fontdb validation, skipping to avoid text pipeline panic",
                name
            );
            continue;
        }

        // Extract font metrics using ttf-parser
        if let Ok(face) = ttf_parser::Face::parse(&data, 0) {
            let upm = face.units_per_em();
            let (win_ascent, win_descent) = if let Some(os2) = face.tables().os2 {
                (
                    os2.windows_ascender() as f32 / upm as f32,
                    // windows_descender() returns negative value, we store positive for easier calculation
                    (-os2.windows_descender()) as f32 / upm as f32,
                )
            } else {
                // Fallback to hhea metrics
                (
                    face.ascender() as f32 / upm as f32,
                    (-face.descender()) as f32 / upm as f32,
                )
            };
            font_metrics.insert(
                name.clone(),
                FontMetrics {
                    win_ascent,
                    win_descent,
                    units_per_em: upm,
                },
            );
            bevy::log::trace!(
                "Font '{}' metrics: win_ascent={:.4}, win_descent={:.4}, upm={}",
                name,
                win_ascent,
                win_descent,
                upm
            );
        }

        let font = Font::try_from_bytes(data.clone()).map_err(|e| {
            AmError::InvalidFormat(format!("Failed to load font {}: {:?}", name, e))
        })?;
        let label = format!("font_{}", name);
        let handle = load_context.add_labeled_asset(label, font);
        fonts.insert(name, handle);
    }

    // Validate the scene and generate report
    let validation_report = crate::validation::ValidationReport::validate(&scene);
    #[cfg(not(target_arch = "wasm32"))]
    validation_report.log_report(&scene.title);
    #[cfg(target_arch = "wasm32")]
    validation_report.log_report_wasm(&scene.title);

    Ok(AmProject {
        scene,
        images,
        fonts,
        font_metrics,
        embedded_images,
        validation_report,
    })
}

/// Load from standalone .xml file.
async fn load_xml(bytes: &[u8], _load_context: &mut LoadContext<'_>) -> Result<AmProject, AmError> {
    let content = String::from_utf8_lossy(bytes);
    let scene: AmScene = quick_xml::de::from_str(&content)?;

    // Validate the scene and generate report
    let validation_report = crate::validation::ValidationReport::validate(&scene);
    #[cfg(not(target_arch = "wasm32"))]
    validation_report.log_report(&scene.title);
    #[cfg(target_arch = "wasm32")]
    validation_report.log_report_wasm(&scene.title);

    Ok(AmProject {
        scene,
        images: HashMap::new(),
        fonts: HashMap::new(),
        font_metrics: HashMap::new(),
        embedded_images: HashMap::new(),
        validation_report,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_parsing_from_string() {
        let xml = r##"<?xml version='1.0' encoding='UTF-8' ?>
        <scene title="Test" width="1280" height="960" fps="60" totalTime="2000" bgcolor="#ff000000">
            <shape id="123" label="Test Shape" startTime="0" endTime="1000" fillType="color" s=".rect">
                <transform>
                    <location value="640.0,480.0,0.0" />
                </transform>
                <property name="size" type="vec2" value="100.0,100.0" />
            </shape>
        </scene>
        "##;

        let scene: AmScene = quick_xml::de::from_str(xml).expect("Failed to parse XML");
        assert_eq!(scene.title, "Test");
        assert_eq!(scene.width, 1280);
        assert_eq!(scene.height, 960);
        assert_eq!(scene.fps, 60);
    }
}

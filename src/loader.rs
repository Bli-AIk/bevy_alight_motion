//! Asset loader for Alight Motion project files.

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext, RenderAssetUsages};
use bevy::prelude::*;
use std::collections::HashMap;
use std::io::{Cursor, Read};

use crate::error::AmError;
use crate::schema::AmScene;

/// Optional override configuration for amproj assets.
///
/// Placed alongside the `.amproj` file/directory as `<name>.amproj.toml`.
/// Provides manual content URI → filename mappings for cases where
/// the XML `<media>` elements lack a `filename` attribute.
#[derive(Debug, Default, serde::Deserialize)]
struct AmProjectOverride {
    /// Content URI → local filename mappings.
    /// Keys are Android content URIs (e.g. `content://media/external/images/media/1000048179`),
    /// values are filenames within the amproj directory.
    #[serde(default)]
    media: HashMap<String, String>,
}

impl AmProjectOverride {
    /// Try to load an override config from `<amproj_path>.toml`.
    fn load_for(amproj_path: &std::path::Path) -> Option<Self> {
        let toml_path = amproj_path.with_extension("amproj.toml");
        let content = std::fs::read_to_string(&toml_path).ok()?;
        match toml::from_str::<AmProjectOverride>(&content) {
            Ok(config) => {
                info!("Loaded amproj override config: {:?}", toml_path);
                Some(config)
            }
            Err(e) => {
                warn!(
                    "Failed to parse amproj override config {:?}: {}",
                    toml_path, e
                );
                None
            }
        }
    }
}

/// Font metrics extracted from TTF/OTF files.
#[derive(Debug, Clone, Default)]
pub struct FontMetrics {
    /// Ascender height normalized to 1.0 = em height (from OS/2 usWinAscent).
    pub win_ascent: f32,
    /// Descender depth normalized to 1.0 = em height (from OS/2 usWinDescent).
    pub win_descent: f32,
    /// Units per em for normalization.
    pub units_per_em: u16,
    /// hhea ascender normalized to em height (positive).
    pub hhea_ascent: f32,
    /// hhea descender normalized to em height (positive).
    pub hhea_descent: f32,
}

impl FontMetrics {
    /// Calculate the vertical center offset relative to baseline.
    /// This is (win_ascent - win_descent) / 2.
    /// Note: win_descent is stored as positive value.
    pub fn win_center(&self) -> f32 {
        (self.win_ascent - self.win_descent) / 2.0
    }

    /// Compute the line height ratio matching Android's StaticLayout float-based metrics.
    /// AM uses (descent - ascent) * spacingMult, where ascent/descent are hhea float values.
    pub fn am_line_height_ratio(&self, _font_size: f32) -> f32 {
        self.hhea_ascent + self.hhea_descent
    }

    /// Compute line height ratio adjusted for CJK fallback fonts.
    /// When text contains CJK characters, Android uses the CJK fallback font's
    /// (Noto Sans CJK, hhea ratio ≈ 1.448) line metrics which are taller than
    /// most Latin fonts. We take the max of the primary font and CJK ratio.
    pub fn am_line_height_ratio_cjk_aware(&self, _font_size: f32, text: &str) -> f32 {
        let primary = self.hhea_ascent + self.hhea_descent;
        if contains_cjk(text) {
            // Noto Sans CJK: UPM=1000, hhea_ascent=1160, hhea_descent=288
            const CJK_FALLBACK_LINE_HEIGHT_RATIO: f32 = 1.448;
            primary.max(CJK_FALLBACK_LINE_HEIGHT_RATIO)
        } else {
            primary
        }
    }

    /// Compute the Y offset to compensate for AM's StaticLayout `includePad(true)`.
    /// AM uses win metrics (usWinAscent/usWinDescent) for first/last line padding,
    /// then centers the padded box at the element position. Bevy centers based on
    /// hhea line height metrics. The height difference shifts the visual text center.
    pub fn include_pad_y_offset(&self, font_size: f32) -> f32 {
        let win_total = self.win_ascent + self.win_descent;
        let hhea_total = self.hhea_ascent + self.hhea_descent;
        -(win_total - hhea_total) * font_size
    }
}

/// Check if text contains CJK characters (U+4E00..U+9FFF, U+3400..U+4DBF, etc.)
/// Used to determine line height based on CJK fallback font metrics.
pub fn contains_cjk(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(c,
            '\u{4E00}'..='\u{9FFF}'   // CJK Unified Ideographs
            | '\u{3400}'..='\u{4DBF}' // CJK Extension A
            | '\u{3000}'..='\u{303F}' // CJK Symbols and Punctuation
            | '\u{3040}'..='\u{309F}' // Hiragana
            | '\u{30A0}'..='\u{30FF}' // Katakana
            | '\u{AC00}'..='\u{D7AF}' // Hangul Syllables
        )
    })
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
    /// Raw font data for embedded fonts (for round-trip write-back).
    pub embedded_fonts: HashMap<String, Vec<u8>>,
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
        let asset_path = load_context.path().clone();
        let path_ref = asset_path.path();
        let is_amproj = path_ref.extension().is_some_and(|ext| ext == "amproj");

        let mut bytes = Vec::new();
        match reader.read_to_end(&mut bytes).await {
            Err(e) if is_amproj && e.kind() == std::io::ErrorKind::IsADirectory => {
                let fs_path = resolve_asset_fs_path(path_ref);
                return load_amproj_dir(&fs_path, load_context).await;
            }
            Err(e) => return Err(e.into()),
            Ok(_) => {}
        }

        let extension = path_ref
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("");

        match extension.to_lowercase().as_str() {
            "amproj" => {
                if zip::ZipArchive::new(std::io::Cursor::new(&bytes)).is_err() {
                    // Not a valid ZIP — try loading as unpacked directory
                    let fs_path = resolve_asset_fs_path(path_ref);
                    if fs_path.is_dir() {
                        return load_amproj_dir(&fs_path, load_context).await;
                    }
                }
                load_amproj(&bytes, load_context).await
            }
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

/// Resolve a Bevy asset path to an absolute filesystem path.
/// Replicates Bevy's `FileAssetReader` root resolution: `CARGO_MANIFEST_DIR/assets/` or `<cwd>/assets/`.
fn resolve_asset_fs_path(asset_path: &std::path::Path) -> std::path::PathBuf {
    let base = std::env::var("CARGO_MANIFEST_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
    base.join("assets").join(asset_path)
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
            debug!("Loaded embedded image: {}", uri);
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
            images.insert(uri.clone(), handle.clone());
            // Also store with "am:" prefix so real AM exports (fillImage="am:...") work
            let am_uri = format!("am:{}", label);
            images.insert(am_uri, handle);
            debug!("Loaded image: {} (detected format: {})", uri, format);
        } else {
            warn!("Failed to load image: {} (tried format: {})", uri, format);
        }
    }

    // Load embedded fonts as labeled assets and extract metrics
    let mut fonts = HashMap::new();
    let mut font_metrics = HashMap::new();
    let mut preserved_fonts: HashMap<String, Vec<u8>> = HashMap::new();
    for (name, data) in &embedded_fonts {
        // Try loading font with fontdb first to check if it's valid
        // fontdb is what Bevy's text pipeline uses internally
        // 先用 fontdb 测试字体是否有效，fontdb 是 Bevy 文本管线内部使用的
        let mut test_db = fontdb::Database::new();
        test_db.load_font_data(data.clone());
        if test_db.faces().count() == 0 {
            warn!(
                "Font '{}' failed fontdb validation, skipping to avoid text pipeline panic",
                name
            );
            continue;
        }

        // Preserve raw font data for round-trip write-back
        preserved_fonts.insert(name.clone(), data.clone());

        // Extract font metrics using ttf-parser
        if let Ok(face) = ttf_parser::Face::parse(data, 0) {
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
            let hhea_ascent = face.ascender() as f32 / upm as f32;
            let hhea_descent = (-face.descender()) as f32 / upm as f32;
            font_metrics.insert(
                name.clone(),
                FontMetrics {
                    win_ascent,
                    win_descent,
                    units_per_em: upm,
                    hhea_ascent,
                    hhea_descent,
                },
            );
            trace!(
                "Font '{}' metrics: win_ascent={:.4}, win_descent={:.4}, hhea_ascent={:.4}, hhea_descent={:.4}, upm={}",
                name, win_ascent, win_descent, hhea_ascent, hhea_descent, upm
            );
        }

        let font = Font::try_from_bytes(data.clone()).map_err(|e| {
            AmError::InvalidFormat(format!("Failed to load font {}: {:?}", name, e))
        })?;
        let label = format!("font_{}", name);
        let handle = load_context.add_labeled_asset(label, font);
        fonts.insert(name.clone(), handle);
    }

    // Resolve Google Fonts references to system fonts
    // Parse "googlefonts?name=FontName&weight=N" and try to find matching system font
    #[cfg(not(target_arch = "wasm32"))]
    {
        let google_font_refs: Vec<String> = collect_google_font_refs(&scene.layers)
            .into_iter()
            .collect();

        for font_ref in google_font_refs {
            if fonts.contains_key(&font_ref) {
                continue;
            }
            if let Some(path) = resolve_google_font_to_system(&font_ref)
                && let Ok(data) = std::fs::read(&path)
            {
                // Validate with fontdb
                let mut test_db = fontdb::Database::new();
                test_db.load_font_data(data.clone());
                if test_db.faces().count() == 0 {
                    warn!("System font at '{}' failed fontdb validation", path);
                    continue;
                }
                // Extract metrics
                if let Ok(face) = ttf_parser::Face::parse(&data, 0) {
                    let upm = face.units_per_em();
                    let (win_ascent, win_descent) = if let Some(os2) = face.tables().os2 {
                        (
                            os2.windows_ascender() as f32 / upm as f32,
                            (-os2.windows_descender()) as f32 / upm as f32,
                        )
                    } else {
                        (
                            face.ascender() as f32 / upm as f32,
                            (-face.descender()) as f32 / upm as f32,
                        )
                    };
                    let hhea_ascent = face.ascender() as f32 / upm as f32;
                    let hhea_descent = (-face.descender()) as f32 / upm as f32;
                    font_metrics.insert(
                        font_ref.clone(),
                        FontMetrics {
                            win_ascent,
                            win_descent,
                            units_per_em: upm,
                            hhea_ascent,
                            hhea_descent,
                        },
                    );
                }
                match Font::try_from_bytes(data) {
                    Ok(font) => {
                        let label = format!("font_system_{}", font_ref);
                        let handle = load_context.add_labeled_asset(label, font);
                        fonts.insert(font_ref.clone(), handle);
                        info!("Resolved '{}' to system font: {}", font_ref, path);
                    }
                    Err(e) => {
                        warn!("Failed to load system font '{}': {:?}", path, e);
                    }
                }
            }
        }
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
        embedded_fonts: preserved_fonts,
        validation_report,
    })
}

/// Load from .amproj directory (unpacked format).
async fn load_amproj_dir(
    dir_path: &std::path::Path,
    load_context: &mut LoadContext<'_>,
) -> Result<AmProject, AmError> {
    let mut xml_content = None;
    let mut embedded_images: HashMap<String, Vec<u8>> = HashMap::new();
    let mut embedded_fonts: HashMap<String, Vec<u8>> = HashMap::new();

    let entries = std::fs::read_dir(dir_path)
        .map_err(|e| AmError::InvalidFormat(format!("Failed to read amproj directory: {}", e)))?;

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if name.ends_with(".xml") {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| AmError::InvalidFormat(format!("Failed to read XML file: {}", e)))?;
            xml_content = Some(content);
        } else if name.ends_with(".png")
            || name.ends_with(".jpg")
            || name.ends_with(".jpeg")
            || name.ends_with(".webp")
        {
            let data = std::fs::read(&path)
                .map_err(|e| AmError::InvalidFormat(format!("Failed to read image file: {}", e)))?;
            let uri = format!("amproj:{}", name);
            debug!("Loaded image from directory: {}", uri);
            embedded_images.insert(uri, data);
        } else if name.ends_with(".ttf") || name.ends_with(".otf") {
            let data = std::fs::read(&path)
                .map_err(|e| AmError::InvalidFormat(format!("Failed to read font file: {}", e)))?;
            embedded_fonts.insert(name.to_string(), data);
        }
    }

    let xml_content = xml_content.ok_or_else(|| {
        AmError::InvalidFormat("No XML file found in amproj directory".to_string())
    })?;

    let scene: AmScene = quick_xml::de::from_str(&xml_content)?;

    let mut images = HashMap::new();
    for (uri, data) in &embedded_images {
        let label = uri.trim_start_matches("amproj:");

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
                let extension = label.rsplit('.').next().unwrap_or("png").to_lowercase();
                if extension == "jpg" || extension == "jpeg" {
                    "jpeg"
                } else if extension == "webp" {
                    "webp"
                } else {
                    "png"
                }
            };

        if let Ok(image) = Image::from_buffer(
            data,
            bevy::image::ImageType::Extension(format),
            bevy::image::CompressedImageFormats::NONE,
            true,
            bevy::image::ImageSampler::Default,
            RenderAssetUsages::all(),
        ) {
            let handle = load_context.add_labeled_asset(label.to_string(), image);
            images.insert(uri.clone(), handle.clone());
            let am_uri = format!("am:{}", label);
            images.insert(am_uri, handle);
            debug!(
                "Loaded image from directory: {} (detected format: {})",
                uri, format
            );
        } else {
            warn!(
                "Failed to load image from directory: {} (tried format: {})",
                uri, format
            );
        }
    }

    let mut fonts = HashMap::new();
    let mut font_metrics = HashMap::new();
    let mut preserved_fonts: HashMap<String, Vec<u8>> = HashMap::new();
    for (name, data) in &embedded_fonts {
        let mut test_db = fontdb::Database::new();
        test_db.load_font_data(data.clone());
        if test_db.faces().count() == 0 {
            warn!(
                "Font '{}' failed fontdb validation, skipping to avoid text pipeline panic",
                name
            );
            continue;
        }

        preserved_fonts.insert(name.clone(), data.clone());

        if let Ok(face) = ttf_parser::Face::parse(data, 0) {
            let upm = face.units_per_em();
            let (win_ascent, win_descent) = if let Some(os2) = face.tables().os2 {
                (
                    os2.windows_ascender() as f32 / upm as f32,
                    (-os2.windows_descender()) as f32 / upm as f32,
                )
            } else {
                (
                    face.ascender() as f32 / upm as f32,
                    (-face.descender()) as f32 / upm as f32,
                )
            };
            let hhea_ascent = face.ascender() as f32 / upm as f32;
            let hhea_descent = (-face.descender()) as f32 / upm as f32;
            font_metrics.insert(
                name.clone(),
                FontMetrics {
                    win_ascent,
                    win_descent,
                    units_per_em: upm,
                    hhea_ascent,
                    hhea_descent,
                },
            );
        }

        let font = Font::try_from_bytes(data.clone()).map_err(|e| {
            AmError::InvalidFormat(format!("Failed to load font {}: {:?}", name, e))
        })?;
        let label = format!("font_{}", name);
        let handle = load_context.add_labeled_asset(label, font);
        fonts.insert(name.clone(), handle);
    }

    let validation_report = crate::validation::ValidationReport::validate(&scene);
    #[cfg(not(target_arch = "wasm32"))]
    validation_report.log_report(&scene.title);
    #[cfg(target_arch = "wasm32")]
    validation_report.log_report_wasm(&scene.title);

    info!("Loaded amproj directory: {:?}", dir_path);

    // Map content provider URIs (content://...) to loaded image handles using <media> elements.
    // Device-extracted directories use content URIs in shapes' fillImage attributes,
    // while images are stored with amproj:filename keys.
    for media in &scene.media {
        if !media.uri.is_empty() && !media.filename.is_empty() {
            let amproj_key = format!("amproj:{}", media.filename);
            if let Some(handle) = images.get(&amproj_key).cloned() {
                images.insert(media.uri.clone(), handle);
            }
        }
    }

    // Apply override config for content URIs that lack automatic filename mappings.
    if let Some(overrides) = AmProjectOverride::load_for(dir_path) {
        for (content_uri, filename) in &overrides.media {
            let amproj_key = format!("amproj:{}", filename);
            if let Some(handle) = images.get(&amproj_key).cloned() {
                images.insert(content_uri.clone(), handle);
                debug!("Override mapped {} -> {}", content_uri, filename);
            } else {
                warn!(
                    "Override config references '{}' but no image '{}' found in amproj directory",
                    content_uri, filename
                );
            }
        }
    }

    Ok(AmProject {
        scene,
        images,
        fonts,
        font_metrics,
        embedded_images,
        embedded_fonts: preserved_fonts,
        validation_report,
    })
}

/// Collect unique Google Fonts references from all text layers (including nested scenes).
#[cfg(not(target_arch = "wasm32"))]
fn collect_google_font_refs(
    layers: &[crate::schema::AmLayer],
) -> std::collections::HashSet<String> {
    use crate::schema::AmLayer;
    let mut refs = std::collections::HashSet::new();
    for layer in layers {
        match layer {
            AmLayer::Text(t) if t.font.starts_with("googlefonts?") => {
                refs.insert(t.font.clone());
            }
            AmLayer::EmbedScene(e) => {
                refs.extend(collect_google_font_refs(&e.scene.layers));
            }
            _ => {}
        }
    }
    refs
}

/// Resolve a "googlefonts?name=FontName&weight=N" reference to a system font path.
#[cfg(not(target_arch = "wasm32"))]
fn resolve_google_font_to_system(font_ref: &str) -> Option<String> {
    let query = font_ref.strip_prefix("googlefonts?")?;
    let mut name = None;
    let mut weight = 400u16;
    for param in query.split('&') {
        if let Some(val) = param.strip_prefix("name=") {
            name = Some(val);
        } else if let Some(val) = param.strip_prefix("weight=") {
            weight = val.parse().unwrap_or(400);
        }
    }
    let font_name = name?;

    // Map weight to font suffix
    let suffix = match weight {
        100 => "Thin",
        200 => "ExtraLight",
        300 => "Light",
        400 => "Regular",
        500 => "Medium",
        600 => "SemiBold",
        700 => "Bold",
        800 => "ExtraBold",
        900 => "Black",
        _ => "Regular",
    };

    // Try common system font paths
    let candidates = [
        format!("/usr/share/fonts/TTF/{}-{}.ttf", font_name, suffix),
        format!(
            "/usr/share/fonts/truetype/{}/{}-{}.ttf",
            font_name.to_lowercase(),
            font_name,
            suffix
        ),
        format!(
            "/usr/share/fonts/google-{}/{}-{}.ttf",
            font_name.to_lowercase(),
            font_name,
            suffix
        ),
        format!("/usr/share/fonts/TTF/{}.ttf", font_name),
    ];

    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return Some(path.clone());
        }
    }

    // Try fc-match as last resort
    if let Ok(output) = std::process::Command::new("fc-match")
        .args(["-f", "%{file}", &format!("{}:weight={}", font_name, weight)])
        .output()
        && output.status.success()
    {
        let path = String::from_utf8_lossy(&output.stdout).to_string();
        if std::path::Path::new(&path).exists() && path.contains(font_name) {
            return Some(path);
        }
    }

    None
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
        embedded_fonts: HashMap::new(),
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

//! Acts as the main asset-loading pipeline for Alight Motion projects.
//! It reads zipped `.amproj` archives or directory exports, parses the XML scene,
//! gathers embedded media, applies override files, and builds the fully resolved
//! `AmProject` asset consumed by the runtime.
//!
//! Alight Motion 项目的主资源加载管线。它负责读取压缩 `.amproj`
//! 归档或目录形式的导出结果，解析 XML 场景、收集嵌入媒体、应用覆盖配置，并最终
//! 构建运行时要消费的完整 `AmProject` 资源。

use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use bevy::asset::LoadContext;
use bevy::asset::io::Reader;
use bevy::prelude::*;

use crate::error::AmError;
use crate::loader::AmProject;
use crate::loader::font_loading::{load_embedded_fonts, resolve_google_fonts};
use crate::loader::image_loading::load_embedded_images;
use crate::loader::override_config::AmProjectOverride;
use crate::schema::AmScene;

struct RawProjectContent {
    scene: AmScene,
    embedded_images: HashMap<String, Vec<u8>>,
    embedded_fonts: HashMap<String, Vec<u8>>,
}

pub(super) async fn load_asset(
    reader: &mut dyn Reader,
    load_context: &mut LoadContext<'_>,
) -> Result<AmProject, AmError> {
    let asset_path = load_context.path().clone();
    let path_ref = asset_path.path();
    let is_amproj = path_ref.extension().is_some_and(|ext| ext == "amproj");

    let mut bytes = Vec::new();
    match reader.read_to_end(&mut bytes).await {
        Err(e) if is_amproj && e.kind() == std::io::ErrorKind::IsADirectory => {
            return load_amproj_dir(&resolve_asset_fs_path(path_ref), load_context).await;
        }
        Err(e) => return Err(e.into()),
        Ok(_) => {}
    }

    match path_ref
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "amproj" => {
            let fs_path = resolve_asset_fs_path(path_ref);
            if zip::ZipArchive::new(Cursor::new(&bytes)).is_err() && fs_path.is_dir() {
                return load_amproj_dir(&fs_path, load_context).await;
            }
            let content = read_amproj_archive(&bytes)?;
            build_project(content, load_context, None)
        }
        "xml" => load_xml(&bytes),
        extension => Err(AmError::InvalidFormat(format!(
            "Unknown file extension: {}",
            extension
        ))),
    }
}

fn resolve_asset_fs_path(asset_path: &Path) -> PathBuf {
    let base = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
    base.join("assets").join(asset_path)
}

fn read_amproj_archive(bytes: &[u8]) -> Result<RawProjectContent, AmError> {
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;

    let mut xml_content = None;
    let mut embedded_images = HashMap::new();
    let mut embedded_fonts = HashMap::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = match std::str::from_utf8(file.name_raw()) {
            Ok(utf8_name) => utf8_name.to_string(),
            Err(_) => file.name().to_string(),
        };

        if name.ends_with(".xml") {
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            xml_content = Some(content);
        } else if is_supported_image(&name) {
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;
            let uri = format!("amproj:{}", name);
            debug!("Loaded embedded image: {}", uri);
            embedded_images.insert(uri, data);
        } else if is_supported_font(&name) {
            let mut data = Vec::new();
            file.read_to_end(&mut data)?;
            embedded_fonts.insert(name, data);
        }
    }

    let xml_content = xml_content
        .ok_or_else(|| AmError::InvalidFormat("No XML file found in amproj archive".to_string()))?;
    let scene: AmScene = quick_xml::de::from_str(&xml_content)?;

    Ok(RawProjectContent {
        scene,
        embedded_images,
        embedded_fonts,
    })
}

async fn load_amproj_dir(
    dir_path: &Path,
    load_context: &mut LoadContext<'_>,
) -> Result<AmProject, AmError> {
    let content = read_amproj_directory(dir_path)?;
    let project = build_project(content, load_context, Some(dir_path))?;
    info!("Loaded amproj directory: {:?}", dir_path);
    Ok(project)
}

fn read_amproj_directory(dir_path: &Path) -> Result<RawProjectContent, AmError> {
    let mut xml_content = None;
    let mut embedded_images = HashMap::new();
    let mut embedded_fonts = HashMap::new();

    let entries = std::fs::read_dir(dir_path)
        .map_err(|e| AmError::InvalidFormat(format!("Failed to read amproj directory: {}", e)))?;

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if name.ends_with(".xml") {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| AmError::InvalidFormat(format!("Failed to read XML file: {}", e)))?;
            xml_content = Some(content);
        } else if is_supported_image(name) {
            let data = std::fs::read(&path)
                .map_err(|e| AmError::InvalidFormat(format!("Failed to read image file: {}", e)))?;
            let uri = format!("amproj:{}", name);
            debug!("Loaded image from directory: {}", uri);
            embedded_images.insert(uri, data);
        } else if is_supported_font(name) {
            let data = std::fs::read(&path)
                .map_err(|e| AmError::InvalidFormat(format!("Failed to read font file: {}", e)))?;
            embedded_fonts.insert(name.to_string(), data);
        }
    }

    let xml_content = xml_content.ok_or_else(|| {
        AmError::InvalidFormat("No XML file found in amproj directory".to_string())
    })?;
    let scene: AmScene = quick_xml::de::from_str(&xml_content)?;

    Ok(RawProjectContent {
        scene,
        embedded_images,
        embedded_fonts,
    })
}

fn build_project(
    content: RawProjectContent,
    load_context: &mut LoadContext<'_>,
    dir_path: Option<&Path>,
) -> Result<AmProject, AmError> {
    let RawProjectContent {
        scene,
        embedded_images,
        embedded_fonts,
    } = content;

    let mut images = load_embedded_images(&embedded_images, load_context);
    let loaded_fonts = load_embedded_fonts(&embedded_fonts, load_context)?;
    let mut fonts = loaded_fonts.fonts;
    let mut font_metrics = loaded_fonts.font_metrics;

    resolve_google_fonts(&scene.layers, &mut fonts, &mut font_metrics, load_context);

    let validation_report = crate::validation::ValidationReport::validate(&scene);
    #[cfg(not(target_arch = "wasm32"))]
    validation_report.log_report(&scene.title);
    #[cfg(target_arch = "wasm32")]
    validation_report.log_report_wasm(&scene.title);

    if let Some(dir_path) = dir_path {
        apply_directory_image_aliases(dir_path, &scene, &mut images);
    }

    Ok(AmProject {
        scene,
        images,
        fonts,
        font_metrics,
        embedded_images,
        embedded_fonts: loaded_fonts.preserved_fonts,
        validation_report,
    })
}

fn apply_directory_image_aliases(
    dir_path: &Path,
    scene: &AmScene,
    images: &mut HashMap<String, Handle<Image>>,
) {
    let overrides = AmProjectOverride::load_for(dir_path);
    let override_uris: HashSet<&str> = overrides
        .as_ref()
        .map(|config| config.media.keys().map(|key| key.as_str()).collect())
        .unwrap_or_default();

    for media in &scene.media {
        if media.uri.is_empty() || media.filename.is_empty() {
            continue;
        }
        if override_uris.contains(media.uri.as_str()) {
            debug!(
                "Skipping auto-resolve for {} (overridden in .amproj.toml)",
                media.uri
            );
            continue;
        }
        let amproj_key = format!("amproj:{}", media.filename);
        if let Some(handle) = images.get(&amproj_key).cloned() {
            images.insert(media.uri.clone(), handle);
        }
    }

    if let Some(overrides) = overrides {
        for (content_uri, filename) in &overrides.media {
            let amproj_key = format!("amproj:{}", filename);
            if let Some(handle) = images.get(&amproj_key).cloned() {
                images.insert(content_uri.clone(), handle);
                debug!("Override: {} -> {}", content_uri, filename);
            } else {
                warn!(
                    "Override config references '{}' but image '{}' not found in amproj directory",
                    content_uri, filename
                );
            }
        }
    }
}

fn load_xml(bytes: &[u8]) -> Result<AmProject, AmError> {
    let content = String::from_utf8_lossy(bytes);
    let scene: AmScene = quick_xml::de::from_str(&content)?;

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

fn is_supported_image(name: &str) -> bool {
    name.ends_with(".png")
        || name.ends_with(".jpg")
        || name.ends_with(".jpeg")
        || name.ends_with(".webp")
}

fn is_supported_font(name: &str) -> bool {
    name.ends_with(".ttf") || name.ends_with(".otf")
}

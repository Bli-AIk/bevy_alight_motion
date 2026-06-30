//! Pure project loading utilities for `.amproj` archives, directories, and XML files.

use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::Path;

use crate::error::AmError;
use crate::schema::AmScene;
use crate::validation::ValidationReport;

#[derive(Debug, Clone)]
pub struct RawProjectContent {
    pub scene: AmScene,
    pub embedded_images: HashMap<String, Vec<u8>>,
    pub embedded_fonts: HashMap<String, Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct AmProject {
    pub scene: AmScene,
    pub embedded_images: HashMap<String, Vec<u8>>,
    pub embedded_fonts: HashMap<String, Vec<u8>>,
    pub validation_report: ValidationReport,
}

impl AmProject {
    pub fn from_raw(content: RawProjectContent) -> Self {
        let validation_report = ValidationReport::validate(&content.scene);
        Self {
            scene: content.scene,
            embedded_images: content.embedded_images,
            embedded_fonts: content.embedded_fonts,
            validation_report,
        }
    }
}

pub fn load_project_from_path(path: impl AsRef<Path>) -> Result<AmProject, AmError> {
    let path = path.as_ref();
    let content = if path.is_dir() {
        read_amproj_directory(path)?
    } else {
        let bytes = std::fs::read(path)?;
        let extension = path
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("");
        read_project_bytes(&bytes, extension)?
    };

    Ok(AmProject::from_raw(content))
}

pub fn read_project_bytes(bytes: &[u8], extension: &str) -> Result<RawProjectContent, AmError> {
    match extension.to_ascii_lowercase().as_str() {
        "amproj" => read_amproj_archive(bytes),
        "xml" => read_xml_bytes(bytes),
        extension => Err(AmError::InvalidFormat(format!(
            "Unknown file extension: {}",
            extension
        ))),
    }
}

pub fn read_amproj_archive(bytes: &[u8]) -> Result<RawProjectContent, AmError> {
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
            log::debug!("Loaded embedded image: {}", uri);
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

pub fn read_amproj_directory(dir_path: &Path) -> Result<RawProjectContent, AmError> {
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
            log::debug!("Loaded image from directory: {}", uri);
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

pub fn read_xml_bytes(bytes: &[u8]) -> Result<RawProjectContent, AmError> {
    let content = String::from_utf8_lossy(bytes);
    let scene: AmScene = quick_xml::de::from_str(&content)?;

    Ok(RawProjectContent {
        scene,
        embedded_images: HashMap::new(),
        embedded_fonts: HashMap::new(),
    })
}

pub fn is_supported_image(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.ends_with(".png")
        || name.ends_with(".jpg")
        || name.ends_with(".jpeg")
        || name.ends_with(".webp")
}

pub fn is_supported_font(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.ends_with(".ttf") || name.ends_with(".otf")
}

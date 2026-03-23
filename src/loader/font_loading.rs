//! This file handles font ingestion during project loading.
//! It turns embedded font blobs or resolved system-font fallbacks into Bevy font
//! handles, preserves raw bytes when later stages still need them, and extracts
//! the metrics that text layout code depends on.
//!
//! 这个文件负责项目加载阶段的字体导入。它会把嵌入字体数据或解析出的系统字体回退
//! 转成 Bevy 字体句柄，在后续阶段仍需要时保留原始字节，并提取文本布局所依赖的
//! 字体度量信息。

use std::collections::HashMap;

use bevy::asset::LoadContext;
use bevy::prelude::*;

use crate::error::AmError;
use crate::loader::font_metrics::{FontMetrics, extract_font_metrics};
use crate::schema::AmLayer;

pub(super) struct LoadedFonts {
    pub fonts: HashMap<String, Handle<Font>>,
    pub font_metrics: HashMap<String, FontMetrics>,
    pub preserved_fonts: HashMap<String, Vec<u8>>,
}

pub(super) fn load_embedded_fonts(
    embedded_fonts: &HashMap<String, Vec<u8>>,
    load_context: &mut LoadContext<'_>,
) -> Result<LoadedFonts, AmError> {
    let mut fonts = HashMap::new();
    let mut font_metrics = HashMap::new();
    let mut preserved_fonts = HashMap::new();

    for (name, data) in embedded_fonts {
        if !passes_fontdb_validation(data) {
            warn!(
                "Font '{}' failed fontdb validation, skipping to avoid text pipeline panic",
                name
            );
            continue;
        }

        preserved_fonts.insert(name.clone(), data.clone());

        if let Some(metrics) = extract_font_metrics(data) {
            trace!(
                "Font '{}' metrics: win_ascent={:.4}, win_descent={:.4}, hhea_ascent={:.4}, hhea_descent={:.4}, upm={}",
                name,
                metrics.win_ascent,
                metrics.win_descent,
                metrics.hhea_ascent,
                metrics.hhea_descent,
                metrics.units_per_em
            );
            font_metrics.insert(name.clone(), metrics);
        }

        let font = Font::try_from_bytes(data.clone()).map_err(|e| {
            AmError::InvalidFormat(format!("Failed to load font {}: {:?}", name, e))
        })?;
        let handle = load_context.add_labeled_asset(format!("font_{}", name), font);
        fonts.insert(name.clone(), handle);
    }

    Ok(LoadedFonts {
        fonts,
        font_metrics,
        preserved_fonts,
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn resolve_google_fonts(
    layers: &[AmLayer],
    fonts: &mut HashMap<String, Handle<Font>>,
    font_metrics: &mut HashMap<String, FontMetrics>,
    load_context: &mut LoadContext<'_>,
) {
    let google_font_refs: Vec<String> = collect_google_font_refs(layers).into_iter().collect();

    for font_ref in google_font_refs {
        if fonts.contains_key(&font_ref) {
            continue;
        }
        if let Some(path) = resolve_google_font_to_system(&font_ref)
            && let Ok(data) = std::fs::read(&path)
        {
            try_load_system_font(&font_ref, data, &path, fonts, font_metrics, load_context);
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn resolve_google_fonts(
    _layers: &[AmLayer],
    _fonts: &mut HashMap<String, Handle<Font>>,
    _font_metrics: &mut HashMap<String, FontMetrics>,
    _load_context: &mut LoadContext<'_>,
) {
}

fn passes_fontdb_validation(data: &[u8]) -> bool {
    let mut test_db = fontdb::Database::new();
    test_db.load_font_data(data.to_vec());
    test_db.faces().count() > 0
}

#[cfg(not(target_arch = "wasm32"))]
fn try_load_system_font(
    font_ref: &str,
    data: Vec<u8>,
    path: &str,
    fonts: &mut HashMap<String, Handle<Font>>,
    font_metrics: &mut HashMap<String, FontMetrics>,
    load_context: &mut LoadContext<'_>,
) {
    if !passes_fontdb_validation(&data) {
        warn!("System font at '{}' failed fontdb validation", path);
        return;
    }

    if let Some(metrics) = extract_font_metrics(&data) {
        font_metrics.insert(font_ref.to_string(), metrics);
    }

    match Font::try_from_bytes(data) {
        Ok(font) => {
            let handle = load_context.add_labeled_asset(format!("font_system_{}", font_ref), font);
            fonts.insert(font_ref.to_string(), handle);
            info!("Resolved '{}' to system font: {}", font_ref, path);
        }
        Err(e) => {
            warn!("Failed to load system font '{}': {:?}", path, e);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_google_font_refs(layers: &[AmLayer]) -> std::collections::HashSet<String> {
    let mut refs = std::collections::HashSet::new();
    for layer in layers {
        match layer {
            AmLayer::Text(text) if text.font.starts_with("googlefonts?") => {
                refs.insert(text.font.clone());
            }
            AmLayer::EmbedScene(embed) => {
                refs.extend(collect_google_font_refs(&embed.scene.layers));
            }
            _ => {}
        }
    }
    refs
}

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
        format!(
            "/usr/share/fonts/truetype/{}/unhinted/{}TTF/{}-{}.ttf",
            font_name.to_lowercase(),
            font_name,
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

    let style_name = match weight {
        100 => "Thin",
        200 => "Extra-Light",
        300 => "Light",
        400 => "Regular",
        500 => "Medium",
        600 => "Semi-Bold",
        700 => "Bold",
        800 => "Extra-Bold",
        900 => "Black",
        _ => "Regular",
    };
    if let Ok(output) = std::process::Command::new("fc-match")
        .args([
            "-f",
            "%{file}",
            &format!("{}:style={}", font_name, style_name),
        ])
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

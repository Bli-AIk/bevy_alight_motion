//! This file defines the top-level scene schema parsed from Alight Motion XML.
//! It models project metadata, media declarations, and the root layer list, so
//! every later loader or runtime stage can work from one typed representation of
//! the exported scene document.
//!
//! 这个文件定义了从 Alight Motion XML 解析出的顶层场景 schema。它描述项目元数据、
//! 媒体声明以及根图层列表，让后续所有加载和运行时阶段都能建立在同一份类型化场景
//! 文档表示之上。

use serde::{Deserialize, Serialize};

use super::layers::AmLayer;

/// Root scene node containing project metadata and layers.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename = "scene")]
pub struct AmScene {
    #[serde(rename = "@title", default)]
    pub title: String,
    #[serde(rename = "@width", default = "default_canvas_size")]
    pub width: u32,
    #[serde(rename = "@height", default = "default_canvas_size")]
    pub height: u32,
    #[serde(rename = "@exportWidth", default = "default_canvas_size")]
    pub export_width: u32,
    #[serde(rename = "@exportHeight", default = "default_canvas_size")]
    pub export_height: u32,
    #[serde(rename = "@fps", default = "default_fps")]
    pub fps: u32,
    #[serde(rename = "@totalTime", default)]
    pub total_time: u32,
    #[serde(rename = "@bgcolor", default = "default_bgcolor")]
    pub bgcolor: String,
    #[serde(rename = "@amver", default)]
    pub amver: i32,
    #[serde(rename = "@retime", default)]
    pub retime: String,
    #[serde(rename = "@precompose", default)]
    pub precompose: String,
    #[serde(rename = "media", default)]
    pub media: Vec<AmMedia>,
    #[serde(rename = "$value", default)]
    pub layers: Vec<AmLayer>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmMedia {
    #[serde(rename = "@uri", default)]
    pub uri: String,
    #[serde(rename = "@filename", default)]
    pub filename: String,
    #[serde(rename = "@type", default)]
    pub media_type: String,
    #[serde(rename = "@width", default)]
    pub width: u32,
    #[serde(rename = "@height", default)]
    pub height: u32,
    #[serde(rename = "@size", default)]
    pub size: u32,
    #[serde(rename = "@sig", default)]
    pub sig: String,
}

fn default_canvas_size() -> u32 {
    1280
}

fn default_fps() -> u32 {
    60
}

fn default_bgcolor() -> String {
    "#ff000000".to_string()
}

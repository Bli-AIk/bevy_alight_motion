//! Defines the optional sidecar override format for `.amproj` assets.
//! It lets authors patch missing or ambiguous media filenames with a small TOML
//! file, which keeps the main loader simpler while still supporting imperfect
//! exports from real-world Alight Motion projects.
//!
//! 定义了 `.amproj` 资源可选的旁路覆盖配置格式。它允许作者用一个小型
//! TOML 文件修补缺失或含糊的媒体文件名，从而在保持主加载器简洁的同时，仍能兼容
//! 现实中并不总是完整的 Alight Motion 导出结果。

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;

/// Optional override configuration for amproj assets.
///
/// Placed alongside the `.amproj` file/directory as `<name>.amproj.toml`.
/// Provides manual content URI → filename mappings for cases where
/// the XML `<media>` elements lack a `filename` attribute.
#[derive(Debug, Default, serde::Deserialize)]
pub(super) struct AmProjectOverride {
    /// Content URI → local filename mappings.
    /// Keys are Android content URIs (e.g. `content://media/external/images/media/1000048179`),
    /// values are filenames within the amproj directory.
    #[serde(default)]
    pub media: HashMap<String, String>,
}

impl AmProjectOverride {
    /// Try to load an override config from `<amproj_path>.toml`.
    pub fn load_for(amproj_path: &Path) -> Option<Self> {
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

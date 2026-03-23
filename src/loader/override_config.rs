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

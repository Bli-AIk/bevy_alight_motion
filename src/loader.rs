//! Asset loader for Alight Motion project files.

mod font_loading;
mod font_metrics;
mod image_loading;
mod override_config;
mod project_loading;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;
use std::collections::HashMap;

use crate::error::AmError;
use crate::schema::AmScene;

pub use font_metrics::{FontMetrics, contains_cjk};

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
        project_loading::load_asset(reader, load_context).await
    }

    fn extensions(&self) -> &[&str] {
        &["amproj", "xml"]
    }
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

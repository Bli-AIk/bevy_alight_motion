//! Defines small plugin-level resources shared across the runtime.
//! It covers project-to-window resolution policy and the single white-pixel image
//! that solid-color visuals reuse, so startup code and scene spawning can depend
//! on one centralized definition.
//!
//! 定义了运行时共用的少量插件级资源。它包含项目到窗口的分辨率策略，
//! 以及纯色视觉对象复用的单白像素图片，让启动逻辑和场景生成都可以依赖同一份
//! 集中定义。

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;

/// Resource to configure how the AM project is scaled relative to the window.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq)]
pub enum AmProjectResolution {
    /// No scaling (1:1 pixel mapping).
    #[default]
    None,
    /// Scale the project to fit within the window, preserving aspect ratio.
    FitWindow,
    /// Scale the project to cover the window, preserving aspect ratio.
    CoverWindow,
    /// Scale the project to a fixed width, preserving aspect ratio.
    FixedWidth(f32),
    /// Scale the project to a fixed height, preserving aspect ratio.
    FixedHeight(f32),
    /// Scale the project to fit within a fixed viewport size, preserving aspect ratio.
    /// Useful for headless rendering where no window is available.
    FixedSize(f32, f32),
}

/// Resource holding effect names that should be skipped at runtime.
///
/// Inserted by comparison tooling when `disabled_effects` is configured in
/// `comparison_config.toml`.  The unified-effect system checks this resource
/// and bypasses any listed effect (e.g. `"pixelate"`).
#[derive(Resource, Debug, Clone, Default)]
pub struct DisabledEffects {
    names: std::collections::HashSet<String>,
}

impl DisabledEffects {
    pub fn new(names: impl IntoIterator<Item = String>) -> Self {
        Self {
            names: names.into_iter().collect(),
        }
    }

    pub fn contains(&self, effect_name: &str) -> bool {
        self.names.contains(effect_name)
    }
}

/// Resource holding the white pixel texture used for solid color sprites.
#[derive(Resource)]
pub struct AmWhitePixel(pub Handle<Image>);

pub(super) fn create_white_pixel() -> Image {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

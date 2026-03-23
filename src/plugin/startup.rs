//! This file contains startup-only helpers for the plugin runtime.
//! It creates the shared white-pixel texture and loads system fonts for fallback
//! on native platforms, so later text and sprite code can assume those defaults
//! already exist.
//!
//! 这个文件存放插件运行时只在启动阶段执行的辅助逻辑。它会创建共享的白像素纹理，
//! 并在原生平台上加载系统字体作为回退来源，让后续文本和精灵代码可以默认这些基础
//! 资源已经就绪。

use bevy::image::Image;
use bevy::prelude::*;

use crate::plugin::resources::{AmWhitePixel, create_white_pixel};

pub(super) fn setup_white_pixel_system(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let handle = images.add(create_white_pixel());
    commands.insert_resource(AmWhitePixel(handle));
}

/// Load system fonts into the CosmicFontSystem for font fallback.
/// This enables rendering of CJK, Arabic, Hindi, and other scripts
/// even when the primary font doesn't have those glyphs.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn load_system_fonts_for_fallback(
    mut font_system: ResMut<bevy::text::CosmicFontSystem>,
) {
    font_system.0.db_mut().load_system_fonts();
    let count = font_system.0.db().faces().count();
    bevy::log::info!("Loaded {} system font faces for fallback", count);
}

#[cfg(target_arch = "wasm32")]
pub(super) fn load_system_fonts_for_fallback() {}

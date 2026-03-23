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

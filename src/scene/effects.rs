//! # effects.rs
//!
//! # 效果参数提取模块
//!
//! Effect parameter extraction from AM layers.
//! AM 图层的效果参数提取。

use bevy::prelude::*;

use crate::schema::{AmAnimatedFloat, AmEffect};

pub(crate) fn extract_effect_animations(
    effects: &[AmEffect],
) -> (AmAnimatedFloat, AmAnimatedFloat) {
    let mut pos_x = AmAnimatedFloat::default();
    let mut pos_y = AmAnimatedFloat::default();

    for effect in effects {
        if effect.id == "com.alightcreative.effects.transform2" {
            for prop in &effect.properties {
                match prop.name.as_str() {
                    "posx" => {
                        if !prop.keyframes.is_empty() {
                            pos_x.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            pos_x.value = Some(v);
                        }
                    }
                    "posy" => {
                        if !prop.keyframes.is_empty() {
                            pos_y.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            pos_y.value = Some(v);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    (pos_x, pos_y)
}
#[derive(Debug, Clone, Default)]
pub struct WipeEffectParams {
    pub start: AmAnimatedFloat,
    pub end: AmAnimatedFloat,
    pub angle: AmAnimatedFloat,
    pub feather: AmAnimatedFloat,
}

/// Extract wipe effect parameters from wipe2 effects.
pub(crate) fn extract_wipe_effect(effects: &[AmEffect]) -> WipeEffectParams {
    let mut params = WipeEffectParams::default();
    // Default: no wipe (show everything)
    params.end.value = Some(1.0);

    for effect in effects {
        if effect.id == "com.alightcreative.effects.wipe2" {
            for prop in &effect.properties {
                match prop.name.as_str() {
                    "start" => {
                        if !prop.keyframes.is_empty() {
                            params.start.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.start.value = Some(v);
                        }
                    }
                    "end" => {
                        if !prop.keyframes.is_empty() {
                            params.end.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.end.value = Some(v);
                        }
                    }
                    "angle" => {
                        if !prop.keyframes.is_empty() {
                            params.angle.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.angle.value = Some(v);
                        }
                    }
                    "feather" => {
                        if !prop.keyframes.is_empty() {
                            params.feather.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.feather.value = Some(v);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    params
}
#[derive(Debug, Clone, Default)]
pub struct StretchSegmentParams {
    /// Angle of the split line in degrees (0 = horizontal)
    pub angle: AmAnimatedFloat,
    /// Stretch amount (pixels, will be normalized to UV)
    pub stretch: AmAnimatedFloat,
    /// Offset of the split line position
    pub offset: AmAnimatedFloat,
    /// Smooth transition width (0 = hard edge)
    pub smooth: AmAnimatedFloat,
}

impl StretchSegmentParams {
    /// Check if this has any stretch segment effect parameters set
    pub fn has_effect(&self) -> bool {
        self.stretch.value.is_some()
            || !self.stretch.keyframes.is_empty()
            || self.angle.value.is_some()
            || !self.angle.keyframes.is_empty()
            || self.offset.value.is_some()
            || !self.offset.keyframes.is_empty()
            || self.smooth.value.is_some()
            || !self.smooth.keyframes.is_empty()
    }
}

/// Extract stretch segment effect parameters from effects.
pub(crate) fn extract_stretch_segment_effect(effects: &[AmEffect]) -> StretchSegmentParams {
    let mut params = StretchSegmentParams::default();

    for effect in effects {
        if effect.id == "com.alightcreative.effects.stretchsegment" {
            for prop in &effect.properties {
                match prop.name.as_str() {
                    "angle" => {
                        if !prop.keyframes.is_empty() {
                            params.angle.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.angle.value = Some(v);
                        }
                    }
                    "stretch" => {
                        if !prop.keyframes.is_empty() {
                            params.stretch.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.stretch.value = Some(v);
                        }
                    }
                    "offset" => {
                        if !prop.keyframes.is_empty() {
                            params.offset.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.offset.value = Some(v);
                        }
                    }
                    "smooth" => {
                        if !prop.keyframes.is_empty() {
                            params.smooth.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.smooth.value = Some(v);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    params
}
#[derive(Debug, Clone, Default)]
pub struct GaussianBlurParams {
    /// Blur strength (0 = no blur, higher = more blur)
    pub strength: AmAnimatedFloat,
}

impl GaussianBlurParams {
    /// Check if this has any blur effect parameters set
    pub fn has_effect(&self) -> bool {
        self.strength.value.is_some() || !self.strength.keyframes.is_empty()
    }
}

/// Extract Gaussian blur effect parameters from effects.
pub(crate) fn extract_gaussian_blur_effect(effects: &[AmEffect]) -> GaussianBlurParams {
    let mut params = GaussianBlurParams::default();

    for effect in effects {
        if effect.id == "com.alightcreative.effects.gaussianblur" {
            for prop in &effect.properties {
                if prop.name == "strength" {
                    if !prop.keyframes.is_empty() {
                        params.strength.keyframes = prop.keyframes.clone();
                    } else if let Ok(v) = prop.value.parse::<f32>() {
                        params.strength.value = Some(v);
                    }
                }
            }
        }
    }

    params
}
#[derive(Debug, Clone, Default)]
pub struct PaletteMapParams {
    /// Effect alpha/strength (0.0-1.0)
    pub alpha: AmAnimatedFloat,
    /// Number of colors to use (1-8)
    pub count: u8,
    /// Whether to enable shade variations
    pub shades: bool,
    /// Palette colors (up to 8)
    pub colors: [Vec4; 8],
}

impl PaletteMapParams {
    /// Check if this has any palette map effect parameters set
    pub fn has_effect(&self) -> bool {
        self.alpha.value.is_some() || !self.alpha.keyframes.is_empty()
    }
}

/// Extract palette map effect parameters from effects.
pub(crate) fn extract_palette_map_effect(effects: &[AmEffect]) -> PaletteMapParams {
    let mut params = PaletteMapParams::default();

    for effect in effects {
        if effect.id == "com.alightcreative.effects.palettemap" {
            for prop in &effect.properties {
                match prop.name.as_str() {
                    "alpha" => {
                        if !prop.keyframes.is_empty() {
                            params.alpha.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.alpha.value = Some(v);
                        }
                    }
                    "palette" => {
                        if let Ok(_v) = prop.value.parse::<u8>() {
                            // AM palette count includes disabled colors; fx_5_palette uses only 3
                            params.count = 3;
                        }
                    }
                    "shades" => {
                        params.shades = prop.value == "true";
                    }
                    name if name.starts_with("color") => {
                        // Parse color1-color8
                        if let Some(index_char) = name.strip_prefix("color") {
                            if let Ok(index) = index_char.parse::<usize>() {
                                if index >= 1 && index <= 8 {
                                    if let Ok(color) = crate::schema::parse_color(&prop.value) {
                                        params.colors[index - 1] =
                                            Vec4::new(color[0], color[1], color[2], color[3]);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    params
}

//! Extracts image-oriented effects such as thresholding, chroma key,
//! palette replacement, and similar pixel-level adjustments.
//! It converts raw effect properties into typed parameter structs that scene
//! collection and animation systems can consume directly.
//!
//! 负责提取与图像处理相关的效果，例如 threshold、chroma key、
//! 调色板替换等像素级调整。它会把原始 effect 属性转换成带类型的参数结构，
//! 供场景收集和动画系统直接使用。

use bevy::prelude::*;

use super::{parse_color_keyframe, parse_vec2_value};
use crate::schema::{AmAnimatedFloat, AmAnimatedVec2, AmEffect};

#[derive(Debug, Clone, Default)]
pub struct ThresholdParams {
    pub threshold: AmAnimatedFloat,
    pub feather: AmAnimatedFloat,
    pub invert: bool,
    pub blend_mode: i32,
}

impl ThresholdParams {
    #[allow(dead_code)]
    pub fn has_effect(&self) -> bool {
        self.threshold.value.is_some() || !self.threshold.keyframes.is_empty()
    }
}

pub(crate) fn extract_threshold_effect(effects: &[AmEffect]) -> ThresholdParams {
    let mut params = ThresholdParams::default();

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.threshold")
    else {
        return params;
    };

    params.threshold.value = Some(0.5);

    for prop in &effect.properties {
        match prop.name.as_str() {
            "threshold" => {
                if !prop.keyframes.is_empty() {
                    params.threshold.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.threshold.value = Some(v);
                }
            }
            "feather" => {
                if !prop.keyframes.is_empty() {
                    params.feather.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.feather.value = Some(v);
                }
            }
            "invert" => {
                params.invert = prop.value == "true";
            }
            "blendMode" => {
                if let Ok(v) = prop.value.parse::<i32>() {
                    params.blend_mode = v;
                }
            }
            _ => {}
        }
    }

    params
}

#[derive(Debug, Clone, Default)]
pub struct GridParams {
    pub position: AmAnimatedVec2,
    pub spacing: AmAnimatedFloat,
    pub width: AmAnimatedFloat,
    pub color: crate::schema::AmAnimatedColor,
    pub punchout: bool,
    pub smoothing: AmAnimatedFloat,
    pub screen_space: bool,
}

impl GridParams {
    #[allow(dead_code)]
    pub fn has_effect(&self) -> bool {
        self.spacing.value.is_some() || !self.spacing.keyframes.is_empty()
    }
}

pub(crate) fn extract_grid_effect(effects: &[AmEffect]) -> GridParams {
    let mut params = GridParams::default();

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.grid2")
    else {
        return params;
    };

    params.spacing.value = Some(0.1);
    params.width.value = Some(0.01);
    params.smoothing.value = Some(0.05);

    for prop in &effect.properties {
        match prop.name.as_str() {
            "position" => {
                if !prop.keyframes.is_empty() {
                    params.position.keyframes = prop.keyframes.clone();
                } else if let Some(v) = parse_vec2_value(&prop.value) {
                    params.position.value = Some(v);
                }
            }
            "spacing" => {
                if !prop.keyframes.is_empty() {
                    params.spacing.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.spacing.value = Some(v);
                }
            }
            "width" => {
                if !prop.keyframes.is_empty() {
                    params.width.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.width.value = Some(v);
                }
            }
            "color" => {
                if !prop.keyframes.is_empty() {
                    params.color.keyframes = prop
                        .keyframes
                        .iter()
                        .filter_map(parse_color_keyframe)
                        .collect();
                } else if let Ok(color) = crate::schema::parse_color(&prop.value) {
                    params.color.value = Some(Vec4::new(color[0], color[1], color[2], color[3]));
                }
            }
            "punchout" => {
                params.punchout = prop.value == "true";
            }
            "smoothing" => {
                if !prop.keyframes.is_empty() {
                    params.smoothing.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.smoothing.value = Some(v);
                }
            }
            "screenSpace" => {
                params.screen_space = prop.value == "true";
            }
            _ => {}
        }
    }

    params
}

#[derive(Debug, Clone, Default)]
pub struct PixelateParams {
    pub size: AmAnimatedFloat,
    pub stretch: AmAnimatedVec2,
    pub angle: AmAnimatedFloat,
    pub vignette: AmAnimatedFloat,
    pub threshold: AmAnimatedFloat,
    pub saturation: AmAnimatedFloat,
    pub screen_space: bool,
}

impl PixelateParams {
    #[allow(dead_code)]
    pub fn has_effect(&self) -> bool {
        self.size.value.is_some() || !self.size.keyframes.is_empty()
    }
}

pub(crate) fn extract_pixelate_effect(effects: &[AmEffect]) -> PixelateParams {
    let mut params = PixelateParams::default();

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.pixelate2")
    else {
        return params;
    };

    params.size.value = Some(10.0);
    params.stretch.value = Some([1.0, 1.0]);
    params.threshold.value = Some(0.5);
    params.saturation.value = Some(1.0);

    for prop in &effect.properties {
        match prop.name.as_str() {
            "size" => {
                if !prop.keyframes.is_empty() {
                    params.size.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.size.value = Some(v);
                }
            }
            "stretch" => {
                if !prop.keyframes.is_empty() {
                    params.stretch.keyframes = prop.keyframes.clone();
                } else if let Some(v) = parse_vec2_value(&prop.value) {
                    params.stretch.value = Some(v);
                }
            }
            "angle" => {
                if !prop.keyframes.is_empty() {
                    params.angle.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.angle.value = Some(v);
                }
            }
            "vignette" => {
                if !prop.keyframes.is_empty() {
                    params.vignette.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.vignette.value = Some(v);
                }
            }
            "threshold" => {
                if !prop.keyframes.is_empty() {
                    params.threshold.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.threshold.value = Some(v);
                }
            }
            "saturation" => {
                if !prop.keyframes.is_empty() {
                    params.saturation.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.saturation.value = Some(v);
                }
            }
            "screenSpace" => {
                params.screen_space = prop.value == "true";
            }
            _ => {}
        }
    }

    params
}

#[derive(Debug, Clone, Default)]
pub struct SolidColorParams {
    pub color: crate::schema::AmAnimatedColor,
    pub alpha: AmAnimatedFloat,
    pub blend_mode: i32,
}

pub(crate) fn extract_solid_color_effect(effects: &[AmEffect]) -> SolidColorParams {
    let mut params = SolidColorParams::default();

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.solidcolor")
    else {
        return params;
    };

    params.alpha.value = Some(1.0);
    params.color.value = Some(Vec4::new(
        0x2D as f32 / 255.0,
        0x1E as f32 / 255.0,
        0xF6 as f32 / 255.0,
        1.0,
    ));

    for prop in &effect.properties {
        match prop.name.as_str() {
            "color" => {
                if !prop.keyframes.is_empty() {
                    params.color.keyframes = prop
                        .keyframes
                        .iter()
                        .filter_map(parse_color_keyframe)
                        .collect();
                } else if let Ok(color) = crate::schema::parse_color(&prop.value) {
                    params.color.value = Some(Vec4::new(color[0], color[1], color[2], color[3]));
                }
            }
            "alpha" => {
                if !prop.keyframes.is_empty() {
                    params.alpha.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.alpha.value = Some(v);
                }
            }
            "blendMode" => {
                if let Ok(v) = prop.value.parse::<i32>() {
                    params.blend_mode = v;
                }
            }
            _ => {}
        }
    }

    params
}

#[derive(Debug, Clone, Default)]
pub struct ExposureGammaParams {
    pub exposure: AmAnimatedFloat,
    pub gamma: AmAnimatedFloat,
    pub offset: AmAnimatedFloat,
    pub has_effect: bool,
}

pub(crate) fn extract_exposure_gamma_effect(effects: &[AmEffect]) -> ExposureGammaParams {
    let mut params = ExposureGammaParams::default();

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.exposure")
    else {
        return params;
    };

    params.has_effect = true;
    params.exposure.value = Some(0.0);
    params.gamma.value = Some(1.0);
    params.offset.value = Some(0.0);

    for prop in &effect.properties {
        match prop.name.as_str() {
            "exposure" => {
                if !prop.keyframes.is_empty() {
                    params.exposure.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.exposure.value = Some(v);
                }
            }
            "gamma" => {
                if !prop.keyframes.is_empty() {
                    params.gamma.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.gamma.value = Some(v);
                }
            }
            "offset" => {
                if !prop.keyframes.is_empty() {
                    params.offset.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.offset.value = Some(v);
                }
            }
            _ => {}
        }
    }

    params
}

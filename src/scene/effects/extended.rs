//! Text, counter, displacement, and color-split effect parameter extraction.
//! 文本、计数器、位移和色分离效果参数提取

use crate::schema::{AmAnimatedFloat, AmEffect, AmProperty};

// --- Text Spacing Effect ---

/// Text spacing effect parameters (letter spacing and line spacing).
#[derive(Debug, Clone, Default)]
pub struct TextSpacingParams {
    /// Letter spacing in em units (0.0 = default)
    pub letter_spacing: AmAnimatedFloat,
    /// Line spacing multiplier (1.0 = default)
    pub line_spacing: AmAnimatedFloat,
}

/// Extract text spacing params from effects.
pub(crate) fn extract_text_spacing_effect(effects: &[AmEffect]) -> TextSpacingParams {
    let mut params = TextSpacingParams::default();
    params.line_spacing.value = Some(1.0);

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.textspacing")
    else {
        return params;
    };

    for prop in &effect.properties {
        match prop.name.as_str() {
            "letterspacing" => {
                crate::scene::effects::extract_float_prop(prop, &mut params.letter_spacing);
            }
            "linespacing" => {
                crate::scene::effects::extract_float_prop(prop, &mut params.line_spacing);
            }
            _ => {}
        }
    }

    params
}

// --- Text Progress Effect ---

/// Text progress effect parameters.
#[derive(Debug, Clone, Default)]
pub struct TextProgressParams {
    /// Start of visible text range (0.0-1.0)
    pub start: AmAnimatedFloat,
    /// End of visible text range (0.0-1.0)
    pub end: AmAnimatedFloat,
    /// Cursor style (0=none, 1=line, 2=block, 3=underscore)
    pub cursor: i32,
    /// Whether cursor blinks
    pub blink: bool,
}

/// Extract text progress params from effects.
pub(crate) fn extract_text_progress_effect(effects: &[AmEffect]) -> TextProgressParams {
    let mut params = TextProgressParams::default();
    params.end.value = Some(1.0);

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.textprogress")
    else {
        return params;
    };

    for prop in &effect.properties {
        match prop.name.as_str() {
            "start" => {
                crate::scene::effects::extract_float_prop(prop, &mut params.start);
            }
            "end" => {
                crate::scene::effects::extract_float_prop(prop, &mut params.end);
            }
            "cursor" => {
                if let Ok(v) = prop.value.parse::<f32>() {
                    params.cursor = v as i32;
                }
            }
            "blink" => {
                params.blink = prop.value == "true" || prop.value == "1";
            }
            _ => {}
        }
    }

    params
}

/// Counter effect parameters.
#[derive(Debug, Clone, Default)]
pub struct CounterParams {
    pub offset: AmAnimatedFloat,
    pub scale: AmAnimatedFloat,
}

/// Extract counter effect (`com.alightcreative.effects.counter`) parameters.
pub(crate) fn extract_counter_effect(effects: &[AmEffect]) -> CounterParams {
    let mut params = CounterParams::default();
    params.scale.value = Some(1.0);

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.counter")
    else {
        return params;
    };

    for prop in &effect.properties {
        match prop.name.as_str() {
            "offset" => {
                crate::scene::effects::extract_float_prop(prop, &mut params.offset);
            }
            "scale" => {
                crate::scene::effects::extract_float_prop(prop, &mut params.scale);
            }
            _ => {}
        }
    }

    params
}

/// Simplex displace effect parameters (`com.alightcreative.effects.randomdisplace`).
/// Uses simplex noise to apply spatially-varying position displacement.
/// 随机位移效果参数 — 使用 Simplex 噪声对位置进行基于空间坐标的随机位移
#[derive(Debug, Clone)]
pub struct SimplexDisplaceParams {
    pub enabled: bool,
    /// Displacement magnitude (pixels)
    pub mag: AmAnimatedFloat,
    /// Noise temporal evolution
    pub evolution: AmAnimatedFloat,
    /// Noise seed value
    pub seed: AmAnimatedFloat,
    /// Spatial frequency (0.0-2.0)
    pub scatter: AmAnimatedFloat,
}

impl Default for SimplexDisplaceParams {
    fn default() -> Self {
        Self {
            enabled: false,
            mag: AmAnimatedFloat {
                value: Some(50.0),
                keyframes: Vec::new(),
            },
            evolution: AmAnimatedFloat::default(),
            seed: AmAnimatedFloat::default(),
            scatter: AmAnimatedFloat {
                value: Some(0.5),
                keyframes: Vec::new(),
            },
        }
    }
}

/// Extract simplex displace effect parameters.
/// 从效果中提取随机位移效果参数
pub(crate) fn extract_simplex_displace_effect(effects: &[AmEffect]) -> SimplexDisplaceParams {
    let mut params = SimplexDisplaceParams::default();

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.randomdisplace")
    else {
        return params;
    };

    params.enabled = true;

    for prop in &effect.properties {
        match prop.name.as_str() {
            "mag" => extract_sd_float(prop, &mut params.mag, 50.0),
            "evolution" => extract_sd_float(prop, &mut params.evolution, 0.0),
            "seed" => extract_sd_float(prop, &mut params.seed, 0.0),
            "scatter" => extract_sd_float(prop, &mut params.scatter, 0.5),
            _ => {}
        }
    }

    params
}

/// Helper: extract a float property with keyframes for displacement/split effects.
pub(crate) fn extract_sd_float(prop: &AmProperty, target: &mut AmAnimatedFloat, default: f32) {
    if !prop.keyframes.is_empty() {
        target.value = prop.value.parse::<f32>().ok().or(Some(default));
        target.keyframes = prop.keyframes.clone();
    } else if let Ok(v) = prop.value.parse::<f32>() {
        target.value = Some(v);
    }
}

/// RGB split effect parameters (`com.alightcreative.effects.rgbsep`).
/// Separates RGB channels along a direction for chromatic aberration.
/// RGB 分离效果参数 — 沿指定方向分离 RGB 通道产生色差效果
#[derive(Debug, Clone)]
pub struct RgbSplitParams {
    pub enabled: bool,
    /// Channel offset strength (range: -8.0 to 8.0)
    pub strength: AmAnimatedFloat,
    /// Separation direction angle (degrees)
    pub angle: AmAnimatedFloat,
    /// Which channel stays centered (0=R, 1=G, 2=B)
    pub center_channel: i32,
    /// Compositing mode (0=Mask, 1=Luma, 2=Light, 3=Dark)
    pub mode: i32,
}

impl Default for RgbSplitParams {
    fn default() -> Self {
        Self {
            enabled: false,
            strength: AmAnimatedFloat {
                value: Some(0.15),
                keyframes: Vec::new(),
            },
            angle: AmAnimatedFloat::default(),
            center_channel: 1,
            mode: 2,
        }
    }
}

/// Extract RGB split effect parameters.
/// 从效果中提取 RGB 分离效果参数
pub(crate) fn extract_rgb_split_effect(effects: &[AmEffect]) -> RgbSplitParams {
    let mut params = RgbSplitParams::default();

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.rgbsep")
    else {
        return params;
    };

    params.enabled = true;

    for prop in &effect.properties {
        match prop.name.as_str() {
            "strength" => extract_sd_float(prop, &mut params.strength, 0.15),
            "angle" => extract_sd_float(prop, &mut params.angle, 0.0),
            "centerChannel" => {
                if let Ok(v) = prop.value.parse::<i32>() {
                    params.center_channel = v;
                }
            }
            "mode" => {
                if let Ok(v) = prop.value.parse::<i32>() {
                    params.mode = v;
                }
            }
            _ => {}
        }
    }

    params
}

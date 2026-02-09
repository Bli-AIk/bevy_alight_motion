//! # effects.rs
//!
//! # 效果参数提取模块
//!
//! Effect parameter extraction from AM layers.
//! AM 图层的效果参数提取。

use bevy::prelude::*;

use crate::schema::{AmAnimatedColor, AmAnimatedFloat, AmAnimatedVec2, AmEffect};

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
                        if let Some(index_char) = name.strip_prefix("color")
                            && let Ok(index) = index_char.parse::<usize>()
                            && (1..=8).contains(&index)
                            && let Ok(color) = crate::schema::parse_color(&prop.value)
                        {
                            params.colors[index - 1] =
                                Vec4::new(color[0], color[1], color[2], color[3]);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    params
}

/// Replace Color effect parameters
/// Replaces pixels of oldcolor with newcolor, with threshold and feather controls
#[derive(Debug, Clone, Default)]
pub struct ReplaceColorParams {
    /// The color to replace (RGBA, static)
    pub old_color: Vec4,
    /// The replacement color (RGBA, animated)
    pub new_color: crate::schema::AmAnimatedColor,
    /// Threshold for color matching (0.0-1.0)
    pub threshold: AmAnimatedFloat,
    /// Feather/falloff for smooth transitions (0.0-1.0)
    pub feather: AmAnimatedFloat,
    /// Effect alpha/strength (0.0-1.0)
    pub alpha: AmAnimatedFloat,
    /// Lock luminance - preserve original brightness
    pub lock_luminance: bool,
}

impl ReplaceColorParams {}

/// Extract replace color effect parameters from effects.
pub(crate) fn extract_replace_color_effect(effects: &[AmEffect]) -> ReplaceColorParams {
    let mut params = ReplaceColorParams::default();
    params.alpha.value = Some(1.0); // Default: full effect

    for effect in effects {
        if effect.id == "com.alightcreative.replacecolor" {
            for prop in &effect.properties {
                match prop.name.as_str() {
                    "oldcolor" => {
                        if let Ok(color) = crate::schema::parse_color(&prop.value) {
                            params.old_color = Vec4::new(color[0], color[1], color[2], color[3]);
                        }
                    }
                    "newcolor" => {
                        if !prop.keyframes.is_empty() {
                            params.new_color.keyframes = prop
                                .keyframes
                                .iter()
                                .filter_map(|kf| {
                                    if let Ok(color) = crate::schema::parse_color(&kf.value) {
                                        Some(crate::schema::AmKeyframe {
                                            time: kf.time,
                                            value: format!(
                                                "{},{},{},{}",
                                                color[0], color[1], color[2], color[3]
                                            ),
                                            easing: kf.easing.clone(),
                                        })
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                        } else if let Ok(color) = crate::schema::parse_color(&prop.value) {
                            params.new_color.value =
                                Some(Vec4::new(color[0], color[1], color[2], color[3]));
                        }
                    }
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
                    "alpha" => {
                        if !prop.keyframes.is_empty() {
                            params.alpha.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.alpha.value = Some(v);
                        }
                    }
                    "lockLuminance" => {
                        params.lock_luminance = prop.value == "true";
                    }
                    _ => {}
                }
            }
        }
    }

    params
}

/// Scale Assist effect parameters
/// axis: 1=X only, 2=Y only, 3=XY both
#[derive(Debug, Clone, Default)]
pub struct ScaleAssistParams {
    /// Which axis to apply scale (1=X, 2=Y, 3=XY)
    pub axis: i32,
    /// Scale multiplier (animated)
    pub scale: AmAnimatedFloat,
    /// Damping factor (animated)
    pub damp: AmAnimatedFloat,
}

impl ScaleAssistParams {}

/// Extract scale assist effect parameters from effects.
pub(crate) fn extract_scale_assist_effect(effects: &[AmEffect]) -> ScaleAssistParams {
    let mut params = ScaleAssistParams::default();
    params.scale.value = Some(1.0); // Default: no scaling
    params.damp.value = Some(1.0); // Default: no damping

    for effect in effects {
        if effect.id == "com.alightcreative.effects.scaleassist" {
            for prop in &effect.properties {
                match prop.name.as_str() {
                    "axis" => {
                        if let Ok(v) = prop.value.parse::<i32>() {
                            params.axis = v;
                        }
                    }
                    "scale" => {
                        if !prop.keyframes.is_empty() {
                            params.scale.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.scale.value = Some(v);
                        }
                    }
                    "damp" => {
                        if !prop.keyframes.is_empty() {
                            params.damp.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.damp.value = Some(v);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    params
}

/// Repeat effect parameters
/// Creates multiple copies with cumulative transforms
#[derive(Debug, Clone, Default)]
pub struct RepeatParams {
    /// Number of copies (0 = no effect)
    pub count: AmAnimatedFloat,
    /// Time offset between copies (not yet implemented)
    pub time: AmAnimatedFloat,
    /// X,Y offset per copy (pixels)
    pub offset: AmAnimatedVec2,
    /// Rotation angle per copy (degrees)
    pub angle: AmAnimatedFloat,
    /// Scale multiplier per copy (1.0 = same size)
    pub scale: AmAnimatedFloat,
    /// Alpha multiplier per copy (1.0 = same opacity)
    pub alpha: AmAnimatedFloat,
}

impl RepeatParams {
    /// Check if this has any repeat effect parameters set
    /// 检查是否有任何重复效果参数设置
    #[allow(dead_code)]
    pub fn has_effect(&self) -> bool {
        self.count.value.is_some_and(|v| v > 0.0) || !self.count.keyframes.is_empty()
    }
}

/// Extract repeat effect parameters from effects.
pub(crate) fn extract_repeat_effect(effects: &[AmEffect]) -> RepeatParams {
    let mut params = RepeatParams::default();
    // Defaults
    params.scale.value = Some(1.0);
    params.alpha.value = Some(1.0);

    for effect in effects {
        if effect.id == "com.alightcreative.effects.repeat" {
            for prop in &effect.properties {
                match prop.name.as_str() {
                    "count" => {
                        if !prop.keyframes.is_empty() {
                            params.count.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.count.value = Some(v);
                        }
                    }
                    "time" => {
                        if !prop.keyframes.is_empty() {
                            params.time.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.time.value = Some(v);
                        }
                    }
                    "offset" => {
                        if !prop.keyframes.is_empty() {
                            params.offset.keyframes = prop.keyframes.clone();
                        } else {
                            // Parse "x,y" format
                            let parts: Vec<&str> = prop.value.split(',').collect();
                            if parts.len() == 2
                                && let Ok(x) = parts[0].trim().parse::<f32>()
                                && let Ok(y) = parts[1].trim().parse::<f32>()
                            {
                                params.offset.value = Some([x, y]);
                            }
                        }
                    }
                    "angle" => {
                        if !prop.keyframes.is_empty() {
                            params.angle.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.angle.value = Some(v);
                        }
                    }
                    "scale" => {
                        if !prop.keyframes.is_empty() {
                            params.scale.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.scale.value = Some(v);
                        }
                    }
                    "alpha" => {
                        if !prop.keyframes.is_empty() {
                            params.alpha.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.alpha.value = Some(v);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    params
}

/// Linear Repeat effect parameters
/// Creates multiple copies arranged in a line with advanced distribution controls
/// 线性重复效果参数
/// 创建沿线排列的多个副本，具有高级分布控制
#[derive(Debug, Clone, Default)]
pub struct LinearRepeatParams {
    /// Number of copies (0 = no effect)
    pub count: AmAnimatedFloat,
    /// Position offset for the repeat line (pixels)
    pub position: AmAnimatedVec2,
    /// Additional offset per copy (pixels)
    pub offset: AmAnimatedVec2,
    /// Rotation angle per copy (degrees)
    pub angle: AmAnimatedFloat,
    /// Scale multiplier per copy (1.0 = same size)
    pub scale: AmAnimatedFloat,
    /// Alpha multiplier per copy (1.0 = same opacity)
    pub alpha: AmAnimatedFloat,
    /// Fill color for copies (animated)
    pub fill_color: AmAnimatedColor,
    /// Color blend factor (0 = original, 1+ = blend to fill_color)
    pub blend: AmAnimatedFloat,
    /// Whether to alternate colors between copies
    pub color_alt_copies: bool,
    /// Start of visible range (0.0-1.0)
    pub start: AmAnimatedFloat,
    /// End of visible range (0.0-1.0)
    pub end: AmAnimatedFloat,
    /// Phase shift for distribution
    pub phase: AmAnimatedFloat,
    /// Ease-in factor for distribution
    pub ease_in: AmAnimatedFloat,
    /// Ease-out factor for distribution
    pub ease_out: AmAnimatedFloat,
    /// Overlap factor between copies
    pub overlap: AmAnimatedFloat,
    /// Distribution shape (0 = linear)
    pub shape: i32,
    /// Whether to invert the effect
    pub invert: bool,
    /// Whether to randomize copy order
    pub random_order: bool,
    /// Random seed
    pub seed: f32,
}

impl LinearRepeatParams {
    // No methods needed currently - kept for potential future use
}

/// Extract linear repeat effect parameters from effects.
/// 从效果中提取线性重复效果参数
pub(crate) fn extract_linear_repeat_effect(effects: &[AmEffect]) -> LinearRepeatParams {
    let mut params = LinearRepeatParams::default();
    // Defaults
    params.scale.value = Some(1.0);
    params.alpha.value = Some(1.0);
    params.end.value = Some(1.0);

    for effect in effects {
        if effect.id == "com.alightcreative.effects.repeat.line" {
            for prop in &effect.properties {
                match prop.name.as_str() {
                    "count" => {
                        if !prop.keyframes.is_empty() {
                            params.count.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.count.value = Some(v);
                        }
                    }
                    "position" => {
                        if !prop.keyframes.is_empty() {
                            params.position.keyframes = prop.keyframes.clone();
                        } else {
                            let parts: Vec<&str> = prop.value.split(',').collect();
                            if parts.len() == 2
                                && let Ok(x) = parts[0].trim().parse::<f32>()
                                && let Ok(y) = parts[1].trim().parse::<f32>()
                            {
                                params.position.value = Some([x, y]);
                            }
                        }
                    }
                    "offset" => {
                        if !prop.keyframes.is_empty() {
                            params.offset.keyframes = prop.keyframes.clone();
                        } else {
                            let parts: Vec<&str> = prop.value.split(',').collect();
                            if parts.len() == 2
                                && let Ok(x) = parts[0].trim().parse::<f32>()
                                && let Ok(y) = parts[1].trim().parse::<f32>()
                            {
                                params.offset.value = Some([x, y]);
                            }
                        }
                    }
                    "angle" => {
                        if !prop.keyframes.is_empty() {
                            params.angle.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.angle.value = Some(v);
                        }
                    }
                    "scale" => {
                        if !prop.keyframes.is_empty() {
                            params.scale.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.scale.value = Some(v);
                        }
                    }
                    "alpha" => {
                        if !prop.keyframes.is_empty() {
                            params.alpha.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.alpha.value = Some(v);
                        }
                    }
                    "fillColor" => {
                        if !prop.keyframes.is_empty() {
                            params.fill_color.keyframes = prop
                                .keyframes
                                .iter()
                                .filter_map(|kf| {
                                    if let Ok(color) = crate::schema::parse_color(&kf.value) {
                                        Some(crate::schema::AmKeyframe {
                                            time: kf.time,
                                            value: format!(
                                                "{},{},{},{}",
                                                color[0], color[1], color[2], color[3]
                                            ),
                                            easing: kf.easing.clone(),
                                        })
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                        } else if let Ok(color) = crate::schema::parse_color(&prop.value) {
                            params.fill_color.value =
                                Some(Vec4::new(color[0], color[1], color[2], color[3]));
                        }
                    }
                    "blend" => {
                        if !prop.keyframes.is_empty() {
                            params.blend.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.blend.value = Some(v);
                        }
                    }
                    "colorAltCopies" => {
                        params.color_alt_copies = prop.value == "true";
                    }
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
                    "phase" => {
                        if !prop.keyframes.is_empty() {
                            params.phase.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.phase.value = Some(v);
                        }
                    }
                    "easeIn" => {
                        if !prop.keyframes.is_empty() {
                            params.ease_in.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.ease_in.value = Some(v);
                        }
                    }
                    "easeOut" => {
                        if !prop.keyframes.is_empty() {
                            params.ease_out.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.ease_out.value = Some(v);
                        }
                    }
                    "overlap" => {
                        if !prop.keyframes.is_empty() {
                            params.overlap.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.overlap.value = Some(v);
                        }
                    }
                    "shape" => {
                        if let Ok(v) = prop.value.parse::<i32>() {
                            params.shape = v;
                        }
                    }
                    "invert" => {
                        params.invert = prop.value == "true";
                    }
                    "randomOrder" => {
                        params.random_order = prop.value == "true";
                    }
                    "seed" => {
                        if let Ok(v) = prop.value.parse::<f32>() {
                            params.seed = v;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    params
}

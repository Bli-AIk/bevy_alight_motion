//! Swing, Threshold, Grid, Pixelate, and Simplex Displace effect parameter extraction.

use bevy::prelude::*;

use crate::schema::{AmAnimatedFloat, AmAnimatedVec2, AmEffect, AmKeyframe};

fn parse_vec2_value(value: &str) -> Option<[f32; 2]> {
    let parts: Vec<&str> = value.split(',').collect();
    if parts.len() != 2 {
        return None;
    }
    let x = parts[0].trim().parse::<f32>().ok()?;
    let y = parts[1].trim().parse::<f32>().ok()?;
    Some([x, y])
}

fn parse_color_keyframe(kf: &AmKeyframe) -> Option<AmKeyframe> {
    let color = crate::schema::parse_color(&kf.value).ok()?;
    Some(AmKeyframe {
        time: kf.time,
        value: format!("{},{},{},{}", color[0], color[1], color[2], color[3]),
        easing: kf.easing.clone(),
    })
}

/// Swing effect parameters
/// Creates oscillating rotation animation
/// 摇摆效果参数
/// 创建振荡旋转动画
#[derive(Debug, Clone, Default)]
pub struct SwingParams {
    /// Frequency of oscillation (oscillations per second)
    /// 振荡频率（每秒振荡次数）
    pub freq: AmAnimatedFloat,
    /// Minimum angle (degrees)
    /// 最小角度（度）
    pub a1: AmAnimatedFloat,
    /// Maximum angle (degrees)
    /// 最大角度（度）
    pub a2: AmAnimatedFloat,
    /// Phase offset (0.0-1.0)
    /// 相位偏移（0.0-1.0）
    pub phase: AmAnimatedFloat,
    /// Swing type (0 = sine, 1 = triangle, etc.)
    /// 摇摆类型（0 = 正弦，1 = 三角等）
    pub swing_type: i32,
}

impl SwingParams {
    /// Check if this has any swing effect parameters set
    /// 检查是否设置了任何摇摆效果参数
    #[allow(dead_code)]
    pub fn has_effect(&self) -> bool {
        self.freq.value.is_some()
            || !self.freq.keyframes.is_empty()
            || self.a1.value.is_some()
            || !self.a1.keyframes.is_empty()
            || self.a2.value.is_some()
            || !self.a2.keyframes.is_empty()
    }
}

/// Extract swing effect parameters from effects.
/// 从效果中提取摇摆效果参数
pub(crate) fn extract_swing_effect(effects: &[AmEffect]) -> SwingParams {
    let mut params = SwingParams::default();

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.swing2")
    else {
        return params;
    };

    // Default values (only set when effect exists)
    params.a1.value = Some(-30.0);
    params.a2.value = Some(30.0);
    params.freq.value = Some(1.0);

    for prop in &effect.properties {
        match prop.name.as_str() {
            "freq" => {
                if !prop.keyframes.is_empty() {
                    params.freq.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.freq.value = Some(v);
                }
            }
            "a1" => {
                if !prop.keyframes.is_empty() {
                    params.a1.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.a1.value = Some(v);
                }
            }
            "a2" => {
                if !prop.keyframes.is_empty() {
                    params.a2.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.a2.value = Some(v);
                }
            }
            "phase" => {
                if !prop.keyframes.is_empty() {
                    params.phase.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.phase.value = Some(v);
                }
            }
            "type" => {
                if let Ok(v) = prop.value.parse::<i32>() {
                    params.swing_type = v;
                }
            }
            _ => {}
        }
    }

    params
}

/// Extract spin effect RPM parameter from effects.
/// 从效果中提取旋转效果RPM参数
pub(crate) fn extract_spin_rpm(effects: &[AmEffect]) -> AmAnimatedFloat {
    let mut rpm = AmAnimatedFloat::default();
    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.spin")
    else {
        return rpm;
    };

    // Default RPM is 60 (from spin.xml)
    rpm.value = Some(60.0);
    for prop in &effect.properties {
        if prop.name == "rpm" {
            if !prop.keyframes.is_empty() {
                rpm.keyframes = prop.keyframes.clone();
            } else if let Ok(v) = prop.value.parse::<f32>() {
                rpm.value = Some(v);
            }
        }
    }
    rpm
}

/// Threshold effect parameters
/// Converts image to high-contrast black and white
/// 阈值效果参数
/// 将图像转换为高对比度黑白
#[derive(Debug, Clone, Default)]
pub struct ThresholdParams {
    /// Threshold value (0.0-1.0)
    /// 阈值（0.0-1.0）
    pub threshold: AmAnimatedFloat,
    /// Feather/softness (0.0-1.0)
    /// 羽化/柔和度（0.0-1.0）
    pub feather: AmAnimatedFloat,
    /// Invert the effect
    /// 反转效果
    pub invert: bool,
    /// Blend mode (0 = normal)
    /// 混合模式（0 = 正常）
    pub blend_mode: i32,
}

impl ThresholdParams {
    /// Check if this has any threshold effect parameters set
    /// 检查是否设置了任何阈值效果参数
    #[allow(dead_code)]
    pub fn has_effect(&self) -> bool {
        self.threshold.value.is_some() || !self.threshold.keyframes.is_empty()
    }
}

/// Extract threshold effect parameters from effects.
/// 从效果中提取阈值效果参数
pub(crate) fn extract_threshold_effect(effects: &[AmEffect]) -> ThresholdParams {
    let mut params = ThresholdParams::default();

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.threshold")
    else {
        return params;
    };

    // Effect found - set default value that may be overridden
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

/// Grid effect parameters
/// Overlays a grid pattern on the image
/// 网格效果参数
/// 在图像上叠加网格图案
#[derive(Debug, Clone, Default)]
pub struct GridParams {
    /// Grid position offset
    /// 网格位置偏移
    pub position: AmAnimatedVec2,
    /// Grid spacing (0.0-1.0)
    /// 网格间距（0.0-1.0）
    pub spacing: AmAnimatedFloat,
    /// Line width (0.0-1.0)
    /// 线宽（0.0-1.0）
    pub width: AmAnimatedFloat,
    /// Grid color
    /// 网格颜色
    pub color: crate::schema::AmAnimatedColor,
    /// Punchout mode (creates holes instead of lines)
    /// 打孔模式（创建孔洞而不是线条）
    pub punchout: bool,
    /// Smoothing/anti-aliasing
    /// 平滑/抗锯齿
    pub smoothing: AmAnimatedFloat,
    /// Screen space mode
    /// 屏幕空间模式
    pub screen_space: bool,
}

impl GridParams {
    /// Check if this has any grid effect parameters set
    /// 检查是否设置了任何网格效果参数
    #[allow(dead_code)]
    pub fn has_effect(&self) -> bool {
        self.spacing.value.is_some() || !self.spacing.keyframes.is_empty()
    }
}

/// Extract grid effect parameters from effects.
/// 从效果中提取网格效果参数
pub(crate) fn extract_grid_effect(effects: &[AmEffect]) -> GridParams {
    let mut params = GridParams::default();

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.grid2")
    else {
        return params;
    };

    // Grid effect found - set default values that may be overridden
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

/// Pixelate effect parameters
/// Reduces image resolution to create a pixelated effect
/// 像素化效果参数
/// 降低图像分辨率以创建像素化效果
#[derive(Debug, Clone, Default)]
pub struct PixelateParams {
    /// Pixel block size (larger = more pixelated)
    /// 像素块大小（越大越像素化）
    pub size: AmAnimatedFloat,
    /// Stretch factor for X and Y axes
    /// X和Y轴的拉伸系数
    pub stretch: AmAnimatedVec2,
    /// Rotation angle of the pixel grid (degrees)
    /// 像素网格的旋转角度（度）
    pub angle: AmAnimatedFloat,
    /// Vignette darkening effect (0 = none, 1 = full)
    /// 暗角效果（0 = 无，1 = 完全）
    pub vignette: AmAnimatedFloat,
    /// Threshold for color posterization
    /// 颜色色调分离的阈值
    pub threshold: AmAnimatedFloat,
    /// Saturation adjustment (1 = normal)
    /// 饱和度调整（1 = 正常）
    pub saturation: AmAnimatedFloat,
    /// Use screen-space coordinates
    /// 使用屏幕空间坐标
    pub screen_space: bool,
}

impl PixelateParams {
    /// Check if this has any pixelate effect parameters set
    /// 检查是否设置了任何像素化效果参数
    #[allow(dead_code)]
    pub fn has_effect(&self) -> bool {
        self.size.value.is_some() || !self.size.keyframes.is_empty()
    }
}

/// Oscillate effect parameters (com.alightcreative.effects.oscillate3)
/// Moves the layer position back and forth repeatedly
/// 振荡效果参数
/// 使图层位置来回反复移动
#[derive(Debug, Clone, Default)]
pub struct OscillateParams {
    /// Direction mode (0=angle, 1=depth/z, 2=orbit)
    pub direction: i32,
    /// Movement angle (degrees)
    pub angle: AmAnimatedFloat,
    /// Oscillation frequency (Hz)
    pub freq: AmAnimatedFloat,
    /// Movement magnitude (pixels)
    pub mag: AmAnimatedFloat,
    /// Wave type (0=sine, 1=triangle)
    pub wave_type: i32,
    /// Phase offset
    pub phase: AmAnimatedFloat,
}

/// Extract oscillate3 effect parameters from effects.
/// 从效果中提取振荡效果参数
pub(crate) fn extract_oscillate_effect(effects: &[AmEffect]) -> OscillateParams {
    let mut params = OscillateParams::default();

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.oscillate3")
    else {
        return params;
    };

    // Default values from oscillate3.xml
    params.angle.value = Some(45.0);
    params.freq.value = Some(2.0);
    params.mag.value = Some(25.0);

    for prop in &effect.properties {
        match prop.name.as_str() {
            "direction" => {
                if let Ok(v) = prop.value.parse::<i32>() {
                    params.direction = v;
                }
            }
            "angle" => {
                if !prop.keyframes.is_empty() {
                    params.angle.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.angle.value = Some(v);
                }
            }
            "freq" => {
                if !prop.keyframes.is_empty() {
                    params.freq.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.freq.value = Some(v);
                }
            }
            "mag" => {
                if !prop.keyframes.is_empty() {
                    params.mag.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.mag.value = Some(v);
                }
            }
            "type" => {
                if let Ok(v) = prop.value.parse::<i32>() {
                    params.wave_type = v;
                }
            }
            "phase" => {
                if !prop.keyframes.is_empty() {
                    params.phase.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.phase.value = Some(v);
                }
            }
            _ => {}
        }
    }

    params
}

/// Jitter effect parameters (com.alightcreative.effects.jitter)
/// Simplex noise-based position displacement.
/// 抖动效果参数 - 基于 simplex 噪声的位置位移
#[derive(Debug, Clone)]
pub struct JitterParams {
    /// Movement angle (degrees) - may be keyframed
    pub angle: AmAnimatedFloat,
    /// Frequency (steps per second) - may be keyframed
    pub freq: AmAnimatedFloat,
    /// Magnitude (pixels) - may be keyframed
    pub mag: AmAnimatedFloat,
    /// Noise seed - may be keyframed
    pub seed: AmAnimatedFloat,
    /// Perpendicular slack (0.0-1.0) - may be keyframed
    pub slack: AmAnimatedFloat,
    /// Z-axis jitter magnitude - may be keyframed
    pub zjitter: AmAnimatedFloat,
    /// Whether the effect is present
    pub enabled: bool,
}

impl Default for JitterParams {
    fn default() -> Self {
        Self {
            angle: AmAnimatedFloat {
                value: Some(45.0),
                keyframes: Vec::new(),
            },
            freq: AmAnimatedFloat {
                value: Some(30.0),
                keyframes: Vec::new(),
            },
            mag: AmAnimatedFloat {
                value: Some(25.0),
                keyframes: Vec::new(),
            },
            seed: AmAnimatedFloat {
                value: Some(0.0),
                keyframes: Vec::new(),
            },
            slack: AmAnimatedFloat {
                value: Some(0.0),
                keyframes: Vec::new(),
            },
            zjitter: AmAnimatedFloat {
                value: Some(0.0),
                keyframes: Vec::new(),
            },
            enabled: false,
        }
    }
}

/// Extract jitter effect parameters from effects.
/// 从效果中提取抖动效果参数
pub(crate) fn extract_jitter_effect(effects: &[AmEffect]) -> JitterParams {
    let mut params = JitterParams::default();

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.jitter")
    else {
        return params;
    };

    params.enabled = true;

    // Helper to parse a property as AmAnimatedFloat (with keyframe support)
    fn parse_animated_float(prop: &crate::schema::AmProperty, default: f32) -> AmAnimatedFloat {
        if !prop.keyframes.is_empty() {
            AmAnimatedFloat {
                value: prop.value.parse::<f32>().ok().or(Some(default)),
                keyframes: prop.keyframes.clone(),
            }
        } else if let Ok(v) = prop.value.parse::<f32>() {
            AmAnimatedFloat {
                value: Some(v),
                keyframes: Vec::new(),
            }
        } else {
            AmAnimatedFloat {
                value: Some(default),
                keyframes: Vec::new(),
            }
        }
    }

    for prop in &effect.properties {
        match prop.name.as_str() {
            "angle" => params.angle = parse_animated_float(prop, 45.0),
            "freq" => params.freq = parse_animated_float(prop, 30.0),
            "mag" => params.mag = parse_animated_float(prop, 25.0),
            "seed" => params.seed = parse_animated_float(prop, 0.0),
            "slack" => params.slack = parse_animated_float(prop, 0.0),
            "zjitter" => params.zjitter = parse_animated_float(prop, 0.0),
            _ => {}
        }
    }

    params
}

/// Echo keyframe effect parameters (com.alightcreative.effects.repeat.echokf)
/// Creates time-shifted echo copies of an element.
/// 回声关键帧效果参数 - 创建元素的时移回声副本
#[derive(Debug, Clone)]
pub struct EchokfParams {
    /// Time spacing per echo (seconds) - may be keyframed
    pub seconds: AmAnimatedFloat,
    /// Number of echo copies - may be keyframed
    pub count: AmAnimatedFloat,
    /// Alpha keyframes for echo fade (evaluated at element's time)
    pub alpha: AmAnimatedFloat,
    /// Composite mode: 0=atop (echoes on top), 1=behind (echoes behind)
    pub mode: i32,
    /// Whether the effect is present
    pub enabled: bool,
}

impl Default for EchokfParams {
    fn default() -> Self {
        Self {
            seconds: AmAnimatedFloat {
                value: Some(0.5),
                keyframes: Vec::new(),
            },
            count: AmAnimatedFloat {
                value: Some(1.0),
                keyframes: Vec::new(),
            },
            alpha: AmAnimatedFloat::default(),
            mode: 1,
            enabled: false,
        }
    }
}

impl EchokfParams {
    /// Get max count (for spawning the right number of echoes).
    pub fn max_count(&self) -> u32 {
        if self.count.keyframes.is_empty() {
            self.count.value.unwrap_or(1.0) as u32
        } else {
            // Find maximum value across all keyframes
            let kf_max = self
                .count
                .keyframes
                .iter()
                .filter_map(|kf| kf.value.parse::<f32>().ok())
                .fold(f32::NEG_INFINITY, f32::max);
            let max = self.count.value.unwrap_or(0.0).max(kf_max);
            max.ceil() as u32
        }
    }

    /// Get static seconds value (fallback for non-keyframed case).
    pub fn static_seconds(&self) -> f32 {
        self.seconds.value.unwrap_or(0.5)
    }

    /// Whether count or seconds are keyframed (need runtime updates).
    pub fn is_dynamic(&self) -> bool {
        !self.count.keyframes.is_empty() || !self.seconds.keyframes.is_empty()
    }
}

/// Extract echokf effect parameters from effects.
/// 从效果中提取回声关键帧效果参数
pub(crate) fn extract_echokf_effect(effects: &[AmEffect]) -> EchokfParams {
    let mut params = EchokfParams::default();

    let effect = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.repeat.echokf");
    let Some(effect) = effect else {
        return params;
    };

    params.enabled = true;

    for prop in &effect.properties {
        match prop.name.as_str() {
            "seconds" => {
                if !prop.keyframes.is_empty() {
                    params.seconds = AmAnimatedFloat {
                        value: prop.value.parse::<f32>().ok(),
                        keyframes: prop.keyframes.clone(),
                    };
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.seconds = AmAnimatedFloat {
                        value: Some(v),
                        keyframes: Vec::new(),
                    };
                }
            }
            "count" => {
                if !prop.keyframes.is_empty() {
                    params.count = AmAnimatedFloat {
                        value: prop.value.parse::<f32>().ok(),
                        keyframes: prop.keyframes.clone(),
                    };
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.count = AmAnimatedFloat {
                        value: Some(v),
                        keyframes: Vec::new(),
                    };
                }
            }
            "alpha" => {
                if !prop.keyframes.is_empty() {
                    params.alpha = AmAnimatedFloat {
                        value: prop.value.parse::<f32>().ok(),
                        keyframes: prop.keyframes.clone(),
                    };
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.alpha = AmAnimatedFloat {
                        value: Some(v),
                        keyframes: Vec::new(),
                    };
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

/// Solid color effect parameters (com.alightcreative.solidcolor)
/// Overlays a solid color on the content
/// 纯色效果参数
/// 在内容上叠加纯色
#[derive(Debug, Clone, Default)]
pub struct SolidColorParams {
    /// Overlay color
    pub color: crate::schema::AmAnimatedColor,
    /// Alpha/mix amount (0.0-1.0)
    pub alpha: AmAnimatedFloat,
    /// Blend mode (0=normal, 1=multiply, 2=screen)
    pub blend_mode: i32,
}

/// Extract solid color effect parameters from effects.
/// 从效果中提取纯色效果参数
pub(crate) fn extract_solid_color_effect(effects: &[AmEffect]) -> SolidColorParams {
    let mut params = SolidColorParams::default();

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.solidcolor")
    else {
        return params;
    };

    // Default values from solidcolor.xml
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

/// Extract pixelate effect parameters from effects.
/// 从效果中提取像素化效果参数
pub(crate) fn extract_pixelate_effect(effects: &[AmEffect]) -> PixelateParams {
    let mut params = PixelateParams::default();

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.pixelate2")
    else {
        return params;
    };

    // Effect found - set default values that may be overridden
    params.size.value = Some(10.0); // Default pixel size
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

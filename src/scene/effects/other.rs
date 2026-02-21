//! Swing, Threshold, Grid, and Pixelate effect parameter extraction.

use bevy::prelude::*;

use crate::schema::{AmAnimatedFloat, AmAnimatedVec2, AmEffect};

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

    // Check if swing effect exists before setting defaults
    let has_swing = effects
        .iter()
        .any(|e| e.id == "com.alightcreative.effects.swing2");
    if !has_swing {
        return params;
    }

    // Default values (only set when effect exists)
    params.a1.value = Some(-30.0);
    params.a2.value = Some(30.0);
    params.freq.value = Some(1.0);

    for effect in effects {
        if effect.id == "com.alightcreative.effects.swing2" {
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
        }
    }

    params
}

/// Extract spin effect RPM parameter from effects.
/// 从效果中提取旋转效果RPM参数
pub(crate) fn extract_spin_rpm(effects: &[AmEffect]) -> AmAnimatedFloat {
    let mut rpm = AmAnimatedFloat::default();
    for effect in effects {
        if effect.id == "com.alightcreative.effects.spin" {
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
            break;
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
    // Only set default value if the effect is present
    // params.threshold.value stays None until threshold effect is found

    for effect in effects {
        if effect.id == "com.alightcreative.effects.threshold" {
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
    // Only set default values when grid effect is actually present

    for effect in effects {
        if effect.id == "com.alightcreative.effects.grid2" {
            // Grid effect found - set default values that may be overridden
            params.spacing.value = Some(0.1);
            params.width.value = Some(0.01);
            params.smoothing.value = Some(0.05);

            for prop in &effect.properties {
                match prop.name.as_str() {
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
                            params.color.value =
                                Some(Vec4::new(color[0], color[1], color[2], color[3]));
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

    let has_effect = effects
        .iter()
        .any(|e| e.id == "com.alightcreative.effects.oscillate3");
    if !has_effect {
        return params;
    }

    // Default values from oscillate3.xml
    params.angle.value = Some(45.0);
    params.freq.value = Some(2.0);
    params.mag.value = Some(25.0);

    for effect in effects {
        if effect.id == "com.alightcreative.effects.oscillate3" {
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

    let has_effect = effects
        .iter()
        .any(|e| e.id == "com.alightcreative.solidcolor");
    if !has_effect {
        return params;
    }

    // Default values from solidcolor.xml
    params.alpha.value = Some(1.0);
    params.color.value = Some(Vec4::new(
        0x2D as f32 / 255.0,
        0x1E as f32 / 255.0,
        0xF6 as f32 / 255.0,
        1.0,
    ));

    for effect in effects {
        if effect.id == "com.alightcreative.solidcolor" {
            for prop in &effect.properties {
                match prop.name.as_str() {
                    "color" => {
                        if !prop.keyframes.is_empty() {
                            params.color.keyframes = prop
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
                            params.color.value =
                                Some(Vec4::new(color[0], color[1], color[2], color[3]));
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
        }
    }

    params
}

/// Extract pixelate effect parameters from effects.
/// 从效果中提取像素化效果参数
pub(crate) fn extract_pixelate_effect(effects: &[AmEffect]) -> PixelateParams {
    let mut params = PixelateParams::default();

    for effect in effects {
        if effect.id == "com.alightcreative.effects.pixelate2" {
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
                        } else {
                            let parts: Vec<&str> = prop.value.split(',').collect();
                            if parts.len() == 2
                                && let Ok(x) = parts[0].trim().parse::<f32>()
                                && let Ok(y) = parts[1].trim().parse::<f32>()
                            {
                                params.stretch.value = Some([x, y]);
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
        }
    }

    params
}

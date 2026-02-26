//! Repeat, Linear Repeat, Radial Repeat, and Path Repeat effect parameter extraction.

use bevy::prelude::*;

use crate::schema::{AmAnimatedColor, AmAnimatedFloat, AmAnimatedVec2, AmEffect};

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
    pub seed: AmAnimatedFloat,
}

impl LinearRepeatParams {
    // No methods needed currently - kept for potential future use
}

/// Parse a single linear repeat effect's properties into a LinearRepeatParams.
fn parse_linear_repeat_properties(effect: &AmEffect) -> LinearRepeatParams {
    let mut params = LinearRepeatParams::default();
    params.scale.value = Some(1.0);
    params.alpha.value = Some(1.0);
    params.end.value = Some(1.0);

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
                if !prop.keyframes.is_empty() {
                    params.seed.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.seed.value = Some(v);
                }
            }
            _ => {}
        }
    }

    params
}

/// Extract linear repeat effect parameters from effects (supports up to 2 stacked effects).
/// 从效果中提取线性重复效果参数（支持最多2个叠加效果）
pub(crate) fn extract_linear_repeat_effects(
    effects: &[AmEffect],
) -> (LinearRepeatParams, Option<LinearRepeatParams>) {
    let mut found: Vec<LinearRepeatParams> = Vec::new();

    for effect in effects {
        if effect.id == "com.alightcreative.effects.repeat.line" {
            found.push(parse_linear_repeat_properties(effect));
        }
    }

    let second = if found.len() > 1 {
        Some(found.remove(1))
    } else {
        None
    };
    let first = if !found.is_empty() {
        found.remove(0)
    } else {
        let mut p = LinearRepeatParams::default();
        p.scale.value = Some(1.0);
        p.alpha.value = Some(1.0);
        p.end.value = Some(1.0);
        p
    };

    (first, second)
}

/// Radial Repeat effect parameters.
/// Arranges copies in a circular pattern around a pivot point.
#[derive(Debug, Clone, Default)]
pub struct RadialRepeatParams {
    pub count: AmAnimatedFloat,
    pub radius: AmAnimatedFloat,
    pub orientation: AmAnimatedFloat,
    pub start_angle: AmAnimatedFloat,
    pub sweep: AmAnimatedFloat,
    pub base_scale: AmAnimatedFloat,
    pub offset: AmAnimatedVec2,
    pub angle: AmAnimatedFloat,
    pub scale: AmAnimatedFloat,
    pub alpha: AmAnimatedFloat,
    pub fill_color: AmAnimatedColor,
    pub blend: AmAnimatedFloat,
    pub color_alt_copies: bool,
    pub start: AmAnimatedFloat,
    pub end: AmAnimatedFloat,
    pub phase: AmAnimatedFloat,
    pub ease_in: AmAnimatedFloat,
    pub ease_out: AmAnimatedFloat,
    pub overlap: AmAnimatedFloat,
    pub shape: i32,
    pub invert: bool,
    pub random_order: bool,
    pub seed: f32,
}

/// Extract radial repeat effect parameters from effects.
pub(crate) fn extract_radial_repeat_effect(effects: &[AmEffect]) -> RadialRepeatParams {
    let mut params = RadialRepeatParams::default();
    params.base_scale.value = Some(1.0);
    params.scale.value = Some(1.0);
    params.alpha.value = Some(1.0);
    params.end.value = Some(1.0);
    params.sweep.value = Some(360.0);

    for effect in effects {
        if effect.id == "com.alightcreative.effects.repeat.radial" {
            for prop in &effect.properties {
                match prop.name.as_str() {
                    "count" => {
                        if !prop.keyframes.is_empty() {
                            params.count.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.count.value = Some(v);
                        }
                    }
                    "radius" => {
                        if !prop.keyframes.is_empty() {
                            params.radius.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.radius.value = Some(v);
                        }
                    }
                    "orientation" => {
                        if !prop.keyframes.is_empty() {
                            params.orientation.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.orientation.value = Some(v);
                        }
                    }
                    "startAngle" => {
                        if !prop.keyframes.is_empty() {
                            params.start_angle.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.start_angle.value = Some(v);
                        }
                    }
                    "sweep" => {
                        if !prop.keyframes.is_empty() {
                            params.sweep.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.sweep.value = Some(v);
                        }
                    }
                    "baseScale" => {
                        if !prop.keyframes.is_empty() {
                            params.base_scale.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.base_scale.value = Some(v);
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

/// Path Repeat effect parameters (com.alightcreative.effects.repeat.path)
/// Places copies of the element along the outline of the previous element in the scene.
#[derive(Debug, Clone, Default)]
pub struct PathRepeatParams {
    pub count: AmAnimatedFloat,
    pub start_pos: AmAnimatedFloat,
    pub end_pos: AmAnimatedFloat,
    pub path_phase: AmAnimatedFloat,
    pub tangent: bool,
    pub offset: AmAnimatedVec2,
    pub angle: AmAnimatedFloat,
    pub scale: AmAnimatedFloat,
    pub alpha: AmAnimatedFloat,
    pub fill_color: AmAnimatedColor,
    pub blend: AmAnimatedFloat,
    pub color_alt_copies: bool,
    // Easing params (shared with other repeat effects)
    pub start: AmAnimatedFloat,
    pub end: AmAnimatedFloat,
    pub phase: AmAnimatedFloat,
    pub ease_in: AmAnimatedFloat,
    pub ease_out: AmAnimatedFloat,
    pub overlap: AmAnimatedFloat,
    pub shape: i32,
    pub invert: bool,
    pub random_order: bool,
    pub seed: f32,
}

impl PathRepeatParams {
    /// Check if this has any path repeat effect parameters set.
    pub fn has_effect(&self) -> bool {
        self.count.value.is_some_and(|v| v > 0.0) || !self.count.keyframes.is_empty()
    }
}

/// Extract path repeat effect parameters from effects.
pub(crate) fn extract_path_repeat_effect(effects: &[AmEffect]) -> PathRepeatParams {
    let mut params = PathRepeatParams::default();
    params.scale.value = Some(1.0);
    params.alpha.value = Some(1.0);
    params.end_pos.value = Some(1.0);
    params.end.value = Some(1.0);

    for effect in effects {
        if effect.id == "com.alightcreative.effects.repeat.path" {
            for prop in &effect.properties {
                match prop.name.as_str() {
                    "count" => {
                        if !prop.keyframes.is_empty() {
                            params.count.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.count.value = Some(v);
                        }
                    }
                    "startPos" => {
                        if !prop.keyframes.is_empty() {
                            params.start_pos.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.start_pos.value = Some(v);
                        }
                    }
                    "endPos" => {
                        if !prop.keyframes.is_empty() {
                            params.end_pos.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.end_pos.value = Some(v);
                        }
                    }
                    "pathPhase" => {
                        if !prop.keyframes.is_empty() {
                            params.path_phase.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.path_phase.value = Some(v);
                        }
                    }
                    "tangent" => {
                        params.tangent = prop.value == "true";
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

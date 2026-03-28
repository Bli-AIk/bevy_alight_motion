//! Repeat, Linear Repeat, Radial Repeat, and Path Repeat effect parameter extraction.

use bevy::prelude::*;

use crate::schema::{
    AmAnimatedColor, AmAnimatedFloat, AmAnimatedVec2, AmEffect, AmKeyframe, AmProperty,
};

/// Apply keyframes-or-static-value pattern for a float property.
fn apply_animated_float(prop: &AmProperty, target: &mut AmAnimatedFloat) {
    if !prop.keyframes.is_empty() {
        target.keyframes = prop.keyframes.clone();
    } else if let Ok(v) = prop.value.parse::<f32>() {
        target.value = Some(v);
    }
}

/// Apply keyframes-or-static-value pattern for a vec2 property.
fn apply_animated_vec2(prop: &AmProperty, target: &mut AmAnimatedVec2) {
    if !prop.keyframes.is_empty() {
        target.keyframes = prop.keyframes.clone();
    } else if let Ok(v) = crate::schema::parse_vec2(&prop.value) {
        target.value = Some(v);
    }
}

/// Convert a color keyframe string into a parsed keyframe.
fn map_color_keyframe(kf: &AmKeyframe) -> Option<AmKeyframe> {
    let c = crate::schema::parse_color(&kf.value).ok()?;
    Some(AmKeyframe {
        time: kf.time,
        value: format!("{},{},{},{}", c[0], c[1], c[2], c[3]),
        easing: kf.easing.clone(),
    })
}

/// Apply keyframes-or-static-value pattern for a color property.
fn apply_animated_color(prop: &AmProperty, target: &mut AmAnimatedColor) {
    if !prop.keyframes.is_empty() {
        target.keyframes = prop
            .keyframes
            .iter()
            .filter_map(map_color_keyframe)
            .collect();
    } else if let Ok(c) = crate::schema::parse_color(&prop.value) {
        target.value = Some(Vec4::new(c[0], c[1], c[2], c[3]));
    }
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
        if effect.id != "com.alightcreative.effects.repeat" {
            continue;
        }
        for prop in &effect.properties {
            match prop.name.as_str() {
                "count" => apply_animated_float(prop, &mut params.count),
                "time" => apply_animated_float(prop, &mut params.time),
                "offset" => apply_animated_vec2(prop, &mut params.offset),
                "angle" => apply_animated_float(prop, &mut params.angle),
                "scale" => apply_animated_float(prop, &mut params.scale),
                "alpha" => apply_animated_float(prop, &mut params.alpha),
                _ => {}
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
            "count" => apply_animated_float(prop, &mut params.count),
            "position" => apply_animated_vec2(prop, &mut params.position),
            "offset" => apply_animated_vec2(prop, &mut params.offset),
            "angle" => apply_animated_float(prop, &mut params.angle),
            "scale" => apply_animated_float(prop, &mut params.scale),
            "alpha" => apply_animated_float(prop, &mut params.alpha),
            "fillColor" => apply_animated_color(prop, &mut params.fill_color),
            "blend" => apply_animated_float(prop, &mut params.blend),
            "colorAltCopies" => params.color_alt_copies = prop.value == "true",
            "start" => apply_animated_float(prop, &mut params.start),
            "end" => apply_animated_float(prop, &mut params.end),
            "phase" => apply_animated_float(prop, &mut params.phase),
            "easeIn" => apply_animated_float(prop, &mut params.ease_in),
            "easeOut" => apply_animated_float(prop, &mut params.ease_out),
            "overlap" => apply_animated_float(prop, &mut params.overlap),
            "shape" => {
                if let Ok(v) = prop.value.parse::<i32>() {
                    params.shape = v;
                }
            }
            "invert" => params.invert = prop.value == "true",
            "randomOrder" => params.random_order = prop.value == "true",
            "seed" => apply_animated_float(prop, &mut params.seed),
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

/// Returns whether the first `repeat.line` effect appears after a `stretchsegment`.
///
/// AM applies repeat to the current scene element state, so when stretchsegment
/// precedes repeat.line the repeat source should use the stretched mesh bounds.
pub(crate) fn linear_repeat_after_stretch_segment(effects: &[AmEffect]) -> bool {
    let mut seen_stretch_segment = false;

    for effect in effects {
        match effect.id.as_str() {
            "com.alightcreative.effects.stretchsegment" => {
                seen_stretch_segment = true;
            }
            "com.alightcreative.effects.repeat.line" => {
                return seen_stretch_segment;
            }
            _ => {}
        }
    }

    false
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
        if effect.id != "com.alightcreative.effects.repeat.radial" {
            continue;
        }
        for prop in &effect.properties {
            match prop.name.as_str() {
                "count" => apply_animated_float(prop, &mut params.count),
                "radius" => apply_animated_float(prop, &mut params.radius),
                "orientation" => apply_animated_float(prop, &mut params.orientation),
                "startAngle" => apply_animated_float(prop, &mut params.start_angle),
                "sweep" => apply_animated_float(prop, &mut params.sweep),
                "baseScale" => apply_animated_float(prop, &mut params.base_scale),
                "offset" => apply_animated_vec2(prop, &mut params.offset),
                "angle" => apply_animated_float(prop, &mut params.angle),
                "scale" => apply_animated_float(prop, &mut params.scale),
                "alpha" => apply_animated_float(prop, &mut params.alpha),
                "fillColor" => apply_animated_color(prop, &mut params.fill_color),
                "blend" => apply_animated_float(prop, &mut params.blend),
                "colorAltCopies" => params.color_alt_copies = prop.value == "true",
                "start" => apply_animated_float(prop, &mut params.start),
                "end" => apply_animated_float(prop, &mut params.end),
                "phase" => apply_animated_float(prop, &mut params.phase),
                "easeIn" => apply_animated_float(prop, &mut params.ease_in),
                "easeOut" => apply_animated_float(prop, &mut params.ease_out),
                "overlap" => apply_animated_float(prop, &mut params.overlap),
                "shape" => params.shape = prop.value.parse().unwrap_or(params.shape),
                "invert" => params.invert = prop.value == "true",
                "randomOrder" => params.random_order = prop.value == "true",
                "seed" => params.seed = prop.value.parse().unwrap_or(params.seed),
                _ => {}
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
        if effect.id != "com.alightcreative.effects.repeat.path" {
            continue;
        }
        for prop in &effect.properties {
            match prop.name.as_str() {
                "count" => apply_animated_float(prop, &mut params.count),
                "startPos" => apply_animated_float(prop, &mut params.start_pos),
                "endPos" => apply_animated_float(prop, &mut params.end_pos),
                "pathPhase" => apply_animated_float(prop, &mut params.path_phase),
                "tangent" => params.tangent = prop.value == "true",
                "offset" => apply_animated_vec2(prop, &mut params.offset),
                "angle" => apply_animated_float(prop, &mut params.angle),
                "scale" => apply_animated_float(prop, &mut params.scale),
                "alpha" => apply_animated_float(prop, &mut params.alpha),
                "fillColor" => apply_animated_color(prop, &mut params.fill_color),
                "blend" => apply_animated_float(prop, &mut params.blend),
                "colorAltCopies" => params.color_alt_copies = prop.value == "true",
                "start" => apply_animated_float(prop, &mut params.start),
                "end" => apply_animated_float(prop, &mut params.end),
                "phase" => apply_animated_float(prop, &mut params.phase),
                "easeIn" => apply_animated_float(prop, &mut params.ease_in),
                "easeOut" => apply_animated_float(prop, &mut params.ease_out),
                "overlap" => apply_animated_float(prop, &mut params.overlap),
                "shape" => params.shape = prop.value.parse().unwrap_or(params.shape),
                "invert" => params.invert = prop.value == "true",
                "randomOrder" => params.random_order = prop.value == "true",
                "seed" => params.seed = prop.value.parse().unwrap_or(params.seed),
                _ => {}
            }
        }
    }

    params
}

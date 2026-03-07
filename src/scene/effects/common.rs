//! Common effect parameter extraction (Wipe, Stretch, Stretch2, Blur, PaletteMap, ReplaceColor, ScaleAssist).

use bevy::prelude::*;

use crate::schema::{AmAnimatedColor, AmAnimatedFloat, AmEffect, AmKeyframe, AmProperty};

/// Effect IDs for transform variants.
const TRANSFORM2_ID: &str = "com.alightcreative.effects.transform2";
/// Legacy transform effect (older Alight Motion versions).
/// Uses different property names: offset(vec2) → posx/posy, scale → posz.
const TRANSFORM_LEGACY_ID: &str = "com.alightcreative.effects.transform";

fn is_transform_effect(id: &str) -> bool {
    id == TRANSFORM2_ID || id == TRANSFORM_LEGACY_ID
}

/// All extracted transform2 effect parameters.
#[derive(Debug, Clone, Default)]
pub struct Transform2Params {
    pub pos_x: AmAnimatedFloat,
    pub pos_y: AmAnimatedFloat,
    pub pos_z: AmAnimatedFloat,
    pub angle: AmAnimatedFloat,
    pub xinv: bool,
    pub yinv: bool,
    pub zinv: bool,
    pub ainv: bool,
}

/// Parse a single transform2/transform effect into `Transform2Params`.
///
/// Handles both transform2 (modern: posx/posy/posz/angle) and legacy transform
/// (older: offset(vec2)/scale/angle). Legacy properties are mapped:
///   offset.x → posx, offset.y → posy, scale → posz.
fn parse_transform_params(effect: &AmEffect) -> Transform2Params {
    let is_legacy = effect.id == TRANSFORM_LEGACY_ID;
    let mut params = Transform2Params::default();

    for prop in &effect.properties {
        match prop.name.as_str() {
            "posx" if !is_legacy => {
                if !prop.keyframes.is_empty() {
                    params.pos_x.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.pos_x.value = Some(v);
                }
            }
            "posy" if !is_legacy => {
                if !prop.keyframes.is_empty() {
                    params.pos_y.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.pos_y.value = Some(v);
                }
            }
            // Legacy transform uses "offset" (vec2) instead of separate posx/posy.
            // Split vec2 "x,y" values into individual float keyframes.
            "offset" if is_legacy => {
                if !prop.keyframes.is_empty() {
                    let (kf_x, kf_y) = split_vec2_keyframes(&prop.keyframes);
                    params.pos_x.keyframes = kf_x;
                    params.pos_y.keyframes = kf_y;
                } else if let Some((x, y)) = parse_vec2_value(&prop.value) {
                    params.pos_x.value = Some(x);
                    params.pos_y.value = Some(y);
                }
            }
            "posz" if !is_legacy => {
                if !prop.keyframes.is_empty() {
                    params.pos_z.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.pos_z.value = Some(v);
                }
            }
            // Legacy transform uses "scale" instead of "posz".
            "scale" if is_legacy => {
                if !prop.keyframes.is_empty() {
                    params.pos_z.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.pos_z.value = Some(v);
                }
            }
            "angle" => {
                if !prop.keyframes.is_empty() {
                    params.angle.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.angle.value = Some(v);
                }
            }
            "xinv" => params.xinv = prop.value == "true",
            "yinv" => params.yinv = prop.value == "true",
            "zinv" => params.zinv = prop.value == "true",
            "ainv" => params.ainv = prop.value == "true",
            // Legacy-only properties (alpha, fill, maskToLayer, sample) are ignored.
            _ => {}
        }
    }

    params
}

/// Split vec2 keyframes ("x,y" values) into separate x and y float keyframes.
fn split_vec2_keyframes(keyframes: &[AmKeyframe]) -> (Vec<AmKeyframe>, Vec<AmKeyframe>) {
    let mut kf_x = Vec::with_capacity(keyframes.len());
    let mut kf_y = Vec::with_capacity(keyframes.len());

    for kf in keyframes {
        let (x_str, y_str) = match kf.value.split_once(',') {
            Some((x, y)) => (x.trim().to_string(), y.trim().to_string()),
            None => (kf.value.clone(), "0.0".to_string()),
        };

        kf_x.push(AmKeyframe {
            time: kf.time,
            value: x_str,
            easing: kf.easing.clone(),
        });
        kf_y.push(AmKeyframe {
            time: kf.time,
            value: y_str,
            easing: kf.easing.clone(),
        });
    }

    (kf_x, kf_y)
}

/// Parse a "x,y" string into (f32, f32).
fn parse_vec2_value(value: &str) -> Option<(f32, f32)> {
    let (x_str, y_str) = value.split_once(',')?;
    let x = x_str.trim().parse::<f32>().ok()?;
    let y = y_str.trim().parse::<f32>().ok()?;
    Some((x, y))
}

/// Apply property value or keyframes to an `AmAnimatedFloat`.
fn apply_animated_float(target: &mut AmAnimatedFloat, prop: &AmProperty) {
    if !prop.keyframes.is_empty() {
        target.keyframes = prop.keyframes.clone();
    } else if let Ok(v) = prop.value.parse::<f32>() {
        target.value = Some(v);
    }
}

/// Parse a color string into a `Vec4` (RGBA).
fn parse_color_vec4(value: &str) -> Option<Vec4> {
    let c = crate::schema::parse_color(value).ok()?;
    Some(Vec4::new(c[0], c[1], c[2], c[3]))
}

/// Apply a custom palette color property (color1-color8).
fn apply_custom_color(colors: &mut [Vec4; 8], name: &str, value: &str) {
    let Some(index_str) = name.strip_prefix("color") else {
        return;
    };
    let Ok(index) = index_str.parse::<usize>() else {
        return;
    };
    if !(1..=8).contains(&index) {
        return;
    }
    let Some(color) = parse_color_vec4(value) else {
        return;
    };
    colors[index - 1] = color;
}

/// Apply property value or keyframes to an `AmAnimatedColor`.
fn apply_animated_color(target: &mut AmAnimatedColor, prop: &AmProperty) {
    if !prop.keyframes.is_empty() {
        target.keyframes = prop
            .keyframes
            .iter()
            .filter_map(|kf| {
                let c = crate::schema::parse_color(&kf.value).ok()?;
                Some(AmKeyframe {
                    time: kf.time,
                    value: format!("{},{},{},{}", c[0], c[1], c[2], c[3]),
                    easing: kf.easing.clone(),
                })
            })
            .collect();
    } else if let Some(color) = parse_color_vec4(&prop.value) {
        target.value = Some(color);
    }
}

#[allow(dead_code)]
pub(crate) fn extract_effect_animations(effects: &[AmEffect]) -> Transform2Params {
    for effect in effects {
        if is_transform_effect(&effect.id) {
            return parse_transform_params(effect);
        }
    }
    Transform2Params::default()
}

/// Extract ALL transform2/transform effects (supports multiple stacked instances).
pub(crate) fn extract_all_transform2_effects(effects: &[AmEffect]) -> Vec<Transform2Params> {
    effects
        .iter()
        .filter(|e| is_transform_effect(&e.id))
        .map(parse_transform_params)
        .collect()
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
        if effect.id != "com.alightcreative.effects.wipe2" {
            continue;
        }
        for prop in &effect.properties {
            match prop.name.as_str() {
                "start" => apply_animated_float(&mut params.start, prop),
                "end" => apply_animated_float(&mut params.end, prop),
                "angle" => apply_animated_float(&mut params.angle, prop),
                "feather" => apply_animated_float(&mut params.feather, prop),
                _ => (),
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
        if effect.id != "com.alightcreative.effects.stretchsegment" {
            continue;
        }
        for prop in &effect.properties {
            match prop.name.as_str() {
                "angle" => apply_animated_float(&mut params.angle, prop),
                "stretch" => apply_animated_float(&mut params.stretch, prop),
                "offset" => apply_animated_float(&mut params.offset, prop),
                "smooth" => apply_animated_float(&mut params.smooth, prop),
                _ => (),
            }
        }
    }

    params
}

/// Extract ALL stretch segment effects (supports multiple stacked instances).
pub(crate) fn extract_all_stretch_segment_effects(
    effects: &[AmEffect],
) -> Vec<StretchSegmentParams> {
    effects
        .iter()
        .filter(|e| e.id == "com.alightcreative.effects.stretchsegment")
        .map(|effect| {
            let mut params = StretchSegmentParams::default();
            for prop in &effect.properties {
                match prop.name.as_str() {
                    "angle" => apply_animated_float(&mut params.angle, prop),
                    "stretch" => apply_animated_float(&mut params.stretch, prop),
                    "offset" => apply_animated_float(&mut params.offset, prop),
                    "smooth" => apply_animated_float(&mut params.smooth, prop),
                    _ => (),
                }
            }
            params
        })
        .collect()
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
        if effect.id != "com.alightcreative.effects.gaussianblur" {
            continue;
        }
        for prop in &effect.properties {
            if prop.name == "strength" {
                apply_animated_float(&mut params.strength, prop);
            }
        }
    }

    params
}
#[derive(Debug, Clone, Default)]
pub struct PaletteMapParams {
    /// Effect alpha/strength (0.0-1.0)
    pub alpha: AmAnimatedFloat,
    /// Palette selector ID (0-10, NOT color count)
    pub palette_id: u8,
    /// Whether to enable shade variations
    pub shades: bool,
    /// Custom palette colors from XML (up to 8)
    pub custom_colors: [Vec4; 8],
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
        if effect.id != "com.alightcreative.effects.palettemap" {
            continue;
        }
        for prop in &effect.properties {
            match prop.name.as_str() {
                "alpha" => apply_animated_float(&mut params.alpha, prop),
                "palette" => params.palette_id = prop.value.parse().unwrap_or(params.palette_id),
                "shades" => params.shades = prop.value == "true",
                name if name.starts_with("color") => {
                    apply_custom_color(&mut params.custom_colors, name, &prop.value)
                }
                _ => (),
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
    pub new_color: AmAnimatedColor,
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
        if effect.id != "com.alightcreative.replacecolor" {
            continue;
        }
        for prop in &effect.properties {
            match prop.name.as_str() {
                "oldcolor" => {
                    params.old_color = parse_color_vec4(&prop.value).unwrap_or(params.old_color)
                }
                "newcolor" => apply_animated_color(&mut params.new_color, prop),
                "threshold" => apply_animated_float(&mut params.threshold, prop),
                "feather" => apply_animated_float(&mut params.feather, prop),
                "alpha" => apply_animated_float(&mut params.alpha, prop),
                "lockLuminance" => params.lock_luminance = prop.value == "true",
                _ => (),
            }
        }
    }

    params
}

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
        if effect.id != "com.alightcreative.effects.scaleassist" {
            continue;
        }
        for prop in &effect.properties {
            match prop.name.as_str() {
                "axis" => params.axis = prop.value.parse().unwrap_or(params.axis),
                "scale" => apply_animated_float(&mut params.scale, prop),
                "damp" => apply_animated_float(&mut params.damp, prop),
                _ => (),
            }
        }
    }

    params
}

// --- Stretch2 effect (directional UV-space stretch) ---

#[derive(Default)]
pub struct Stretch2Params {
    /// Scale factor along the stretch axis (1.0 = no stretch)
    pub scale: AmAnimatedFloat,
    /// Angle in degrees for the stretch direction
    pub angle: AmAnimatedFloat,
    /// When true, mask stretched result to original layer alpha
    pub content_only: bool,
}

impl Stretch2Params {
    #[allow(dead_code)]
    pub fn has_effect(&self) -> bool {
        self.scale.value.is_some() || !self.scale.keyframes.is_empty()
    }
}

/// Extract stretch2 effect parameters from effects.
pub(crate) fn extract_stretch2_effect(effects: &[AmEffect]) -> Stretch2Params {
    let mut params = Stretch2Params::default();

    for effect in effects {
        if effect.id != "com.alightcreative.effects.stretch2" {
            continue;
        }
        bevy::prelude::warn!("[extract_stretch2] Found stretch2 effect!");
        // Default scale=1 (no stretch)
        params.scale.value = Some(1.0);
        params.angle.value = Some(0.0);

        for prop in &effect.properties {
            match prop.name.as_str() {
                "scale" => apply_animated_float(&mut params.scale, prop),
                "angle" => apply_animated_float(&mut params.angle, prop),
                "contentOnly" => params.content_only = prop.value == "true",
                _ => (),
            }
        }
    }

    params
}

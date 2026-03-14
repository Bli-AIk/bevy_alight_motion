//! Common effect parameter extraction (Wipe, Stretch, Stretch2, Blur, PaletteMap, ReplaceColor, ScaleAssist, Fade, Rays, ChromaKey).

use bevy::prelude::*;

use crate::schema::{AmAnimatedColor, AmAnimatedFloat, AmEffect, AmKeyframe, AmProperty};

/// Effect ID for fade effect.
const FADE_ID: &str = "com.alightcreative.effects.fade";

/// Effect ID for wavewarp2 effect.
const WAVEWARP2_ID: &str = "com.alightcreative.effects.wavewarp2";

/// Effect ID for mirror effect.
const MIRROR_ID: &str = "com.alightcreative.effects.mirror";

/// Effect ID for lift (copy background) effect.
const LIFT_ID: &str = "com.alightcreative.effects.lift";

/// Effect ID for rays (volumetric light rays) effect.
const RAYS_ID: &str = "com.alightcreative.effects.rays";

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

/// Fade effect parameters.
/// 渐入渐出效果参数。
#[derive(Debug, Clone, Default)]
pub struct FadeParams {
    /// Fade-in duration (seconds). / 淡入持续时间（秒）。
    pub in_time: AmAnimatedFloat,
    /// Fade-out duration (seconds). / 淡出持续时间（秒）。
    pub out_time: AmAnimatedFloat,
}

/// Extract fade effect parameters from effects list.
/// 从效果列表中提取渐入渐出效果参数。
pub(crate) fn extract_fade_effect(effects: &[AmEffect]) -> FadeParams {
    let mut params = FadeParams::default();

    for effect in effects {
        if effect.id != FADE_ID {
            continue;
        }
        for prop in &effect.properties {
            match prop.name.as_str() {
                "inTime" => apply_animated_float(&mut params.in_time, prop),
                "outTime" => apply_animated_float(&mut params.out_time, prop),
                _ => (),
            }
        }
    }

    params
}

/// Wave warp effect parameters.
/// 波浪歪曲效果参数。
#[derive(Debug, Clone, Default)]
pub struct Wavewarp2Params {
    /// Wave phase offset. / 波浪相位偏移。
    pub phase: AmAnimatedFloat,
    /// Wave direction angle (degrees). / 波浪方向角度（度）。
    pub a1d: AmAnimatedFloat,
    /// Wave spacing/frequency. / 波浪间距/频率。
    pub m1: AmAnimatedFloat,
    /// Wave displacement magnitude. / 波浪位移幅度。
    pub m2: AmAnimatedFloat,
    /// Warp direction angle offset (degrees). / 翘曲方向角度偏移（度）。
    pub a2d: AmAnimatedFloat,
    /// Magnitude damping [-1, 1]. / 幅度阻尼。
    pub damping: AmAnimatedFloat,
    /// Spacing damping [-1, 1]. / 间距阻尼。
    pub damping_space: AmAnimatedFloat,
    /// Damping origin [0, 1]. / 阻尼原点。
    pub damping_origin: AmAnimatedFloat,
    /// Use screen-space coordinates. / 使用屏幕空间坐标。
    pub screen_space: bool,
    /// Whether this effect is present.
    pub has_effect: bool,
}

/// Extract wavewarp2 effect parameters from effects list.
/// 从效果列表中提取波浪歪曲效果参数。
pub(crate) fn extract_wavewarp2_effect(effects: &[AmEffect]) -> Wavewarp2Params {
    let mut params = Wavewarp2Params::default();

    for effect in effects {
        if effect.id != WAVEWARP2_ID {
            continue;
        }
        params.has_effect = true;
        bevy::prelude::warn!(
            "[extract_wavewarp2] Found wavewarp2 effect! props={}",
            effect.properties.len()
        );
        // Set defaults matching AM
        params.phase.value = Some(0.0);
        params.a1d.value = Some(0.0);
        params.m1.value = Some(20.0);
        params.m2.value = Some(4.0);
        params.a2d.value = Some(90.0);
        params.damping.value = Some(0.0);
        params.damping_space.value = Some(0.0);
        params.damping_origin.value = Some(0.5);

        for prop in &effect.properties {
            match prop.name.as_str() {
                "phase" => apply_animated_float(&mut params.phase, prop),
                "a1d" => apply_animated_float(&mut params.a1d, prop),
                "m1" => apply_animated_float(&mut params.m1, prop),
                "m2" => apply_animated_float(&mut params.m2, prop),
                "a2d" => apply_animated_float(&mut params.a2d, prop),
                "damping" => apply_animated_float(&mut params.damping, prop),
                "dampingSpace" => apply_animated_float(&mut params.damping_space, prop),
                "dampingOrigin" => apply_animated_float(&mut params.damping_origin, prop),
                "screenSpace" => params.screen_space = prop.value == "true",
                _ => (),
            }
        }
    }

    params
}

/// Mirror effect parameters. / 镜子效果参数。
#[derive(Debug, Clone, Default)]
pub struct MirrorParams {
    /// Mirror type: 0=horizontal, 1=vertical. / 镜像方向。
    pub mirror_type: i32,
    /// Blend mode: 0=normal, 1=multiply, 2=screen, 3=over, 4=under.
    pub blend_mode: i32,
    /// Blend alpha. / 混合透明度。
    pub alpha: AmAnimatedFloat,
    /// Mirror axis offset. / 镜像轴偏移。
    pub offset: AmAnimatedFloat,
    /// Whether this effect is present.
    pub has_effect: bool,
}

/// Extract mirror effect parameters from effects list.
/// 从效果列表中提取镜子效果参数。
pub(crate) fn extract_mirror_effect(effects: &[AmEffect]) -> MirrorParams {
    let mut params = MirrorParams::default();

    for effect in effects {
        if effect.id != MIRROR_ID {
            continue;
        }
        params.has_effect = true;
        params.alpha.value = Some(1.0);
        params.offset.value = Some(0.0);

        for prop in &effect.properties {
            match prop.name.as_str() {
                "type" => {
                    params.mirror_type = prop.value.parse::<i32>().unwrap_or(0);
                }
                "blendMode" => {
                    params.blend_mode = prop.value.parse::<i32>().unwrap_or(0);
                }
                "alpha" => apply_animated_float(&mut params.alpha, prop),
                "offset" => apply_animated_float(&mut params.offset, prop),
                _ => (),
            }
        }
    }

    params
}

/// Lift (copy background) effect parameters. / 复制背景效果参数。
#[derive(Debug, Clone, Default)]
pub struct LiftParams {
    /// Fill amount: 0=full background, 1=original content. / 填充量。
    pub fill: AmAnimatedFloat,
    /// Whether this effect is present.
    pub has_effect: bool,
}

/// Extract lift effect parameters from effects list.
/// 从效果列表中提取复制背景效果参数。
pub(crate) fn extract_lift_effect(effects: &[AmEffect]) -> LiftParams {
    let mut params = LiftParams::default();

    for effect in effects {
        if effect.id != LIFT_ID {
            continue;
        }
        params.has_effect = true;
        params.fill.value = Some(0.0);

        for prop in &effect.properties {
            if prop.name == "fill" {
                apply_animated_float(&mut params.fill, prop);
            }
        }
    }

    params
}

/// Rays (volumetric light rays) effect parameters. / 射线效果参数。
#[derive(Debug, Clone, Default)]
pub struct RaysParams {
    /// Center point X (AM coords, ±500). / 中心点X。
    pub center_x: AmAnimatedFloat,
    /// Center point Y (AM coords, ±500). / 中心点Y。
    pub center_y: AmAnimatedFloat,
    /// Ray length/spread (0.0-4.0). / 射线长度。
    pub strength: AmAnimatedFloat,
    /// Brightness multiplier (0.0-5.0). / 亮度倍数。
    pub intensity: AmAnimatedFloat,
    /// Brightness threshold (0.0-1.0). / 亮度阈值。
    pub threshold: AmAnimatedFloat,
    /// Color subtracted before luminance check (sRGB, linear Vec4). / 阈值颜色。
    pub threshold_color: Vec4,
    /// Ray color (sRGB, linear Vec4). / 射线颜色。
    pub fill_color: Vec4,
    /// Blend ratio between original and fill color (0.0-1.0). / 混合比例。
    pub blend: AmAnimatedFloat,
    /// Number of samples (10-800). / 采样数量。
    pub quality: AmAnimatedFloat,
    /// Whether this effect is present.
    pub has_effect: bool,
}

/// Extract rays effect parameters from effects list.
/// 从效果列表中提取射线效果参数。
#[expect(clippy::excessive_nesting)] // reason: match arms inside for loop create inherent nesting
pub(crate) fn extract_rays_effect(effects: &[AmEffect]) -> RaysParams {
    let mut params = RaysParams::default();

    for effect in effects {
        if effect.id != RAYS_ID {
            continue;
        }
        params.has_effect = true;
        // Set defaults matching AM
        params.center_x.value = Some(0.0);
        params.center_y.value = Some(0.0);
        params.strength.value = Some(0.15);
        params.intensity.value = Some(1.0);
        params.threshold.value = Some(0.6);
        params.threshold_color = Vec4::ZERO; // #ff000000 → sRGB black
        // #ff2d1ef6 → keep in sRGB (shader does gamma-space math to match AM)
        params.fill_color = Vec4::new(
            0x2D as f32 / 255.0,
            0x1E as f32 / 255.0,
            0xF6 as f32 / 255.0,
            1.0,
        );
        params.blend.value = Some(0.0);
        params.quality.value = Some(150.0);

        for prop in &effect.properties {
            match prop.name.as_str() {
                "centerPoint" => {
                    if !prop.keyframes.is_empty() {
                        let (kf_x, kf_y) = split_vec2_keyframes(&prop.keyframes);
                        params.center_x.keyframes = kf_x;
                        params.center_y.keyframes = kf_y;
                    } else if let Some((x, y)) = parse_vec2_value(&prop.value) {
                        params.center_x.value = Some(x);
                        params.center_y.value = Some(y);
                    }
                }
                "strength" => apply_animated_float(&mut params.strength, prop),
                "intensity" => apply_animated_float(&mut params.intensity, prop),
                "threshold" => apply_animated_float(&mut params.threshold, prop),
                "thresholdColor" => {
                    if let Some(c) = parse_color_vec4(&prop.value) {
                        // Keep in sRGB space (shader matches AM's gamma-space math)
                        params.threshold_color = c;
                    }
                }
                "fillColor" => {
                    if let Some(c) = parse_color_vec4(&prop.value) {
                        // Keep in sRGB space (shader matches AM's gamma-space math)
                        params.fill_color = c;
                    }
                }
                "blend" => apply_animated_float(&mut params.blend, prop),
                "quality" => apply_animated_float(&mut params.quality, prop),
                _ => (),
            }
        }
    }

    params
}


// ──────── ChromaKey 色度键 ────────

/// Chroma key effect parameters (`com.alightcreative.effects.chromakey`).
/// Removes pixels matching a specified key color (green/blue screen).
/// 色度键效果参数 — 移除匹配指定键色的像素（绿幕/蓝幕抠像）
#[derive(Debug, Clone)]
pub struct ChromaKeyParams {
    pub enabled: bool,
    /// Key color to remove (animated RGBA)
    pub key_color: AmAnimatedColor,
    /// Color matching tolerance (0.0-1.0)
    pub threshold: AmAnimatedFloat,
    /// Edge transition softness (0.0-1.0)
    pub feather: AmAnimatedFloat,
    /// Remove edge color spill
    pub defringe: bool,
    /// Invert keying result (keep key color areas)
    pub invert: bool,
}

impl Default for ChromaKeyParams {
    fn default() -> Self {
        Self {
            enabled: false,
            key_color: AmAnimatedColor::default(),
            threshold: AmAnimatedFloat {
                value: Some(0.1),
                keyframes: Vec::new(),
            },
            feather: AmAnimatedFloat {
                value: Some(0.05),
                keyframes: Vec::new(),
            },
            defringe: false,
            invert: false,
        }
    }
}

/// Extract chroma key effect parameters from effects.
/// 从效果列表中提取色度键效果参数
pub(crate) fn extract_chromakey_effect(effects: &[AmEffect]) -> ChromaKeyParams {
    let mut params = ChromaKeyParams::default();

    let Some(effect) = effects
        .iter()
        .find(|e| e.id == "com.alightcreative.effects.chromakey")
    else {
        return params;
    };

    params.enabled = true;

    for prop in &effect.properties {
        match prop.name.as_str() {
            "keyColor" => apply_animated_color(&mut params.key_color, prop),
            "threshold" => apply_animated_float(&mut params.threshold, prop),
            "feather" => apply_animated_float(&mut params.feather, prop),
            "defringe" => params.defringe = prop.value == "true",
            "invert" => params.invert = prop.value == "true",
            _ => {}
        }
    }

    params
}

//! Extracts filter-like effects during scene collection.
//! 在 scene 收集阶段提取滤镜类效果。
//!
//! Wipe, stretch, replace-color, and other filter-style effects are imported as generic XML
//! properties, but the runtime expects typed parameter structs. This file performs that conversion
//! so downstream spawn and animation code can work with strongly named fields instead of raw effect
//! property scans.
//! Wipe、Stretch、替色等滤镜式效果在导入时都只是通用 XML 属性，但运行时需要的是强类型参数结构。
//! 这个文件负责完成这次转换，让后续的 spawn 与动画代码操作具名字段，而不是重复遍历原始 effect 属性。

use bevy::prelude::*;

use crate::schema::{AmAnimatedColor, AmAnimatedFloat, AmEffect};

use super::shared::{
    apply_animated_color, apply_animated_float, apply_custom_color, parse_color_vec4,
};

#[derive(Debug, Clone, Default)]
pub struct WipeEffectParams {
    pub start: AmAnimatedFloat,
    pub end: AmAnimatedFloat,
    pub angle: AmAnimatedFloat,
    pub feather: AmAnimatedFloat,
}

pub(crate) fn extract_wipe_effect(effects: &[AmEffect]) -> WipeEffectParams {
    let mut params = WipeEffectParams::default();
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
    pub angle: AmAnimatedFloat,
    pub stretch: AmAnimatedFloat,
    pub offset: AmAnimatedFloat,
    pub smooth: AmAnimatedFloat,
}

impl StretchSegmentParams {
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
    pub strength: AmAnimatedFloat,
}

impl GaussianBlurParams {
    pub fn has_effect(&self) -> bool {
        self.strength.value.is_some() || !self.strength.keyframes.is_empty()
    }
}

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
    pub alpha: AmAnimatedFloat,
    pub palette_id: u8,
    pub shades: bool,
    pub custom_colors: [Vec4; 8],
}

impl PaletteMapParams {
    pub fn has_effect(&self) -> bool {
        self.alpha.value.is_some() || !self.alpha.keyframes.is_empty()
    }
}

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

#[derive(Debug, Clone, Default)]
pub struct ReplaceColorParams {
    pub old_color: Vec4,
    pub new_color: AmAnimatedColor,
    pub threshold: AmAnimatedFloat,
    pub feather: AmAnimatedFloat,
    pub alpha: AmAnimatedFloat,
    pub lock_luminance: bool,
}

pub(crate) fn extract_replace_color_effect(effects: &[AmEffect]) -> ReplaceColorParams {
    let mut params = ReplaceColorParams::default();
    params.alpha.value = Some(1.0);

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
    pub axis: i32,
    pub scale: AmAnimatedFloat,
    pub damp: AmAnimatedFloat,
}

pub(crate) fn extract_scale_assist_effect(effects: &[AmEffect]) -> ScaleAssistParams {
    let mut params = ScaleAssistParams::default();
    params.scale.value = Some(1.0);
    params.damp.value = Some(1.0);

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

#[derive(Default)]
pub struct Stretch2Params {
    pub scale: AmAnimatedFloat,
    pub angle: AmAnimatedFloat,
    pub content_only: bool,
}

impl Stretch2Params {
    #[allow(dead_code)]
    pub fn has_effect(&self) -> bool {
        self.scale.value.is_some() || !self.scale.keyframes.is_empty()
    }
}

pub(crate) fn extract_stretch2_effect(effects: &[AmEffect]) -> Stretch2Params {
    let mut params = Stretch2Params::default();

    for effect in effects {
        if effect.id != "com.alightcreative.effects.stretch2" {
            continue;
        }
        bevy::prelude::warn!("[extract_stretch2] Found stretch2 effect!");
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

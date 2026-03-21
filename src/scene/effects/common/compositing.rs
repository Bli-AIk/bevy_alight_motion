use bevy::prelude::*;

use crate::schema::{AmAnimatedColor, AmAnimatedFloat, AmEffect};

use super::shared::{
    apply_animated_color, apply_animated_float, parse_color_vec4, parse_vec2_value,
    split_vec2_keyframes,
};

const FADE_ID: &str = "com.alightcreative.effects.fade";
const WAVEWARP2_ID: &str = "com.alightcreative.effects.wavewarp2";
const MIRROR_ID: &str = "com.alightcreative.effects.mirror";
const LIFT_ID: &str = "com.alightcreative.effects.lift";
const RAYS_ID: &str = "com.alightcreative.effects.rays";

#[derive(Debug, Clone, Default)]
pub struct FadeParams {
    pub in_time: AmAnimatedFloat,
    pub out_time: AmAnimatedFloat,
}

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

#[derive(Debug, Clone, Default)]
pub struct Wavewarp2Params {
    pub phase: AmAnimatedFloat,
    pub a1d: AmAnimatedFloat,
    pub m1: AmAnimatedFloat,
    pub m2: AmAnimatedFloat,
    pub a2d: AmAnimatedFloat,
    pub damping: AmAnimatedFloat,
    pub damping_space: AmAnimatedFloat,
    pub damping_origin: AmAnimatedFloat,
    pub screen_space: bool,
    pub has_effect: bool,
}

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

#[derive(Debug, Clone, Default)]
pub struct MirrorParams {
    pub mirror_type: i32,
    pub blend_mode: i32,
    pub alpha: AmAnimatedFloat,
    pub offset: AmAnimatedFloat,
    pub has_effect: bool,
}

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

#[derive(Debug, Clone, Default)]
pub struct LiftParams {
    pub fill: AmAnimatedFloat,
    pub has_effect: bool,
}

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

#[derive(Debug, Clone, Default)]
pub struct RaysParams {
    pub center_x: AmAnimatedFloat,
    pub center_y: AmAnimatedFloat,
    pub strength: AmAnimatedFloat,
    pub intensity: AmAnimatedFloat,
    pub threshold: AmAnimatedFloat,
    pub threshold_color: Vec4,
    pub fill_color: Vec4,
    pub blend: AmAnimatedFloat,
    pub quality: AmAnimatedFloat,
    pub has_effect: bool,
}

#[expect(clippy::excessive_nesting)]
pub(crate) fn extract_rays_effect(effects: &[AmEffect]) -> RaysParams {
    let mut params = RaysParams::default();

    for effect in effects {
        if effect.id != RAYS_ID {
            continue;
        }
        params.has_effect = true;
        params.center_x.value = Some(0.0);
        params.center_y.value = Some(0.0);
        params.strength.value = Some(0.15);
        params.intensity.value = Some(1.0);
        params.threshold.value = Some(0.6);
        params.threshold_color = Vec4::ZERO;
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
                        params.threshold_color = c;
                    }
                }
                "fillColor" => {
                    if let Some(c) = parse_color_vec4(&prop.value) {
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

#[derive(Debug, Clone)]
pub struct ChromaKeyParams {
    pub enabled: bool,
    pub key_color: AmAnimatedColor,
    pub threshold: AmAnimatedFloat,
    pub feather: AmAnimatedFloat,
    pub defringe: bool,
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

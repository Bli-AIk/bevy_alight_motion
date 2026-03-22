use crate::schema::{AmAnimatedFloat, AmEffect};

use super::shared::{apply_animated_float, parse_vec2_value, split_vec2_keyframes};

const TRANSFORM2_ID: &str = "com.alightcreative.effects.transform2";
const TRANSFORM_V1_ID: &str = "com.alightcreative.effects.transform";
const PARENTHELPER_ID: &str = "com.alightcreative.effects.parenthelper";

fn is_transform_effect(id: &str) -> bool {
    id == TRANSFORM2_ID || id == TRANSFORM_V1_ID
}

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

fn parse_transform_params(effect: &AmEffect) -> Transform2Params {
    let is_v1 = effect.id == TRANSFORM_V1_ID;
    let mut params = Transform2Params::default();

    for prop in &effect.properties {
        match prop.name.as_str() {
            "posx" if !is_v1 => {
                if !prop.keyframes.is_empty() {
                    params.pos_x.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.pos_x.value = Some(v);
                }
            }
            "posy" if !is_v1 => {
                if !prop.keyframes.is_empty() {
                    params.pos_y.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.pos_y.value = Some(v);
                }
            }
            "offset" if is_v1 => {
                if !prop.keyframes.is_empty() {
                    let (kf_x, kf_y) = split_vec2_keyframes(&prop.keyframes);
                    params.pos_x.keyframes = kf_x;
                    params.pos_y.keyframes = kf_y;
                } else if let Some((x, y)) = parse_vec2_value(&prop.value) {
                    params.pos_x.value = Some(x);
                    params.pos_y.value = Some(y);
                }
            }
            "posz" if !is_v1 => {
                if !prop.keyframes.is_empty() {
                    params.pos_z.keyframes = prop.keyframes.clone();
                } else if let Ok(v) = prop.value.parse::<f32>() {
                    params.pos_z.value = Some(v);
                }
            }
            "scale" if is_v1 => {
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
            _ => {}
        }
    }

    params
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

pub(crate) fn extract_all_transform2_effects(effects: &[AmEffect]) -> Vec<Transform2Params> {
    effects
        .iter()
        .filter(|e| is_transform_effect(&e.id))
        .map(parse_transform_params)
        .collect()
}

#[derive(Debug, Clone, Default)]
pub struct ParentHelperParams {
    pub scale_mode: i32,
    pub rotate_mode: i32,
    pub scale_weight: AmAnimatedFloat,
    pub rotate_weight: AmAnimatedFloat,
    pub auto_rotate: i32,
    pub radius_adjust: AmAnimatedFloat,
    pub has_effect: bool,
}

pub(crate) fn extract_parent_helper_effect(effects: &[AmEffect]) -> ParentHelperParams {
    let mut params = ParentHelperParams::default();

    let Some(effect) = effects.iter().find(|e| e.id == PARENTHELPER_ID) else {
        return params;
    };

    params.has_effect = true;
    params.scale_weight.value = Some(1.0);
    params.rotate_weight.value = Some(1.0);

    for prop in &effect.properties {
        match prop.name.as_str() {
            "scaleMode" => {
                if let Ok(v) = prop.value.parse::<i32>() {
                    params.scale_mode = v;
                }
            }
            "rotateMode" => {
                if let Ok(v) = prop.value.parse::<i32>() {
                    params.rotate_mode = v;
                }
            }
            "scaleWeight" => apply_animated_float(&mut params.scale_weight, prop),
            "rotateWeight" => apply_animated_float(&mut params.rotate_weight, prop),
            "autoRotate" => {
                if let Ok(v) = prop.value.parse::<i32>() {
                    params.auto_rotate = v;
                }
            }
            "radiusAdjust" => apply_animated_float(&mut params.radius_adjust, prop),
            _ => {}
        }
    }

    params
}

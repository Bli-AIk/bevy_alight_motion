//! This file parses custom shape property values from layer property lists.
//! It offers typed accessors for float and vec2 properties, including animated
//! variants, so shape collectors can read author-defined parameters without
//! duplicating low-level property scanning code.
//!
//! 这个文件负责从图层属性列表里解析自定义形状属性。它提供 float 和 vec2 属性的
//! 带类型访问器，以及对应的动画版本，让形状收集器无需重复书写底层属性扫描逻辑。

use crate::schema::{AmAnimatedFloat, AmAnimatedVec2, AmKeyframe, AmProperty};

pub(crate) fn get_shape_float_property(properties: &[AmProperty], name: &str, default: f32) -> f32 {
    for prop in properties {
        if prop.name == name && prop.prop_type == "float" {
            if !prop.value.is_empty()
                && let Ok(v) = prop.value.parse::<f32>()
            {
                return v;
            }
            if let Some(kf) = prop.keyframes.first()
                && let Ok(v) = kf.value.parse::<f32>()
            {
                return v;
            }
        }
    }
    default
}

pub(crate) fn get_shape_float_animation(
    properties: &[AmProperty],
    name: &str,
    default: f32,
) -> AmAnimatedFloat {
    for prop in properties {
        if prop.name == name && prop.prop_type == "float" {
            let value = if !prop.value.is_empty() {
                prop.value.parse::<f32>().ok()
            } else {
                None
            };
            return AmAnimatedFloat {
                value: value.or(Some(default)),
                keyframes: prop.keyframes.clone(),
            };
        }
    }
    AmAnimatedFloat {
        value: Some(default),
        keyframes: Vec::new(),
    }
}

pub(crate) fn get_shape_vec2_property(
    properties: &[AmProperty],
    name: &str,
    default: [f32; 2],
) -> [f32; 2] {
    for prop in properties {
        if prop.name == name && prop.prop_type == "vec2" {
            if !prop.value.is_empty()
                && let Ok(v) = crate::schema::parse_vec2(&prop.value)
            {
                return v;
            }
            if let Some(kf) = prop.keyframes.first()
                && let Ok(v) = crate::schema::parse_vec2(&kf.value)
            {
                return v;
            }
        }
    }
    default
}

pub(crate) fn get_shape_vec2_animation(
    properties: &[AmProperty],
    name: &str,
    default: [f32; 2],
) -> AmAnimatedVec2 {
    for prop in properties {
        if prop.name == name && prop.prop_type == "vec2" {
            let value = if !prop.value.is_empty() {
                crate::schema::parse_vec2(&prop.value).ok()
            } else {
                None
            };
            let keyframes: Vec<AmKeyframe> = prop.keyframes.clone();
            return AmAnimatedVec2 {
                value: value.or(Some(default)),
                keyframes,
            };
        }
    }
    AmAnimatedVec2 {
        value: Some(default),
        keyframes: Vec::new(),
    }
}

pub(crate) fn extract_shape_animations(
    shape_type: &str,
    properties: &[AmProperty],
) -> ([AmAnimatedFloat; 4], [AmAnimatedVec2; 5]) {
    let df = || AmAnimatedFloat {
        value: Some(0.0),
        keyframes: Vec::new(),
    };
    let dv = || AmAnimatedVec2 {
        value: Some([0.0, 0.0]),
        keyframes: Vec::new(),
    };

    let props = match shape_type {
        ".roundrect" => [
            get_shape_float_animation(properties, "cornerRadius", 0.0),
            df(),
            df(),
            df(),
        ],
        ".poly" => [
            get_shape_float_animation(properties, "sideCount", 6.0),
            get_shape_float_animation(properties, "radius", 50.0),
            get_shape_float_animation(properties, "offsetAngle", 0.0),
            df(),
        ],
        ".star" => [
            get_shape_float_animation(properties, "pointCount", 5.0),
            get_shape_float_animation(properties, "outerRadius", 50.0),
            get_shape_float_animation(properties, "innerRadius", 25.0),
            get_shape_float_animation(properties, "offsetAngle", 0.0),
        ],
        ".pie" => [
            get_shape_float_animation(properties, "startAngle", 0.0),
            get_shape_float_animation(properties, "endAngle", 270.0),
            get_shape_float_animation(properties, "radius", 50.0),
            df(),
        ],
        ".plus" => [
            get_shape_float_animation(properties, "stemSize", 50.0),
            df(),
            df(),
            df(),
        ],
        ".multifoil" => [
            get_shape_float_animation(properties, "pointCount", 5.0),
            get_shape_float_animation(properties, "outerRadius", 50.0),
            get_shape_float_animation(properties, "innerRadius", 25.0),
            get_shape_float_animation(properties, "offsetAngle", 0.0),
        ],
        ".arc" => [
            get_shape_float_animation(properties, "startAngle", 0.0),
            get_shape_float_animation(properties, "endAngle", 270.0),
            get_shape_float_animation(properties, "radius", 50.0),
            df(),
        ],
        ".line" => {
            let p = [df(), df(), df(), df()];
            return (
                p,
                [
                    get_shape_vec2_animation(properties, "p1", [0.0, 0.0]),
                    get_shape_vec2_animation(properties, "p2", [50.0, 0.0]),
                    dv(),
                    dv(),
                    dv(),
                ],
            );
        }
        ".arrow" => {
            let p = [
                get_shape_float_animation(properties, "lineWidth", 20.0),
                get_shape_float_animation(properties, "headWidth", 40.0),
                get_shape_float_animation(properties, "headLength", 30.0),
                df(),
            ];
            return (
                p,
                [
                    get_shape_vec2_animation(properties, "start", [0.0, 0.0]),
                    get_shape_vec2_animation(properties, "end", [100.0, 0.0]),
                    dv(),
                    dv(),
                    dv(),
                ],
            );
        }
        ".triangle" => {
            let p = [df(), df(), df(), df()];
            return (
                p,
                [
                    get_shape_vec2_animation(properties, "p1", [0.0, -50.0]),
                    get_shape_vec2_animation(properties, "p2", [-50.0, 50.0]),
                    get_shape_vec2_animation(properties, "p3", [50.0, 50.0]),
                    dv(),
                    dv(),
                ],
            );
        }
        ".quad" => {
            let p = [df(), df(), df(), df()];
            return (
                p,
                [
                    get_shape_vec2_animation(properties, "p1", [-50.0, -50.0]),
                    get_shape_vec2_animation(properties, "p2", [50.0, -50.0]),
                    get_shape_vec2_animation(properties, "p3", [50.0, 50.0]),
                    get_shape_vec2_animation(properties, "p4", [-50.0, 50.0]),
                    dv(),
                ],
            );
        }
        ".penta" => {
            let p = [df(), df(), df(), df()];
            return (
                p,
                [
                    get_shape_vec2_animation(properties, "p1", [0.0, -50.0]),
                    get_shape_vec2_animation(properties, "p2", [-47.5, -15.5]),
                    get_shape_vec2_animation(properties, "p3", [-29.4, 40.5]),
                    get_shape_vec2_animation(properties, "p4", [29.4, 40.5]),
                    get_shape_vec2_animation(properties, "p5", [47.5, -15.5]),
                ],
            );
        }
        _ => [df(), df(), df(), df()],
    };
    (props, [dv(), dv(), dv(), dv(), dv()])
}

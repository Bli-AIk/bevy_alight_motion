//! This file contains shared parsers and applicators for scene effect extraction.
//! It holds the small helpers that multiple effect extractors reuse for animated
//! floats, vec2 keyframes, palette colors, and related schema conversions.
//!
//! 这个文件存放场景特效提取阶段共用的解析与赋值辅助函数。多个 effect extractor
//! 都会复用这里的逻辑来处理动画浮点数、vec2 关键帧、调色板颜色以及相关的 schema 转换。

use bevy::prelude::*;

use crate::schema::{AmAnimatedColor, AmAnimatedFloat, AmKeyframe, AmProperty};

pub(super) fn split_vec2_keyframes(keyframes: &[AmKeyframe]) -> (Vec<AmKeyframe>, Vec<AmKeyframe>) {
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

pub(super) fn parse_vec2_value(value: &str) -> Option<(f32, f32)> {
    let (x_str, y_str) = value.split_once(',')?;
    let x = x_str.trim().parse::<f32>().ok()?;
    let y = y_str.trim().parse::<f32>().ok()?;
    Some((x, y))
}

pub(super) fn apply_animated_float(target: &mut AmAnimatedFloat, prop: &AmProperty) {
    if !prop.keyframes.is_empty() {
        target.keyframes = prop.keyframes.clone();
    } else if let Ok(v) = prop.value.parse::<f32>() {
        target.value = Some(v);
    }
}

pub(super) fn parse_color_vec4(value: &str) -> Option<Vec4> {
    let c = crate::schema::parse_color(value).ok()?;
    Some(Vec4::new(c[0], c[1], c[2], c[3]))
}

pub(super) fn apply_custom_color(colors: &mut [Vec4; 8], name: &str, value: &str) {
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

pub(super) fn apply_animated_color(target: &mut AmAnimatedColor, prop: &AmProperty) {
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

//! This file implements the numeric counter text effect.
//! It evaluates counter parameters against the current playback time and rewrites
//! a text layer's visible string so authored placeholders can become animated
//! numbers in the final scene.
//!
//! 这个文件实现数字计数器文本效果。它会根据当前播放时间求值 counter 参数，并重写
//! 文本图层实际显示的字符串，让作者写下的占位文本在最终场景里变成动态数字。

use bevy::prelude::*;
use bevy::sprite::Text2d;

use crate::animation::AmPlayback;
use crate::animation::components::AmAnimated;
use crate::animation::interpolation::interpolate_float;
use crate::scene::AmLayerSpec;

use super::progress::AmOriginalText;

pub fn animate_counter_system(
    playback: Res<AmPlayback>,
    mut query: Query<(&AmAnimated, &AmLayerSpec, &mut Text2d), Without<AmOriginalText>>,
    mut query_with_orig: Query<(&AmAnimated, &AmLayerSpec, &mut Text2d, &AmOriginalText)>,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (animated, spec, mut text2d) in query.iter_mut() {
        if let Some(new_text) = apply_counter(animated, spec, global_time)
            && text2d.0 != new_text
        {
            text2d.0 = new_text;
        }
    }

    for (animated, _spec, mut text2d, orig) in query_with_orig.iter_mut() {
        if let Some(new_text) = apply_counter_to_source(animated, &orig.0, global_time)
            && text2d.0 != new_text
        {
            text2d.0 = new_text;
        }
    }
}

fn apply_counter(animated: &AmAnimated, spec: &AmLayerSpec, global_time: f32) -> Option<String> {
    let has_counter =
        animated.counter_offset.value.is_some() || !animated.counter_offset.keyframes.is_empty();
    if !has_counter {
        return None;
    }

    let content = match spec {
        AmLayerSpec::Text { content, .. } => content.as_str(),
        _ => return None,
    };

    let local_time = animated.calc_local_time(global_time);
    if !animated.is_active(local_time) {
        return None;
    }

    let layer_time = animated.calc_layer_time(local_time);
    let offset = interpolate_float(&animated.counter_offset, layer_time).unwrap_or(0.0);
    let scale = interpolate_float(&animated.counter_scale, layer_time).unwrap_or(1.0);

    Some(counter_transform(content, offset as f64, scale as f64))
}

fn apply_counter_to_source(
    animated: &AmAnimated,
    source: &str,
    global_time: f32,
) -> Option<String> {
    let has_counter =
        animated.counter_offset.value.is_some() || !animated.counter_offset.keyframes.is_empty();
    if !has_counter {
        return None;
    }

    let local_time = animated.calc_local_time(global_time);
    if !animated.is_active(local_time) {
        return None;
    }

    let layer_time = animated.calc_layer_time(local_time);
    let offset = interpolate_float(&animated.counter_offset, layer_time).unwrap_or(0.0);
    let scale = interpolate_float(&animated.counter_scale, layer_time).unwrap_or(1.0);

    Some(counter_transform(source, offset as f64, scale as f64))
}

fn counter_transform(text: &str, offset: f64, scale: f64) -> String {
    let mut result = String::with_capacity(text.len() + 16);
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if let Some((num_str, end_pos)) = try_parse_number(&chars, i) {
            result.push_str(&transform_number(&num_str, offset, scale));
            i = end_pos;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

fn transform_number(num_str: &str, offset: f64, scale: f64) -> String {
    let has_comma = num_str.contains(',');
    let has_decimal = num_str.contains('.');
    let clean: String = num_str.chars().filter(|c| *c != ',').collect();
    let Ok(val) = clean.parse::<f64>() else {
        return num_str.to_string();
    };
    let adjusted = val * scale + offset;
    let dp = if has_decimal {
        num_str
            .split('.')
            .nth(1)
            .map(|s| s.chars().filter(|c| *c != ',').count())
            .unwrap_or(0)
    } else {
        0
    };
    if has_comma {
        format_with_thousands(adjusted, dp)
    } else {
        format!("{:.prec$}", adjusted, prec = dp)
    }
}

fn try_parse_number(chars: &[char], start: usize) -> Option<(String, usize)> {
    let mut pos = start;

    let has_sign = pos < chars.len() && (chars[pos] == '+' || chars[pos] == '-');
    if has_sign {
        pos += 1;
    }

    if pos >= chars.len() {
        return None;
    }

    let first_after_sign = chars[pos];
    if !first_after_sign.is_ascii_digit() && first_after_sign != '.' {
        return None;
    }

    let mut has_digits = false;
    while pos < chars.len() && (chars[pos].is_ascii_digit() || chars[pos] == ',') {
        if chars[pos].is_ascii_digit() {
            has_digits = true;
        }
        pos += 1;
    }

    let has_dot = pos < chars.len() && chars[pos] == '.';
    if has_dot {
        pos += 1;
        while pos < chars.len() && (chars[pos].is_ascii_digit() || chars[pos] == ',') {
            if chars[pos].is_ascii_digit() {
                has_digits = true;
            }
            pos += 1;
        }
    }

    if !has_digits {
        return None;
    }

    if has_sign && pos == start + 1 {
        return None;
    }

    let num_str: String = chars[start..pos].iter().collect();
    Some((num_str, pos))
}

fn format_with_thousands(n: f64, dp: usize) -> String {
    let formatted = format!("{:.prec$}", n, prec = dp);
    let (int_part, dec_part) = if let Some(dot_pos) = formatted.find('.') {
        (&formatted[..dot_pos], Some(&formatted[dot_pos..]))
    } else {
        (formatted.as_str(), None)
    };

    let (sign, digits) = if let Some(stripped) = int_part.strip_prefix('-') {
        ("-", stripped)
    } else {
        ("", int_part)
    };

    let mut with_commas = String::new();
    let len = digits.len();
    for (idx, ch) in digits.chars().enumerate() {
        if idx > 0 && (len - idx) % 3 == 0 {
            with_commas.push(',');
        }
        with_commas.push(ch);
    }

    match dec_part {
        Some(d) => format!("{sign}{with_commas}{d}"),
        None => format!("{sign}{with_commas}"),
    }
}

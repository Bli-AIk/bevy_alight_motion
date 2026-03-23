//! This file contains helpers for deriving fill and stroke state from schema data.
//! It converts authored fill colors, gradients, and stroke declarations into the
//! alpha values and animated parameter forms used by shape collection and runtime
//! rendering setup.
//!
//! 这个文件存放从 schema 数据里推导 fill 和 stroke 状态的辅助函数。它会把作者侧
//! 的填充颜色、渐变和描边声明转换成形状收集与运行时渲染初始化要使用的透明度值和
//! 动画参数形式。

use bevy::math::Vec4;

use crate::schema::{AmAnimatedColor, AmAnimatedFloat, AmFillColor, AmGradient, AmStroke};

pub(crate) fn get_stroke_width_animation(stroke: Option<&AmStroke>) -> AmAnimatedFloat {
    if let Some(stroke) = stroke {
        if let Some(ref size) = stroke.size {
            if !size.keyframes.is_empty() {
                return AmAnimatedFloat {
                    value: size.value,
                    keyframes: size.keyframes.clone(),
                };
            }
            return AmAnimatedFloat {
                value: size.value,
                keyframes: Vec::new(),
            };
        }

        return AmAnimatedFloat {
            value: Some(4.0),
            keyframes: Vec::new(),
        };
    }

    AmAnimatedFloat {
        value: Some(0.0),
        keyframes: Vec::new(),
    }
}

pub(crate) fn get_base_alpha(fill_color: &Option<AmFillColor>, no_fill: bool) -> f32 {
    if no_fill {
        return 0.0;
    }

    if let Some(fc) = fill_color {
        if !fc.value.is_empty() {
            if let Ok(c) = crate::schema::parse_color(&fc.value) {
                return c[3];
            }
        } else if !fc.keyframes.is_empty() {
            let mut sorted: Vec<_> = fc.keyframes.iter().collect();
            sorted.sort_by(|a, b| {
                a.time
                    .partial_cmp(&b.time)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            if let Ok(c) = crate::schema::parse_color(&sorted[0].value) {
                return c[3];
            }
        }
    }
    1.0
}

pub(crate) fn get_initial_fill_color_rgba(
    fill_color: &Option<AmFillColor>,
    no_fill: bool,
) -> [f32; 4] {
    if no_fill {
        return [0.0; 4];
    }
    if let Some(fc) = fill_color {
        if !fc.value.is_empty() {
            if let Ok(c) = crate::schema::parse_color(&fc.value) {
                return c;
            }
        } else if !fc.keyframes.is_empty() {
            let mut sorted: Vec<_> = fc.keyframes.iter().collect();
            sorted.sort_by(|a, b| {
                a.time
                    .partial_cmp(&b.time)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            if let Ok(c) = crate::schema::parse_color(&sorted[0].value) {
                return c;
            }
        }
    }
    [1.0, 1.0, 1.0, 1.0]
}

pub(crate) fn fill_color_to_animated(fill_color: &Option<AmFillColor>) -> AmAnimatedColor {
    match fill_color {
        Some(fc) => {
            let value = if !fc.value.is_empty() {
                crate::schema::parse_color(&fc.value)
                    .ok()
                    .map(|c| Vec4::new(c[0], c[1], c[2], c[3]))
            } else {
                None
            };
            AmAnimatedColor {
                value,
                keyframes: fc.keyframes.clone(),
            }
        }
        None => Default::default(),
    }
}

pub(crate) fn extract_gradient_data(gradient: &Option<AmGradient>) -> (u8, Vec4, Vec4, Vec4) {
    if let Some(g) = gradient {
        let grad_type = match g.gradient_type.as_str() {
            "linear" => 1u8,
            "radial" => 2u8,
            "sweep" => 3u8,
            _ => 0u8,
        };
        if grad_type == 0 {
            return (0, Vec4::ZERO, Vec4::ZERO, Vec4::ZERO);
        }
        let start_color = crate::schema::parse_color(&g.start_color)
            .map(|c| Vec4::new(c[0], c[1], c[2], c[3]))
            .unwrap_or(Vec4::ZERO);
        let end_color = crate::schema::parse_color(&g.end_color)
            .map(|c| Vec4::new(c[0], c[1], c[2], c[3]))
            .unwrap_or(Vec4::ZERO);
        let start_pt = g.start.unwrap_or([0.0, 0.0]);
        let end_pt = g.end.unwrap_or([1.0, 1.0]);
        let points = Vec4::new(start_pt[0], start_pt[1], end_pt[0], end_pt[1]);
        (grad_type, start_color, end_color, points)
    } else {
        (0, Vec4::ZERO, Vec4::ZERO, Vec4::ZERO)
    }
}

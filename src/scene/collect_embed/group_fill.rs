//! Extracts group-fill configuration from embed scenes.
//! It translates the authored fill mode and gradient/color payload into the
//! runtime `AmGroupFill` structure used later by composite rendering.
//!
//! 负责从嵌套场景里提取 group fill 配置。它会把作者侧的 fill 模式以及
//! 渐变/纯色数据转换成后续 composite 渲染要使用的 `AmGroupFill` 运行时结构。

use bevy::prelude::*;

/// Build AmGroupFill from embed scene's fill type and color/gradient data.
pub(super) fn build_group_fill(
    embed: &crate::schema::AmEmbedScene,
) -> Option<crate::effects::AmGroupFill> {
    use crate::effects::{AmGroupFill, GroupFillType};

    match embed.fill_type.as_str() {
        "" => None,
        "none" => Some(AmGroupFill {
            fill_type: GroupFillType::None,
            fill_color: Vec4::ZERO,
        }),
        "color" => {
            let color = if let Some(ref fc) = embed.fill_color {
                if let Ok(c) = crate::schema::parse_color(&fc.value) {
                    let srgb = Color::srgba(c[0], c[1], c[2], c[3]);
                    let linear = srgb.to_linear();
                    Vec4::new(linear.red, linear.green, linear.blue, linear.alpha)
                } else {
                    Vec4::ONE
                }
            } else {
                Vec4::ONE
            };
            Some(AmGroupFill {
                fill_type: GroupFillType::Color,
                fill_color: color,
            })
        }
        "gradient" => {
            if let Some(ref g) = embed.gradient {
                let gradient_type = match g.gradient_type.as_str() {
                    "linear" => 1u8,
                    "radial" => 2u8,
                    "sweep" => 3u8,
                    _ => 1u8,
                };
                let start_color = if let Ok(c) = crate::schema::parse_color(&g.start_color) {
                    Vec4::new(c[0], c[1], c[2], c[3])
                } else {
                    Vec4::ZERO
                };
                let end_color = if let Ok(c) = crate::schema::parse_color(&g.end_color) {
                    Vec4::new(c[0], c[1], c[2], c[3])
                } else {
                    Vec4::ONE
                };
                let start_pt = g.start.unwrap_or([0.0, 0.0]);
                let end_pt = g.end.unwrap_or([1.0, 1.0]);
                Some(AmGroupFill {
                    fill_type: GroupFillType::Gradient {
                        gradient_type,
                        start_color,
                        end_color,
                        points: Vec4::new(start_pt[0], start_pt[1], end_pt[0], end_pt[1]),
                    },
                    fill_color: Vec4::ONE,
                })
            } else {
                Some(AmGroupFill {
                    fill_type: GroupFillType::Gradient {
                        gradient_type: 1,
                        start_color: Vec4::new(0.0, 0.0, 0.0, 1.0),
                        end_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
                        points: Vec4::new(0.0, 0.0, 1.0, 1.0),
                    },
                    fill_color: Vec4::ONE,
                })
            }
        }
        _ => None,
    }
}

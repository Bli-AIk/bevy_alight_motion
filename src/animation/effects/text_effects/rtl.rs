//! Repairs RTL line alignment after Bevy text layout has run.
//! It detects right-to-left paragraphs in Alight Motion text layers and nudges
//! glyph positions so left/right alignment semantics better match the authored
//! expectations from the source project.
//!
//! 负责在 Bevy 文本布局完成后修正 RTL 行对齐。它会识别 Alight Motion
//! 文本图层里的从右到左段落，并调整字形位置，让 left/right 对齐的实际表现更贴近
//! 源项目中的作者预期。

use bevy::prelude::*;
use bevy::sprite::Text2d;
use bevy::text::TextLayoutInfo;

use crate::scene::AmLayerSpec;

#[derive(Component)]
pub struct AmRtlAlignmentFixed;

pub fn fix_rtl_line_alignment_system(
    mut commands: Commands,
    mut query: Query<
        (Entity, &AmLayerSpec, &mut TextLayoutInfo),
        (With<Text2d>, Without<AmRtlAlignmentFixed>),
    >,
) {
    for (entity, spec, mut layout_info) in query.iter_mut() {
        let (content, align, wrap_width) = match spec {
            AmLayerSpec::Text {
                content,
                align,
                wrap_width,
                ..
            } => (content.as_str(), align.as_str(), *wrap_width),
            _ => continue,
        };

        if layout_info.glyphs.is_empty() {
            continue;
        }

        if align == "center" {
            commands.entity(entity).insert(AmRtlAlignmentFixed);
            continue;
        }

        let paragraphs: Vec<&str> = content.split('\n').collect();
        let mut visual_lines: Vec<(usize, f32, Vec<usize>)> = Vec::new();
        for (gi, glyph) in layout_info.glyphs.iter().enumerate() {
            let found = visual_lines.iter().position(|(li, y, _)| {
                *li == glyph.line_index && (y - glyph.position.y).abs() < 2.0
            });
            if let Some(idx) = found {
                visual_lines[idx].2.push(gi);
            } else {
                visual_lines.push((glyph.line_index, glyph.position.y, vec![gi]));
            }
        }

        for (para_idx, _y, glyph_indices) in &visual_lines {
            let para_text = paragraphs.get(*para_idx).copied().unwrap_or("");
            if !is_rtl_paragraph(para_text) {
                continue;
            }

            let right_edge = glyph_indices
                .iter()
                .map(|&i| {
                    let g = &layout_info.glyphs[i];
                    g.position.x + g.size.x / 2.0
                })
                .fold(f32::MIN, f32::max);
            let left_edge = glyph_indices
                .iter()
                .map(|&i| {
                    let g = &layout_info.glyphs[i];
                    g.position.x - g.size.x / 2.0
                })
                .fold(f32::MAX, f32::min);

            let shift_x = match align {
                "left" => wrap_width - right_edge,
                "right" => -left_edge,
                _ => 0.0,
            };

            if shift_x.abs() <= 0.5 {
                continue;
            }
            for &gi in glyph_indices {
                layout_info.glyphs[gi].position.x += shift_x;
            }
        }

        commands.entity(entity).insert(AmRtlAlignmentFixed);
    }
}

fn is_rtl_paragraph(text: &str) -> bool {
    for ch in text.chars() {
        if is_rtl_char(ch) {
            return true;
        }
        if ch.is_alphanumeric() {
            return false;
        }
    }
    false
}

fn is_rtl_char(ch: char) -> bool {
    let c = ch as u32;
    matches!(
        c,
        0x0590..=0x05FF
            | 0x0600..=0x06FF
            | 0x0700..=0x074F
            | 0x0750..=0x077F
            | 0x0780..=0x07BF
            | 0x08A0..=0x08FF
            | 0xFB1D..=0xFDFF
            | 0xFE70..=0xFEFF
    )
}

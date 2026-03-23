//! This file implements animated text spacing adjustments.
//! It captures the original glyph layout, then reapplies per-frame spacing and
//! line-position offsets so authored text-spacing effects can deform Bevy's text
//! output without rebuilding the text mesh from scratch.
//!
//! 这个文件实现文本字距动画调整。它会先记录原始字形布局，再在每帧重新施加字距和
//! 行偏移变化，使作者定义的 text spacing 效果能够作用在 Bevy 产出的文本结果上，
//! 而不必每次都完全重建文本网格。

use bevy::prelude::*;
use bevy::sprite::Text2d;
use bevy::text::{ComputedTextBlock, TextLayoutInfo};

use crate::animation::AmPlayback;
use crate::animation::components::AmAnimated;
use crate::animation::interpolation::interpolate_float;
use crate::scene::AmLayerSpec;

#[derive(Component)]
pub struct AmOriginalGlyphs {
    pub data: Vec<OrigGlyph>,
    pub orig_size_y: f32,
}

pub struct OrigGlyph {
    pub position: Vec2,
    pub size: Vec2,
    pub line_index: usize,
    pub byte_index: usize,
    pub layout_x: f32,
    pub layout_line_y: f32,
    pub is_cjk: bool,
}

fn is_cjk_char(c: char) -> bool {
    let cp = c as u32;
    (0x4E00..=0x9FFF).contains(&cp)
        || (0x3400..=0x4DBF).contains(&cp)
        || (0x3040..=0x309F).contains(&cp)
        || (0x30A0..=0x30FF).contains(&cp)
        || (0xF900..=0xFAFF).contains(&cp)
        || (0xFF01..=0xFF60).contains(&cp)
        || (0x20000..=0x2A6DF).contains(&cp)
}

fn build_layout_map(
    computed: &ComputedTextBlock,
) -> std::collections::HashMap<(usize, usize), (f32, f32, f32)> {
    let mut map = std::collections::HashMap::new();
    for run in computed.buffer().0.layout_runs() {
        for glyph in run.glyphs {
            map.insert((run.line_i, glyph.start), (glyph.w, glyph.x, run.line_y));
        }
    }
    map
}

fn build_orig_glyph_data(
    layout_info: &TextLayoutInfo,
    computed: &ComputedTextBlock,
) -> Vec<OrigGlyph> {
    let layout_map = build_layout_map(computed);
    let line_texts: Vec<&str> = computed.buffer().0.lines.iter().map(|l| l.text()).collect();
    layout_info
        .glyphs
        .iter()
        .map(|g| {
            let (_advance, layout_x, layout_line_y) = layout_map
                .get(&(g.line_index, g.byte_index))
                .copied()
                .unwrap_or((g.size.x, 0.0, 0.0));
            let is_cjk = line_texts
                .get(g.line_index)
                .and_then(|t| t.get(g.byte_index..))
                .and_then(|s| s.chars().next())
                .map(is_cjk_char)
                .unwrap_or(false);
            OrigGlyph {
                position: g.position,
                size: g.size,
                line_index: g.line_index,
                byte_index: g.byte_index,
                layout_x,
                layout_line_y,
                is_cjk,
            }
        })
        .collect()
}

fn lookup_glyph_advance(
    computed: &ComputedTextBlock,
    originals: &[OrigGlyph],
    orig_line: usize,
    idx: usize,
) -> f32 {
    for run in computed.buffer().0.layout_runs() {
        if run.line_i != orig_line {
            continue;
        }
        for g in run.glyphs {
            if g.start == originals[idx].byte_index {
                return g.w;
            }
        }
    }
    originals[idx].size.x
}

fn flush_wrapped_line(
    current_glyphs: &mut Vec<(usize, f32)>,
    vline_data: &mut Vec<(Vec<(usize, f32)>, f32)>,
    content_cursor: &mut f32,
    last_non_space_end: &mut f32,
    last_break: &mut Option<(usize, f32)>,
    originals: &[OrigGlyph],
    line_glyphs: &[(usize, usize, f32)],
    advances: &[f32],
    letter_px: f32,
    wrap_width: f32,
    adv: f32,
) {
    if let Some((split, width)) = last_break.take() {
        let remaining: Vec<(usize, f32)> = current_glyphs.drain(split..).collect();
        vline_data.push((std::mem::take(current_glyphs), (width - letter_px).max(0.0)));
        *content_cursor = 0.0;
        *last_non_space_end = 0.0;
        for &(ri, _) in &remaining {
            current_glyphs.push((ri, *content_cursor));
            let orig_idx_in_line = line_glyphs.iter().position(|&(oi, _, _)| oi == ri).unwrap();
            *content_cursor += advances[orig_idx_in_line] + letter_px;
            let ri_space = originals[ri].size.x < 2.0 && originals[ri].size.y < 2.0;
            if !ri_space {
                *last_non_space_end = *content_cursor;
            }
        }
        *last_break = None;
        if !current_glyphs.is_empty() && *content_cursor + adv + letter_px > wrap_width {
            vline_data.push((
                std::mem::take(current_glyphs),
                (*last_non_space_end - letter_px).max(0.0),
            ));
            *content_cursor = 0.0;
            *last_non_space_end = 0.0;
            *last_break = None;
        }
    } else {
        vline_data.push((
            std::mem::take(current_glyphs),
            (*last_non_space_end - letter_px).max(0.0),
        ));
        *content_cursor = 0.0;
        *last_non_space_end = 0.0;
        *last_break = None;
    }
}

fn layout_orig_line(
    originals: &[OrigGlyph],
    computed: &ComputedTextBlock,
    orig_line: usize,
    letter_px: f32,
    wrap_width: f32,
    align_factor: f32,
    all_vlines: &mut Vec<(Vec<(usize, f32)>, f32)>,
) {
    let mut line_glyphs: Vec<(usize, usize, f32)> = originals
        .iter()
        .enumerate()
        .filter(|(_, g)| g.line_index == orig_line)
        .map(|(i, g)| (i, g.byte_index, g.layout_x))
        .collect();
    line_glyphs.sort_by_key(|&(_, byte, _)| byte);

    if line_glyphs.is_empty() {
        all_vlines.push((Vec::new(), 0.0));
        return;
    }

    let n = line_glyphs.len();
    let mut advances: Vec<f32> = Vec::with_capacity(n);
    for j in 0..n.saturating_sub(1) {
        advances.push(line_glyphs[j + 1].2 - line_glyphs[j].2);
    }
    if n > 0 {
        advances.push(lookup_glyph_advance(
            computed,
            originals,
            orig_line,
            line_glyphs[n - 1].0,
        ));
    }

    let orig_line_width: f32 = advances.iter().sum();
    let orig_align_offset = align_factor * (wrap_width - orig_line_width).max(0.0);
    let first_layout_x = line_glyphs[0].2;
    let left_margin = first_layout_x - orig_align_offset;

    let mut vline_data: Vec<(Vec<(usize, f32)>, f32)> = Vec::new();
    let mut current_glyphs: Vec<(usize, f32)> = Vec::new();
    let mut content_cursor: f32 = 0.0;
    let mut last_break: Option<(usize, f32)> = None;
    let mut last_non_space_end: f32 = 0.0;

    for (seq, &(idx, _, _)) in line_glyphs.iter().enumerate() {
        let adv = advances[seq];
        let is_space = originals[idx].size.x < 2.0 && originals[idx].size.y < 2.0;
        let cur_cjk = originals[idx].is_cjk;
        let prev_cjk = seq > 0 && originals[line_glyphs[seq - 1].0].is_cjk;

        if !current_glyphs.is_empty() && (cur_cjk || prev_cjk) {
            last_break = Some((current_glyphs.len(), last_non_space_end));
        }

        if !current_glyphs.is_empty() && content_cursor + adv + letter_px > wrap_width {
            flush_wrapped_line(
                &mut current_glyphs,
                &mut vline_data,
                &mut content_cursor,
                &mut last_non_space_end,
                &mut last_break,
                originals,
                &line_glyphs,
                &advances,
                letter_px,
                wrap_width,
                adv,
            );
        }

        current_glyphs.push((idx, content_cursor));
        content_cursor += adv + letter_px;

        if !is_space {
            last_non_space_end = content_cursor;
        }

        if is_space {
            last_break = Some((current_glyphs.len(), last_non_space_end));
        }
    }
    if !current_glyphs.is_empty() {
        vline_data.push((current_glyphs, (last_non_space_end - letter_px).max(0.0)));
    }

    for (vg, vw) in vline_data {
        let shifted: Vec<(usize, f32)> = vg
            .into_iter()
            .map(|(i, cx)| (i, left_margin + cx))
            .collect();
        all_vlines.push((shifted, vw));
    }
}

pub fn animate_text_spacing_system(
    playback: Res<AmPlayback>,
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &AmAnimated,
            &AmLayerSpec,
            &mut TextLayoutInfo,
            &ComputedTextBlock,
            Option<&AmOriginalGlyphs>,
        ),
        With<Text2d>,
    >,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (entity, animated, spec, mut layout_info, computed, orig_glyphs) in query.iter_mut() {
        let local_time = animated.calc_local_time(global_time);
        if !animated.is_active(local_time) {
            continue;
        }
        let layer_time = animated.calc_layer_time(local_time);

        let has_letter = animated.textspacing_letter.value.is_some()
            || !animated.textspacing_letter.keyframes.is_empty();
        let has_line = animated.textspacing_line.value.is_some()
            || !animated.textspacing_line.keyframes.is_empty();

        if !has_letter && !has_line {
            continue;
        }

        let (font_size, wrap_width, align) = match spec {
            AmLayerSpec::Text {
                font_size,
                wrap_width,
                align,
                ..
            } => (*font_size, *wrap_width, align.as_str()),
            _ => continue,
        };

        let letter_em = interpolate_float(&animated.textspacing_letter, layer_time).unwrap_or(0.0);
        let line_mult = interpolate_float(&animated.textspacing_line, layer_time).unwrap_or(1.0);
        let letter_px = letter_em * font_size;

        if layout_info.glyphs.is_empty() {
            continue;
        }

        if orig_glyphs.is_none() {
            let data = build_orig_glyph_data(&layout_info, computed);
            commands.entity(entity).insert(AmOriginalGlyphs {
                data,
                orig_size_y: layout_info.size.y,
            });
        }

        if letter_px.abs() < 0.01 && (line_mult - 1.0).abs() < 0.01 {
            let Some(orig) = orig_glyphs else {
                continue;
            };
            for (glyph, og) in layout_info.glyphs.iter_mut().zip(orig.data.iter()) {
                glyph.position = og.position;
            }
            layout_info.size.y = orig.orig_size_y;
            continue;
        }

        let owned_originals = if orig_glyphs.is_none() {
            Some(build_orig_glyph_data(&layout_info, computed))
        } else {
            None
        };
        let originals: &[OrigGlyph] = if let Some(orig) = orig_glyphs {
            &orig.data
        } else {
            owned_originals.as_ref().unwrap()
        };
        let glyphs = &mut layout_info.glyphs;

        let base_line_height = {
            let mut line_ys: Vec<f32> = originals.iter().map(|g| g.layout_line_y).collect();
            line_ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
            line_ys.dedup_by(|a, b| (*a - *b).abs() < 1.0);
            if line_ys.len() >= 2 {
                line_ys[1] - line_ys[0]
            } else {
                font_size * 1.2
            }
        };
        let android_spacing_base = font_size * 1.32;
        let new_line_height = base_line_height + android_spacing_base * (line_mult - 1.0);

        let first_orig_line_y = originals
            .iter()
            .map(|g| g.layout_line_y)
            .reduce(f32::min)
            .unwrap_or(0.0);

        let max_orig_line = originals.iter().map(|g| g.line_index).max().unwrap_or(0);
        let align_factor: f32 = match align {
            "center" => 0.5,
            "right" => 1.0,
            _ => 0.0,
        };

        let mut all_vlines: Vec<(Vec<(usize, f32)>, f32)> = Vec::new();

        for orig_line in 0..=max_orig_line {
            layout_orig_line(
                originals,
                computed,
                orig_line,
                letter_px,
                wrap_width,
                align_factor,
                &mut all_vlines,
            );
        }

        let total_vlines = all_vlines.len();
        let new_height = (total_vlines as f32 - 1.0) * new_line_height + base_line_height;

        for (vline_idx, (vline_glyphs, vline_width)) in all_vlines.iter().enumerate() {
            let new_align_offset = align_factor * (wrap_width - vline_width).max(0.0);
            for &(glyph_idx, layout_x) in vline_glyphs {
                let new_layout_x = layout_x + new_align_offset;
                let delta_x = new_layout_x - originals[glyph_idx].layout_x;
                glyphs[glyph_idx].position.x = originals[glyph_idx].position.x + delta_x;

                let new_manual_line_y = first_orig_line_y + vline_idx as f32 * new_line_height;
                let delta_y = new_manual_line_y - originals[glyph_idx].layout_line_y;
                glyphs[glyph_idx].position.y = originals[glyph_idx].position.y + delta_y;
            }
        }

        layout_info.size.y = new_height;
    }
}

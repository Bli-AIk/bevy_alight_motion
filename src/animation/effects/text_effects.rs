//! Text spacing and text progress animation systems.
//!
//! Letter spacing is applied by re-laying-out glyphs with proper wrapping,
//! since AM applies letter spacing BEFORE text layout (affecting line breaks).
//!
//! Bevy only re-computes TextLayoutInfo when input components change, so our glyph
//! position modifications accumulate across frames. We store original positions in
//! `AmOriginalGlyphs` and always compute from those.

use bevy::prelude::*;
use bevy::sprite::Text2d;
use bevy::text::{ComputedTextBlock, TextLayoutInfo};

use crate::animation::AmPlayback;
use crate::animation::components::AmAnimated;
use crate::animation::interpolation::interpolate_float;
use crate::scene::AmLayerSpec;

/// Marker: RTL line alignment has been corrected for this entity.
#[derive(Component)]
pub struct AmRtlAlignmentFixed;

/// System: fix line-level alignment for RTL paragraphs.
///
/// AM maps "left"→Android ALIGN_NORMAL (LTR=left, RTL=right) and
/// "right"→ALIGN_OPPOSITE (LTR=right, RTL=left). Bevy's Justify::Left/Right
/// forces all lines to the same side. This system shifts RTL lines to match AM behavior.
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

        // Center alignment is symmetric — no RTL fix needed
        if align == "center" {
            commands.entity(entity).insert(AmRtlAlignmentFixed);
            continue;
        }

        let paragraphs: Vec<&str> = content.split('\n').collect();

        // Group glyph indices by visual line (line_index + approximate y)
        let mut visual_lines: Vec<(usize, f32, Vec<usize>)> = Vec::new(); // (para_idx, y, glyph_indices)
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

            // Compute current line extent
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
                "left" => wrap_width - right_edge, // RTL line → right-align
                "right" => -left_edge,             // RTL line → left-align
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

/// Check if a paragraph's base direction is RTL by finding the first strong directional character.
fn is_rtl_paragraph(text: &str) -> bool {
    for ch in text.chars() {
        if is_rtl_char(ch) {
            return true;
        }
        if ch.is_alphanumeric() {
            return false; // strong LTR
        }
    }
    false
}

fn is_rtl_char(ch: char) -> bool {
    let c = ch as u32;
    matches!(
        c,
        0x0590..=0x05FF   // Hebrew
        | 0x0600..=0x06FF // Arabic
        | 0x0700..=0x074F // Syriac
        | 0x0750..=0x077F // Arabic Supplement
        | 0x0780..=0x07BF // Thaana
        | 0x08A0..=0x08FF // Arabic Extended-A
        | 0xFB1D..=0xFDFF // Hebrew/Arabic Presentation Forms
        | 0xFE70..=0xFEFF // Arabic Presentation Forms-B
    )
}

/// Stores original glyph layout data from Bevy's initial text layout.
/// Used as the base for all letter spacing computations to avoid accumulation.
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
    /// Layout x position from cosmic-text glyph.x
    pub layout_x: f32,
    /// Layout line y from cosmic-text run.line_y
    pub layout_line_y: f32,
    /// Whether this glyph is a CJK character (allows line-break before/after).
    pub is_cjk: bool,
}

/// Check if a character is CJK (allows line-break at character boundaries).
fn is_cjk_char(c: char) -> bool {
    let cp = c as u32;
    (0x4E00..=0x9FFF).contains(&cp)   // CJK Unified Ideographs
        || (0x3400..=0x4DBF).contains(&cp) // CJK Extension A
        || (0x3040..=0x309F).contains(&cp) // Hiragana
        || (0x30A0..=0x30FF).contains(&cp) // Katakana
        || (0xF900..=0xFAFF).contains(&cp) // CJK Compatibility Ideographs
        || (0xFF01..=0xFF60).contains(&cp) // Fullwidth Forms
        || (0x20000..=0x2A6DF).contains(&cp) // CJK Extension B
}

/// Build a mapping from (line_index, byte_index) to (advance_w, layout_x, line_y)
/// using cosmic-text buffer data.
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

/// Build original glyph data from current layout positions and cosmic-text buffer.
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

/// Look up the advance width for a specific glyph from the cosmic-text buffer.
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

/// Handle line wrapping when a glyph overflows the wrap width.
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
        // Re-check: if current glyph still overflows after rebasing
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

/// Process one original text line: compute advances, apply CJK-aware wrapping
/// with letter spacing, and append resulting virtual lines to `all_vlines`.
fn layout_orig_line(
    originals: &[OrigGlyph],
    computed: &ComputedTextBlock,
    orig_line: usize,
    letter_px: f32,
    wrap_width: f32,
    align_factor: f32,
    all_vlines: &mut Vec<(Vec<(usize, f32)>, f32)>,
) {
    // Sort glyphs by byte_index; store (orig_index, byte_index, layout_x)
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

    // Compute advances from consecutive layout_x differences (more accurate
    // than glyph.w since it accounts for shaping adjustments).
    // For the last glyph, use the buffer's advance width.
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
    // left_margin in layout-space: where content starts (before alignment)
    let first_layout_x = line_glyphs[0].2;
    let left_margin = first_layout_x - orig_align_offset;

    // CJK-aware wrapping with letter spacing.
    // Android treats each CJK char as an independent break unit.
    let mut vline_data: Vec<(Vec<(usize, f32)>, f32)> = Vec::new();
    let mut current_glyphs: Vec<(usize, f32)> = Vec::new();
    let mut content_cursor: f32 = 0.0;
    // Unified break tracking: (split_at index in current_glyphs, alignment_width)
    // alignment_width excludes trailing whitespace (matching Android getLineWidth).
    let mut last_break: Option<(usize, f32)> = None;
    // Cursor after the last non-space glyph (for alignment width calculation).
    let mut last_non_space_end: f32 = 0.0;

    for (seq, &(idx, _, _)) in line_glyphs.iter().enumerate() {
        let adv = advances[seq];
        let is_space = originals[idx].size.x < 2.0 && originals[idx].size.y < 2.0;
        let cur_cjk = originals[idx].is_cjk;
        let prev_cjk = seq > 0 && originals[line_glyphs[seq - 1].0].is_cjk;

        // Break opportunity BEFORE this glyph (between prev and this)
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

        // Break opportunity AFTER space (space stays on current line)
        if is_space {
            last_break = Some((current_glyphs.len(), last_non_space_end));
        }
    }
    if !current_glyphs.is_empty() {
        vline_data.push((current_glyphs, (last_non_space_end - letter_px).max(0.0)));
    }

    // Store vlines with left_margin baked into content-space positions
    for (vg, vw) in vline_data {
        let shifted: Vec<(usize, f32)> = vg
            .into_iter()
            .map(|(i, cx)| (i, left_margin + cx))
            .collect();
        all_vlines.push((shifted, vw));
    }
}

/// System: animate textspacing values by re-laying-out glyphs with proper wrapping.
///
/// AM applies letter spacing at the Paint level BEFORE StaticLayout, so wrapping
/// considers the wider characters. We replicate this by re-computing glyph positions
/// after Bevy's initial layout, applying letter spacing and line spacing, then
/// re-wrapping lines that overflow wrap_width.
///
/// Positions are computed using delta from original cosmic-text layout positions,
/// preserving atlas offsets exactly.
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

        // AM: paint.setLetterSpacing(em) adds em * fontSize pixels.
        // font_size in AmLayerSpec::Text is already 3x (TEXT_SIZE_MULTIPLIER applied).
        let letter_px = letter_em * font_size;

        if layout_info.glyphs.is_empty() {
            continue;
        }

        // --- Store original glyph positions on first encounter ---
        if orig_glyphs.is_none() {
            let data = build_orig_glyph_data(&layout_info, computed);
            commands.entity(entity).insert(AmOriginalGlyphs {
                data,
                orig_size_y: layout_info.size.y,
            });
        }

        if letter_px.abs() < 0.01 && (line_mult - 1.0).abs() < 0.01 {
            // Restore glyphs and size from originals before skipping.
            let Some(orig) = orig_glyphs else {
                continue;
            };
            for (glyph, og) in layout_info.glyphs.iter_mut().zip(orig.data.iter()) {
                glyph.position = og.position;
            }
            layout_info.size.y = orig.orig_size_y;
            continue;
        }

        // Build fallback originals before taking mutable borrow on glyphs
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

        // --- Determine base line height ---
        // cosmic-text's natural line height (hhea ascent + descent).
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
        // AM applies its spacing multiplier via Android StaticLayout which uses
        // font metrics (including CJK fallback and includePad adjustments) that
        // are larger than cosmic-text's hhea-based line height.  Empirically the
        // effective per-unit-m slope is ≈ fontSize × 1.3 for Roboto + CJK.
        // Keep cosmic-text's natural gap at m=1 for smooth continuity and scale
        // only the EXTRA spacing with the Android-equivalent slope.
        let android_spacing_base = font_size * 1.32;
        let new_line_height = base_line_height + android_spacing_base * (line_mult - 1.0);

        let first_orig_line_y = originals
            .iter()
            .map(|g| g.layout_line_y)
            .reduce(f32::min)
            .unwrap_or(0.0);

        // --- Collect glyphs per original line with true advances ---
        let max_orig_line = originals.iter().map(|g| g.line_index).max().unwrap_or(0);
        let align_factor: f32 = match align {
            "center" => 0.5,
            "right" => 1.0,
            _ => 0.0,
        };

        // --- Pass 1: Assign all glyphs to virtual lines ---
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

        // --- Pass 2: Apply positions using delta from original layout positions ---
        let total_vlines = all_vlines.len();
        // Android StaticLayout does NOT apply spacing to the last line.
        let new_height = (total_vlines as f32 - 1.0) * new_line_height + base_line_height;

        for (vline_idx, (vline_glyphs, vline_width)) in all_vlines.iter().enumerate() {
            let new_align_offset = align_factor * (wrap_width - vline_width).max(0.0);
            for &(glyph_idx, layout_x) in vline_glyphs {
                // X: delta from original cosmic-text layout position
                let new_layout_x = layout_x + new_align_offset;
                let delta_x = new_layout_x - originals[glyph_idx].layout_x;
                glyphs[glyph_idx].position.x = originals[glyph_idx].position.x + delta_x;

                // Y: delta from original cosmic-text line_y
                let new_manual_line_y = first_orig_line_y + vline_idx as f32 * new_line_height;
                let delta_y = new_manual_line_y - originals[glyph_idx].layout_line_y;
                glyphs[glyph_idx].position.y = originals[glyph_idx].position.y + delta_y;
            }
        }

        // Update the layout size so Anchor::CENTER accounts for the new text block height.
        layout_info.size.y = new_height;
    }
}

/// Marker component on cursor sprite child entities.
#[derive(Component)]
pub struct AmProgressCursor;

/// Stored on the text entity; references its cursor sprite child.
#[derive(Component)]
pub struct AmProgressCursorRef(pub Entity);

/// Stores the original text content before text progress slicing.
#[derive(Component)]
pub struct AmOriginalText(pub String);

/// System: animate textprogress effect — slice text content and append cursor character.
///
/// Matches AM's JS implementation:
///   `el.text = el.text.slice(Math.round(len * start), Math.round(len * end)) + cursorChar`
///
/// Uses Text2d modification (text-slicing) so Bevy re-lays-out the text,
/// matching AM's behavior of re-rendering sliced text each frame.
pub fn animate_text_progress_system(
    playback: Res<AmPlayback>,
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &AmAnimated,
        &AmLayerSpec,
        &mut Text2d,
        Option<&AmProgressCursorRef>,
        Option<&AmOriginalText>,
    )>,
    mut cursor_query: Query<&mut Visibility, With<AmProgressCursor>>,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (entity, animated, spec, mut text2d, cursor_ref, orig_text) in query.iter_mut() {
        let has_progress = animated.textprogress_start.value.is_some()
            || !animated.textprogress_start.keyframes.is_empty()
            || !animated.textprogress_end.keyframes.is_empty();

        if !has_progress {
            continue;
        }

        let cursor_type = animated.textprogress_cursor;
        let blink = animated.textprogress_blink;

        let original_content = match spec {
            AmLayerSpec::Text { content, .. } => content.as_str(),
            _ => continue,
        };

        let local_time = animated.calc_local_time(global_time);
        if !animated.is_active(local_time) {
            if let Some(cref) = cursor_ref
                && let Ok(mut vis) = cursor_query.get_mut(cref.0)
            {
                *vis = Visibility::Hidden;
            }
            continue;
        }

        let layer_time = animated.calc_layer_time(local_time);

        let start = interpolate_float(&animated.textprogress_start, layer_time)
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        let end = interpolate_float(&animated.textprogress_end, layer_time)
            .unwrap_or(1.0)
            .clamp(0.0, 1.0);

        // Store original text on first run
        if orig_text.is_none() {
            commands
                .entity(entity)
                .insert(AmOriginalText(original_content.to_string()));
        }

        let source_text = orig_text.map(|o| o.0.as_str()).unwrap_or(original_content);
        let char_count = source_text.chars().count();

        if char_count == 0 {
            continue;
        }

        // AM: Math.round(el.text.length * p.start), Math.round(el.text.length * p.end)
        let slice_start = if start <= 1e-5 {
            0
        } else {
            (start * char_count as f32).round().min(char_count as f32) as usize
        };
        let slice_end = if (end - 1.0).abs() < 1e-5 {
            char_count
        } else {
            (end * char_count as f32).round().min(char_count as f32) as usize
        };

        // AM cursor characters (Unicode block elements)
        let cursor_char = match cursor_type {
            0 => "",
            1 => "_",
            2 => "\u{2588}", // █
            3 => "\u{258C}", // ▌
            4 => "\u{2581}", // ▁
            5 => "\u{258F}", // ▏
            6 => "\u{2595}", // ▕
            7 => "\u{25AF}", // ▯
            8 => "\u{258E}", // ▎
            _ => "",
        };

        // AM blink: only blinks when text is static (same visible chars as previous frame).
        let show_cursor = if cursor_type == 0 {
            false
        } else if blink {
            (global_time as u64 % 1000) < 500
        } else {
            true
        };

        // Compute sliced text + cursor (exactly like AM)
        let mut sliced: String = source_text
            .chars()
            .skip(slice_start)
            .take(slice_end.saturating_sub(slice_start))
            .collect();

        if show_cursor {
            sliced.push_str(cursor_char);
        }

        // Only update Text2d if content changed (avoids unnecessary re-layout)
        if text2d.0 != sliced {
            text2d.0 = sliced;
        }

        // Hide cursor sprites if they exist
        if let Some(cref) = cursor_ref
            && let Ok(mut vis) = cursor_query.get_mut(cref.0)
        {
            *vis = Visibility::Hidden;
        }
    }
}

/// Counter effect: replaces numeric segments in text with offset/scaled values.
///
/// Matches AM's `com.alightcreative.effects.counter` JS behavior:
/// - Splits text into number segments and non-number characters
/// - For each number: `adjusted = parseFloat(num) * scale + offset`
/// - Preserves decimal places and thousand-separator formatting
pub fn animate_counter_system(
    playback: Res<AmPlayback>,
    mut query: Query<(&AmAnimated, &AmLayerSpec, &mut Text2d), Without<AmOriginalText>>,
    mut query_with_orig: Query<(&AmAnimated, &AmLayerSpec, &mut Text2d, &AmOriginalText)>,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    // Process entities without AmOriginalText (first frame or no text progress)
    for (animated, spec, mut text2d) in query.iter_mut() {
        if let Some(new_text) = apply_counter(animated, spec, global_time)
            && text2d.0 != new_text
        {
            text2d.0 = new_text;
        }
    }

    // Process entities with AmOriginalText (text progress is also active)
    // Counter modifies the original content, then text progress slices it
    for (animated, _spec, mut text2d, orig) in query_with_orig.iter_mut() {
        if let Some(new_text) = apply_counter_to_source(animated, &orig.0, global_time)
            && text2d.0 != new_text
        {
            text2d.0 = new_text;
        }
    }
}

/// Check if counter effect is active and apply it to text from AmLayerSpec.
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

    let result = counter_transform(content, offset as f64, scale as f64);
    Some(result)
}

/// Apply counter to arbitrary source text (for entities with AmOriginalText).
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

/// Transform text content by replacing numeric segments with adjusted values.
///
/// Matches AM's JS regex: `/([-+]?[0-9,]*\.[0-9,]*)|([-+]?[0-9,]+)|(.)/giu`
/// For each number segment: `adjusted = parseFloat(num) * scale + offset`
fn counter_transform(text: &str, offset: f64, scale: f64) -> String {
    let mut result = String::with_capacity(text.len() + 16);
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Try to match a number starting at position i
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

/// Transform a single number string by applying counter offset and scale.
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

/// Try to parse a number segment starting at position `start` in the char array.
/// Returns (number_string, end_position) if a number is found.
///
/// Matches AM's JS regex alternatives:
/// 1. `[-+]?[0-9,]*\.[0-9,]*` (decimal number)
/// 2. `[-+]?[0-9,]+` (integer)
fn try_parse_number(chars: &[char], start: usize) -> Option<(String, usize)> {
    let mut pos = start;

    // Optional sign
    let has_sign = pos < chars.len() && (chars[pos] == '+' || chars[pos] == '-');
    if has_sign {
        pos += 1;
    }

    // Check if next char is a digit or comma (for integer part) or dot (for decimal)
    if pos >= chars.len() {
        return None;
    }

    let first_after_sign = chars[pos];
    if !first_after_sign.is_ascii_digit() && first_after_sign != '.' {
        return None;
    }

    // Consume digits and commas before decimal point
    let mut has_digits = false;
    while pos < chars.len() && (chars[pos].is_ascii_digit() || chars[pos] == ',') {
        if chars[pos].is_ascii_digit() {
            has_digits = true;
        }
        pos += 1;
    }

    // Check for decimal point
    let has_dot = pos < chars.len() && chars[pos] == '.';
    if has_dot {
        pos += 1;
        // Consume digits and commas after decimal point
        while pos < chars.len() && (chars[pos].is_ascii_digit() || chars[pos] == ',') {
            if chars[pos].is_ascii_digit() {
                has_digits = true;
            }
            pos += 1;
        }
    }

    // Must have at least one digit to be a valid number
    if !has_digits {
        return None;
    }

    // A lone sign without digits is not a number
    if has_sign && pos == start + 1 {
        return None;
    }

    let num_str: String = chars[start..pos].iter().collect();
    Some((num_str, pos))
}

/// Format a number with thousands separators and specified decimal places.
fn format_with_thousands(n: f64, dp: usize) -> String {
    let formatted = format!("{:.prec$}", n, prec = dp);
    let (int_part, dec_part) = if let Some(dot_pos) = formatted.find('.') {
        (&formatted[..dot_pos], Some(&formatted[dot_pos..]))
    } else {
        (formatted.as_str(), None)
    };

    // Handle negative numbers
    let (sign, digits) = if let Some(stripped) = int_part.strip_prefix('-') {
        ("-", stripped)
    } else {
        ("", int_part)
    };

    // Insert commas
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

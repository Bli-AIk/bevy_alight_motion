//! Text spacing and text progress animation systems.
//!
//! Letter spacing is applied by re-laying-out glyphs with proper wrapping,
//! since AM applies letter spacing BEFORE text layout (affecting line breaks).
//!
//! Bevy only re-computes TextLayoutInfo when input components change, so our glyph
//! position modifications accumulate across frames. We store original positions in
//! `AmOriginalGlyphs` and always compute from those.

use bevy::prelude::*;
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

            if shift_x.abs() > 0.5 {
                for &gi in glyph_indices {
                    layout_info.glyphs[gi].position.x += shift_x;
                }
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
    debug_q: Query<Entity, (With<AmAnimated>, With<Text2d>)>,
    debug_q2: Query<Entity, With<Text2d>>,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    if global_time < 200.0 {
        eprintln!(
            "DBG_SPACING gt={:.0} full={} am+t2d={} t2d={}",
            global_time,
            query.iter().count(),
            debug_q.iter().count(),
            debug_q2.iter().count()
        );
    }

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

        if global_time < 100.0 {
            eprintln!(
                "DBG_INNER gt={:.0} has_letter={} kf={} glyphs={}",
                global_time,
                has_letter,
                animated.textspacing_letter.keyframes.len(),
                layout_info.glyphs.len()
            );
        }

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
            // Read layout positions from cosmic-text buffer
            let mut layout_map: std::collections::HashMap<(usize, usize), (f32, f32, f32)> =
                std::collections::HashMap::new();
            for run in computed.buffer().0.layout_runs() {
                for glyph in run.glyphs {
                    layout_map.insert((run.line_i, glyph.start), (glyph.w, glyph.x, run.line_y));
                }
            }
            // Collect line texts for CJK detection
            let line_texts: Vec<&str> =
                computed.buffer().0.lines.iter().map(|l| l.text()).collect();
            let data: Vec<OrigGlyph> = layout_info
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
                .collect();
            commands.entity(entity).insert(AmOriginalGlyphs {
                data,
                orig_size_y: layout_info.size.y,
            });
        }

        if letter_px.abs() < 0.01 && (line_mult - 1.0).abs() < 0.01 {
            // Restore glyphs and size from originals before skipping.
            if let Some(orig) = orig_glyphs {
                for (i, glyph) in layout_info.glyphs.iter_mut().enumerate() {
                    if i < orig.data.len() {
                        glyph.position = orig.data[i].position;
                    }
                }
                layout_info.size.y = orig.orig_size_y;
            }
            continue;
        }

        let glyphs = &mut layout_info.glyphs;

        // Use stored originals if available, otherwise read from current glyphs (first frame)
        let owned_originals: Vec<OrigGlyph>;
        let originals: &[OrigGlyph] = if let Some(orig) = orig_glyphs {
            &orig.data
        } else {
            // First frame: originals not yet in ECS, read buffer for true advances
            let mut layout_map: std::collections::HashMap<(usize, usize), (f32, f32, f32)> =
                std::collections::HashMap::new();
            for run in computed.buffer().0.layout_runs() {
                for glyph in run.glyphs {
                    layout_map.insert((run.line_i, glyph.start), (glyph.w, glyph.x, run.line_y));
                }
            }
            let line_texts: Vec<&str> =
                computed.buffer().0.lines.iter().map(|l| l.text()).collect();
            owned_originals = glyphs
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
                .collect();
            &owned_originals
        };

        // --- Determine base line height from cosmic-text run.line_y values ---
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
        let new_line_height = base_line_height * line_mult;

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
        // Each entry: (glyph assignments with content-relative x, line width)
        let mut all_vlines: Vec<(Vec<(usize, f32)>, f32)> = Vec::new();

        for orig_line in 0..=max_orig_line {
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
                continue;
            }

            // Compute advances from consecutive layout_x differences (more accurate
            // than glyph.w since it accounts for shaping adjustments).
            // For the last glyph, use the buffer's advance width.
            let n = line_glyphs.len();
            let mut advances: Vec<f32> = Vec::with_capacity(n);
            for j in 0..n {
                if j + 1 < n {
                    advances.push(line_glyphs[j + 1].2 - line_glyphs[j].2);
                } else {
                    // Last glyph: use layout_x + advance = line_end, so advance = line_w - layout_x
                    // Approximate with original glyph size as fallback
                    let idx = line_glyphs[j].0;
                    // Read advance from buffer if available
                    let mut adv = originals[idx].size.x;
                    for run in computed.buffer().0.layout_runs() {
                        if run.line_i == orig_line {
                            for g in run.glyphs {
                                if g.start == originals[idx].byte_index {
                                    adv = g.w;
                                    break;
                                }
                            }
                        }
                    }
                    advances.push(adv);
                }
            }

            let orig_line_width: f32 = advances.iter().sum();
            // DEBUG: print advances for first entity only, first frame
            if orig_line == 0 && playback.current_time_ms < 10.0 {
                eprintln!(
                    "DEBUG advances line {}: total={:.1} wrap_width={:.1} letter_px={:.1}",
                    orig_line, orig_line_width, wrap_width, letter_px
                );
                for (j, &adv) in advances.iter().enumerate() {
                    let ch = line_glyphs
                        .get(j)
                        .and_then(|&(idx, _, _)| {
                            let bi = originals[idx].byte_index;
                            computed
                                .buffer()
                                .0
                                .lines
                                .get(orig_line)
                                .and_then(|l| l.text().get(bi..))
                                .and_then(|s| s.chars().next())
                        })
                        .unwrap_or('?');
                    eprintln!(
                        "  [{j}] char='{}' adv={:.2} layout_x={:.2}",
                        ch, adv, line_glyphs[j].2
                    );
                }
            }
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
                    if let Some((split, width)) = last_break.take() {
                        let remaining: Vec<(usize, f32)> = current_glyphs.drain(split..).collect();
                        vline_data.push((
                            std::mem::take(&mut current_glyphs),
                            (width - letter_px).max(0.0),
                        ));
                        content_cursor = 0.0;
                        last_non_space_end = 0.0;
                        for &(ri, _) in &remaining {
                            current_glyphs.push((ri, content_cursor));
                            let orig_idx_in_line =
                                line_glyphs.iter().position(|&(oi, _, _)| oi == ri).unwrap();
                            content_cursor += advances[orig_idx_in_line] + letter_px;
                            let ri_space = originals[ri].size.x < 2.0 && originals[ri].size.y < 2.0;
                            if !ri_space {
                                last_non_space_end = content_cursor;
                            }
                        }
                        last_break = None;
                        // Re-check: if current glyph still overflows after rebasing
                        if !current_glyphs.is_empty()
                            && content_cursor + adv + letter_px > wrap_width
                        {
                            vline_data.push((
                                std::mem::take(&mut current_glyphs),
                                (last_non_space_end - letter_px).max(0.0),
                            ));
                            content_cursor = 0.0;
                            last_non_space_end = 0.0;
                            last_break = None;
                        }
                    } else {
                        vline_data.push((
                            std::mem::take(&mut current_glyphs),
                            (last_non_space_end - letter_px).max(0.0),
                        ));
                        content_cursor = 0.0;
                        last_non_space_end = 0.0;
                        last_break = None;
                    }
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

/// System: animate textprogress effect — hide glyphs outside the visible range and render cursor.
pub fn animate_text_progress_system(
    playback: Res<AmPlayback>,
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &AmAnimated,
            &AmLayerSpec,
            &mut TextLayoutInfo,
            Option<&AmProgressCursorRef>,
            Option<&AmOriginalGlyphs>,
        ),
        With<Text2d>,
    >,
    mut cursor_query: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<AmProgressCursor>>,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (entity, animated, spec, mut layout_info, cursor_ref, orig_glyphs) in query.iter_mut() {
        let has_progress = animated.textprogress_start.value.is_some()
            || !animated.textprogress_start.keyframes.is_empty()
            || !animated.textprogress_end.keyframes.is_empty();

        if !has_progress {
            continue;
        }

        let cursor_type = animated.textprogress_cursor;
        let blink = animated.textprogress_blink;

        let (font_size, wrap_width) = match spec {
            AmLayerSpec::Text {
                font_size,
                wrap_width,
                ..
            } => (*font_size, *wrap_width),
            _ => continue,
        };

        let local_time = animated.calc_local_time(global_time);
        if !animated.is_active(local_time) {
            if let Some(cref) = cursor_ref
                && let Ok((_, _, mut vis)) = cursor_query.get_mut(cref.0)
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

        let layout_size = layout_info.size;
        let total = layout_info.glyphs.len();
        if total == 0 {
            continue;
        }

        // Store originals for glyph restoration (only if text_spacing hasn't done it)
        let has_spacing = animated.textspacing_letter.value.is_some()
            || !animated.textspacing_letter.keyframes.is_empty()
            || animated.textspacing_line.value.is_some()
            || !animated.textspacing_line.keyframes.is_empty();

        if orig_glyphs.is_none() && !has_spacing {
            let data: Vec<OrigGlyph> = layout_info
                .glyphs
                .iter()
                .map(|g| OrigGlyph {
                    position: g.position,
                    size: g.size,
                    line_index: g.line_index,
                    byte_index: g.byte_index,
                    layout_x: 0.0,
                    layout_line_y: 0.0,
                    is_cjk: false,
                })
                .collect();
            commands.entity(entity).insert(AmOriginalGlyphs {
                data,
                orig_size_y: layout_info.size.y,
            });
        }

        let glyphs = &mut layout_info.glyphs;

        // AM clips text via path-length; we approximate with character count.
        // Path-based clipping partially shows boundary characters; we can only show/hide whole chars.
        // Round for start (boundary char more likely partially hidden → hide it),
        // Floor for end (boundary char more likely partially visible → hide it for consistency).
        let visible_start = if start <= 1e-5 {
            0
        } else {
            (start * total as f32).round().min(total as f32) as usize
        };
        let visible_end = if (end - 1.0).abs() < 1e-5 {
            total
        } else {
            (end * total as f32).floor().min(total as f32) as usize
        };
        let last_visible = if visible_end > visible_start {
            Some(visible_end - 1)
        } else {
            None
        };

        // Restore previously-hidden glyphs to original positions.
        if !has_spacing && let Some(orig) = orig_glyphs {
            for i in visible_start..visible_end.min(total) {
                if glyphs[i].position.x <= -9999.0 {
                    glyphs[i].position = orig.data[i].position;
                }
            }
        }

        // Compute cursor position BEFORE hiding glyphs.
        let cursor_info = if cursor_type != 0 && last_visible.is_some() {
            let last_idx = last_visible.unwrap();
            let g = &glyphs[last_idx];
            let glyph_right = g.position.x + g.size.x / 2.0;

            let (cw, ch) = match cursor_type {
                1 => (font_size * 0.5, (font_size * 0.055).max(3.0)), // underscore: wide, thin
                2 => (font_size * 0.55, font_size * 0.85),            // block: character-sized
                3 => ((font_size * 0.08).max(2.0), font_size * 0.85), // pipe: thin, tall
                _ => (0.0, 0.0),
            };

            let cursor_cx = glyph_right + cw / 2.0;
            let cursor_cy = match cursor_type {
                1 => g.position.y + g.size.y / 2.0 - ch / 2.0, // bottom of glyph
                _ => g.position.y,                             // vertically centered
            };

            // Text-box coords → entity local coords
            // Anchor is corrected to center on wrapWidth, so offset by wrapWidth/2
            let local_x = cursor_cx - wrap_width / 2.0;
            let local_y = layout_size.y / 2.0 - cursor_cy;

            let visible = if blink {
                (global_time as u64 % 1000) < 500
            } else {
                true
            };

            Some((local_x, local_y, cw, ch, visible))
        } else {
            None
        };

        // Hide glyphs outside visible range
        for (i, glyph) in glyphs.iter_mut().enumerate() {
            if i < visible_start || i >= visible_end {
                glyph.position.x = -10000.0;
            }
        }

        // Spawn or update cursor entity
        if let Some((cx, cy, cw, ch, visible)) = cursor_info {
            let vis = if visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if let Some(cref) = cursor_ref {
                if let Ok((mut tf, mut sp, mut v)) = cursor_query.get_mut(cref.0) {
                    tf.translation.x = cx;
                    tf.translation.y = cy;
                    sp.custom_size = Some(Vec2::new(cw, ch));
                    *v = vis;
                }
            } else {
                let cursor_e = commands
                    .spawn((
                        AmProgressCursor,
                        Sprite {
                            color: Color::WHITE,
                            custom_size: Some(Vec2::new(cw, ch)),
                            ..default()
                        },
                        Transform::from_xyz(cx, cy, 0.01),
                        vis,
                        ChildOf(entity),
                    ))
                    .id();
                commands
                    .entity(entity)
                    .insert(AmProgressCursorRef(cursor_e));
            }
        } else if let Some(cref) = cursor_ref
            && let Ok((_, _, mut vis)) = cursor_query.get_mut(cref.0)
        {
            *vis = Visibility::Hidden;
        }
    }
}

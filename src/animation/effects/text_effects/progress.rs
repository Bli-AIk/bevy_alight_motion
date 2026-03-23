use bevy::prelude::*;
use bevy::sprite::Text2d;

use crate::animation::AmPlayback;
use crate::animation::components::AmAnimated;
use crate::animation::interpolation::interpolate_float;
use crate::scene::AmLayerSpec;

#[derive(Component)]
pub struct AmProgressCursor;

#[derive(Component)]
pub struct AmProgressCursorRef(pub Entity);

#[derive(Component)]
pub struct AmOriginalText(pub String);

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

        let cursor_char = match cursor_type {
            0 => "",
            1 => "_",
            2 => "\u{2588}",
            3 => "\u{258C}",
            4 => "\u{2581}",
            5 => "\u{258F}",
            6 => "\u{2595}",
            7 => "\u{25AF}",
            8 => "\u{258E}",
            _ => "",
        };

        let show_cursor = if cursor_type == 0 {
            false
        } else if blink {
            (global_time as u64 % 1000) < 500
        } else {
            true
        };

        let mut sliced: String = source_text
            .chars()
            .skip(slice_start)
            .take(slice_end.saturating_sub(slice_start))
            .collect();

        if show_cursor {
            sliced.push_str(cursor_char);
        }

        if text2d.0 != sliced {
            text2d.0 = sliced;
        }

        if let Some(cref) = cursor_ref
            && let Ok(mut vis) = cursor_query.get_mut(cref.0)
        {
            *vis = Visibility::Hidden;
        }
    }
}

use bevy::prelude::*;

use crate::animation::interpolation::interpolate_float;
use crate::animation::{AmAnimated, AmPlayback};
use crate::scene::{AmForceHidden, AmLayerMarker};

pub fn animate_opacity_system(
    playback: Res<AmPlayback>,
    mut query: Query<(&AmAnimated, &mut Sprite)>,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (animated, mut sprite) in query.iter_mut() {
        let local_time = animated.calc_local_time(global_time);
        if !animated.is_active(local_time) {
            sprite.color.set_alpha(0.0);
            continue;
        }

        let layer_time = animated.calc_layer_time(local_time);
        let opacity = interpolate_float(&animated.opacity, layer_time).unwrap_or(1.0);
        let mut final_alpha = (opacity * animated.base_alpha).clamp(0.0, 1.0);
        final_alpha *= animated.calc_fade_alpha(layer_time);
        if let Some(ref echo_cfg) = animated.echo_alpha_config {
            final_alpha *= echo_cfg.evaluate(global_time);
        }
        let corrected = if final_alpha > 0.001 && final_alpha < 0.999 {
            if final_alpha <= 0.04045 {
                final_alpha / 12.92
            } else {
                ((final_alpha + 0.055) / 1.055).powf(2.4)
            }
        } else {
            final_alpha
        };
        sprite.color.set_alpha(corrected);
    }
}

pub fn animate_text_opacity_system(
    playback: Res<AmPlayback>,
    mut query: Query<
        (
            &AmAnimated,
            &mut bevy::text::TextColor,
            &mut Visibility,
            &AmLayerMarker,
            Option<&AmForceHidden>,
        ),
        With<Text2d>,
    >,
) {
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;
    let text_count = query.iter().count();

    static mut FRAME_COUNT: u32 = 0;
    unsafe {
        FRAME_COUNT += 1;
        if FRAME_COUNT % 300 == 1 {
            bevy::log::trace!(
                "[TEXT] Processing {} text entities at time={:.0}",
                text_count,
                global_time
            );
        }
    }

    for (animated, mut text_color, mut visibility, marker, force_hidden) in query.iter_mut() {
        let local_time = animated.calc_local_time(global_time);

        if !animated.is_active(local_time) || force_hidden.is_some() {
            if force_hidden.is_none() && *visibility != Visibility::Hidden {
                bevy::log::trace!(
                    "[TEXT] Hiding '{}' (id={}): time={:.0}, range=[{}, {}]",
                    marker.label,
                    marker.id,
                    local_time,
                    animated.start_time,
                    animated.end_time
                );
            }
            *visibility = Visibility::Hidden;
            text_color.0.set_alpha(0.0);
            continue;
        }

        if *visibility == Visibility::Hidden {
            bevy::log::trace!(
                "[TEXT] Showing '{}' (id={}): time={:.0}, range=[{}, {}]",
                marker.label,
                marker.id,
                local_time,
                animated.start_time,
                animated.end_time
            );
        }
        *visibility = Visibility::Inherited;

        let layer_time = animated.calc_layer_time(local_time);
        let opacity = interpolate_float(&animated.opacity, layer_time).unwrap_or(1.0);
        let mut final_alpha = opacity * animated.base_alpha;
        final_alpha *= animated.calc_fade_alpha(layer_time);
        if let Some(ref echo_cfg) = animated.echo_alpha_config {
            final_alpha *= echo_cfg.evaluate(global_time);
        }
        text_color.0.set_alpha(final_alpha.clamp(0.0, 1.0));
    }
}

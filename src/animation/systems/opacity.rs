//! Applies opacity curves to non-SDF sprite-backed layers.
//! 把透明度曲线应用到非 SDF 的 sprite 图层。
//!
//! Standard sprite/text layers can use Bevy-side alpha directly, but they still need Alight
//! Motion's layer activity rules, fade effect, and echo alpha adjustments. This file evaluates
//! those curves and writes the final alpha back to the visible sprite state.
//! 普通 sprite 或文本图层可以直接使用 Bevy 侧 alpha，但它们仍然要遵守 Alight Motion 的图层激活规则、
//! Fade 效果和 echo alpha 修正。这个文件负责求值这些曲线，并把最终透明度写回可见的 sprite 状态。

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

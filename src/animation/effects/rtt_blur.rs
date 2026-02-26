//! RTT-based Gaussian blur animation system.

use bevy::prelude::*;

use crate::animation::components::{AmAnimated, AmPlayback};
use crate::animation::interpolation::interpolate_float;

/// System to animate RTT-based Gaussian blur effect.
/// This updates the GaussianBlurEffect component's radius based on animation keyframes.
pub fn animate_rtt_blur_system(
    playback: Res<AmPlayback>,
    mut query: Query<(&AmAnimated, &mut crate::gaussian_blur::GaussianBlurEffect)>,
) {
    // Skip animation only when force stopped
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (animated, mut blur_effect) in query.iter_mut() {
        // Use local time for visibility check (affected by speed)
        let local_time = animated.calc_local_time(global_time);

        // Check if layer is active at current local time
        if !animated.is_active(local_time) {
            continue;
        }

        // Use animation local time for interpolation
        let layer_time = animated.calc_layer_time(local_time);

        // Check if this layer has blur animation
        let has_blur =
            animated.blur_strength.value.is_some() || !animated.blur_strength.keyframes.is_empty();

        if has_blur {
            let blur_strength =
                interpolate_float(&animated.blur_strength, layer_time).unwrap_or(0.0);
            // AM strength 2.0 produces very strong blur
            // Use strength * 80 for closer match to AM's blur intensity
            let blur_radius_px = blur_strength * 80.0;

            if (blur_effect.radius - blur_radius_px).abs() > 0.1 {
                bevy::log::debug!(
                    "[BlurAnim] Updating blur radius: {:.1} -> {:.1} (strength={:.3})",
                    blur_effect.radius,
                    blur_radius_px,
                    blur_strength
                );
                blur_effect.radius = blur_radius_px;
            }
        }
    }
}

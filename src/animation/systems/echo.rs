use bevy::prelude::*;

use crate::animation::{AmAnimated, AmEchoRuntime, AmPlayback, EchoAlphaConfig};
use crate::scene::AmForceHidden;

pub fn update_echo_runtime_system(
    playback: Res<AmPlayback>,
    mut echo_query: Query<(
        Entity,
        &AmEchoRuntime,
        &mut AmAnimated,
        &mut Visibility,
        Option<&AmForceHidden>,
    )>,
    children_query: Query<&Children>,
    mut child_animated_query: Query<&mut AmAnimated, Without<AmEchoRuntime>>,
) {
    for (entity, echo_rt, mut animated, mut visibility, force_hidden) in echo_query.iter_mut() {
        if force_hidden.is_some() {
            *visibility = Visibility::Hidden;
            continue;
        }

        let global_time = playback.current_time_ms;
        let parent_local = (global_time - echo_rt.embed_time_offset) * echo_rt.embed_speed;
        let parent_duration = echo_rt.embed_end - echo_rt.embed_start;
        let frac_t = if parent_duration > 0.0 {
            ((parent_local - echo_rt.embed_start) / parent_duration).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let current_count =
            crate::animation::interpolation::interpolate_float(&echo_rt.count_kf, frac_t)
                .unwrap_or(1.0)
                .round() as u32;

        if echo_rt.echo_index >= current_count {
            *visibility = Visibility::Hidden;
            continue;
        } else {
            *visibility = Visibility::Inherited;
        }

        let current_seconds =
            crate::animation::interpolation::interpolate_float(&echo_rt.seconds_kf, frac_t)
                .unwrap_or(0.5);
        let r0 = if current_count > 0 {
            echo_rt.echo_index as f32 / current_count as f32
        } else {
            0.0
        };
        let time_shift_ms = (1.0 - r0) * current_seconds * 1000.0;

        animated.echo_time_shift_ms = time_shift_ms;

        let current_alpha =
            crate::animation::interpolation::interpolate_float(&echo_rt.alpha_kf, frac_t)
                .unwrap_or(1.0);
        let mix = current_alpha * (1.0 - r0) + r0;
        let echo_cfg = EchoAlphaConfig {
            alpha_keyframes: crate::schema::AmAnimatedFloat {
                value: Some(mix),
                keyframes: Vec::new(),
            },
            fraction: 0.0,
            parent_start: echo_rt.embed_start as i32,
            parent_end: echo_rt.embed_end as i32,
            parent_time_offset: echo_rt.embed_time_offset,
            parent_speed: echo_rt.embed_speed,
        };
        animated.echo_alpha_config = Some(echo_cfg.clone());

        propagate_echo_to_descendants(
            entity,
            time_shift_ms,
            &echo_cfg,
            &children_query,
            &mut child_animated_query,
        );
    }
}

fn propagate_echo_to_descendants(
    parent: Entity,
    time_shift_ms: f32,
    echo_cfg: &EchoAlphaConfig,
    children_query: &Query<&Children>,
    child_animated_query: &mut Query<&mut AmAnimated, Without<AmEchoRuntime>>,
) {
    let Ok(children) = children_query.get(parent) else {
        return;
    };
    for child in children.iter() {
        if let Ok(mut child_animated) = child_animated_query.get_mut(child) {
            child_animated.echo_time_shift_ms = time_shift_ms;
            child_animated.echo_alpha_config = Some(echo_cfg.clone());
        }
        propagate_echo_to_descendants(
            child,
            time_shift_ms,
            echo_cfg,
            children_query,
            child_animated_query,
        );
    }
}

//! Advances the global playback clock.
//! It is intentionally small: one system updates project time, handles looping,
//! and stops playback at the end when looping is disabled.
//!
//! 负责推进全局播放时钟。它刻意保持很小：只有一个系统更新项目时间、
//! 处理循环逻辑，并在禁用循环时于结尾处停止播放。

use bevy::prelude::*;

use crate::animation::AmPlayback;

pub fn advance_playback_system(time: Res<Time>, mut playback: ResMut<AmPlayback>) {
    if !playback.playing {
        return;
    }

    playback.current_time_ms += time.delta_secs() * 1000.0 * playback.speed;

    if playback.current_time_ms >= playback.total_time_ms {
        if playback.looping {
            playback.current_time_ms %= playback.total_time_ms;
        } else {
            playback.current_time_ms = playback.total_time_ms;
            playback.playing = false;
        }
    }
}

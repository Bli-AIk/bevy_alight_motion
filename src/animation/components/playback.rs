//! Defines the global playback resource for Alight Motion timelines.
//! It keeps the current clock, total duration, looping mode, and stop/play flags
//! in one place so all animation systems can advance from a shared notion of
//! project time.
//!
//! 定义了 Alight Motion 时间轴的全局播放资源。它把当前时钟、总时长、
//! 循环模式以及停止/播放标志集中在一起，让所有动画系统都能建立在同一份项目时间
//! 语义之上推进。

use bevy::prelude::*;

/// Resource to control animation playback.
#[derive(Resource, Debug, Clone)]
pub struct AmPlayback {
    pub current_time_ms: f32,
    pub total_time_ms: f32,
    pub playing: bool,
    pub speed: f32,
    pub looping: bool,
    pub force_stopped: bool,
}

impl Default for AmPlayback {
    fn default() -> Self {
        Self {
            current_time_ms: 0.0,
            total_time_ms: 2000.0,
            playing: true,
            speed: 1.0,
            looping: true,
            force_stopped: false,
        }
    }
}

impl AmPlayback {
    pub fn with_duration(total_time_ms: f32) -> Self {
        Self {
            total_time_ms,
            ..Default::default()
        }
    }

    pub fn reset(&mut self) {
        self.current_time_ms = 0.0;
    }

    pub fn toggle(&mut self) {
        self.playing = !self.playing;
    }

    pub fn toggle_force_stop(&mut self) {
        self.force_stopped = !self.force_stopped;
    }
}

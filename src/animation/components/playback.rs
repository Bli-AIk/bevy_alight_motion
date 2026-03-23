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

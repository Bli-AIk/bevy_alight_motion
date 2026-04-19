//! # plugin.rs
//!
//! Bevy plugin entry points for Alight Motion support.

mod build;
mod project_loading;
mod resources;
mod startup;
mod warmup;

use bevy::prelude::*;

pub use project_loading::load_am_project;
pub use resources::{AmProjectResolution, AmWhitePixel, DisabledEffects};

/// System sets for the Alight Motion plugin.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AlightMotionSystemSet {
    /// Entity spawning, lifecycle management, and RTT setup.
    Lifecycle,
    /// Keyframe animation and effect updates.
    Animation,
    /// Mask calculation (runs in PostUpdate after TransformPropagate).
    Mask,
}

/// Plugin providing Alight Motion support for Bevy.
pub struct AlightMotionPlugin;

impl Plugin for AlightMotionPlugin {
    fn build(&self, app: &mut App) {
        build::build_plugin(app);
    }
}

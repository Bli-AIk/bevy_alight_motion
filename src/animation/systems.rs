//! Core animation systems for transform, opacity, and playback control.

mod camera;
mod debug;
mod echo;
mod opacity;
mod playback;
mod shared;
mod size;
mod transform;
mod transform_perspective;

pub use camera::animate_am_camera_system;
pub use debug::debug_layer_global_z_system;
pub use echo::update_echo_runtime_system;
pub use opacity::{animate_opacity_system, animate_text_opacity_system};
pub use playback::advance_playback_system;
pub use size::animate_size_system;
pub use transform::animate_transform_system;

pub(crate) use shared::{
    compute_normalized_frame_delta, compute_perspective_zoom, resolve_unwrapped_rotation_deg,
};

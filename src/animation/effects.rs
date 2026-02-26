//! # effects
//!
//! # 效果模块
//!
//! Unified effect animation systems for wipe, stretch, blur, and palette effects.
//! Contains animate_unified_effect_system, update_unified_mask_system, animate_rtt_blur_system, etc.
//!
//! 统一效果动画系统，用于擦除、拉伸、模糊和调色板效果。
//! 包含 animate_unified_effect_system、update_unified_mask_system、animate_rtt_blur_system 等。

mod mask;
pub(crate) mod path_repeat;
mod repeat;
mod rtt_blur;
pub(crate) mod text_effects;
mod unified;

pub use mask::update_unified_mask_system;
pub use path_repeat::animate_path_repeat_system;
pub use rtt_blur::animate_rtt_blur_system;
pub use text_effects::{
    animate_text_progress_system, animate_text_spacing_system, fix_rtl_line_alignment_system,
};
pub use unified::animate_unified_effect_system;

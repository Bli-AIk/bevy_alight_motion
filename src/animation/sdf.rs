//! # sdf.rs
//!
//! # SDF 模块
//!
//! SDF (Signed Distance Field) shape animation systems and related functionality.
//! SDF（有符号距离场）形状动画系统及相关功能。

mod clipping;
mod mask;
mod opacity;
mod repeat;
mod scale;
mod stretch;

pub use clipping::apply_mask_clipping_system;
pub use mask::update_sdf_mask_system;
pub use opacity::animate_sdf_opacity_system;
pub use repeat::animate_sdf_repeat_system;
pub use scale::{
    animate_sdf_scale_system, compensate_sdf_ancestor_scale_for_children_system,
    compensate_sdf_parent_scale_system,
};
pub use stretch::animate_sdf_stretch_system;

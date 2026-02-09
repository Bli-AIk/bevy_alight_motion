//! # effects.rs
//!
//! # 效果定义入口
//!
//! Entry point for all effect definitions.
//! 所有效果定义的入口模块。

pub mod gaussian_blur;
pub mod grid;
pub mod linear_repeat;
pub mod palette_map;
pub mod pixelate;
pub mod repeat;
pub mod replace_color;
pub mod scale_assist;
pub mod stretch_segment;
pub mod swing;
pub mod threshold;
pub mod transform2;
pub mod wipe2;

use super::types::EffectDef;

/// 获取所有效果定义 / Get all effect definitions
pub fn all() -> &'static [&'static EffectDef] {
    &[
        &transform2::EFFECT,
        &wipe2::EFFECT,
        &stretch_segment::EFFECT,
        &gaussian_blur::EFFECT,
        &grid::EFFECT,
        &threshold::EFFECT,
        &palette_map::EFFECT,
        &replace_color::EFFECT,
        &scale_assist::EFFECT,
        &pixelate::EFFECT,
        &repeat::EFFECT,
        &linear_repeat::EFFECT,
        &swing::EFFECT,
    ]
}

/// 按 ID 查找效果 / Find effect by ID
pub fn find(id: &str) -> Option<&'static EffectDef> {
    all().iter().find(|e| e.id == id).copied()
}

/// 按短名称查找效果 / Find effect by short name
pub fn find_by_short_name(short_name: &str) -> Option<&'static EffectDef> {
    all().iter().find(|e| e.short_name == short_name).copied()
}

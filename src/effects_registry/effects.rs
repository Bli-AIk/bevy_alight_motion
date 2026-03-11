//! # effects.rs
//!
//! # 效果定义入口
//!
//! Entry point for all effect definitions.
//! 所有效果定义的入口模块。

pub mod echokf;
pub mod fade;
pub mod gaussian_blur;
pub mod grid;
pub mod jitter;
pub mod linear_repeat;
pub mod oscillate;
pub mod palette_map;
pub mod path_repeat;
pub mod pixelate;
pub mod radial_repeat;
pub mod repeat;
pub mod replace_color;
pub mod scale_assist;
pub mod solidcolor;
pub mod spin;
pub mod stretch2;
pub mod stretch_segment;
pub mod swing;
pub mod textprogress;
pub mod textspacing;
pub mod threshold;
pub mod transform2;
pub mod transform_legacy;
pub mod wavewarp2;
pub mod wipe2;

use super::types::EffectDef;

/// 获取所有效果定义 / Get all effect definitions
pub fn all() -> &'static [&'static EffectDef] {
    &[
        &transform2::EFFECT,
        &transform_legacy::EFFECT,
        &wipe2::EFFECT,
        &stretch_segment::EFFECT,
        &stretch2::EFFECT,
        &gaussian_blur::EFFECT,
        &grid::EFFECT,
        &threshold::EFFECT,
        &palette_map::EFFECT,
        &replace_color::EFFECT,
        &scale_assist::EFFECT,
        &pixelate::EFFECT,
        &oscillate::EFFECT,
        &jitter::EFFECT,
        &repeat::EFFECT,
        &linear_repeat::EFFECT,
        &radial_repeat::EFFECT,
        &path_repeat::EFFECT,
        &echokf::EFFECT,
        &fade::EFFECT,
        &solidcolor::EFFECT,
        &swing::EFFECT,
        &spin::EFFECT,
        &textprogress::EFFECT,
        &textspacing::EFFECT,
        &wavewarp2::EFFECT,
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

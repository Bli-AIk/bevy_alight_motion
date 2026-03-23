//! Packs and repacks colors for the SDF material uniform layout.
//! 为 SDF 材质 uniform 布局打包和重打包颜色。
//!
//! The SDF shader stores colors as packed integer bits embedded in floats, which keeps the uniform
//! format aligned with the existing shader contract. This file centralizes that packing logic so
//! runtime systems do not need to duplicate bit-manipulation code when changing fill or border
//! alpha values.
//! SDF shader 会把颜色编码成嵌入 float 的整数位模式，以保持与现有 shader 协议一致。
//! 这个文件把这套打包逻辑集中起来，避免运行时系统在修改填充色或边框 alpha 时到处重复写位运算。

use bevy::prelude::*;

/// Pack RGBA color into a u32 stored as f32 bits.
/// Format: 0xRRGGBBAA
pub fn pack_color(color: Color) -> f32 {
    let rgba = color.to_srgba();
    let r = (rgba.red * 255.0) as u32;
    let g = (rgba.green * 255.0) as u32;
    let b = (rgba.blue * 255.0) as u32;
    let a = (rgba.alpha * 255.0) as u32;
    let packed = (r << 24) | (g << 16) | (b << 8) | a;
    f32::from_bits(packed)
}

/// Repack a color with a new alpha value.
pub fn repack_with_alpha(packed: f32, new_alpha: f32) -> f32 {
    let bits = packed.to_bits();
    let rgb = bits & 0xFFFF_FF00;
    let a = ((new_alpha.clamp(0.0, 1.0) * 255.0) as u32) & 0xFF;
    f32::from_bits(rgb | a)
}

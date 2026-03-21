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

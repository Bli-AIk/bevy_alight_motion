//! Defines scene-level effect metadata shared across collection and spawning.
//! It includes enums and small structs that describe blend modes, extracted effect
//! parameters, and other effect-related identities that are cheaper to keep near
//! the scene layer model than inside the runtime animation component itself.
//!
//! 定义了场景层面共用的特效元数据。它包含混合模式枚举以及若干小型结构，
//! 用来描述提取后的效果参数和相关身份信息；这些内容更适合放在场景图层模型附近，而不是
//! 直接塞进运行时动画组件本体里。

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum AmBlendingMode {
    #[default]
    Normal = 0,
    Multiply = 1,
    Darken = 2,
    DarkerColor = 3,
    ColorBurn = 4,
    LinearBurn = 5,
    Screen = 6,
    Lighten = 7,
    LighterColor = 8,
    ColorDodge = 9,
    LinearDodge = 10,
    Overlay = 11,
    SoftLight = 12,
    HardLight = 13,
    SoftOverlay = 14,
    VividLight = 15,
    PinLight = 16,
    Difference = 17,
    Exclusion = 18,
    Subtract = 19,
    Divide = 20,
    Hue = 21,
    Saturation = 22,
    Color = 23,
    Luminance = 24,
    Mask = 100,
    Exclude = 101,
}

impl AmBlendingMode {
    pub fn parse_am(s: &str) -> Self {
        match s {
            "" => Self::Normal,
            "normal" => Self::Normal,
            "multiply" => Self::Multiply,
            "darken" => Self::Darken,
            "darker-color" => Self::DarkerColor,
            "color-burn" => Self::ColorBurn,
            "linear-burn" => Self::LinearBurn,
            "screen" => Self::Screen,
            "lighten" => Self::Lighten,
            "lighter-color" => Self::LighterColor,
            "color-dodge" => Self::ColorDodge,
            "linear-dodge" => Self::LinearDodge,
            "overlay" => Self::Overlay,
            "soft-light" => Self::SoftLight,
            "hard-light" => Self::HardLight,
            "soft-overlay" => Self::SoftOverlay,
            "vivid-light" => Self::VividLight,
            "pin-light" => Self::PinLight,
            "diff" => Self::Difference,
            "exclusion" => Self::Exclusion,
            "subtract" => Self::Subtract,
            "divide" => Self::Divide,
            "hue" => Self::Hue,
            "saturation" => Self::Saturation,
            "color" => Self::Color,
            "luminance" => Self::Luminance,
            "mask" => Self::Mask,
            "exclude" => Self::Exclude,
            _ => {
                bevy::log::warn!("Unknown blend mode: '{}', defaulting to Normal", s);
                Self::Normal
            }
        }
    }

    pub fn is_blend(self) -> bool {
        !matches!(self, Self::Normal | Self::Mask | Self::Exclude)
    }

    pub fn as_f32(self) -> f32 {
        (self as u32) as f32
    }
}

#[derive(Debug, Clone, Default)]
pub struct AmMaskEntry {
    pub center: Vec2,
    pub half_size: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
    pub is_circle: bool,
    pub start_time: i32,
    pub end_time: i32,
    pub mask_layer_id: u64,
    pub is_exclude: bool,
    pub mask_parent_layer_id: u64,
    pub is_embed_mask: bool,
    pub embed_scene_size: Option<(f32, f32)>,
}

#[derive(Debug, Clone, Default, Component)]
pub struct AmMaskInfo {
    pub masks: Vec<AmMaskEntry>,
}

impl AmMaskInfo {
    pub fn get_active_mask(&self, time_ms: u64) -> Option<&AmMaskEntry> {
        let t = time_ms as i64;
        self.masks
            .iter()
            .find(|mask| t >= mask.start_time as i64 && t < mask.end_time as i64)
    }

    pub fn get_active_masks(&self, time_ms: u64) -> Vec<&AmMaskEntry> {
        let t = time_ms as i64;
        let mut seen = std::collections::HashSet::new();
        self.masks
            .iter()
            .filter(|mask| t >= mask.start_time as i64 && t < mask.end_time as i64)
            .filter(|mask| seen.insert(mask.mask_layer_id))
            .collect()
    }
}

#[derive(Component, Debug, Clone)]
pub struct AmPaletteMapParams {
    pub count: u8,
    pub colors: [Vec4; 8],
    pub initial_alpha: f32,
}

impl AmPaletteMapParams {
    pub fn from_params(params: &super::super::effects::PaletteMapParams) -> Self {
        let initial_alpha = if !params.alpha.keyframes.is_empty() {
            params.alpha.keyframes[0].value.parse().unwrap_or(0.0)
        } else {
            params.alpha.value.unwrap_or(1.0)
        };

        let c = &params.custom_colors;
        let v = |r: f32, g: f32, b: f32| Vec4::new(r, g, b, 1.0);

        let (mut colors, count) = match (params.palette_id, params.shades) {
            (0, _) => {
                let mut arr = [Vec4::ZERO; 8];
                arr[0] = v(0.0, 0.0, 0.0);
                arr[1] = v(0.333, 1.0, 0.333);
                arr[2] = v(1.0, 0.333, 0.333);
                arr[3] = v(1.0, 1.0, 0.333);
                (arr, 4u8)
            }
            (1, _) => {
                let mut arr = [Vec4::ZERO; 8];
                arr[0] = v(0.0, 0.0, 0.0);
                arr[1] = v(0.333, 1.0, 1.0);
                arr[2] = v(1.0, 0.333, 1.0);
                arr[3] = v(1.0, 1.0, 1.0);
                (arr, 4)
            }
            (10, _) => {
                let mut arr = [Vec4::ZERO; 8];
                arr[0] = v(0.333, 0.333, 1.0);
                arr[1] = v(0.333, 1.0, 1.0);
                arr[2] = v(1.0, 0.333, 1.0);
                arr[3] = v(1.0, 1.0, 1.0);
                (arr, 4)
            }
            (4, _) => {
                let mut arr = [Vec4::ZERO; 8];
                arr[0] = v(0.0, 0.0, 0.0);
                arr[1] = v(0.333, 0.333, 1.0);
                arr[2] = v(0.333, 1.0, 0.333);
                arr[3] = v(0.333, 1.0, 1.0);
                arr[4] = v(1.0, 0.333, 0.333);
                arr[5] = v(1.0, 0.333, 1.0);
                arr[6] = v(1.0, 1.0, 0.333);
                arr[7] = v(1.0, 1.0, 1.0);
                (arr, 8)
            }
            (5, _) => {
                let mut arr = [Vec4::ZERO; 8];
                arr[0] = v(0.0, 0.0, 0.0);
                arr[1] = v(0.333, 0.333, 0.333);
                arr[2] = v(0.667, 0.667, 0.667);
                arr[3] = v(1.0, 1.0, 1.0);
                (arr, 4)
            }
            (6, false) => {
                let mut arr = [Vec4::ZERO; 8];
                arr[0] = c[0];
                arr[1] = c[1];
                arr[2] = c[2];
                (arr, 3)
            }
            (6, true) => {
                let mut arr = [Vec4::ZERO; 8];
                arr[0] = c[0];
                arr[1] = c[1];
                arr[2] = c[2];
                arr[3] = Vec4::ZERO;
                arr[4] = c[0] * 0.667;
                arr[5] = c[1] * 0.667;
                arr[6] = c[2] * 0.667;
                arr[4].w = c[0].w;
                arr[5].w = c[1].w;
                arr[6].w = c[2].w;
                (arr, 7)
            }
            (7, false) => {
                let mut arr = [Vec4::ZERO; 8];
                arr[0] = c[0];
                arr[1] = c[1];
                arr[2] = c[2];
                arr[3] = c[3];
                (arr, 4)
            }
            (7, true) => {
                let mut arr = [Vec4::ZERO; 8];
                arr[0] = c[0];
                arr[1] = c[1];
                arr[2] = c[2];
                arr[3] = c[3];
                arr[4] = c[0] * 0.667;
                arr[5] = c[1] * 0.667;
                arr[6] = c[2] * 0.667;
                arr[7] = c[3] * 0.667;
                arr[4].w = c[0].w;
                arr[5].w = c[1].w;
                arr[6].w = c[2].w;
                arr[7].w = c[3].w;
                (arr, 8)
            }
            (8, false) => {
                let mut arr = [Vec4::ZERO; 8];
                arr[0] = c[0];
                arr[1] = c[1];
                arr[2] = c[2];
                arr[3] = c[3];
                arr[4] = c[4];
                arr[5] = c[5];
                (arr, 6)
            }
            (9, false) => {
                let mut arr = [Vec4::ZERO; 8];
                arr[..8].copy_from_slice(&c[..8]);
                (arr, 8)
            }
            (2, _) | (3, _) => {
                let mut arr = [Vec4::ZERO; 8];
                if params.palette_id == 2 {
                    arr[0] = v(0.0, 0.0, 0.0);
                    arr[1] = v(0.0, 0.0, 0.667);
                    arr[2] = v(0.0, 0.667, 0.0);
                    arr[3] = v(0.0, 0.667, 0.667);
                    arr[4] = v(0.667, 0.0, 0.0);
                    arr[5] = v(0.667, 0.0, 0.667);
                    arr[6] = v(0.667, 0.333, 0.0);
                    arr[7] = v(0.667, 0.667, 0.667);
                } else {
                    arr[0] = v(0.0, 0.0, 0.0);
                    arr[1] = v(0.0, 0.0, 0.667);
                    arr[2] = v(0.0, 0.667, 0.0);
                    arr[3] = v(0.333, 0.667, 1.0);
                    arr[4] = v(0.667, 0.0, 0.0);
                    arr[5] = v(0.667, 0.0, 0.667);
                    arr[6] = v(0.333, 0.667, 0.0);
                    arr[7] = v(0.667, 0.667, 0.667);
                }
                (arr, 8)
            }
            _ => {
                let mut arr = [Vec4::ZERO; 8];
                arr[..8].copy_from_slice(&c[..8]);
                (arr, 8)
            }
        };

        for color in colors.iter_mut().take(count as usize) {
            if color.w == 0.0 {
                color.w = 1.0;
            }
        }

        Self {
            count,
            colors,
            initial_alpha,
        }
    }
}

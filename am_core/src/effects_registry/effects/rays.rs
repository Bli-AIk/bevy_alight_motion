//! # rays.rs
//!
//! # 射线效果定义
//!
//! Rays effect definition - volumetric light rays (god rays) effect.
//! 射线效果定义 - 体积光线效果。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.rays",
    short_name: "rays",
    zh: "射线 (Rays)",
    en: "Rays",
    desc_zh: "从亮度超过阈值的区域发出体积光线，模拟光线散射效果。",
    desc_en: "Volumetric light rays emanating from bright areas above threshold, simulating light scattering.",
    support: Partial,
    xml: r##"<effect id="com.alightcreative.effects.rays">
    <property name="centerPoint" type="vec2" value="0.000000,0.000000" />
    <property name="strength" type="float" value="0.150000" />
    <property name="intensity" type="float" value="1.000000" />
    <property name="threshold" type="float" value="0.600000" />
    <property name="thresholdColor" type="color" value="#ff000000" />
    <property name="fillColor" type="color" value="#ff2d1ef6" />
    <property name="blend" type="float" value="0.000000" />
    <property name="quality" type="float" value="150.000000" />
</effect>"##,
    tests: [
        "effects/rays/basic",
        "effects/rays/center",
        "effects/rays/intensity",
        "effects/rays/length",
        "effects/rays/threshold",
    ],
    fields: [
        define_field! {
            name: "centerPoint",
            zh: "中心点",
            en: "Center Point",
            type: FieldType::Vec2,
            support: Full,
            default: "0.0, 0.0",
            desc_zh: "射线生成中心点 (AM坐标 ±500)",
            desc_en: "Ray emission center point (AM coords ±500)",
        },
        define_field! {
            name: "strength",
            zh: "长度",
            en: "Length/Strength",
            type: FieldType::Float,
            support: Full,
            default: "0.15",
            desc_zh: "射线长度/扩散范围 (0.0-4.0)",
            desc_en: "Ray length/spread (0.0-4.0)",
        },
        define_field! {
            name: "intensity",
            zh: "强度",
            en: "Intensity",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "射线亮度倍数 (0.0-5.0)",
            desc_en: "Ray brightness multiplier (0.0-5.0)",
        },
        define_field! {
            name: "threshold",
            zh: "阈值",
            en: "Threshold",
            type: FieldType::Float,
            support: Full,
            default: "0.6",
            desc_zh: "亮度阈值，只有超过此值的像素产生射线 (0.0-1.0)",
            desc_en: "Brightness threshold for ray source (0.0-1.0)",
        },
        define_field! {
            name: "thresholdColor",
            zh: "阈值颜色",
            en: "Threshold Color",
            type: FieldType::Color,
            support: Full,
            default: "#FF000000",
            desc_zh: "计算亮度前减去的颜色",
            desc_en: "Color subtracted before luminance calculation",
        },
        define_field! {
            name: "fillColor",
            zh: "填充颜色",
            en: "Fill Color",
            type: FieldType::Color,
            support: Full,
            default: "#FF2D1EF6",
            desc_zh: "射线颜色",
            desc_en: "Color of the rays",
        },
        define_field! {
            name: "blend",
            zh: "混合",
            en: "Blend",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "原始像素与填充颜色的混合比例 (0.0-1.0)",
            desc_en: "Blend ratio between original pixel and fill color (0.0-1.0)",
        },
        define_field! {
            name: "quality",
            zh: "质量",
            en: "Quality",
            type: FieldType::Float,
            support: Full,
            default: "150.0",
            desc_zh: "采样数量 (10-800)",
            desc_en: "Number of samples (10-800)",
        },
    ],
}

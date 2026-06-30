//! Registers metadata for the built-in Jitter effect.
//! 注册内置 Jitter 效果的元数据。
//!
//! Jitter uses noise-driven displacement, so its registry entry needs to document direction,
//! frequency, magnitude, and seed-related controls clearly for users and tooling.
//! Jitter 依赖噪声位移，因此它的注册表条目需要把方向、频率、幅度和 seed 相关控制项清楚写明，
//! 方便用户和工具链正确理解。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.jitter",
    short_name: "jitter",
    zh: "抖动 (Jitter)",
    en: "Jitter",
    desc_zh: "使用 Simplex 噪声对图层位置进行随机抖动。",
    desc_en: "Applies random position displacement using Simplex noise.",
    support: Full,
    xml: r##"<effect id="com.alightcreative.effects.jitter">
    <property name="angle" type="float" value="45.0" />
    <property name="freq" type="float" value="30.0" />
    <property name="mag" type="float" value="25.0" />
    <property name="seed" type="float" value="0.0" />
    <property name="slack" type="float" value="0.0" />
    <property name="zjitter" type="float" value="0.0" />
</effect>"##,
    tests: ["effects/jetter/basic.amproj"],
    fields: [
        define_field! {
            name: "angle",
            zh: "角度",
            en: "Angle",
            type: FieldType::Float,
            support: Full,
            default: "45.0",
            desc_zh: "运动方向角度（度）",
            desc_en: "Movement direction angle (degrees)",
        },
        define_field! {
            name: "freq",
            zh: "频率",
            en: "Frequency",
            type: FieldType::Float,
            support: Full,
            default: "30.0",
            desc_zh: "噪声频率（步/秒）",
            desc_en: "Noise frequency (steps per second)",
        },
        define_field! {
            name: "mag",
            zh: "幅度",
            en: "Magnitude",
            type: FieldType::Float,
            support: Full,
            default: "25.0",
            desc_zh: "位移幅度（像素）",
            desc_en: "Displacement magnitude (pixels)",
        },
        define_field! {
            name: "seed",
            zh: "种子",
            en: "Seed",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "噪声种子值",
            desc_en: "Noise seed value",
        },
        define_field! {
            name: "slack",
            zh: "松弛",
            en: "Slack",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "垂直方向松弛量（0.0-1.0）",
            desc_en: "Perpendicular slack amount (0.0-1.0)",
        },
        define_field! {
            name: "zjitter",
            zh: "Z轴抖动",
            en: "Z Jitter",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "Z轴方向抖动幅度",
            desc_en: "Z-axis jitter magnitude",
        },
    ],
}

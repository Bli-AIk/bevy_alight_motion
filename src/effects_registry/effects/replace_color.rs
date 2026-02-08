//! # replace_color.rs
//!
//! # ReplaceColor 效果定义
//!
//! ReplaceColor effect definition - color replacement.
//! ReplaceColor 效果定义 - 颜色替换效果。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.replacecolor",
    short_name: "replacecolor",
    zh: "颜色替换 (Replace Color)",
    en: "Replace Color",
    desc_zh: "在给定的容差范围内，将指定的源颜色替换为目标颜色。支持 sRGB 到线性颜色空间转换和动画关键帧。",
    desc_en: "Replaces a source color with a target color within a given tolerance. Supports sRGB to linear color space conversion and animation keyframes.",
    support: Full,
    xml: r##"<effect id="com.alightcreative.replacecolor">
    <property name="oldcolor" type="color" value="#ffff0000" />
    <property name="newcolor" type="color" value="#ff00ff00" />
    <property name="threshold" type="float" value="0.1" />
    <property name="feather" type="float" value="0.0" />
    <property name="alpha" type="float" value="1.0" />
    <property name="lockluminance" type="bool" value="false" />
</effect>"##,
    tests: ["fx_8_replace_color.amproj"],
    fields: [
        define_field! {
            name: "oldcolor",
            zh: "旧颜色",
            en: "Old Color",
            type: FieldType::Color,
            support: Full,
            desc_zh: "要替换的源颜色",
            desc_en: "The source color to replace",
        },
        define_field! {
            name: "newcolor",
            zh: "新颜色",
            en: "New Color",
            type: FieldType::Color,
            support: Full,
            desc_zh: "替换后的目标颜色",
            desc_en: "The target color to replace with",
        },
        define_field! {
            name: "threshold",
            zh: "阈值",
            en: "Threshold",
            type: FieldType::Float,
            support: Full,
            default: "0.1",
            desc_zh: "颜色匹配容差",
            desc_en: "Color matching tolerance",
        },
        define_field! {
            name: "feather",
            zh: "羽化",
            en: "Feather",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "边缘过渡柔和度",
            desc_en: "Edge transition softness",
        },
        define_field! {
            name: "alpha",
            zh: "透明度",
            en: "Alpha",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "效果强度",
            desc_en: "Effect strength",
        },
        define_field! {
            name: "lockluminance",
            zh: "锁定亮度",
            en: "Lock Luminance",
            type: FieldType::Bool,
            support: Full,
            default: "false",
            desc_zh: "保持原始像素的亮度",
            desc_en: "Preserve original pixel luminance",
        },
    ],
}

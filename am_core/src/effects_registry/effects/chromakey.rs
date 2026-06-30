//! # chromakey.rs
//!
//! # ChromaKey 效果定义
//!
//! ChromaKey effect definition - chroma key (green/blue screen removal).
//! 色度键效果定义 — 色度键（绿/蓝幕抠像）。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.chromakey",
    short_name: "chromakey",
    zh: "色度键 (Chroma Key)",
    en: "Chroma Key",
    desc_zh: "基于色度的抠像效果，移除指定颜色的像素（如绿幕/蓝幕）。",
    desc_en: "Chroma-based keying effect that removes pixels matching a specified color (green/blue screen).",
    support: Full,
    xml: r##"<effect id="com.alightcreative.effects.chromakey">
    <property name="keyColor" type="color" value="#ff00ff00" />
    <property name="threshold" type="float" value="0.100000" />
    <property name="feather" type="float" value="0.050000" />
    <property name="defringe" type="bool" value="false" />
    <property name="invert" type="bool" value="false" />
</effect>"##,
    tests: ["effects/chroma-key/basic/test.amproj"],
    fields: [
        define_field! {
            name: "keyColor",
            zh: "键色",
            en: "Key Color",
            type: FieldType::Color,
            support: Full,
            desc_zh: "要移除的目标颜色",
            desc_en: "The target color to remove",
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
            default: "0.05",
            desc_zh: "边缘过渡柔和度",
            desc_en: "Edge transition softness",
        },
        define_field! {
            name: "defringe",
            zh: "去边",
            en: "Defringe",
            type: FieldType::Bool,
            support: Full,
            default: "false",
            desc_zh: "移除边缘色溢",
            desc_en: "Remove edge color spill",
        },
        define_field! {
            name: "invert",
            zh: "反转",
            en: "Invert",
            type: FieldType::Bool,
            support: Full,
            default: "false",
            desc_zh: "反转抠像结果（保留键色区域）",
            desc_en: "Invert keying result (keep key color areas)",
        },
    ],
}

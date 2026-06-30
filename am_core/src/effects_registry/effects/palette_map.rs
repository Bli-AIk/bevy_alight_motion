//! # palette_map.rs
//!
//! # PaletteMap 效果定义
//!
//! PaletteMap effect definition - color palette mapping.
//! PaletteMap 效果定义 - 调色板映射效果。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.palettemap",
    short_name: "palette-map",
    zh: "调色板映射 (Palette Map)",
    en: "Palette Map",
    desc_zh: "将图像颜色映射到指定的调色板颜色。支持最多 8 个调色板颜色。",
    desc_en: "Maps image colors to specified palette colors. Supports up to 8 palette colors.",
    support: Partial,
    xml: r##"<effect id="com.alightcreative.effects.palettemap">
    <property name="color1" type="color" value="#ff000000" />
    <property name="color2" type="color" value="#ffffffff" />
    <property name="count" type="float" value="2.0" />
    <property name="shades" type="bool" value="false" />
    <property name="alpha" type="float" value="1.0" />
</effect>"##,
    tests: ["effects/palette/basic.amproj"],
    fields: [
        define_field! {
            name: "color1",
            zh: "颜色 1",
            en: "Color 1",
            type: FieldType::Color,
            support: Full,
            default: "#ff000000",
            desc_zh: "调色板颜色 1",
            desc_en: "Palette color 1",
        },
        define_field! {
            name: "color2",
            zh: "颜色 2",
            en: "Color 2",
            type: FieldType::Color,
            support: Full,
            default: "#ffffffff",
            desc_zh: "调色板颜色 2",
            desc_en: "Palette color 2",
        },
        define_field! {
            name: "color3",
            zh: "颜色 3",
            en: "Color 3",
            type: FieldType::Color,
            support: Full,
            desc_zh: "调色板颜色 3（可选）",
            desc_en: "Palette color 3 (optional)",
        },
        define_field! {
            name: "color4",
            zh: "颜色 4",
            en: "Color 4",
            type: FieldType::Color,
            support: Full,
            desc_zh: "调色板颜色 4（可选）",
            desc_en: "Palette color 4 (optional)",
        },
        define_field! {
            name: "color5",
            zh: "颜色 5",
            en: "Color 5",
            type: FieldType::Color,
            support: Full,
            desc_zh: "调色板颜色 5（可选）",
            desc_en: "Palette color 5 (optional)",
        },
        define_field! {
            name: "color6",
            zh: "颜色 6",
            en: "Color 6",
            type: FieldType::Color,
            support: Full,
            desc_zh: "调色板颜色 6（可选）",
            desc_en: "Palette color 6 (optional)",
        },
        define_field! {
            name: "color7",
            zh: "颜色 7",
            en: "Color 7",
            type: FieldType::Color,
            support: Full,
            desc_zh: "调色板颜色 7（可选）",
            desc_en: "Palette color 7 (optional)",
        },
        define_field! {
            name: "color8",
            zh: "颜色 8",
            en: "Color 8",
            type: FieldType::Color,
            support: Full,
            desc_zh: "调色板颜色 8（可选）",
            desc_en: "Palette color 8 (optional)",
        },
        define_field! {
            name: "count",
            zh: "颜色数量",
            en: "Color Count",
            type: FieldType::Float,
            support: Full,
            default: "2.0",
            desc_zh: "使用的颜色数量",
            desc_en: "Number of colors to use",
        },
        define_field! {
            name: "shades",
            zh: "阴影模式",
            en: "Shades Mode",
            type: FieldType::Bool,
            support: Partial,
            default: "false",
            desc_zh: "是否启用阴影渐变（基础支持，颜色过渡算法与 AM 存在细微差异）",
            desc_en: "Enable shade gradients (basic support, color transition differs slightly from AM)",
        },
        define_field! {
            name: "alpha",
            zh: "混合强度",
            en: "Alpha",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "效果混合强度",
            desc_en: "Effect blend strength",
        },
    ],
}

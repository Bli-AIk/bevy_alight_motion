//! Registers metadata for the built-in Solid Color effect.
//! 注册内置 Solid Color 效果的元数据。
//!
//! Solid Color is a lightweight overlay effect, but it still participates in support reporting and
//! generated docs. This file records its blend-related schema for the registry.
//! Solid Color 虽然是一个相对轻量的叠色效果，但它仍然参与支持报告和自动生成文档。
//! 这个文件把它和混合相关的字段结构记录到注册表里。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.solidcolor",
    short_name: "solidcolor",
    zh: "纯色 (Solid Color)",
    en: "Solid Color",
    desc_zh: "在内容上叠加一层纯色，支持混合模式和透明度控制。",
    desc_en: "Overlays a solid color on the content with blend mode and alpha control.",
    support: Partial,
    xml: r##"<effect id="com.alightcreative.solidcolor">
    <property name="color" type="color" value="#2D1EF6FF" />
    <property name="alpha" type="float" value="1.0" />
    <property name="blendMode" type="int" value="0" />
</effect>"##,
    tests: ["effects/solid-color/basic.amproj"],
    fields: [
        define_field! {
            name: "color",
            zh: "颜色",
            en: "Color",
            type: FieldType::Color,
            support: Full,
            default: "#2D1EF6FF",
            desc_zh: "叠加颜色（RGBA）",
            desc_en: "Overlay color (RGBA)",
        },
        define_field! {
            name: "alpha",
            zh: "透明度",
            en: "Alpha",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "效果强度（0.0-1.0）",
            desc_en: "Effect strength (0.0-1.0)",
        },
        define_field! {
            name: "blendMode",
            zh: "混合模式",
            en: "Blend Mode",
            type: FieldType::Int,
            support: Partial,
            default: "0",
            desc_zh: "混合模式（0=正常, 1=正片叠底, 2=滤色）",
            desc_en: "Blend mode (0=normal, 1=multiply, 2=screen)",
        },
    ],
}

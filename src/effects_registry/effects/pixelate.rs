//! # pixelate.rs
//!
//! # Pixelate 效果定义
//!
//! Pixelate effect definition - makes the image look pixelated.
//! Pixelate 效果定义 - 使图像看起来像像素化。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.pixelate2",
    short_name: "pixelate",
    zh: "像素化 (Pixelate)",
    en: "Pixelate",
    desc_zh: "降低图像分辨率，产生像素化效果。",
    desc_en: "Reduces image resolution to create a pixelated effect.",
    support: Full,
    xml: r#"<effect id="com.alightcreative.effects.pixelate2">
    <property name="size" type="float" value="10.0" />
    <property name="stretch" type="vec2" value="1.0,1.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="vignette" type="float" value="0.0" />
    <property name="screenSpace" type="boolean" value="false" />
    <property name="threshold" type="float" value="0.5" />
    <property name="saturation" type="float" value="1.0" />
</effect>"#,
    tests: [],
    fields: [
        define_field! {
            name: "size",
            zh: "大小",
            en: "Size",
            type: FieldType::Float,
            support: Full,
            default: "10.0",
            desc_zh: "像素大小 (1-100)",
            desc_en: "Pixel size (1-100)",
        },
        define_field! {
            name: "stretch",
            zh: "拉伸",
            en: "Stretch",
            type: FieldType::Vec2,
            support: Full,
            default: "1.0,1.0",
            desc_zh: "像素拉伸比例",
            desc_en: "Pixel stretch ratio",
        },
        define_field! {
            name: "angle",
            zh: "角度",
            en: "Angle",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "像素网格旋转角度",
            desc_en: "Pixel grid rotation angle",
        },
        define_field! {
            name: "vignette",
            zh: "晕影",
            en: "Vignette",
            type: FieldType::Float,
            support: Partial,
            default: "0.0",
            desc_zh: "晕影强度 (尚未完全支持)",
            desc_en: "Vignette strength (partial support)",
        },
        define_field! {
            name: "screenSpace",
            zh: "屏幕空间",
            en: "Screen Space",
            type: FieldType::Bool,
            support: Partial,
            default: "false",
            desc_zh: "是否使用屏幕空间坐标 (尚未完全支持)",
            desc_en: "Whether to use screen space coordinates (partial support)",
        },
    ],
}

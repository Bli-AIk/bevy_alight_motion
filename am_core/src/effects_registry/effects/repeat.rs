//! # repeat.rs
//!
//! # Repeat 效果定义
//!
//! Repeat effect definition - creates multiple copies with cumulative transforms.
//! Repeat 效果定义 - 创建多个带累积变换的副本。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.repeat",
    short_name: "repeat",
    zh: "重复 (Repeat)",
    en: "Repeat",
    desc_zh: "创建图层的多个副本，每个副本应用累积的偏移、旋转、缩放和透明度变换。",
    desc_en: "Creates multiple copies of the layer with cumulative offset, rotation, scale, and alpha transforms.",
    support: Partial,
    xml: r#"<effect id="com.alightcreative.effects.repeat">
    <property name="count" type="float" value="3.0" />
    <property name="time" type="float" value="0.0" />
    <property name="offset" type="vec2" value="50.0,50.0" />
    <property name="angle" type="float" value="15.0" />
    <property name="scale" type="float" value="0.9" />
    <property name="alpha" type="float" value="0.8" />
</effect>"#,
    tests: [
        "effects/repeat/basic.amproj",
    ],
    fields: [
        define_field! {
            name: "count",
            zh: "数量",
            en: "Count",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "创建的副本数量",
            desc_en: "Number of copies to create",
        },
        define_field! {
            name: "time",
            zh: "时间偏移",
            en: "Time Offset",
            type: FieldType::Float,
            support: Unsupported,
            default: "0.0",
            desc_zh: "副本之间的时间偏移（尚未实现）",
            desc_en: "Time offset between copies (not yet implemented)",
        },
        define_field! {
            name: "offset",
            zh: "偏移",
            en: "Offset",
            type: FieldType::Vec2,
            support: Full,
            default: "0.0,0.0",
            desc_zh: "每个副本的 X,Y 偏移（像素）",
            desc_en: "X,Y offset per copy (pixels)",
        },
        define_field! {
            name: "angle",
            zh: "角度",
            en: "Angle",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "每个副本的旋转角度（度）",
            desc_en: "Rotation angle per copy (degrees)",
        },
        define_field! {
            name: "scale",
            zh: "缩放",
            en: "Scale",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "每个副本的缩放乘数",
            desc_en: "Scale multiplier per copy",
        },
        define_field! {
            name: "alpha",
            zh: "透明度",
            en: "Alpha",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "每个副本的透明度乘数",
            desc_en: "Alpha multiplier per copy",
        },
    ],
}

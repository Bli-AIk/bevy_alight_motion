//! # wipe2.rs
//!
//! # Wipe2 效果定义
//!
//! Wipe2 effect definition - directional wipe transitions.
//! Wipe2 效果定义 - 方向擦除过渡效果。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.wipe2",
    short_name: "wipe2",
    zh: "擦拭 (Wipe2)",
    en: "Wipe2",
    desc_zh: "从图层的相对两侧遮盖矩形片段。使用关键帧动画创建擦拭过渡。",
    desc_en: "Covers rectangular segments from opposite sides of the layer. Use keyframe animation to create wipe transitions.",
    support: Full,
    xml: r#"<effect id="com.alightcreative.effects.wipe2">
    <property name="start" type="float" value="0.0" />
    <property name="end" type="float" value="1.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="feather" type="float" value="0.0" />
</effect>"#,
    tests: [],
    fields: [
        define_field! {
            name: "start",
            zh: "起始",
            en: "Start",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "可见范围起点 (0.0-1.0)",
            desc_en: "Visible range start point (0.0-1.0)",
        },
        define_field! {
            name: "end",
            zh: "结束",
            en: "End",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "可见范围终点 (0.0-1.0)",
            desc_en: "Visible range end point (0.0-1.0)",
        },
        define_field! {
            name: "angle",
            zh: "角度",
            en: "Angle",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "擦除方向角度",
            desc_en: "Wipe direction angle",
        },
        define_field! {
            name: "feather",
            zh: "羽化",
            en: "Feather",
            type: FieldType::Float,
            support: Partial,
            default: "0.0",
            desc_zh: "边缘柔和度（基本支持，尚未校对）",
            desc_en: "Edge softness (basic support, not yet calibrated)",
        },
    ],
}

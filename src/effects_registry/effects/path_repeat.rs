//! Registers metadata for the built-in Path Repeat effect.
//! 注册内置 Path Repeat 效果的元数据。
//!
//! Path Repeat creates copies along a path and exposes a relatively rich parameter surface. The
//! registry entry in this file is the source used by docs, tests, and support reporting to explain
//! that parameter set.
//! Path Repeat 会沿路径分布多个副本，参数面也相对丰富。这个文件里的注册表条目就是文档、测试和支持报告
//! 解释这套参数的真源。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.repeat.path",
    short_name: "path-repeat",
    zh: "路径重复 (Path Repeat)",
    en: "Path Repeat",
    desc_zh: "沿路径分布图层的多个副本，支持切线对齐、缩放、透明度等参数。",
    desc_en: "Distributes copies of the layer along a path with tangent alignment, scale, alpha, and easing controls.",
    support: Partial,
    xml: r##"<effect id="com.alightcreative.effects.repeat.path">
    <property name="count" type="float" value="3.0" />
    <property name="startPos" type="float" value="0.0" />
    <property name="endPos" type="float" value="1.0" />
    <property name="pathPhase" type="float" value="0.0" />
    <property name="tangent" type="bool" value="false" />
    <property name="offset" type="vec2" value="0.0,0.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="scale" type="float" value="1.0" />
    <property name="alpha" type="float" value="1.0" />
    <property name="fillColor" type="color" value="#FFFFFFFF" />
    <property name="blend" type="float" value="0.0" />
</effect>"##,
    tests: [
        "effects/path-repeat/basic.amproj",
        "effects/path-repeat/animation.amproj",
    ],
    fields: [
        define_field! {
            name: "count",
            zh: "数量",
            en: "Count",
            type: FieldType::Float,
            support: Full,
            default: "3.0",
            desc_zh: "路径上分布的副本数量",
            desc_en: "Number of copies along the path",
        },
        define_field! {
            name: "startPos",
            zh: "起始位置",
            en: "Start Position",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "路径起始位置（0.0-1.0）",
            desc_en: "Path start position (0.0-1.0)",
        },
        define_field! {
            name: "endPos",
            zh: "结束位置",
            en: "End Position",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "路径结束位置（0.0-1.0）",
            desc_en: "Path end position (0.0-1.0)",
        },
        define_field! {
            name: "pathPhase",
            zh: "路径相位",
            en: "Path Phase",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "路径上的相位偏移",
            desc_en: "Phase offset along the path",
        },
        define_field! {
            name: "tangent",
            zh: "切线对齐",
            en: "Tangent",
            type: FieldType::Bool,
            support: Full,
            default: "false",
            desc_zh: "副本是否沿切线方向旋转",
            desc_en: "Whether copies rotate to follow path tangent",
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
        define_field! {
            name: "fillColor",
            zh: "填充颜色",
            en: "Fill Color",
            type: FieldType::Color,
            support: Full,
            default: "#FFFFFFFF",
            desc_zh: "交替副本的填充颜色",
            desc_en: "Fill color for alternate copies",
        },
        define_field! {
            name: "blend",
            zh: "混合",
            en: "Blend",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "填充颜色混合量",
            desc_en: "Fill color blend amount",
        },
    ],
}

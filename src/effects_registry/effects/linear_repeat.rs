//! # linear_repeat.rs
//!
//! # Linear Repeat 效果定义
//!
//! Linear Repeat effect definition - creates multiple copies arranged in a line.
//! 线性重复效果定义 - 创建沿直线排列的多个副本。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.repeat.line",
    short_name: "linear-repeat",
    zh: "线性重复 (Linear Repeat)",
    en: "Linear Repeat",
    desc_zh: "创建沿直线排列的图层副本，支持位置、偏移、旋转、缩放、透明度和颜色混合等高级控制。",
    desc_en: "Creates copies of the layer arranged in a line with advanced controls for position, offset, rotation, scale, alpha, and color blending.",
    support: Partial,
    xml: r##"<effect id="com.alightcreative.effects.repeat.line">
    <property name="count" type="float" value="5.0" />
    <property name="position" type="vec2" value="200.0,0.0" />
    <property name="offset" type="vec2" value="0.0,0.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="scale" type="float" value="1.0" />
    <property name="alpha" type="float" value="1.0" />
    <property name="fillColor" type="color" value="#ffff0000" />
    <property name="blend" type="float" value="0.0" />
    <property name="colorAltCopies" type="bool" value="false" />
    <property name="start" type="float" value="0.0" />
    <property name="end" type="float" value="1.0" />
    <property name="phase" type="float" value="0.0" />
    <property name="easeIn" type="float" value="0.0" />
    <property name="easeOut" type="float" value="0.0" />
    <property name="overlap" type="float" value="0.0" />
    <property name="shape" type="int" value="0" />
    <property name="invert" type="bool" value="false" />
    <property name="randomOrder" type="bool" value="false" />
    <property name="seed" type="float" value="0.0" />
</effect>"##,
    tests: [
        "effects/linear-repeat/basic.amproj",
        "effects/linear-repeat/dual.amproj",
        "effects/linear-repeat/dual-16-9.amproj",
        "effects/linear-repeat/random.amproj",
        "effects/linear-repeat/random_generated1/1.amproj",
        "effects/linear-repeat/random_generated1/2.amproj",
        "effects/linear-repeat/random_generated1/3.amproj",
        "effects/linear-repeat/random_generated2/1.amproj",
        "effects/linear-repeat/random_generated2/2.amproj",
        "effects/linear-repeat/random_generated2/3.amproj",
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
            name: "position",
            zh: "位置",
            en: "Position",
            type: FieldType::Vec2,
            support: Full,
            default: "0.0,0.0",
            desc_zh: "从第一个副本到最后一个副本的总位移",
            desc_en: "Total displacement from first to last copy",
        },
        define_field! {
            name: "offset",
            zh: "偏移",
            en: "Offset",
            type: FieldType::Vec2,
            support: Full,
            default: "0.0,0.0",
            desc_zh: "应用于所有副本的恒定偏移",
            desc_en: "Constant offset applied to all copies",
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
            default: "#ffffffff",
            desc_zh: "用于颜色混合的填充颜色",
            desc_en: "Fill color for color blending",
        },
        define_field! {
            name: "blend",
            zh: "混合",
            en: "Blend",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "填充颜色的混合量",
            desc_en: "Amount of fill color blending",
        },
        define_field! {
            name: "colorAltCopies",
            zh: "交替颜色",
            en: "Alternate Colors",
            type: FieldType::Bool,
            support: Full,
            default: "false",
            desc_zh: "是否交替应用颜色",
            desc_en: "Whether to alternate color application",
        },
        define_field! {
            name: "start",
            zh: "开始",
            en: "Start",
            type: FieldType::Float,
            support: Partial,
            default: "0.0",
            desc_zh: "分布的起始点（0-1）",
            desc_en: "Start point of distribution (0-1)",
        },
        define_field! {
            name: "end",
            zh: "结束",
            en: "End",
            type: FieldType::Float,
            support: Partial,
            default: "1.0",
            desc_zh: "分布的结束点（0-1）",
            desc_en: "End point of distribution (0-1)",
        },
        define_field! {
            name: "phase",
            zh: "相位",
            en: "Phase",
            type: FieldType::Float,
            support: Partial,
            default: "0.0",
            desc_zh: "分布的相位偏移",
            desc_en: "Phase shift for distribution",
        },
        define_field! {
            name: "easeIn",
            zh: "缓入",
            en: "Ease In",
            type: FieldType::Float,
            support: Partial,
            default: "0.0",
            desc_zh: "分布的缓入因子",
            desc_en: "Ease-in factor for distribution",
        },
        define_field! {
            name: "easeOut",
            zh: "缓出",
            en: "Ease Out",
            type: FieldType::Float,
            support: Partial,
            default: "0.0",
            desc_zh: "分布的缓出因子",
            desc_en: "Ease-out factor for distribution",
        },
        define_field! {
            name: "overlap",
            zh: "重叠",
            en: "Overlap",
            type: FieldType::Float,
            support: Unsupported,
            default: "0.0",
            desc_zh: "副本之间的重叠因子",
            desc_en: "Overlap factor between copies",
        },
        define_field! {
            name: "shape",
            zh: "形状",
            en: "Shape",
            type: FieldType::Float,
            support: Partial,
            default: "0",
            desc_zh: "分布形状（0=线性）",
            desc_en: "Distribution shape (0=linear)",
        },
        define_field! {
            name: "invert",
            zh: "反转",
            en: "Invert",
            type: FieldType::Bool,
            support: Unsupported,
            default: "false",
            desc_zh: "是否反转效果",
            desc_en: "Whether to invert the effect",
        },
        define_field! {
            name: "randomOrder",
            zh: "随机顺序",
            en: "Random Order",
            type: FieldType::Bool,
            support: Unsupported,
            default: "false",
            desc_zh: "是否随机化副本顺序",
            desc_en: "Whether to randomize copy order",
        },
        define_field! {
            name: "seed",
            zh: "种子",
            en: "Seed",
            type: FieldType::Float,
            support: Unsupported,
            default: "0.0",
            desc_zh: "随机种子",
            desc_en: "Random seed",
        },
    ],
}

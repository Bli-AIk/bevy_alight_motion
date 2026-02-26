use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.repeat.radial",
    short_name: "radial_repeat",
    zh: "径向重复 (Radial Repeat)",
    en: "Radial Repeat",
    desc_zh: "沿圆形路径创建图层的多个副本，支持半径、扫掠角度、缩放等参数。",
    desc_en: "Creates multiple copies of the layer along a circular path with configurable radius, sweep angle, scale, and more.",
    support: Full,
    xml: r##"<effect id="com.alightcreative.effects.repeat.radial">
    <property name="count" type="float" value="5.0" />
    <property name="radius" type="float" value="100.0" />
    <property name="orientation" type="float" value="0.0" />
    <property name="startAngle" type="float" value="0.0" />
    <property name="sweep" type="float" value="360.0" />
    <property name="baseScale" type="float" value="1.0" />
    <property name="offset" type="vec2" value="0.0,0.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="scale" type="float" value="1.0" />
    <property name="alpha" type="float" value="1.0" />
    <property name="fillColor" type="color" value="#ffffffff" />
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
    tests: ["effects/radial-repeat/basic.amproj"],
    fields: [
        define_field! {
            name: "count",
            zh: "数量",
            en: "Count",
            type: FieldType::Float,
            support: Full,
            default: "5.0",
            desc_zh: "副本数量",
            desc_en: "Number of copies",
        },
        define_field! {
            name: "radius",
            zh: "半径",
            en: "Radius",
            type: FieldType::Float,
            support: Full,
            default: "100.0",
            desc_zh: "圆形路径的半径",
            desc_en: "Radius of the circular path",
        },
        define_field! {
            name: "orientation",
            zh: "朝向",
            en: "Orientation",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "副本朝向角度",
            desc_en: "Orientation angle of copies",
        },
        define_field! {
            name: "startAngle",
            zh: "起始角度",
            en: "Start Angle",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "起始角度（度）",
            desc_en: "Start angle (degrees)",
        },
        define_field! {
            name: "sweep",
            zh: "扫掠",
            en: "Sweep",
            type: FieldType::Float,
            support: Full,
            default: "360.0",
            desc_zh: "扫掠角度（度）",
            desc_en: "Sweep angle (degrees)",
        },
        define_field! {
            name: "baseScale",
            zh: "基础缩放",
            en: "Base Scale",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "所有副本的基础缩放",
            desc_en: "Base scale for all copies",
        },
        define_field! {
            name: "offset",
            zh: "偏移",
            en: "Offset",
            type: FieldType::Vec2,
            support: Full,
            default: "0.0,0.0",
            desc_zh: "每个副本的偏移",
            desc_en: "Offset per copy",
        },
        define_field! {
            name: "angle",
            zh: "角度",
            en: "Angle",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "每个副本的旋转角度",
            desc_en: "Rotation angle per copy",
        },
        define_field! {
            name: "scale",
            zh: "缩放",
            en: "Scale",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "每个副本的缩放",
            desc_en: "Scale per copy",
        },
        define_field! {
            name: "alpha",
            zh: "透明度",
            en: "Alpha",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "每个副本的透明度",
            desc_en: "Alpha per copy",
        },
        define_field! {
            name: "fillColor",
            zh: "填充颜色",
            en: "Fill Color",
            type: FieldType::Color,
            support: Full,
            default: "#ffffffff",
            desc_zh: "副本填充颜色",
            desc_en: "Fill color for copies",
        },
        define_field! {
            name: "blend",
            zh: "混合",
            en: "Blend",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "颜色混合量",
            desc_en: "Color blend amount",
        },
        define_field! {
            name: "colorAltCopies",
            zh: "交替着色",
            en: "Alternate Color",
            type: FieldType::Bool,
            support: Full,
            default: "false",
            desc_zh: "交替副本着色",
            desc_en: "Alternate copy coloring",
        },
        define_field! {
            name: "start",
            zh: "开始",
            en: "Start",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "可见范围开始",
            desc_en: "Visibility range start",
        },
        define_field! {
            name: "end",
            zh: "结束",
            en: "End",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "可见范围结束",
            desc_en: "Visibility range end",
        },
        define_field! {
            name: "phase",
            zh: "相位",
            en: "Phase",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "动画相位偏移",
            desc_en: "Animation phase offset",
        },
        define_field! {
            name: "easeIn",
            zh: "缓入",
            en: "Ease In",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "缓入量",
            desc_en: "Ease in amount",
        },
        define_field! {
            name: "easeOut",
            zh: "缓出",
            en: "Ease Out",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "缓出量",
            desc_en: "Ease out amount",
        },
        define_field! {
            name: "overlap",
            zh: "重叠",
            en: "Overlap",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "副本重叠量",
            desc_en: "Copy overlap amount",
        },
        define_field! {
            name: "shape",
            zh: "形状",
            en: "Shape",
            type: FieldType::Int,
            support: Full,
            default: "0",
            desc_zh: "排列形状类型",
            desc_en: "Arrangement shape type",
        },
        define_field! {
            name: "invert",
            zh: "反转",
            en: "Invert",
            type: FieldType::Bool,
            support: Full,
            default: "false",
            desc_zh: "反转排列顺序",
            desc_en: "Invert arrangement order",
        },
        define_field! {
            name: "randomOrder",
            zh: "随机顺序",
            en: "Random Order",
            type: FieldType::Bool,
            support: Full,
            default: "false",
            desc_zh: "随机排列顺序",
            desc_en: "Randomize arrangement order",
        },
        define_field! {
            name: "seed",
            zh: "种子",
            en: "Seed",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "随机种子",
            desc_en: "Random seed",
        },
    ],
}

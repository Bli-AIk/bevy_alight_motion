//! Registers metadata for the built-in Parent Helper effect.
//! 注册内置 Parent Helper 效果的元数据。
//!
//! Parent Helper is unusual because it changes transform inheritance semantics rather than directly
//! drawing pixels. This file gives the registry the vocabulary to describe those inheritance modes
//! and how the runtime implementation interprets them.
//! Parent Helper 的特殊之处在于它修改的是变换继承语义，而不是直接绘制像素。
//! 这个文件为注册表提供描述这些继承模式的词汇，也让运行时实现的解释规则可以在文档层被看见。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.parenthelper",
    short_name: "parenthelper",
    zh: "父级助手 (Parenting Helper)",
    en: "Parenting Helper",
    desc_zh: "调整子图层对父级缩放和旋转的继承方式，并支持自动旋转修正。",
    desc_en: "Adjusts how a child layer inherits parent scale and rotation, with optional auto-rotation correction.",
    support: Partial,
    xml: r##"<effect id="com.alightcreative.effects.parenthelper" locallyApplied="true">
    <property name="scaleMode" type="int" value="0" />
    <property name="rotateMode" type="int" value="0" />
    <property name="scaleWeight" type="float" value="1.0" />
    <property name="rotateWeight" type="float" value="1.0" />
    <property name="autoRotate" type="int" value="0" />
    <property name="radiusAdjust" type="float" value="0.0" />
</effect>"##,
    tests: ["effects/parenthelper/basic"],
    fields: [
        define_field! {
            name: "scaleMode",
            zh: "缩放模式",
            en: "Scale Mode",
            type: FieldType::Enum(&["0 Normal", "1 Locked", "2 Weighted"]),
            support: Full,
            default: "0",
            desc_zh: "控制子图层如何继承父级缩放。",
            desc_en: "Controls how the child inherits parent scale.",
        },
        define_field! {
            name: "rotateMode",
            zh: "旋转模式",
            en: "Rotate Mode",
            type: FieldType::Enum(&["0 Normal", "1 Locked", "2 Weighted"]),
            support: Full,
            default: "0",
            desc_zh: "控制子图层如何继承父级旋转。",
            desc_en: "Controls how the child inherits parent rotation.",
        },
        define_field! {
            name: "scaleWeight",
            zh: "缩放权重",
            en: "Scale Weight",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "加权模式下的父级缩放继承权重。",
            desc_en: "Parent scale inheritance weight in weighted mode.",
        },
        define_field! {
            name: "rotateWeight",
            zh: "旋转权重",
            en: "Rotate Weight",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "加权模式下的父级旋转继承权重。",
            desc_en: "Parent rotation inheritance weight in weighted mode.",
        },
        define_field! {
            name: "autoRotate",
            zh: "自动旋转",
            en: "Auto Rotate",
            type: FieldType::Enum(&["0 Off", "1 X Axis", "2 Y Axis"]),
            support: Full,
            default: "0",
            desc_zh: "基于位置沿 X/Y 轴追加自动旋转。",
            desc_en: "Adds position-based auto rotation along the X or Y axis.",
        },
        define_field! {
            name: "radiusAdjust",
            zh: "半径调整",
            en: "Radius Adjust",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "自动旋转时用于修正旋转半径。",
            desc_en: "Adjusts the effective radius used by auto-rotate.",
        },
    ],
}

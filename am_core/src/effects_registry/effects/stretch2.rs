//! Registers metadata for the built-in Stretch effect.
//! 注册内置 Stretch 效果的元数据。
//!
//! Stretch changes UV-space sampling rather than geometric scale, which makes its semantics easy to
//! misunderstand. The registry entry here is the bilingual contract used to describe those controls.
//! Stretch 改变的是 UV 空间采样，而不是几何缩放，因此它的语义很容易被误解。
//! 这里的注册表条目就是那份双语契约，用来解释这些控制项真正作用的是什么。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.stretch2",
    short_name: "stretch2",
    zh: "拉伸 (Stretch)",
    en: "Stretch",
    desc_zh: "沿指定角度方向在UV空间拉伸图层。",
    desc_en: "Stretches the layer along a specified angle in UV space.",
    support: Partial,
    xml: r##"<effect id="com.alightcreative.effects.stretch2">
    <property name="scale" type="float" value="1.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="contentOnly" type="bool" value="false" />
</effect>"##,
    tests: [],
    fields: [
        define_field! {
            name: "scale",
            zh: "缩放",
            en: "Scale",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "拉伸轴方向缩放因子（1.0=无拉伸）",
            desc_en: "Scale factor along stretch axis (1.0 = no stretch)",
        },
        define_field! {
            name: "angle",
            zh: "角度",
            en: "Angle",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "拉伸方向角度（度）",
            desc_en: "Stretch direction angle (degrees)",
        },
        define_field! {
            name: "contentOnly",
            zh: "仅内容",
            en: "Content Only",
            type: FieldType::Bool,
            support: Full,
            default: "false",
            desc_zh: "是否将拉伸结果裁剪到原始图层 Alpha",
            desc_en: "Whether to mask stretched result to original layer alpha",
        },
    ],
}

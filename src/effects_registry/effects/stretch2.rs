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

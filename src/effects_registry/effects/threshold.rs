use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.threshold",
    short_name: "threshold",
    zh: "阈值 (Threshold)",
    en: "Threshold",
    desc_zh: "将图像转换为只有黑色和白色的高对比度图像。",
    desc_en: "Converts the image to a high-contrast image with only black and white.",
    support: Full,
    xml: r##"<effect id="com.alightcreative.effects.threshold">
    <property name="threshold" type="float" value="0.5" />
    <property name="feather" type="float" value="0.0" />
    <property name="invert" type="bool" value="false" />
    <property name="blendMode" type="int" value="0" />
</effect>"##,
    tests: ["effects/threshold/basic.amproj"],
    fields: [
        define_field! {
            name: "threshold",
            zh: "阈值",
            en: "Threshold",
            type: FieldType::Float,
            support: Full,
            default: "0.5",
            desc_zh: "亮度截止值",
            desc_en: "Luminance cutoff",
        },
        define_field! {
            name: "feather",
            zh: "羽化",
            en: "Feather",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "边缘柔和度",
            desc_en: "Edge softness",
        },
        define_field! {
            name: "invert",
            zh: "反转",
            en: "Invert",
            type: FieldType::Bool,
            support: Full,
            default: "false",
            desc_zh: "反转效果",
            desc_en: "Invert the effect",
        },
        define_field! {
            name: "blendMode",
            zh: "混合模式",
            en: "Blend Mode",
            type: FieldType::Int,
            support: Partial,
            default: "0",
            desc_zh: "混合模式",
            desc_en: "Blend mode",
        },
    ],
}

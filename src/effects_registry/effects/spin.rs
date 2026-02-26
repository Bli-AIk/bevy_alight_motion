use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.spin",
    short_name: "spin",
    zh: "旋转 (Spin)",
    en: "Spin",
    desc_zh: "使图层以指定速度持续旋转。",
    desc_en: "Makes the layer continuously rotate at a specified speed.",
    support: Full,
    xml: r##"<effect id="com.alightcreative.effects.spin">
    <property name="rpm" type="float" value="60.0" />
</effect>"##,
    tests: ["effects/spin/basic.amproj"],
    fields: [
        define_field! {
            name: "rpm",
            zh: "转速",
            en: "RPM",
            type: FieldType::Float,
            support: Full,
            default: "60.0",
            desc_zh: "每分钟旋转次数",
            desc_en: "Revolutions per minute",
        },
    ],
}

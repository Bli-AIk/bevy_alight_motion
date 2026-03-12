use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.textspacing",
    short_name: "textspacing",
    zh: "文字间距 (Text Spacing)",
    en: "Text Spacing",
    desc_zh: "控制文本的字间距和行间距。",
    desc_en: "Controls letter spacing and line spacing for text layers.",
    support: Unsupported,
    xml: r##"<effect id="com.alightcreative.effects.textspacing">
    <property name="letterspacing" type="float" value="0.0" />
    <property name="linespacing" type="float" value="1.0" />
</effect>"##,
    tests: ["effects/text-spacing/basic.amproj"],
    fields: [
        define_field! {
            name: "letterspacing",
            zh: "字间距",
            en: "Letter Spacing",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "字间距（em 单位，0.0=默认）",
            desc_en: "Letter spacing in em units (0.0 = default)",
        },
        define_field! {
            name: "linespacing",
            zh: "行间距",
            en: "Line Spacing",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "行间距倍数（1.0=默认）",
            desc_en: "Line spacing multiplier (1.0 = default)",
        },
    ],
}

//! Registers metadata for the built-in Text Progress effect.
//! 注册内置 Text Progress 效果的元数据。
//!
//! Text Progress is currently treated as unsupported at runtime, but it still needs a complete
//! registry description so unsupported status, fields, and reference tests remain visible.
//! Text Progress 目前在运行时仍被视为未支持，但它依然需要一份完整的注册表描述，
//! 这样未支持状态、字段定义和参考测试才能继续被看见。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.textprogress",
    short_name: "textprogress",
    zh: "文字进度 (Text Progress)",
    en: "Text Progress",
    desc_zh: "显示文字的部分内容，实现打字机效果。",
    desc_en: "Displays part of the text as a string, enabling typewriter effects.",
    support: Unsupported,
    xml: r##"<effect id="com.alightcreative.effects.textprogress">
    <property name="start" type="float" value="0.0" />
    <property name="end" type="float" value="1.0" />
    <property name="cursor" type="int" value="0" />
    <property name="blink" type="bool" value="false" />
</effect>"##,
    tests: ["effects/text-progress/basic.amproj"],
    fields: [
        define_field! {
            name: "start",
            zh: "起始",
            en: "Start",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "文本显示起始位置（0-1）",
            desc_en: "Text display start position (0-1)",
        },
        define_field! {
            name: "end",
            zh: "结束",
            en: "End",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "文本显示结束位置（0-1）",
            desc_en: "Text display end position (0-1)",
        },
        define_field! {
            name: "cursor",
            zh: "光标",
            en: "Cursor",
            type: FieldType::Int,
            support: Partial,
            default: "0",
            desc_zh: "光标样式（0=无，1-8=不同样式）",
            desc_en: "Cursor style (0=none, 1-8=different styles)",
        },
        define_field! {
            name: "blink",
            zh: "闪烁",
            en: "Blink",
            type: FieldType::Bool,
            support: Partial,
            default: "false",
            desc_zh: "光标闪烁",
            desc_en: "Cursor blink",
        },
    ],
}

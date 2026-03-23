//! Registers metadata for the built-in Counter text effect.
//! 注册内置 Counter 文本效果的元数据。
//!
//! This file declares how the registry, docs, and test discovery systems should understand the
//! Counter effect: its public id, field schema, bilingual descriptions, and sample XML shape.
//! 这个文件声明注册表、文档生成器和测试发现链应该如何理解 Counter 效果：包括公开 id、字段结构、
//! 双语说明以及示例 XML 片段。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.counter",
    short_name: "counter",
    zh: "计数 (Counter)",
    en: "Counter",
    desc_zh: "将文本中的数字替换为经过偏移和缩放的值，实现计数器动画效果。",
    desc_en: "Replaces numeric values in text with offset/scaled values for counter animation.",
    support: Unsupported,
    xml: r##"<effect id="com.alightcreative.effects.counter">
    <property name="offset" type="float" value="0.0" />
    <property name="scale" type="float" value="1.0" />
</effect>"##,
    tests: ["effects/count/basic/test.amproj"],
    fields: [
        define_field! {
            name: "offset",
            zh: "偏移",
            en: "Offset",
            type: FieldType::Float,
            support: Full,
            default: "0",
            desc_zh: "添加到数字值的偏移量",
            desc_en: "Value added to numeric values",
        },
        define_field! {
            name: "scale",
            zh: "倍率",
            en: "Scale",
            type: FieldType::Float,
            support: Full,
            default: "1",
            desc_zh: "数字值的乘数",
            desc_en: "Multiplier for numeric values",
        },
    ],
}

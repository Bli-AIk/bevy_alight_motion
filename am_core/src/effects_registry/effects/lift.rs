//! Registers metadata for the built-in Lift / Copy Background effect.
//! 注册内置 Lift / Copy Background 效果的元数据。
//!
//! Lift is primarily useful when combined with downstream compositing effects, so the registry entry
//! explains it as a background-copy primitive rather than a standalone visible transform.
//! Lift 往往是和后续合成效果一起使用的，因此这里把它登记为一种背景复制原语，而不是孤立的可见变换。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.lift",
    short_name: "lift",
    zh: "复制背景 (Copy Background)",
    en: "Copy Background",
    desc_zh: "将该层后面的像素复制到当前层中，配合其他效果使用（如模糊背景）。",
    desc_en: "Copies pixels from behind this layer into the current layer. Use with other effects to process the background.",
    support: Full,
    xml: r##"<effect id="com.alightcreative.effects.lift">
    <property name="fill" type="float" value="0.0" />
</effect>"##,
    tests: [
        "effects/lift/replacecolor",
        "effects/lift/wavewarp2",
    ],
    fields: [
        define_field! {
            name: "fill",
            zh: "填充",
            en: "Fill",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "0=完全显示背景，1=完全显示原始内容",
            desc_en: "0=show background fully, 1=show original content fully",
        },
    ],
}

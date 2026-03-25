//! Registers metadata for the built-in Spin effect.
//!
//! 注册内置 Spin 效果的元数据。
//!
//! Spin is intentionally small: a continuous-rotation effect described almost entirely by RPM.
//! Keeps even simple built-ins in the same registry-driven docs and test
//! discovery pipeline as the larger effects.
//!
//! Spin 是一个刻意保持很小的连续旋转效果，核心参数几乎只有 RPM。
//! 即便如此简单的内置效果，也会进入同一套注册表驱动的文档与测试发现流程。

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

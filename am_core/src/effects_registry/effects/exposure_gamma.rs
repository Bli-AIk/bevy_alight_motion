//! Registers metadata for the built-in Exposure / Gamma effect.
//! 注册内置 Exposure / Gamma 效果的元数据。
//!
//! This entry documents the tone-adjustment effect that changes exposure, gamma, and offset on
//! media layers. The registry consumes it for docs, support matrices, and test linkage.
//! 这个条目描述的是会调整媒体图层曝光、伽马和偏移的色调效果。
//! 注册表会消费它来生成文档、支持矩阵以及关联测试信息。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.exposure",
    short_name: "exposure",
    zh: "曝光 / 伽马 (Exposure / Gamma)",
    en: "Exposure / Gamma",
    desc_zh: "调整照片/视频的曝光、伽马曲线和偏移。",
    desc_en: "Adjusts photo/video exposure, gamma curve, and offset.",
    support: Full,
    xml: r##"<effect id="com.alightcreative.effects.exposure">
    <property name="exposure" type="float" value="0.0" />
    <property name="gamma" type="float" value="1.0" />
    <property name="offset" type="float" value="0.0" />
</effect>"##,
    tests: [
        "effects/exposure-gamma/exposure/test.amproj",
        "effects/exposure-gamma/gamma/test.amproj",
        "effects/exposure-gamma/exposure-offset/test.amproj",
        "effects/exposure-gamma/gamma-offset/test.amproj",
        "effects/exposure-gamma/complex/test.amproj",
    ],
    fields: [
        define_field! {
            name: "exposure",
            zh: "曝光",
            en: "Exposure",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "曝光调整 (-2.0 到 2.0)",
            desc_en: "Exposure adjustment (-2.0 to 2.0)",
        },
        define_field! {
            name: "gamma",
            zh: "伽马",
            en: "Gamma",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "伽马曲线 (0.01 到 9.99)",
            desc_en: "Gamma curve (0.01 to 9.99)",
        },
        define_field! {
            name: "offset",
            zh: "偏移",
            en: "Offset",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "亮度偏移 (-0.9 到 0.9)",
            desc_en: "Brightness offset (-0.9 to 0.9)",
        },
    ],
}

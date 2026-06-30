//! Registers metadata for the built-in Fade effect.
//! 注册内置 Fade 效果的元数据。
//!
//! Fade is one of the simplest but most frequently used timeline effects. This file captures the
//! exact public field names and descriptions used by the registry so generated docs and tests stay
//! aligned with the authored XML representation.
//! Fade 是最基础也最常用的时间线效果之一。这个文件记录注册表所依赖的公开字段名和说明，
//! 让生成的文档与测试始终和作者侧 XML 表达保持一致。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.fade",
    short_name: "fade",
    zh: "渐入渐出 (Fade)",
    en: "Fade In/Out",
    desc_zh: "自动对图层透明度进行渐入渐出动画。在图层开头淡入，在图层结尾淡出。",
    desc_en: "Automatically animates the layer opacity to fade the layer in at its beginning, and out again at its end.",
    support: Full,
    xml: r##"<effect id="com.alightcreative.effects.fade" locallyApplied="true">
    <property name="inTime" type="float" value="0.500000" />
    <property name="outTime" type="float" value="0.500000" />
</effect>"##,
    tests: ["effects/fade/fade.amproj"],
    fields: [
        define_field! {
            name: "inTime",
            zh: "淡入时间",
            en: "Fade In Duration",
            type: FieldType::Float,
            support: Full,
            default: "0.5",
            desc_zh: "淡入持续时间（秒）",
            desc_en: "Fade in duration in seconds",
        },
        define_field! {
            name: "outTime",
            zh: "淡出时间",
            en: "Fade Out Duration",
            type: FieldType::Float,
            support: Full,
            default: "0.5",
            desc_zh: "淡出持续时间（秒）",
            desc_en: "Fade out duration in seconds",
        },
    ],
}

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.repeat.echokf",
    short_name: "echokf",
    zh: "回声关键帧 (Echo Keyframes)",
    en: "Echo Keyframes",
    desc_zh: "创建元素的时移回声副本，支持关键帧控制时间间隔、数量和透明度。",
    desc_en: "Creates time-shifted echo copies of an element with keyframe control over timing, count, and alpha.",
    support: Full,
    xml: r##"<effect id="com.alightcreative.effects.repeat.echokf">
    <property name="seconds" type="float" value="0.5" />
    <property name="count" type="float" value="1.0" />
    <property name="alpha" type="float" value="1.0" />
    <property name="mode" type="int" value="1" />
</effect>"##,
    tests: ["effects/echo-keyframes/basic.amproj"],
    fields: [
        define_field! {
            name: "seconds",
            zh: "时间间隔",
            en: "Seconds",
            type: FieldType::Float,
            support: Full,
            default: "0.5",
            desc_zh: "每个回声副本的时间间隔（秒）",
            desc_en: "Time spacing per echo copy (seconds)",
        },
        define_field! {
            name: "count",
            zh: "数量",
            en: "Count",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "回声副本数量",
            desc_en: "Number of echo copies",
        },
        define_field! {
            name: "alpha",
            zh: "透明度",
            en: "Alpha",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "回声衰减透明度曲线",
            desc_en: "Echo fade alpha curve",
        },
        define_field! {
            name: "mode",
            zh: "合成模式",
            en: "Composite Mode",
            type: FieldType::Int,
            support: Full,
            default: "1",
            desc_zh: "合成模式（0=上方, 1=下方）",
            desc_en: "Composite mode (0=atop, 1=behind)",
        },
    ],
}

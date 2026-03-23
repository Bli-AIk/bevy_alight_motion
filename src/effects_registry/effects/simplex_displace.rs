use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.randomdisplace",
    short_name: "simplex_displace",
    zh: "随机位移 (Simplex Displace)",
    en: "Simplex Displace",
    desc_zh: "使用 Simplex 噪声对图层位置进行基于空间的随机位移。",
    desc_en: "Applies spatially-varying random position displacement using Simplex noise.",
    support: Full,
    xml: r##"<effect id="com.alightcreative.effects.randomdisplace" locallyApplied="true">
    <property name="mag" type="float" value="50.0" />
    <property name="evolution" type="float" value="0.0" />
    <property name="seed" type="float" value="0.0" />
    <property name="scatter" type="float" value="0.5" />
</effect>"##,
    tests: [
        "effects/jetter/basic/test.amproj",
        "effects/jetter/size/test.amproj",
        "effects/jetter/evolution/test.amproj",
        "effects/jetter/seed/test.amproj",
        "effects/jetter/scatter/test.amproj",
        "effects/jetter/complex/test.amproj",
    ],
    fields: [
        define_field! {
            name: "mag",
            zh: "幅度",
            en: "Magnitude",
            type: FieldType::Float,
            support: Full,
            default: "50.0",
            desc_zh: "位移幅度（像素）",
            desc_en: "Displacement magnitude (pixels)",
        },
        define_field! {
            name: "evolution",
            zh: "演变",
            en: "Evolution",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "噪声时间演变参数",
            desc_en: "Noise temporal evolution parameter",
        },
        define_field! {
            name: "seed",
            zh: "种子",
            en: "Seed",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "噪声种子值",
            desc_en: "Noise seed value",
        },
        define_field! {
            name: "scatter",
            zh: "散布",
            en: "Scatter",
            type: FieldType::Float,
            support: Full,
            default: "0.5",
            desc_zh: "空间频率（0.0-2.0）",
            desc_en: "Spatial frequency (0.0-2.0)",
        },
    ],
}

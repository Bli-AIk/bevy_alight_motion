use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.mirror",
    short_name: "mirror",
    zh: "镜子 (Mirror)",
    en: "Mirror",
    desc_zh: "沿水平或垂直轴镜像图层内容，支持多种混合模式和透明度控制。",
    desc_en: "Mirrors layer content along horizontal or vertical axis with blend mode and alpha control.",
    support: Full,
    xml: r##"<effect id="com.alightcreative.effects.mirror">
    <property name="type" type="int" value="0" />
    <property name="blendMode" type="int" value="0" />
    <property name="alpha" type="float" value="1.0" />
    <property name="offset" type="float" value="0.0" />
</effect>"##,
    tests: [
        "effects/mirror/basic",
        "effects/mirror/alpha",
        "effects/mirror/offset",
    ],
    fields: [
        define_field! {
            name: "type",
            zh: "方向",
            en: "Type",
            type: FieldType::Int,
            support: Full,
            default: "0",
            desc_zh: "镜像方向：0=水平，1=垂直",
            desc_en: "Mirror direction: 0=horizontal, 1=vertical",
        },
        define_field! {
            name: "blendMode",
            zh: "混合模式",
            en: "Blend Mode",
            type: FieldType::Int,
            support: Full,
            default: "0",
            desc_zh: "混合模式：0=普通，1=正片叠底，2=滤色，3=上层，4=下层",
            desc_en: "Blend mode: 0=normal, 1=multiply, 2=screen, 3=over, 4=under",
        },
        define_field! {
            name: "alpha",
            zh: "透明度",
            en: "Alpha",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "镜像内容的混合透明度",
            desc_en: "Blend alpha for the mirrored content",
        },
        define_field! {
            name: "offset",
            zh: "偏移",
            en: "Offset",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "镜像轴的偏移量",
            desc_en: "Offset of the mirror axis",
        },
    ],
}

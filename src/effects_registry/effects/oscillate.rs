use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.oscillate3",
    short_name: "oscillate3",
    zh: "振荡 (Oscillate)",
    en: "Oscillate",
    desc_zh: "使图层沿指定方向以正弦/三角波进行周期性振荡运动。",
    desc_en: "Makes the layer oscillate periodically along a specified direction using sine/triangle waves.",
    support: Partial,
    xml: r##"<effect id="com.alightcreative.effects.oscillate3">
    <property name="direction" type="int" value="0" />
    <property name="angle" type="float" value="45.0" />
    <property name="freq" type="float" value="2.0" />
    <property name="mag" type="float" value="25.0" />
    <property name="type" type="int" value="0" />
    <property name="phase" type="float" value="0.0" />
</effect>"##,
    tests: [],
    fields: [
        define_field! {
            name: "direction",
            zh: "方向模式",
            en: "Direction Mode",
            type: FieldType::Int,
            support: Partial,
            default: "0",
            desc_zh: "运动方向模式（0=角度, 1=深度/Z轴, 2=轨道）",
            desc_en: "Movement direction mode (0=angle, 1=depth/Z, 2=orbit)",
        },
        define_field! {
            name: "angle",
            zh: "角度",
            en: "Angle",
            type: FieldType::Float,
            support: Full,
            default: "45.0",
            desc_zh: "运动方向角度（度）",
            desc_en: "Movement direction angle (degrees)",
        },
        define_field! {
            name: "freq",
            zh: "频率",
            en: "Frequency",
            type: FieldType::Float,
            support: Full,
            default: "2.0",
            desc_zh: "振荡频率（Hz）",
            desc_en: "Oscillation frequency (Hz)",
        },
        define_field! {
            name: "mag",
            zh: "幅度",
            en: "Magnitude",
            type: FieldType::Float,
            support: Full,
            default: "25.0",
            desc_zh: "运动幅度（像素）",
            desc_en: "Movement magnitude (pixels)",
        },
        define_field! {
            name: "type",
            zh: "波形",
            en: "Wave Type",
            type: FieldType::Int,
            support: Full,
            default: "0",
            desc_zh: "波形类型（0=正弦, 1=三角波）",
            desc_en: "Wave type (0=sine, 1=triangle)",
        },
        define_field! {
            name: "phase",
            zh: "相位",
            en: "Phase",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "相位偏移",
            desc_en: "Phase offset",
        },
    ],
}

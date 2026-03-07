use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.swing2",
    short_name: "swing2",
    zh: "摇摆 (Swing)",
    en: "Swing",
    desc_zh: "使图层以指定频率和幅度来回摇摆旋转。",
    desc_en: "Makes the layer swing back and forth with specified frequency and amplitude.",
    support: Full,
    xml: r##"<effect id="com.alightcreative.effects.swing2">
    <property name="freq" type="float" value="1.0" />
    <property name="a1" type="float" value="-30.0" />
    <property name="a2" type="float" value="30.0" />
    <property name="phase" type="float" value="0.0" />
    <property name="type" type="int" value="0" />
</effect>"##,
    tests: [
        "effects/swing/basic.amproj",
        "effects/swing/animation.amproj",
        "effects/swing/multi.amproj",
    ],
    fields: [
        define_field! {
            name: "freq",
            zh: "频率",
            en: "Frequency",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "振荡频率（Hz）",
            desc_en: "Oscillation frequency in Hz",
        },
        define_field! {
            name: "a1",
            zh: "最小角度",
            en: "Min Angle",
            type: FieldType::Float,
            support: Full,
            default: "-30.0",
            desc_zh: "振荡最小角度（度）",
            desc_en: "Minimum oscillation angle (degrees)",
        },
        define_field! {
            name: "a2",
            zh: "最大角度",
            en: "Max Angle",
            type: FieldType::Float,
            support: Full,
            default: "30.0",
            desc_zh: "振荡最大角度（度）",
            desc_en: "Maximum oscillation angle (degrees)",
        },
        define_field! {
            name: "phase",
            zh: "相位",
            en: "Phase",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "振荡相位偏移（度）",
            desc_en: "Oscillation phase offset (degrees)",
        },
        define_field! {
            name: "type",
            zh: "类型",
            en: "Type",
            type: FieldType::Int,
            support: Full,
            default: "0",
            desc_zh: "振荡波形类型（0=正弦，1=三角）",
            desc_en: "Oscillation waveform type (0=sine, 1=triangle)",
        },
    ],
}

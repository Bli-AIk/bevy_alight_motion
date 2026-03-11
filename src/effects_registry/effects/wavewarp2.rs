use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.wavewarp2",
    short_name: "wavewarp2",
    zh: "波浪歪曲 (Wave Warp)",
    en: "Wave Warp",
    desc_zh: "基于余弦波形在UV空间中歪曲图层，可用于模拟旗帜飘动、水面波纹等效果。",
    desc_en: "Distorts the layer based on a cosine wave pattern in UV space. Can simulate flag waving, water ripple effects, etc.",
    support: Full,
    xml: r##"<effect id="com.alightcreative.effects.wavewarp2">
    <property name="phase" type="float" value="0.0" />
    <property name="a1d" type="float" value="0.0" />
    <property name="m1" type="float" value="20.0" />
    <property name="m2" type="float" value="4.0" />
    <property name="a2d" type="float" value="90.0" />
    <property name="damping" type="float" value="0.0" />
    <property name="dampingSpace" type="float" value="0.0" />
    <property name="dampingOrigin" type="float" value="0.5" />
    <property name="screenSpace" type="bool" value="false" />
</effect>"##,
    tests: [
        "effects/wavewarp/basic",
        "effects/wavewarp/rotation",
        "effects/wavewarp/phase_warp",
        "effects/wavewarp/damping",
    ],
    fields: [
        define_field! {
            name: "phase",
            zh: "相位",
            en: "Phase",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "波浪的相位偏移（每个整数相位等于完整波形周期）",
            desc_en: "Phase offset of the wave (each integer = one full wave cycle)",
        },
        define_field! {
            name: "a1d",
            zh: "波浪方向角度",
            en: "Direction Angle",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "波浪传播方向的角度（度）",
            desc_en: "Angle of wave propagation direction (degrees)",
        },
        define_field! {
            name: "m1",
            zh: "间距",
            en: "Spacing",
            type: FieldType::Float,
            support: Full,
            default: "20.0",
            desc_zh: "波浪的间距/频率，值越大波浪越密",
            desc_en: "Wave spacing/frequency. Higher values = more waves",
        },
        define_field! {
            name: "m2",
            zh: "幅度",
            en: "Magnitude",
            type: FieldType::Float,
            support: Full,
            default: "4.0",
            desc_zh: "波浪位移的幅度（百分比）",
            desc_en: "Wave displacement magnitude (percentage)",
        },
        define_field! {
            name: "a2d",
            zh: "翘曲角度",
            en: "Warp Angle",
            type: FieldType::Float,
            support: Full,
            default: "90.0",
            desc_zh: "位移方向相对于波浪方向的角度偏移（度）",
            desc_en: "Displacement direction offset relative to wave direction (degrees)",
        },
        define_field! {
            name: "damping",
            zh: "阻尼幅度",
            en: "Damping",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "基于位置的幅度衰减 [-1, 1]",
            desc_en: "Position-based magnitude damping [-1, 1]",
        },
        define_field! {
            name: "dampingSpace",
            zh: "阻尼间距",
            en: "Damping Space",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "基于位置的间距衰减 [-1, 1]",
            desc_en: "Position-based spacing damping [-1, 1]",
        },
        define_field! {
            name: "dampingOrigin",
            zh: "阻尼原点",
            en: "Damping Origin",
            type: FieldType::Float,
            support: Full,
            default: "0.5",
            desc_zh: "阻尼效果的参考原点 [0, 1]",
            desc_en: "Reference origin for damping effect [0, 1]",
        },
        define_field! {
            name: "screenSpace",
            zh: "屏幕空间",
            en: "Screen Space",
            type: FieldType::Bool,
            support: Full,
            default: "false",
            desc_zh: "是否使用屏幕空间坐标（而非图层空间）",
            desc_en: "Whether to use screen-space coordinates instead of layer-space",
        },
    ],
}

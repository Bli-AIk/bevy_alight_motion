//! Registers metadata for the built-in RGB Split effect.
//! 注册内置 RGB Split 效果的元数据。
//!
//! RGB Split is a common chromatic-aberration effect with several perceptual modes. The registry
//! entry documents those controls so generated docs can explain the intended rendering behavior.
//! RGB Split 是常见的色差效果，并带有多种感知模式。这个注册表条目把这些控制项明确定义出来，
//! 让生成的文档能够解释它预期的渲染行为。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.rgbsep",
    short_name: "rgb_split",
    zh: "RGB 分离 (RGB Split)",
    en: "RGB Split",
    desc_zh: "将 RGB 通道沿指定方向分离，产生色差效果。",
    desc_en: "Separates RGB channels along a direction to create chromatic aberration.",
    support: Full,
    xml: r##"<effect id="com.alightcreative.effects.rgbsep">
    <property name="strength" type="float" value="0.15" />
    <property name="angle" type="float" value="0.0" />
    <property name="centerChannel" type="int" value="1" />
    <property name="mode" type="int" value="2" />
</effect>"##,
    tests: [
        "effects/rgb-split/light-black/test.amproj",
        "effects/rgb-split/light-white/test.amproj",
        "effects/rgb-split/light-gray/test.amproj",
        "effects/rgb-split/dark-black/test.amproj",
        "effects/rgb-split/dark-white/test.amproj",
        "effects/rgb-split/dark-gray/test.amproj",
        "effects/rgb-split/mask-black/test.amproj",
        "effects/rgb-split/mask-white/test.amproj",
        "effects/rgb-split/mask-gray/test.amproj",
        "effects/rgb-split/lum-black/test.amproj",
        "effects/rgb-split/lum-white/test.amproj",
        "effects/rgb-split/lum-gray/test.amproj",
    ],
    fields: [
        define_field! {
            name: "strength",
            zh: "强度",
            en: "Strength",
            type: FieldType::Float,
            support: Full,
            default: "0.15",
            desc_zh: "通道偏移强度",
            desc_en: "Channel offset magnitude",
        },
        define_field! {
            name: "angle",
            zh: "角度",
            en: "Angle",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "分离方向角度（度）",
            desc_en: "Separation direction angle (degrees)",
        },
        define_field! {
            name: "centerChannel",
            zh: "中心通道",
            en: "Center Channel",
            type: FieldType::Int,
            support: Full,
            default: "1",
            desc_zh: "保持在中心的通道 (0=R, 1=G, 2=B)",
            desc_en: "Channel that stays centered (0=R, 1=G, 2=B)",
        },
        define_field! {
            name: "mode",
            zh: "模式",
            en: "Mode",
            type: FieldType::Int,
            support: Full,
            default: "2",
            desc_zh: "混合模式 (0=遮罩, 1=亮度, 2=明, 3=暗)",
            desc_en: "Blending mode (0=Mask, 1=Luma, 2=Light, 3=Dark)",
        },
    ],
}

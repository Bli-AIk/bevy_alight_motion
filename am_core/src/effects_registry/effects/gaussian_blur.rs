//! # gaussian_blur.rs
//!
//! # GaussianBlur 效果定义
//!
//! GaussianBlur effect definition - image blur effect.
//! GaussianBlur 效果定义 - 图像模糊效果。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.gaussianblur",
    short_name: "gaussian-blur",
    zh: "高斯模糊 (Gaussian Blur)",
    en: "Gaussian Blur",
    desc_zh: "使用多 pass 模糊实现平滑的高斯模糊效果，支持超出原始边界的发光扩散。",
    desc_en: "Multi-pass blur implementation for smooth Gaussian blur effect, supports glow expansion beyond original boundaries.",
    support: Partial,
    xml: r#"<effect id="com.alightcreative.effects.gaussianblur">
    <property name="strength" type="float" value="0.0" />
</effect>"#,
    tests: ["effects/gaussian-blur/basic.amproj"],
    fields: [
        define_field! {
            name: "strength",
            zh: "模糊强度",
            en: "Blur Strength",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "模糊强度像素值",
            desc_en: "Blur strength in pixels",
        },
    ],
}

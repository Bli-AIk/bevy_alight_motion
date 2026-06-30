//! # fill.rs
//!
//! # 填充定义
//!
//! Fill builtin definitions (color, media).
//! 填充内置功能定义（颜色填充、媒体填充）。

use crate::define_builtin;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_builtin! {
    id: "fill.color",
    short_name: "color-fill",
    category: Fill,
    zh: "颜色填充",
    en: "Color Fill",
    desc_zh: "使用纯色填充形状。颜色格式为 #AARRGGBB。",
    desc_en: "Fills the shape with a solid color. Color format is #AARRGGBB.",
    support: Full,
    xml: r##"<shape fillType="color">
    <fillColor value="#ffff0000" />
</shape>"##,
    tests: ["basic/shape/shape.amproj"],
    fields: [
        define_field! {
            name: "fillColor",
            zh: "填充颜色",
            en: "Fill Color",
            type: FieldType::Color,
            support: Full,
            default: "#ffffffff",
            desc_zh: "填充颜色值 (#AARRGGBB 格式)",
            desc_en: "Fill color value (#AARRGGBB format)",
        },
    ],
}

pub mod media {
    use crate::define_builtin;
    use crate::define_field;
    use crate::effects_registry::types::FieldType;

    define_builtin! {
        id: "fill.media",
        short_name: "media-fill",
        category: Fill,
        zh: "媒体填充",
        en: "Media Fill",
        desc_zh: "使用图像纹理填充形状。支持 JPEG 和 PNG 格式。",
        desc_en: "Fills the shape with an image texture. Supports JPEG and PNG formats.",
        support: Full,
        xml: r#"<shape fillType="media" fillImage="amproj:image.png">
    <property name="size" type="vec2" value="100.0,100.0" />
</shape>"#,
        tests: ["basic/shape/shape.amproj"],
        fields: [
            define_field! {
                name: "fillImage",
                zh: "填充图像",
                en: "Fill Image",
                type: FieldType::String,
                support: Full,
                desc_zh: "图像资源 URI (amproj:filename.png)",
                desc_en: "Image resource URI (amproj:filename.png)",
            },
        ],
    }
}

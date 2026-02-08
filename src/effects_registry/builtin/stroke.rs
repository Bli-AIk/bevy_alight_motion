//! # stroke.rs
//!
//! # 描边定义
//!
//! Stroke builtin definitions.
//! 描边内置功能定义。

use crate::define_builtin;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_builtin! {
    id: "stroke",
    short_name: "stroke",
    category: Properties,
    zh: "描边",
    en: "Stroke",
    desc_zh: "形状边框描边。使用 SDF 渲染，描边宽度在缩放动画中保持不变。",
    desc_en: "Shape border stroke. Uses SDF rendering, stroke width stays constant during scale animation.",
    support: Full,
    xml: r##"<shape s=".rect">
    <path-stroke direction="centered" cap="round" join="round">
        <color value="#ff000000" />
        <size value="2.0" />
    </path-stroke>
</shape>"##,
    tests: ["basic_shape.amproj", "basic_shape_ex.amproj"],
    fields: [
        define_field! {
            name: "direction",
            zh: "方向",
            en: "Direction",
            type: FieldType::Enum(&["centered", "inside", "outside"]),
            support: Full,
            default: "centered",
            desc_zh: "描边方向（居中、内部、外部）",
            desc_en: "Stroke direction (centered, inside, outside)",
        },
        define_field! {
            name: "cap",
            zh: "端点样式",
            en: "Cap Style",
            type: FieldType::Enum(&["square", "round", "butt"]),
            support: Full,
            default: "round",
            desc_zh: "线条端点样式",
            desc_en: "Line cap style",
        },
        define_field! {
            name: "join",
            zh: "连接样式",
            en: "Join Style",
            type: FieldType::Enum(&["miter", "round", "bevel"]),
            support: Full,
            default: "round",
            desc_zh: "线条连接样式（斜接、圆角、斜切）",
            desc_en: "Line join style (miter, round, bevel)",
        },
        define_field! {
            name: "color",
            zh: "颜色",
            en: "Color",
            type: FieldType::Color,
            support: Full,
            default: "#ff000000",
            desc_zh: "描边颜色",
            desc_en: "Stroke color",
        },
        define_field! {
            name: "size",
            zh: "宽度",
            en: "Width",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "描边宽度（像素）",
            desc_en: "Stroke width (pixels)",
        },
    ],
}

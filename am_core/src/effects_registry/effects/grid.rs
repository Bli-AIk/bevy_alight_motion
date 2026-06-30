//! # grid.rs
//!
//! # Grid 效果定义
//!
//! Grid effect definition - creates a grid pattern overlay or punchout.
//! Grid 效果定义 - 创建网格图案叠加或挖空。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.grid2",
    short_name: "grid",
    zh: "网格 (Grid)",
    en: "Grid",
    desc_zh: "在图层上叠加网格图案或将其挖空。",
    desc_en: "Overlays a grid pattern on the layer or punches it out.",
    support: Full,
    xml: r##"<effect id="com.alightcreative.effects.grid2">
    <property name="position" type="vec2" value="0.0,0.0" />
    <property name="spacing" type="float" value="0.1" />
    <property name="width" type="float" value="0.01" />
    <property name="color" type="color" value="#ff000000" />
    <property name="punchout" type="bool" value="false" />
    <property name="smoothing" type="float" value="0.05" />
    <property name="screenSpace" type="bool" value="false" />
</effect>"##,
    tests: ["effects/grid/basic.amproj"],
    fields: [
        define_field! {
            name: "position",
            zh: "位置",
            en: "Position",
            type: FieldType::Vec2,
            support: Full,
            default: "0.0,0.0",
            desc_zh: "网格偏移位置",
            desc_en: "Grid offset position",
        },
        define_field! {
            name: "spacing",
            zh: "间距",
            en: "Spacing",
            type: FieldType::Float,
            support: Full,
            default: "0.1",
            desc_zh: "网格线之间的间距",
            desc_en: "Space between grid lines",
        },
        define_field! {
            name: "width",
            zh: "宽度",
            en: "Width",
            type: FieldType::Float,
            support: Full,
            default: "0.01",
            desc_zh: "网格线宽度",
            desc_en: "Grid line width",
        },
        define_field! {
            name: "color",
            zh: "颜色",
            en: "Color",
            type: FieldType::Color,
            support: Full,
            default: "#ff000000",
            desc_zh: "网格线颜色",
            desc_en: "Grid line color",
        },
        define_field! {
            name: "punchout",
            zh: "挖空",
            en: "Punch Out",
            type: FieldType::Bool,
            support: Full,
            default: "false",
            desc_zh: "是否从图像中挖空网格线",
            desc_en: "Whether to punch out grid lines from image",
        },
        define_field! {
            name: "smoothing",
            zh: "平滑",
            en: "Smoothing",
            type: FieldType::Float,
            support: Full,
            default: "0.05",
            desc_zh: "边缘平滑度",
            desc_en: "Edge smoothing",
        },
        define_field! {
            name: "screenSpace",
            zh: "屏幕空间",
            en: "Screen Space",
            type: FieldType::Bool,
            support: Full,
            default: "false",
            desc_zh: "使用屏幕坐标而非图层坐标",
            desc_en: "Use screen coordinates instead of layer coordinates",
        },
    ],
}

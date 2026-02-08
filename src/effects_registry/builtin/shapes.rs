//! # shapes.rs
//!
//! # 形状定义
//!
//! Shape builtin definitions (rect, circle).
//! 形状内置功能定义（矩形、圆形）。

use crate::define_builtin;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_builtin! {
    id: "shape.rect",
    short_name: "rect",
    category: Shapes,
    zh: "矩形",
    en: "Rectangle",
    desc_zh: "基础矩形形状，支持 SDF 渲染和精灵渲染。",
    desc_en: "Basic rectangle shape, supports SDF and sprite rendering.",
    support: Full,
    xml: r##"<shape id="1" label="矩形 1" startTime="0" endTime="1000" fillType="color" s=".rect">
    <transform>
        <location value="640.0,480.0,0.0" />
        <rotation value="0.0" />
        <scale value="1.0,1.0" />
        <opacity value="1.0" />
    </transform>
    <property name="size" type="vec2" value="100.0,100.0" />
    <fillColor value="#ffff0000" />
</shape>"##,
    tests: ["basic/shape/shape.amproj", "basic/shape/ex.amproj"],
    fields: [
        define_field! {
            name: "size",
            zh: "尺寸",
            en: "Size",
            type: FieldType::Vec2,
            support: Full,
            default: "100.0,100.0",
            desc_zh: "形状的宽度和高度",
            desc_en: "Width and height of the shape",
        },
    ],
}

pub mod circle {
    use crate::define_builtin;
    use crate::define_field;
    use crate::effects_registry::types::FieldType;

    define_builtin! {
        id: "shape.circle",
        short_name: "circle",
        category: Shapes,
        zh: "圆形",
        en: "Circle",
        desc_zh: "基础圆形形状，使用 SDF 渲染。支持非均匀缩放形成椭圆。",
        desc_en: "Basic circle shape using SDF rendering. Supports non-uniform scaling for ellipses.",
        support: Full,
        xml: r##"<shape id="1" label="圆形 1" startTime="0" endTime="1000" fillType="color" s=".circle">
    <transform>
        <location value="640.0,480.0,0.0" />
        <rotation value="0.0" />
        <scale value="1.0,1.0" />
        <opacity value="1.0" />
    </transform>
    <property name="size" type="vec2" value="100.0,100.0" />
    <fillColor value="#ff00ff00" />
</shape>"##,
        tests: ["basic/shape/shape.amproj"],
        fields: [
            define_field! {
                name: "size",
                zh: "尺寸",
                en: "Size",
                type: FieldType::Vec2,
                support: Full,
                default: "100.0,100.0",
                desc_zh: "圆形的宽度和高度（非均匀值形成椭圆）",
                desc_en: "Width and height of the circle (non-uniform values create ellipse)",
            },
        ],
    }
}

//! # shapes.rs
//!
//! # 形状定义
//!
//! Shape builtin definitions.
//! 形状内置功能定义。

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

pub mod roundrect {
    use crate::define_builtin;
    use crate::define_field;
    use crate::effects_registry::types::FieldType;

    define_builtin! {
        id: "shape.roundrect",
        short_name: "roundrect",
        category: Shapes,
        zh: "圆角矩形",
        en: "Rounded Rectangle",
        desc_zh: "圆角矩形形状，使用 SDF 渲染。",
        desc_en: "Rounded rectangle shape using SDF rendering.",
        support: Full,
        xml: r##"<shape s=".roundrect">
    <property name="size" type="vec2" value="200.0,100.0" />
    <property name="roundness" type="float" value="0.3" />
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
                desc_zh: "形状的宽度和高度",
                desc_en: "Width and height of the shape",
            },
            define_field! {
                name: "roundness",
                zh: "圆角度",
                en: "Roundness",
                type: FieldType::Float,
                support: Full,
                default: "0.3",
                desc_zh: "圆角程度 (0.0-1.0)",
                desc_en: "Corner roundness (0.0-1.0)",
            },
        ],
    }
}

pub mod triangle {
    use crate::define_builtin;
    use crate::define_field;
    use crate::effects_registry::types::FieldType;

    define_builtin! {
        id: "shape.triangle",
        short_name: "triangle",
        category: Shapes,
        zh: "三角形",
        en: "Triangle",
        desc_zh: "三角形形状，使用 SDF 渲染。",
        desc_en: "Triangle shape using SDF rendering.",
        support: Full,
        xml: r##"<shape s=".triangle">
    <property name="size" type="vec2" value="100.0,100.0" />
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
                desc_zh: "三角形的宽度和高度",
                desc_en: "Width and height of the triangle",
            },
        ],
    }
}

pub mod star {
    use crate::define_builtin;
    use crate::define_field;
    use crate::effects_registry::types::FieldType;

    define_builtin! {
        id: "shape.star",
        short_name: "star",
        category: Shapes,
        zh: "星形",
        en: "Star",
        desc_zh: "星形形状，使用 SDF 渲染。",
        desc_en: "Star shape using SDF rendering.",
        support: Full,
        xml: r##"<shape s=".star">
    <property name="size" type="vec2" value="100.0,100.0" />
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
                desc_zh: "星形的宽度和高度",
                desc_en: "Width and height of the star",
            },
        ],
    }
}

pub mod poly {
    use crate::define_builtin;
    use crate::define_field;
    use crate::effects_registry::types::FieldType;

    define_builtin! {
        id: "shape.poly",
        short_name: "poly",
        category: Shapes,
        zh: "多边形",
        en: "Polygon",
        desc_zh: "正多边形（六边形）形状，使用 SDF 渲染。",
        desc_en: "Regular polygon (hexagon) shape using SDF rendering.",
        support: Full,
        xml: r##"<shape s=".poly">
    <property name="size" type="vec2" value="100.0,100.0" />
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
                desc_zh: "多边形的宽度和高度",
                desc_en: "Width and height of the polygon",
            },
        ],
    }
}

pub mod quad {
    use crate::define_builtin;
    use crate::define_field;
    use crate::effects_registry::types::FieldType;

    define_builtin! {
        id: "shape.quad",
        short_name: "quad",
        category: Shapes,
        zh: "菱形",
        en: "Quad",
        desc_zh: "菱形（四边形）形状，使用 SDF 渲染。",
        desc_en: "Diamond (quadrilateral) shape using SDF rendering.",
        support: Full,
        xml: r##"<shape s=".quad">
    <property name="size" type="vec2" value="100.0,100.0" />
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
                desc_zh: "菱形的宽度和高度",
                desc_en: "Width and height of the quad",
            },
        ],
    }
}

pub mod penta {
    use crate::define_builtin;
    use crate::define_field;
    use crate::effects_registry::types::FieldType;

    define_builtin! {
        id: "shape.penta",
        short_name: "penta",
        category: Shapes,
        zh: "五边形",
        en: "Pentagon",
        desc_zh: "正五边形形状，使用 SDF 渲染。",
        desc_en: "Regular pentagon shape using SDF rendering.",
        support: Full,
        xml: r##"<shape s=".penta">
    <property name="size" type="vec2" value="100.0,100.0" />
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
                desc_zh: "五边形的宽度和高度",
                desc_en: "Width and height of the pentagon",
            },
        ],
    }
}

pub mod pie {
    use crate::define_builtin;
    use crate::define_field;
    use crate::effects_registry::types::FieldType;

    define_builtin! {
        id: "shape.pie",
        short_name: "pie",
        category: Shapes,
        zh: "扇形",
        en: "Pie",
        desc_zh: "扇形形状，使用 SDF 渲染。",
        desc_en: "Pie/sector shape using SDF rendering.",
        support: Full,
        xml: r##"<shape s=".pie">
    <property name="size" type="vec2" value="100.0,100.0" />
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
                desc_zh: "扇形的宽度和高度",
                desc_en: "Width and height of the pie",
            },
        ],
    }
}

pub mod plus {
    use crate::define_builtin;
    use crate::define_field;
    use crate::effects_registry::types::FieldType;

    define_builtin! {
        id: "shape.plus",
        short_name: "plus",
        category: Shapes,
        zh: "十字形",
        en: "Plus",
        desc_zh: "十字形（加号）形状，使用 SDF 渲染。",
        desc_en: "Plus/cross shape using SDF rendering.",
        support: Full,
        xml: r##"<shape s=".plus">
    <property name="size" type="vec2" value="100.0,100.0" />
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
                desc_zh: "十字形的宽度和高度",
                desc_en: "Width and height of the plus",
            },
        ],
    }
}

pub mod multifoil {
    use crate::define_builtin;
    use crate::define_field;
    use crate::effects_registry::types::FieldType;

    define_builtin! {
        id: "shape.multifoil",
        short_name: "multifoil",
        category: Shapes,
        zh: "多叶形",
        en: "Multifoil",
        desc_zh: "多叶形状，使用 SDF 渲染。",
        desc_en: "Multifoil shape using SDF rendering.",
        support: Full,
        xml: r##"<shape s=".multifoil">
    <property name="size" type="vec2" value="100.0,100.0" />
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
                desc_zh: "多叶形的宽度和高度",
                desc_en: "Width and height of the multifoil",
            },
        ],
    }
}

pub mod arc {
    use crate::define_builtin;
    use crate::define_field;
    use crate::effects_registry::types::FieldType;

    define_builtin! {
        id: "shape.arc",
        short_name: "arc",
        category: Shapes,
        zh: "弧形",
        en: "Arc",
        desc_zh: "弧形形状，使用 SDF 渲染。",
        desc_en: "Arc shape using SDF rendering.",
        support: Full,
        xml: r##"<shape s=".arc">
    <property name="size" type="vec2" value="100.0,100.0" />
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
                desc_zh: "弧形的宽度和高度",
                desc_en: "Width and height of the arc",
            },
        ],
    }
}

pub mod line {
    use crate::define_builtin;
    use crate::define_field;
    use crate::effects_registry::types::FieldType;

    define_builtin! {
        id: "shape.line",
        short_name: "line",
        category: Shapes,
        zh: "线段",
        en: "Line",
        desc_zh: "线段形状，使用 SDF 渲染。",
        desc_en: "Line shape using SDF rendering.",
        support: Full,
        xml: r##"<shape s=".line">
    <property name="size" type="vec2" value="100.0,0.0" />
</shape>"##,
        tests: ["basic/shape/shape.amproj"],
        fields: [
            define_field! {
                name: "size",
                zh: "尺寸",
                en: "Size",
                type: FieldType::Vec2,
                support: Full,
                default: "100.0,0.0",
                desc_zh: "线段的长度和宽度",
                desc_en: "Length and width of the line",
            },
        ],
    }
}

pub mod ngon {
    use crate::define_builtin;
    use crate::define_field;
    use crate::effects_registry::types::FieldType;

    define_builtin! {
        id: "shape.ngon",
        short_name: "ngon",
        category: Shapes,
        zh: "正 N 边形",
        en: "N-gon",
        desc_zh: "任意正 N 边形，使用数字（如 s=\"12\"、s=\"30\"）指定边数。使用 SDF 渲染。",
        desc_en: "Regular N-sided polygon, specified with numeric sides (e.g. s=\"12\", s=\"30\"). Uses SDF rendering.",
        support: Full,
        xml: r##"<shape s="12">
    <property name="size" type="vec2" value="100.0,100.0" />
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
                desc_zh: "形状的宽度和高度",
                desc_en: "Width and height of the shape",
            },
        ],
    }
}

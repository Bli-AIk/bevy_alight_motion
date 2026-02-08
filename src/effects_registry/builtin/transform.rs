//! # transform.rs
//!
//! # 变换属性定义
//!
//! Transform property definitions.
//! 变换属性定义。

use crate::define_builtin;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_builtin! {
    id: "transform",
    short_name: "transform",
    zh: "变换",
    en: "Transform",
    desc_zh: "图层的基础变换属性，包括位置、旋转、缩放、透明度和锚点。坐标系统：AM 使用左上角原点，Bevy 使用中心原点，库自动转换。",
    desc_en: "Basic transform properties for layers including position, rotation, scale, opacity and anchor. Coordinate system: AM uses top-left origin, Bevy uses center origin, the library converts automatically.",
    support: Full,
    xml: r#"<transform lockAspectRatio="false">
    <location value="640.0,480.0,0.0" />
    <pivot value="0.0,0.0" />
    <rotation value="45.0" />
    <scale value="1.5,1.5" />
    <opacity value="0.8" />
</transform>"#,
    tests: ["basic_shape.amproj", "basic_pivot.amproj"],
    fields: [
        define_field! {
            name: "location",
            zh: "位置",
            en: "Location",
            type: FieldType::Vec3,
            support: Full,
            default: "0.0,0.0,0.0",
            desc_zh: "图层位置 (x, y, z)",
            desc_en: "Layer position (x, y, z)",
        },
        define_field! {
            name: "rotation",
            zh: "旋转",
            en: "Rotation",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "Z 轴旋转角度（度）",
            desc_en: "Z-axis rotation angle (degrees)",
        },
        define_field! {
            name: "scale",
            zh: "缩放",
            en: "Scale",
            type: FieldType::Vec2,
            support: Full,
            default: "1.0,1.0",
            desc_zh: "缩放比例 (x, y)",
            desc_en: "Scale factor (x, y)",
        },
        define_field! {
            name: "opacity",
            zh: "透明度",
            en: "Opacity",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "透明度 (0.0-1.0)",
            desc_en: "Opacity (0.0-1.0)",
        },
        define_field! {
            name: "pivot",
            zh: "锚点",
            en: "Pivot",
            type: FieldType::Vec2,
            support: Full,
            default: "0.0,0.0",
            desc_zh: "旋转和缩放的锚点位置",
            desc_en: "Anchor point for rotation and scaling",
        },
        define_field! {
            name: "lockAspectRatio",
            zh: "锁定宽高比",
            en: "Lock Aspect Ratio",
            type: FieldType::Bool,
            support: Full,
            default: "false",
            desc_zh: "是否锁定宽高比",
            desc_en: "Whether to lock aspect ratio",
        },
    ],
}

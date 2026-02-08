//! # stretch_segment.rs
//!
//! # StretchSegment 效果定义
//!
//! StretchSegment effect definition - UV domain distortion.
//! StretchSegment 效果定义 - UV 域变形效果。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.stretchsegment",
    short_name: "stretch-segment",
    zh: "拉伸片段 (Stretch Segment)",
    en: "Stretch Segment",
    desc_zh: "UV 域变形效果，沿分割线拉伸图像。拉伸公式: new_width = orig_width * (1.0 + stretch_px / (orig_width / 5.76))",
    desc_en: "UV domain distortion effect that stretches the image along a dividing line. Formula: new_width = orig_width * (1.0 + stretch_px / (orig_width / 5.76))",
    support: Partial,
    xml: r#"<effect id="com.alightcreative.effects.stretchsegment">
    <property name="stretch" type="float" value="0.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="offset" type="float" value="0.0" />
    <property name="smooth" type="float" value="0.0" />
</effect>"#,
    tests: [
        "effects/stretch-segment/basic.amproj",
        "effects/stretch-segment/ex.amproj",
        "effects/stretch-segment/ex2.amproj",
        "effects/stretch-segment/ex3.amproj",
        "effects/stretch-segment/ex4.amproj",
        "effects/stretch-segment/ex5.amproj",
    ],
    fields: [
        define_field! {
            name: "stretch",
            zh: "拉伸",
            en: "Stretch",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "拉伸量（像素）",
            desc_en: "Stretch amount (pixels)",
        },
        define_field! {
            name: "angle",
            zh: "角度",
            en: "Angle",
            type: FieldType::Float,
            support: Partial,
            default: "0.0",
            desc_zh: "分割线角度（基本支持，存在轻微视觉差异）",
            desc_en: "Dividing line angle (basic support, minor visual differences)",
        },
        define_field! {
            name: "offset",
            zh: "偏移",
            en: "Offset",
            type: FieldType::Float,
            support: Partial,
            default: "0.0",
            desc_zh: "分割线位置偏移（基本支持，存在轻微视觉差异）",
            desc_en: "Dividing line position offset (basic support, minor visual differences)",
        },
        define_field! {
            name: "smooth",
            zh: "平滑",
            en: "Smooth",
            type: FieldType::Float,
            support: Unsupported,
            default: "0.0",
            desc_zh: "边缘平滑度（尚未实现）",
            desc_en: "Edge smoothness (not yet implemented)",
        },
    ],
}

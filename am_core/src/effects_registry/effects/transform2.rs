//! # transform2.rs
//!
//! # Transform2 效果定义
//!
//! Transform2 effect definition - additional transform controls.
//! Transform2 效果定义 - 额外的变换控制。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.transform2",
    short_name: "transform2",
    zh: "变换 (Transform2)",
    en: "Transform2",
    desc_zh: "可以复制类似变换、角度、缩放的设置，提供额外的位移控制。",
    desc_en: "Provides additional transform controls similar to the base transform properties.",
    support: Full,
    xml: r##"<effect id="com.alightcreative.effects.transform2" locallyApplied="true">
    <property name="posx" type="float" value="0.0" />
    <property name="posy" type="float" value="0.0" />
    <property name="posz" type="float" value="1.0" />
    <property name="angle" type="float" value="0.0" />
    <property name="xinv" type="bool" value="false" />
    <property name="yinv" type="bool" value="false" />
    <property name="zinv" type="bool" value="false" />
    <property name="ainv" type="bool" value="false" />
</effect>"##,
    tests: [
        "effects/transform/complex1.amproj",
        "effects/transform/complex2.amproj",
    ],
    fields: [
        define_field! {
            name: "posx",
            zh: "X 偏移",
            en: "X Offset",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "额外的水平位移",
            desc_en: "Additional horizontal offset",
        },
        define_field! {
            name: "posy",
            zh: "Y 偏移",
            en: "Y Offset",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "额外的垂直位移",
            desc_en: "Additional vertical offset",
        },
        define_field! {
            name: "posz",
            zh: "Z 偏移",
            en: "Z Offset",
            type: FieldType::Float,
            support: Full,
            default: "1.0",
            desc_zh: "缩放倍数（Z 轴位移模拟）",
            desc_en: "Scale multiplier (Z axis offset simulation)",
        },
        define_field! {
            name: "angle",
            zh: "角度",
            en: "Angle",
            type: FieldType::Float,
            support: Full,
            default: "0.0",
            desc_zh: "额外的旋转角度（度）",
            desc_en: "Additional rotation angle (degrees)",
        },
        define_field! {
            name: "xinv",
            zh: "X 反转",
            en: "X Invert",
            type: FieldType::Bool,
            support: Unsupported,
            default: "false",
            desc_zh: "水平翻转",
            desc_en: "Horizontal flip",
        },
        define_field! {
            name: "yinv",
            zh: "Y 反转",
            en: "Y Invert",
            type: FieldType::Bool,
            support: Unsupported,
            default: "false",
            desc_zh: "垂直翻转",
            desc_en: "Vertical flip",
        },
        define_field! {
            name: "zinv",
            zh: "Z 反转",
            en: "Z Invert",
            type: FieldType::Bool,
            support: Unsupported,
            default: "false",
            desc_zh: "缩放反转",
            desc_en: "Scale inversion",
        },
        define_field! {
            name: "ainv",
            zh: "角度反转",
            en: "Angle Invert",
            type: FieldType::Bool,
            support: Unsupported,
            default: "false",
            desc_zh: "角度反转",
            desc_en: "Angle inversion",
        },
    ],
}

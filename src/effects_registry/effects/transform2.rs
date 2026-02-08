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
    xml: r#"<effect id="com.alightcreative.effects.transform2">
    <property name="posx" type="float" value="0.0" />
    <property name="posy" type="float" value="0.0" />
</effect>"#,
    tests: [],
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
    ],
}

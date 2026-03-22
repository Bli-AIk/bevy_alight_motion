//! # transform_v1.rs
//!
//! Original transform effect definition.
//! Older AM projects use `com.alightcreative.effects.transform` instead of `transform2`.
//! Both share the same parameters and behavior.

use crate::define_effect;

define_effect! {
    id: "com.alightcreative.effects.transform",
    short_name: "transform_v1",
    zh: "变换 (Transform v1)",
    en: "Transform (v1)",
    desc_zh: "原始变换效果 ID，与 Transform2 参数相同。",
    desc_en: "Original transform effect ID, same parameters as Transform2.",
    support: Full,
    xml: r##"<effect id="com.alightcreative.effects.transform" locallyApplied="true">
    <property name="posx" type="float" value="0.0" />
    <property name="posy" type="float" value="0.0" />
    <property name="posz" type="float" value="1.0" />
    <property name="angle" type="float" value="0.0" />
</effect>"##,
    tests: [],
    fields: [],
}

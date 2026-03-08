//! # transform_legacy.rs
//!
//! Legacy transform effect definition.
//! Older AM projects use "com.alightcreative.effects.transform" instead of transform2.
//! Both share the same parameters and behavior.

use crate::define_effect;

define_effect! {
    id: "com.alightcreative.effects.transform",
    short_name: "transform_legacy",
    zh: "变换 (Transform Legacy)",
    en: "Transform (Legacy)",
    desc_zh: "旧版变换效果，与 Transform2 参数相同。",
    desc_en: "Legacy transform effect, same parameters as Transform2.",
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

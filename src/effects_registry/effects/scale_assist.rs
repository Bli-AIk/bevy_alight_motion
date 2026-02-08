//! # scale_assist.rs
//!
//! # ScaleAssist 效果定义
//!
//! ScaleAssist effect definition - automatic scaling helper.
//! ScaleAssist 效果定义 - 自动缩放辅助效果。

use crate::define_effect;
use crate::define_field;
use crate::effects_registry::types::FieldType;

define_effect! {
    id: "com.alightcreative.effects.scaleassist",
    short_name: "scale-assist",
    zh: "缩放辅助 (Scale Assist)",
    en: "Scale Assist",
    desc_zh: "根据选择的轴向自动调整图层尺寸以适应画布。",
    desc_en: "Automatically adjusts layer size to fit the canvas based on the selected axis.",
    support: Partial,
    xml: r#"<effect id="com.alightcreative.effects.scaleassist">
    <property name="scaleassistaxis" type="float" value="1.0" />
</effect>"#,
    tests: ["fx_6_scaleassist.amproj", "fx_6_ex_scaleassist.amproj"],
    fields: [
        define_field! {
            name: "scaleassistaxis",
            zh: "轴向",
            en: "Axis",
            type: FieldType::Enum(&["1.0 (宽度/Width)", "2.0 (高度/Height)"]),
            support: Full,
            default: "1.0",
            desc_zh: "缩放基准轴 (1=宽度, 2=高度)",
            desc_en: "Scale reference axis (1=width, 2=height)",
        },
    ],
}

//! # macros.rs
//!
//! # 效果定义宏
//!
//! Macros for defining effects with less boilerplate.
//! 用于简化效果定义的宏。

/// 定义效果字段的宏 / Macro for defining effect fields
#[macro_export]
macro_rules! define_field {
    (
        name: $name:literal,
        zh: $display_zh:literal,
        en: $display_en:literal,
        type: $ftype:expr,
        support: $support:ident
        $(, default: $default:literal)?
        $(, desc_zh: $desc_zh:literal)?
        $(, desc_en: $desc_en:literal)?
        $(,)?
    ) => {
        $crate::effects_registry::types::FieldDef {
            name: $name,
            display_name_zh: $display_zh,
            display_name_en: $display_en,
            field_type: $ftype,
            support_level: $crate::effects_registry::types::SupportLevel::$support,
            default_value: define_field!(@opt $($default)?),
            description_zh: define_field!(@opt_str $($desc_zh)?, ""),
            description_en: define_field!(@opt_str $($desc_en)?, ""),
        }
    };
    (@opt) => { None };
    (@opt $val:literal) => { Some($val) };
    (@opt_str , $default:literal) => { $default };
    (@opt_str $val:literal, $default:literal) => { $val };
}

/// 定义效果的宏 / Macro for defining effects
#[macro_export]
macro_rules! define_effect {
    (
        id: $id:literal,
        short_name: $short:literal,
        zh: $name_zh:literal,
        en: $name_en:literal,
        desc_zh: $desc_zh:literal,
        desc_en: $desc_en:literal,
        support: $support:ident,
        xml: $xml:literal,
        tests: [$($test:literal),* $(,)?],
        fields: [$($field:expr),* $(,)?]
        $(,)?
    ) => {
        pub const EFFECT: $crate::effects_registry::types::EffectDef = $crate::effects_registry::types::EffectDef {
            id: $id,
            short_name: $short,
            display_name_zh: $name_zh,
            display_name_en: $name_en,
            description_zh: $desc_zh,
            description_en: $desc_en,
            support_level: $crate::effects_registry::types::SupportLevel::$support,
            fields: &[$($field),*],
            xml_example: $xml,
            test_files: &[$($test),*],
        };
    };
}

/// 定义内置功能的宏 / Macro for defining builtin features
#[macro_export]
macro_rules! define_builtin {
    (
        id: $id:literal,
        short_name: $short:literal,
        category: $category:ident,
        zh: $name_zh:literal,
        en: $name_en:literal,
        desc_zh: $desc_zh:literal,
        desc_en: $desc_en:literal,
        support: $support:ident,
        xml: $xml:literal,
        tests: [$($test:literal),* $(,)?],
        fields: [$($field:expr),* $(,)?]
        $(,)?
    ) => {
        pub const BUILTIN: $crate::effects_registry::types::BuiltinDef = $crate::effects_registry::types::BuiltinDef {
            id: $id,
            short_name: $short,
            category: $crate::effects_registry::types::BuiltinCategory::$category,
            display_name_zh: $name_zh,
            display_name_en: $name_en,
            description_zh: $desc_zh,
            description_en: $desc_en,
            support_level: $crate::effects_registry::types::SupportLevel::$support,
            fields: &[$($field),*],
            xml_example: $xml,
            test_files: &[$($test),*],
        };
    };
}

pub use define_builtin;
pub use define_effect;
pub use define_field;

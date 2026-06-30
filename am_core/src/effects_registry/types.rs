//! # types.rs
//!
//! # 效果注册表核心类型
//!
//! Core type definitions for the effects registry.
//! 效果注册表的核心类型定义。

use serde::Serialize;

/// 支持级别 / Support level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SupportLevel {
    /// 完全支持 / Fully supported
    Full,
    /// 部分支持 / Partially supported (some features may not work)
    Partial,
    /// 不支持 / Not supported at all
    Unsupported,
}

impl SupportLevel {
    /// 获取状态图标 / Get status icon
    pub fn icon(&self) -> &'static str {
        match self {
            SupportLevel::Full => "✅",
            SupportLevel::Partial => "⚠️",
            SupportLevel::Unsupported => "❌",
        }
    }

    /// 获取中文描述 / Get Chinese description
    pub fn description_zh(&self) -> &'static str {
        match self {
            SupportLevel::Full => "完全支持",
            SupportLevel::Partial => "部分支持",
            SupportLevel::Unsupported => "不支持",
        }
    }

    /// 获取英文描述 / Get English description
    pub fn description_en(&self) -> &'static str {
        match self {
            SupportLevel::Full => "Fully Supported",
            SupportLevel::Partial => "Partially Supported",
            SupportLevel::Unsupported => "Not Supported",
        }
    }
}

/// 字段类型 / Field type
#[derive(Debug, Clone, Serialize)]
pub enum FieldType {
    Float,
    Vec2,
    Vec3,
    Color,
    Int,
    Bool,
    String,
    Enum(&'static [&'static str]),
}

impl FieldType {
    /// 获取类型名称 / Get type name
    pub fn name(&self) -> &'static str {
        match self {
            FieldType::Float => "float",
            FieldType::Vec2 => "vec2",
            FieldType::Vec3 => "vec3",
            FieldType::Color => "color",
            FieldType::Int => "int",
            FieldType::Bool => "bool",
            FieldType::String => "string",
            FieldType::Enum(_) => "enum",
        }
    }
}

/// 字段定义 / Field definition
#[derive(Debug, Clone)]
pub struct FieldDef {
    /// AM 属性名 / AM property name (e.g., "posx")
    pub name: &'static str,
    /// 中文显示名 / Chinese display name (e.g., "X 偏移")
    pub display_name_zh: &'static str,
    /// 英文显示名 / English display name (e.g., "X Offset")
    pub display_name_en: &'static str,
    /// 字段类型 / Field type
    pub field_type: FieldType,
    /// 支持级别 / Support level
    pub support_level: SupportLevel,
    /// 默认值 / Default value
    pub default_value: Option<&'static str>,
    /// 中文描述 / Chinese description
    pub description_zh: &'static str,
    /// 英文描述 / English description
    pub description_en: &'static str,
}

/// 效果定义 / Effect definition
#[derive(Debug, Clone)]
pub struct EffectDef {
    /// 效果 ID / Effect ID (e.g., "com.alightcreative.effects.transform2")
    pub id: &'static str,
    /// 短名称 / Short name (e.g., "transform2")
    pub short_name: &'static str,
    /// 中文显示名 / Chinese display name (e.g., "变换")
    pub display_name_zh: &'static str,
    /// 英文显示名 / English display name (e.g., "Transform2")
    pub display_name_en: &'static str,
    /// 中文描述 / Chinese description
    pub description_zh: &'static str,
    /// 英文描述 / English description
    pub description_en: &'static str,
    /// 整体支持级别 / Overall support level
    pub support_level: SupportLevel,
    /// 字段定义列表 / Field definitions
    pub fields: &'static [FieldDef],
    /// XML 示例 / XML example
    pub xml_example: &'static str,
    /// 关联测试文件 / Associated test files
    pub test_files: &'static [&'static str],
}

/// 内置功能分类 / Builtin category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinCategory {
    /// 基本形状 / Basic shapes (rect, circle, etc.)
    Shapes,
    /// 填充 / Fills (color, media, etc.)
    Fill,
    /// 基本属性 / Basic properties (stroke, transform)
    Properties,
}

impl BuiltinCategory {
    /// 中文显示名 / Chinese display name
    pub fn display_name_zh(&self) -> &'static str {
        match self {
            BuiltinCategory::Shapes => "基本形状",
            BuiltinCategory::Fill => "填充",
            BuiltinCategory::Properties => "基本属性",
        }
    }

    /// 英文显示名 / English display name
    pub fn display_name_en(&self) -> &'static str {
        match self {
            BuiltinCategory::Shapes => "Basic Shapes",
            BuiltinCategory::Fill => "Fills",
            BuiltinCategory::Properties => "Properties",
        }
    }
}

/// 内置功能定义 / Builtin feature definition (shapes, fills, etc.)
#[derive(Debug, Clone)]
pub struct BuiltinDef {
    /// 功能 ID / Feature ID (e.g., "shape.rect")
    pub id: &'static str,
    /// 短名称 / Short name (e.g., "rect")
    pub short_name: &'static str,
    /// 分类 / Category
    pub category: BuiltinCategory,
    /// 中文显示名 / Chinese display name
    pub display_name_zh: &'static str,
    /// 英文显示名 / English display name
    pub display_name_en: &'static str,
    /// 中文描述 / Chinese description
    pub description_zh: &'static str,
    /// 英文描述 / English description
    pub description_en: &'static str,
    /// 整体支持级别 / Overall support level
    pub support_level: SupportLevel,
    /// 字段定义列表 / Field definitions
    pub fields: &'static [FieldDef],
    /// XML 示例 / XML example
    pub xml_example: &'static str,
    /// 关联测试文件 / Associated test files
    pub test_files: &'static [&'static str],
}

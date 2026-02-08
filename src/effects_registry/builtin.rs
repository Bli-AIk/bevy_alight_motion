//! # builtin.rs
//!
//! # 内置功能定义入口
//!
//! Entry point for all builtin feature definitions.
//! 所有内置功能定义的入口模块。

pub mod fill;
pub mod shapes;
pub mod stroke;
pub mod transform;

use super::types::BuiltinDef;

/// 获取所有内置功能定义 / Get all builtin definitions
pub fn all() -> &'static [&'static BuiltinDef] {
    &[
        &shapes::BUILTIN,
        &shapes::circle::BUILTIN,
        &fill::BUILTIN,
        &fill::media::BUILTIN,
        &stroke::BUILTIN,
        &transform::BUILTIN,
    ]
}

/// 按 ID 查找内置功能 / Find builtin by ID
pub fn find(id: &str) -> Option<&'static BuiltinDef> {
    all().iter().find(|b| b.id == id).copied()
}

/// 按短名称查找内置功能 / Find builtin by short name
pub fn find_by_short_name(short_name: &str) -> Option<&'static BuiltinDef> {
    all().iter().find(|b| b.short_name == short_name).copied()
}

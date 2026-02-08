//! # effects_registry.rs
//!
//! # 效果注册表模块
//!
//! Effects registry module - single source of truth for all effect definitions.
//! 效果注册表模块 - 所有效果定义的单一数据源。
//!
//! This module provides:
//! - Type definitions for effects and builtins
//! - Effect definitions (Transform2, Wipe2, etc.)
//! - Builtin definitions (shapes, fills, strokes, transforms)
//! - Test results parsing and support level computation
//! - Documentation generator
//! - Implementation scanner (auto-detect implemented fields from source code)
//!
//! 此模块提供：
//! - 效果和内置功能的类型定义
//! - 效果定义（Transform2、Wipe2 等）
//! - 内置功能定义（形状、填充、描边、变换）
//! - 测试结果解析和支持级别计算
//! - 文档生成器
//! - 实现扫描器（从源代码自动检测已实现的字段）

pub mod builtin;
pub mod doc_generator;
pub mod effects;
pub mod impl_scanner;
pub mod macros;
pub mod test_results;
pub mod types;

pub use builtin::all as all_builtins;
pub use effects::all as all_effects;
pub use effects::find as find_effect;
pub use test_results::{DEFAULT_TEST_RESULTS_PATH, TestResults};
pub use types::{BuiltinDef, EffectDef, FieldDef, FieldType, SupportLevel};

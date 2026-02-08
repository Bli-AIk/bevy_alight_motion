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
//! - Documentation generator
//!
//! 此模块提供：
//! - 效果和内置功能的类型定义
//! - 效果定义（Transform2、Wipe2 等）
//! - 内置功能定义（形状、填充、描边、变换）
//! - 文档生成器

pub mod builtin;
pub mod doc_generator;
pub mod effects;
pub mod macros;
pub mod types;

pub use builtin::all as all_builtins;
pub use effects::all as all_effects;
pub use effects::find as find_effect;
pub use types::{BuiltinDef, EffectDef, FieldDef, FieldType, SupportLevel};

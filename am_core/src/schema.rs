//! # schema.rs
//!
//! # Schema 模块
//!
//! Data structures for Alight Motion XML schema.
//! 用于 Alight Motion XML 格式的数据结构。

mod easing;
mod parsing;
mod types;

pub use easing::Easing;
pub use parsing::{parse_color, parse_vec2, parse_vec3};
pub use types::*;

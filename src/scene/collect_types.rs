//! # collect_types.rs
//!
//! # 类型收集模块
//!
//! Functions for collecting specific layer types (null, text).
//! 特定图层类型（空对象、文字）的收集函数。

mod null;
mod text;

pub(crate) use null::collect_null;
pub(crate) use text::collect_text;

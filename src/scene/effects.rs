//! # effects
//!
//! # 效果参数提取模块
//!
//! Effect parameter extraction from AM layers.
//! AM 图层的效果参数提取。

mod common;
mod extended;
mod other;
mod repeat;

pub use common::*;
pub use extended::*;
pub use other::*;
pub use repeat::*;

use crate::schema::{AmAnimatedFloat, AmProperty};

/// Helper: extract a float property (value + keyframes) into an AmAnimatedFloat.
pub(crate) fn extract_float_prop(prop: &AmProperty, target: &mut AmAnimatedFloat) {
    if !prop.keyframes.is_empty() {
        target.keyframes = prop.keyframes.clone();
    } else if let Ok(v) = prop.value.parse::<f32>() {
        target.value = Some(v);
    }
}

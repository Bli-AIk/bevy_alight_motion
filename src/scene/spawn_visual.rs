//! # spawn_visual.rs
//!
//! # 视觉元素生成模块
//!
//! Entity spawning functions for visual AM layers (image, text).
//! 视觉 AM 图层（图片、文字）的实体生成函数。

mod image;
mod text;

pub(crate) use image::spawn_image;
pub(crate) use text::spawn_text;

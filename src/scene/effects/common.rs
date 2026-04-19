//! Re-exports the common effect extractors used during scene collection.
//!
//! 重导出 scene 收集阶段使用的通用效果提取器。
//!
//! Layer collection needs one place to pull effect parsers that are shared across shapes, media,
//! text, and nested content. That boundary module lives here: it groups the transform,
//! compositing, and filter extractors that translate raw `AmEffect` values into runtime-friendly
//! parameter structs.
//!
//! 图层收集阶段需要一个统一入口来拿到跨 shape、媒体、文本和嵌套内容共用的效果解析器。
//! 就是那个边界模块：它把 transform、compositing 和 filter 相关的提取器组织在一起，
//! 把原始 `AmEffect` 转换成更适合运行时使用的参数结构。

mod compositing;
mod filters;
mod shared;
mod transform;

pub use compositing::*;
pub use filters::*;
pub use transform::*;

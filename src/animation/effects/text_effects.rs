//! This file is the aggregation point for text-specific animation systems.
//! It groups counter, progress, right-to-left alignment repair, and spacing
//! adjustments so the plugin can register text effects as one coherent feature set.
//!
//! 这个文件是文本类动画系统的聚合入口。它把计数器、进度揭示、RTL 对齐修正和
//! 字距调整收拢到一起，让插件可以把文本特效作为一组完整能力来注册。

mod counter;
mod progress;
mod rtl;
mod spacing;

pub use counter::animate_counter_system;
pub use progress::animate_text_progress_system;
pub use rtl::fix_rtl_line_alignment_system;
pub use spacing::animate_text_spacing_system;

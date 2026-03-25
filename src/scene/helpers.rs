//! Acts as the aggregation point for scene-collection helper utilities.
//! It re-exports coordinate conversion, fill extraction, shape sizing, and
//! property parsing helpers so collectors and spawners can use one consistent
//! helper surface instead of importing many narrow modules directly.
//!
//! 场景收集辅助工具的聚合入口。它重导出坐标转换、fill 提取、形状尺寸计算
//! 和属性解析等帮助函数，让收集器和生成器可以依赖统一的 helper 表面，而不必到处直连
//! 多个细碎模块。

mod fill;
mod shape_properties;
mod shape_size;
mod transforms;

pub use transforms::am_to_bevy_coords;

pub(crate) use fill::*;
pub(crate) use shape_properties::*;
pub(crate) use shape_size::*;
pub(crate) use transforms::{
    calculate_embed_position_compensation, calculate_pivot_compensation, get_initial_location,
    get_initial_opacity, get_initial_pivot, get_initial_rotation, get_initial_scale,
    get_scale_at_normalized_time, pivot_to_anchor_and_offset, truncate_string,
};

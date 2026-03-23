//! Defines ECS components that mark and describe SDF shape entities.
//! 定义标记并描述 SDF 形状实体的 ECS 组件。
//!
//! Spawn code and animation systems need lightweight component-side access to authored SDF shape
//! metadata such as fill color, stroke, dimensions, and shape type. This file provides those
//! runtime markers so SDF-specific logic can target the correct entities without re-reading source
//! schema data.
//! spawn 代码与动画系统都需要通过组件直接访问作者定义的 SDF 形状元数据，例如填充色、描边、尺寸和
//! 形状类型。这个文件提供的正是这些运行时标记，让 SDF 相关逻辑可以精准定位实体，而不用重复回读原始
//! schema 数据。

use bevy::prelude::*;

use super::SdfShapeType;

/// Component for AM SDF shapes that need special animation handling.
#[derive(Component, Debug, Clone)]
pub struct AmSdfShapeComponent {
    pub fill_color: Color,
    pub stroke_color: Option<Color>,
    pub stroke_width: f32,
    pub corner_radius: f32,
    pub width: f32,
    pub height: f32,
    pub shape_type: SdfShapeType,
}

/// Marker component for SDF shape entities.
#[derive(Component, Debug, Clone, Default)]
pub struct SdfShapeMarker;

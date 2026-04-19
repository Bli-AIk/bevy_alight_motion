//! Enumerates the SDF shape variants supported by the renderer.
//!
//! 枚举渲染器支持的 SDF 形状种类。
//!
//! Scene collection and shader materials communicate shape identity through numeric discriminants.
//! Acts as the typed source of truth for those variants, mapping human-readable Rust enums to
//! the float values expected by the SDF material uniform.
//!
//! scene 收集阶段与 shader 材质之间，会通过数值判别值来传递形状类型。
//! 就是那套判别值的强类型真源：它把可读的 Rust 枚举映射到 SDF 材质 uniform 期望的 float 值。

/// SDF shape types supported by the material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SdfShapeType {
    #[default]
    BoxRound,
    BoxMiter,
    BoxBevel,
    Circle,
    RoundRect,
    Polygon,
    Star,
    Pie,
    Plus,
    Multifoil,
    Line,
    Arc,
    Triangle,
    Quad,
    Penta,
    Path,
    Arrow,
}

impl SdfShapeType {
    pub fn to_f32(self) -> f32 {
        match self {
            Self::BoxRound => 0.0,
            Self::BoxMiter => 1.0,
            Self::BoxBevel => 2.0,
            Self::Circle => 3.0,
            Self::RoundRect => 4.0,
            Self::Polygon => 5.0,
            Self::Star => 6.0,
            Self::Pie => 7.0,
            Self::Plus => 8.0,
            Self::Multifoil => 9.0,
            Self::Line => 10.0,
            Self::Arc => 11.0,
            Self::Triangle => 12.0,
            Self::Quad => 13.0,
            Self::Penta => 14.0,
            Self::Path => 15.0,
            Self::Arrow => 16.0,
        }
    }
}

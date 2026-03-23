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

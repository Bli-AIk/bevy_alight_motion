//! # types.rs
//!
//! # Schema 类型定义模块
//!
//! Data structures for Alight Motion XML schema.
//! 用于 Alight Motion XML 格式的数据结构定义。
//!
//! This module provides strongly-typed representations of AM project files,
//! with robust handling of optional fields and defaults.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::parsing::{parse_vec2, parse_vec3};

/// Root scene node containing project metadata and layers.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename = "scene")]
pub struct AmScene {
    /// Project title.
    #[serde(rename = "@title", default)]
    pub title: String,

    /// Canvas width in pixels.
    #[serde(rename = "@width", default = "default_canvas_size")]
    pub width: u32,

    /// Canvas height in pixels.
    #[serde(rename = "@height", default = "default_canvas_size")]
    pub height: u32,

    /// Export width in pixels.
    #[serde(rename = "@exportWidth", default = "default_canvas_size")]
    pub export_width: u32,

    /// Export height in pixels.
    #[serde(rename = "@exportHeight", default = "default_canvas_size")]
    pub export_height: u32,

    /// Frames per second.
    #[serde(rename = "@fps", default = "default_fps")]
    pub fps: u32,

    /// Total duration in milliseconds.
    #[serde(rename = "@totalTime", default)]
    pub total_time: u32,

    /// Background color in #AARRGGBB format.
    #[serde(rename = "@bgcolor", default = "default_bgcolor")]
    pub bgcolor: String,

    /// AM version number. Can be negative for certain builds.
    #[serde(rename = "@amver", default)]
    pub amver: i32,

    /// Time remapping strategy.
    #[serde(rename = "@retime", default)]
    pub retime: String,

    /// Precompose mode.
    #[serde(rename = "@precompose", default)]
    pub precompose: String,

    /// Media resources.
    #[serde(rename = "media", default)]
    pub media: Vec<AmMedia>,

    /// Scene layers (shapes, nullobjs, embedScenes).
    #[serde(rename = "$value", default)]
    pub layers: Vec<AmLayer>,
}

fn default_canvas_size() -> u32 {
    1280
}

fn default_fps() -> u32 {
    60
}

fn default_bgcolor() -> String {
    "#ff000000".to_string()
}

/// Media resource definition.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmMedia {
    /// Resource URI (e.g., "amproj:filename.png").
    #[serde(rename = "@uri", default)]
    pub uri: String,

    /// Physical filename.
    #[serde(rename = "@filename", default)]
    pub filename: String,

    /// MIME type (e.g., "image/png").
    #[serde(rename = "@type", default)]
    pub media_type: String,

    /// Original width in pixels.
    #[serde(rename = "@width", default)]
    pub width: u32,

    /// Original height in pixels.
    #[serde(rename = "@height", default)]
    pub height: u32,

    /// File size in bytes.
    #[serde(rename = "@size", default)]
    pub size: u32,

    /// SHA1 signature.
    #[serde(rename = "@sig", default)]
    pub sig: String,
}

/// Layer types in the scene.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::large_enum_variant)]
pub enum AmLayer {
    /// Visible shape layer.
    Shape(AmShape),
    /// Null/empty object for grouping.
    Nullobj(AmNullObj),
    /// Embedded sub-scene (pre-composition).
    EmbedScene(AmEmbedScene),
    /// Timeline bookmark marker (non-visual, for organization).
    Bookmark(AmBookmark),
    /// Text layer.
    Text(AmText),
    /// Audio layer.
    Audio(AmAudio),
    /// Camera layer.
    Camera(AmCamera),
    /// Image layer (alternative to shape with media fill).
    Image(AmImage),
    /// Video layer.
    Video(AmVideo),
}

/// Bookmark marker for timeline organization (non-visual).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmBookmark {
    /// Unique identifier.
    #[serde(rename = "@id", default)]
    pub id: u64,

    /// Bookmark label/name.
    #[serde(rename = "@label", default)]
    pub label: String,

    /// Time position in milliseconds.
    #[serde(rename = "@startTime", default)]
    pub start_time: i32,

    /// End time in milliseconds.
    #[serde(rename = "@endTime", default)]
    pub end_time: i32,
}

/// Text layer.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmText {
    /// Unique identifier.
    #[serde(rename = "@id", default)]
    pub id: u64,

    /// Layer label/name.
    #[serde(rename = "@label", default)]
    pub label: String,

    /// In-point in milliseconds.
    #[serde(rename = "@startTime", default)]
    pub start_time: i32,

    /// Out-point in milliseconds.
    #[serde(rename = "@endTime", default)]
    pub end_time: i32,

    /// Parent layer ID.
    #[serde(rename = "@parent", default)]
    pub parent: u64,

    /// Fill type.
    #[serde(rename = "@fillType", default)]
    pub fill_type: String,

    /// Transform data.
    #[serde(default)]
    pub transform: AmTransform,

    /// Text content (from <content> child element).
    #[serde(default)]
    pub content: String,

    /// Font specification (e.g., "imported?name=FontName.ttf").
    #[serde(rename = "@font", default)]
    pub font: String,

    /// Font size in points.
    #[serde(rename = "@size", default)]
    pub size: f32,

    /// Text wrap width.
    #[serde(rename = "@wrapWidth", default)]
    pub wrap_width: f32,

    /// Text alignment (left, center, right).
    #[serde(rename = "@align", default)]
    pub align: String,

    /// Effects applied to this text.
    #[serde(rename = "effect", default)]
    pub effects: Vec<AmEffect>,

    /// Fill color.
    #[serde(rename = "fillColor", default)]
    pub fill_color: Option<AmFillColor>,

    /// Text properties.
    #[serde(rename = "property", default)]
    pub properties: Vec<AmProperty>,
}

/// Audio layer (non-visual, for audio playback).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmAudio {
    /// Unique identifier.
    #[serde(rename = "@id", default)]
    pub id: u64,

    /// Layer label/name.
    #[serde(rename = "@label", default)]
    pub label: String,

    /// In-point in milliseconds.
    #[serde(rename = "@startTime", default)]
    pub start_time: i32,

    /// Out-point in milliseconds.
    #[serde(rename = "@endTime", default)]
    pub end_time: i32,

    /// Parent layer ID.
    #[serde(rename = "@parent", default)]
    pub parent: u64,

    /// Audio source URI.
    #[serde(rename = "@source", default)]
    pub source: String,

    /// Volume level.
    #[serde(rename = "@volume", default = "default_volume")]
    pub volume: f32,

    /// Audio properties.
    #[serde(rename = "property", default)]
    pub properties: Vec<AmProperty>,
}

fn default_volume() -> f32 {
    1.0
}

/// Camera layer.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmCamera {
    /// Unique identifier.
    #[serde(rename = "@id", default)]
    pub id: u64,

    /// Layer label/name.
    #[serde(rename = "@label", default)]
    pub label: String,

    /// In-point in milliseconds.
    #[serde(rename = "@startTime", default)]
    pub start_time: i32,

    /// Out-point in milliseconds.
    #[serde(rename = "@endTime", default)]
    pub end_time: i32,

    /// Parent layer ID.
    #[serde(rename = "@parent", default)]
    pub parent: u64,

    /// Transform data.
    #[serde(default)]
    pub transform: AmTransform,

    /// Camera properties.
    #[serde(rename = "property", default)]
    pub properties: Vec<AmProperty>,

    /// FOV animation (degrees). Default 60°.
    #[serde(default)]
    pub fov: AmAnimatedFloat,
}

/// Image layer (standalone image, similar to shape with media fill).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmImage {
    /// Unique identifier.
    #[serde(rename = "@id", default)]
    pub id: u64,

    /// Layer label/name.
    #[serde(rename = "@label", default)]
    pub label: String,

    /// In-point in milliseconds.
    #[serde(rename = "@startTime", default)]
    pub start_time: i32,

    /// Out-point in milliseconds.
    #[serde(rename = "@endTime", default)]
    pub end_time: i32,

    /// Parent layer ID.
    #[serde(rename = "@parent", default)]
    pub parent: u64,

    /// Fill type.
    #[serde(rename = "@fillType", default)]
    pub fill_type: String,

    /// Fill image URI.
    #[serde(rename = "@fillImage", default)]
    pub fill_image: String,

    /// Transform data.
    #[serde(default)]
    pub transform: AmTransform,

    /// Image properties.
    #[serde(rename = "property", default)]
    pub properties: Vec<AmProperty>,

    /// Effects applied to this image.
    #[serde(rename = "effect", default)]
    pub effects: Vec<AmEffect>,
}

/// Video layer.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmVideo {
    /// Unique identifier.
    #[serde(rename = "@id", default)]
    pub id: u64,

    /// Layer label/name.
    #[serde(rename = "@label", default)]
    pub label: String,

    /// In-point in milliseconds.
    #[serde(rename = "@startTime", default)]
    pub start_time: i32,

    /// Out-point in milliseconds.
    #[serde(rename = "@endTime", default)]
    pub end_time: i32,

    /// Parent layer ID.
    #[serde(rename = "@parent", default)]
    pub parent: u64,

    /// Fill type.
    #[serde(rename = "@fillType", default)]
    pub fill_type: String,

    /// Video source URI.
    #[serde(rename = "@source", default)]
    pub source: String,

    /// Transform data.
    #[serde(default)]
    pub transform: AmTransform,

    /// Video properties.
    #[serde(rename = "property", default)]
    pub properties: Vec<AmProperty>,

    /// Effects applied to this video.
    #[serde(rename = "effect", default)]
    pub effects: Vec<AmEffect>,
}

/// Common layer properties.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmLayerBase {
    /// Unique identifier.
    #[serde(rename = "@id", default)]
    pub id: u64,

    /// Layer label/name.
    #[serde(rename = "@label", default)]
    pub label: String,

    /// In-point in milliseconds.
    #[serde(rename = "@startTime", default)]
    pub start_time: i32,

    /// Out-point in milliseconds.
    #[serde(rename = "@endTime", default)]
    pub end_time: i32,

    /// Parent layer ID (0 if no parent).
    #[serde(rename = "@parent", default)]
    pub parent: u64,

    /// Alternative out-point.
    #[serde(rename = "@outTime", default)]
    pub out_time: Option<i32>,
}

/// Shape layer (visible object).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmShape {
    /// Unique identifier.
    #[serde(rename = "@id", default)]
    pub id: u64,

    /// Layer label/name.
    #[serde(rename = "@label", default)]
    pub label: String,

    /// In-point in milliseconds.
    #[serde(rename = "@startTime", default)]
    pub start_time: i32,

    /// Out-point in milliseconds.
    #[serde(rename = "@endTime", default)]
    pub end_time: i32,

    /// Parent layer ID.
    #[serde(rename = "@parent", default)]
    pub parent: u64,

    /// Fill type ("color" or "media").
    #[serde(rename = "@fillType", default)]
    pub fill_type: String,

    /// Fill image URI (when fillType="media").
    #[serde(rename = "@fillImage", default)]
    pub fill_image: String,

    /// Shape type (e.g., ".rect", ".circle").
    #[serde(rename = "@s", default)]
    pub shape_type: String,

    /// Blending mode (e.g., "mask" for masking layer).
    #[serde(rename = "@blending", default)]
    pub blending: String,

    /// Whether this layer is hidden in the editor (should not be rendered).
    #[serde(rename = "@hidden", default)]
    pub hidden: bool,

    /// Playback speed multiplier (1.0 = normal, 0.5 = half speed).
    /// Affects keyframe interpolation rate but not visibility timing.
    #[serde(rename = "@speed", default = "default_speed")]
    pub speed: f32,

    /// Transform data.
    #[serde(default)]
    pub transform: AmTransform,

    /// Shape properties.
    #[serde(rename = "property", default)]
    pub properties: Vec<AmProperty>,

    /// Effects applied to this shape.
    #[serde(rename = "effect", default)]
    pub effects: Vec<AmEffect>,

    /// Fill color (when fillType="color").
    #[serde(rename = "fillColor", default)]
    pub fill_color: Option<AmFillColor>,

    /// Stroke/border style.
    #[serde(rename = "path-stroke", default)]
    pub stroke: Option<AmStroke>,

    /// Border decorations (can have multiple with different directions).
    #[serde(rename = "border", default)]
    pub borders: Vec<AmStroke>,

    /// Gradient fill data (when fillType="gradient").
    #[serde(default)]
    pub gradient: Option<AmGradient>,

    /// SVG path data for freeform shapes.
    #[serde(rename = "path", default)]
    pub path_element: Option<AmPathElement>,
}

/// Stroke/border properties for shapes.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmStroke {
    /// Stroke direction ("centered", "inside", "outside").
    #[serde(rename = "@direction", default)]
    pub direction: String,

    /// Border ID (for multi-border shapes).
    #[serde(rename = "@id", default)]
    pub id: Option<i32>,

    /// Line cap style ("square", "round", "butt").
    #[serde(rename = "@cap", default)]
    pub cap: String,

    /// Line join style ("miter", "round", "bevel").
    #[serde(rename = "@join", default)]
    pub join: String,

    /// End size for variable width strokes.
    #[serde(rename = "@end-size", default)]
    pub end_size: f32,

    /// Stroke color (child element).
    #[serde(rename = "color", default)]
    pub color: Option<AmStrokeColor>,

    /// Stroke width (child element).
    #[serde(rename = "size", default)]
    pub size: Option<AmStrokeSize>,
}

/// Stroke color element.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmStrokeColor {
    /// Color value in #AARRGGBB format.
    #[serde(rename = "@value", default)]
    pub value: String,
}

/// Stroke size element (can be static or animated).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmStrokeSize {
    /// Static size value (if not animated).
    #[serde(rename = "@value", default)]
    pub value: Option<f32>,

    /// Keyframes (if animated).
    #[serde(rename = "kf", default)]
    pub keyframes: Vec<AmKeyframe>,
}

/// Null object (invisible parent controller).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmNullObj {
    /// Unique identifier.
    #[serde(rename = "@id", default)]
    pub id: u64,

    /// Layer label/name.
    #[serde(rename = "@label", default)]
    pub label: String,

    /// In-point in milliseconds.
    #[serde(rename = "@startTime", default)]
    pub start_time: i32,

    /// Out-point in milliseconds.
    #[serde(rename = "@endTime", default)]
    pub end_time: i32,

    /// Parent layer ID.
    #[serde(rename = "@parent", default)]
    pub parent: u64,

    /// Object type (e.g., "perspective").
    #[serde(rename = "@type", default)]
    pub obj_type: String,

    /// Transform data.
    #[serde(default)]
    pub transform: AmTransform,

    /// Effects applied to this object.
    #[serde(rename = "effect", default)]
    pub effects: Vec<AmEffect>,
}

/// Embedded sub-scene (pre-composition).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmEmbedScene {
    /// Unique identifier.
    #[serde(rename = "@id", default)]
    pub id: u64,

    /// Layer label/name.
    #[serde(rename = "@label", default)]
    pub label: String,

    /// In-point in milliseconds.
    #[serde(rename = "@startTime", default)]
    pub start_time: i32,

    /// Out-point in milliseconds.
    #[serde(rename = "@endTime", default)]
    pub end_time: i32,

    /// Parent layer ID.
    #[serde(rename = "@parent", default)]
    pub parent: u64,

    /// Fill type.
    #[serde(rename = "@fillType", default)]
    pub fill_type: String,

    /// Internal in-point for nested scene playback (clip start).
    #[serde(rename = "@inTime", default)]
    pub in_time: Option<i32>,

    /// Internal out-point for nested scene playback (clip end).
    #[serde(rename = "@outTime", default)]
    pub out_time: Option<i32>,

    /// Playback speed multiplier (1.0 = normal, 0.5 = half speed, 2.0 = double speed)
    #[serde(rename = "@speed", default = "default_speed")]
    pub speed: f32,

    /// Transform data.
    #[serde(default)]
    pub transform: AmTransform,

    /// Fill color.
    #[serde(rename = "fillColor", default)]
    pub fill_color: Option<AmFillColor>,

    /// Effects applied to this embed.
    #[serde(rename = "effect", default)]
    pub effects: Vec<AmEffect>,

    /// Gradient fill data (when fillType="gradient").
    #[serde(default)]
    pub gradient: Option<AmGradient>,

    /// Nested scene.
    pub scene: Box<AmScene>,
}

fn default_speed() -> f32 {
    1.0
}

/// Fill color definition (can be static or animated).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmFillColor {
    /// Static color value in #AARRGGBB format.
    #[serde(rename = "@value", default)]
    pub value: String,

    /// Keyframes for animated fill color.
    #[serde(rename = "kf", default)]
    pub keyframes: Vec<AmKeyframe>,
}

/// Transform container with animated properties.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmTransform {
    /// Lock aspect ratio flag.
    #[serde(rename = "@lockAspectRatio", default)]
    pub lock_aspect_ratio: bool,

    /// Location/position property.
    #[serde(default)]
    pub location: AmAnimatedVec3,

    /// Pivot/anchor point property (affects rotation and scale center).
    #[serde(default)]
    pub pivot: AmAnimatedVec2,

    /// Rotation property (Z-axis, degrees).
    #[serde(default)]
    pub rotation: AmAnimatedFloat,

    /// Scale property.
    #[serde(default)]
    pub scale: AmAnimatedVec2,

    /// Opacity property (0.0-1.0).
    #[serde(default)]
    pub opacity: AmAnimatedFloat,
}

/// Animated Vec3 property (x, y, z).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmAnimatedVec3 {
    /// Static value (if not animated).
    #[serde(
        rename = "@value",
        default,
        deserialize_with = "deserialize_vec3_opt",
        serialize_with = "serialize_vec3_opt"
    )]
    pub value: Option<[f32; 3]>,

    /// Keyframes (if animated).
    #[serde(rename = "kf", default)]
    pub keyframes: Vec<AmKeyframe>,
}

/// Animated Vec2 property (x, y).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmAnimatedVec2 {
    /// Static value (if not animated).
    #[serde(
        rename = "@value",
        default,
        deserialize_with = "deserialize_vec2_opt",
        serialize_with = "serialize_vec2_opt"
    )]
    pub value: Option<[f32; 2]>,

    /// Keyframes (if animated).
    #[serde(rename = "kf", default)]
    pub keyframes: Vec<AmKeyframe>,
}

/// Animated float property.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmAnimatedFloat {
    /// Static value (if not animated).
    #[serde(rename = "@value", default)]
    pub value: Option<f32>,

    /// Keyframes (if animated).
    #[serde(rename = "kf", default)]
    pub keyframes: Vec<AmKeyframe>,
}

/// Animated color (Vec4 RGBA).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmAnimatedColor {
    /// Static value (if not animated).
    pub value: Option<bevy::prelude::Vec4>,

    /// Keyframes (if animated). Values are stored as "r,g,b,a" strings.
    #[serde(default)]
    pub keyframes: Vec<AmKeyframe>,
}

/// Keyframe definition.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmKeyframe {
    /// Normalized time (0.0-1.0).
    #[serde(rename = "@t", default)]
    pub time: f32,

    /// Value at this keyframe (string format varies by property type).
    #[serde(rename = "@v", default)]
    pub value: String,

    /// Easing function (e.g., "cubicBezier 0.0 0.0 0.58 1.0", "step 1.0 0.0").
    #[serde(rename = "@e", default)]
    pub easing: Option<String>,
}

/// Property definition (e.g., size).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmProperty {
    /// Property name.
    #[serde(rename = "@name", default)]
    pub name: String,

    /// Property type (e.g., "vec2", "float").
    #[serde(rename = "@type", default)]
    pub prop_type: String,

    /// Static value.
    #[serde(rename = "@value", default)]
    pub value: String,

    /// Keyframes (if animated).
    #[serde(rename = "kf", default)]
    pub keyframes: Vec<AmKeyframe>,
}

/// Effect definition.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmEffect {
    /// Effect type ID.
    #[serde(rename = "@id", default)]
    pub id: String,

    /// Whether applied locally.
    #[serde(rename = "@locallyApplied", default)]
    pub locally_applied: bool,

    /// Effect properties.
    #[serde(rename = "property", default)]
    pub properties: Vec<AmProperty>,
}

// Custom deserializers/serializers for vector types

fn deserialize_vec3_opt<'de, D>(deserializer: D) -> Result<Option<[f32; 3]>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) if !s.is_empty() => parse_vec3(&s).map(Some).map_err(serde::de::Error::custom),
        _ => Ok(None),
    }
}

fn serialize_vec3_opt<S>(value: &Option<[f32; 3]>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some([x, y, z]) => serializer.serialize_some(&format!("{},{},{}", x, y, z)),
        None => serializer.serialize_none(),
    }
}

fn deserialize_vec2_opt<'de, D>(deserializer: D) -> Result<Option<[f32; 2]>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) if !s.is_empty() => parse_vec2(&s).map(Some).map_err(serde::de::Error::custom),
        _ => Ok(None),
    }
}

fn serialize_vec2_opt<S>(value: &Option<[f32; 2]>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some([x, y]) => serializer.serialize_some(&format!("{},{}", x, y)),
        None => serializer.serialize_none(),
    }
}

/// Gradient fill data for shapes with fillType="gradient".
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmGradient {
    /// Gradient type: "linear", "radial", "sweep"
    #[serde(rename = "@type", default)]
    pub gradient_type: String,

    /// Start color
    #[serde(rename = "@startColor", default)]
    pub start_color: String,

    /// End color
    #[serde(rename = "@endColor", default)]
    pub end_color: String,

    /// Start point (UV coordinates, 0-1)
    #[serde(
        rename = "@start",
        default,
        deserialize_with = "deserialize_vec2_opt",
        serialize_with = "serialize_vec2_opt"
    )]
    pub start: Option<[f32; 2]>,

    /// End point (UV coordinates, 0-1)
    #[serde(
        rename = "@end",
        default,
        deserialize_with = "deserialize_vec2_opt",
        serialize_with = "serialize_vec2_opt"
    )]
    pub end: Option<[f32; 2]>,
}

/// SVG path element for freeform shapes.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmPathElement {
    /// SVG path data string (e.g., "M 0 0 L 100 100")
    #[serde(rename = "@d", default)]
    pub d: String,
}

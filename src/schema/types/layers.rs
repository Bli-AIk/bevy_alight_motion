use serde::{Deserialize, Serialize};

use super::animation::{
    AmAnimatedFloat, AmEffect, AmFillColor, AmKeyframe, AmProperty, AmTransform,
};
use super::scene::AmScene;
use super::serde_helpers::{
    deserialize_float_opt, deserialize_i32_opt, deserialize_vec2_opt, serialize_float_opt,
    serialize_i32_opt, serialize_vec2_opt,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(clippy::large_enum_variant)]
pub enum AmLayer {
    Shape(AmShape),
    Nullobj(AmNullObj),
    EmbedScene(AmEmbedScene),
    Bookmark(AmBookmark),
    Text(AmText),
    Audio(AmAudio),
    Camera(AmCamera),
    Image(AmImage),
    Video(AmVideo),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmBookmark {
    #[serde(rename = "@id", default)]
    pub id: u64,
    #[serde(rename = "@label", default)]
    pub label: String,
    #[serde(rename = "@startTime", default)]
    pub start_time: i32,
    #[serde(rename = "@endTime", default)]
    pub end_time: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmText {
    #[serde(rename = "@id", default)]
    pub id: u64,
    #[serde(rename = "@label", default)]
    pub label: String,
    #[serde(rename = "@startTime", default)]
    pub start_time: i32,
    #[serde(rename = "@endTime", default)]
    pub end_time: i32,
    #[serde(rename = "@parent", default)]
    pub parent: u64,
    #[serde(rename = "@hidden", default)]
    pub hidden: bool,
    #[serde(rename = "@fillType", default)]
    pub fill_type: String,
    #[serde(default)]
    pub transform: AmTransform,
    #[serde(default)]
    pub content: String,
    #[serde(rename = "@font", default)]
    pub font: String,
    #[serde(rename = "@size", default)]
    pub size: f32,
    #[serde(rename = "@wrapWidth", default)]
    pub wrap_width: f32,
    #[serde(rename = "@align", default)]
    pub align: String,
    #[serde(rename = "effect", default)]
    pub effects: Vec<AmEffect>,
    #[serde(rename = "fillColor", default)]
    pub fill_color: Option<AmFillColor>,
    #[serde(rename = "property", default)]
    pub properties: Vec<AmProperty>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmAudio {
    #[serde(rename = "@id", default)]
    pub id: u64,
    #[serde(rename = "@label", default)]
    pub label: String,
    #[serde(rename = "@startTime", default)]
    pub start_time: i32,
    #[serde(rename = "@endTime", default)]
    pub end_time: i32,
    #[serde(rename = "@parent", default)]
    pub parent: u64,
    #[serde(rename = "@source", default)]
    pub source: String,
    #[serde(rename = "@volume", default = "default_volume")]
    pub volume: f32,
    #[serde(rename = "property", default)]
    pub properties: Vec<AmProperty>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmCamera {
    #[serde(rename = "@id", default)]
    pub id: u64,
    #[serde(rename = "@label", default)]
    pub label: String,
    #[serde(rename = "@startTime", default)]
    pub start_time: i32,
    #[serde(rename = "@endTime", default)]
    pub end_time: i32,
    #[serde(rename = "@parent", default)]
    pub parent: u64,
    #[serde(rename = "@hidden", default)]
    pub hidden: bool,
    #[serde(default)]
    pub transform: AmTransform,
    #[serde(rename = "property", default)]
    pub properties: Vec<AmProperty>,
    #[serde(default)]
    pub fov: AmAnimatedFloat,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmImage {
    #[serde(rename = "@id", default)]
    pub id: u64,
    #[serde(rename = "@label", default)]
    pub label: String,
    #[serde(rename = "@startTime", default)]
    pub start_time: i32,
    #[serde(rename = "@endTime", default)]
    pub end_time: i32,
    #[serde(rename = "@parent", default)]
    pub parent: u64,
    #[serde(rename = "@hidden", default)]
    pub hidden: bool,
    #[serde(rename = "@fillType", default)]
    pub fill_type: String,
    #[serde(rename = "@fillImage", default)]
    pub fill_image: String,
    #[serde(default)]
    pub transform: AmTransform,
    #[serde(rename = "property", default)]
    pub properties: Vec<AmProperty>,
    #[serde(rename = "effect", default)]
    pub effects: Vec<AmEffect>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmVideo {
    #[serde(rename = "@id", default)]
    pub id: u64,
    #[serde(rename = "@label", default)]
    pub label: String,
    #[serde(rename = "@startTime", default)]
    pub start_time: i32,
    #[serde(rename = "@endTime", default)]
    pub end_time: i32,
    #[serde(rename = "@parent", default)]
    pub parent: u64,
    #[serde(rename = "@fillType", default)]
    pub fill_type: String,
    #[serde(rename = "@source", default)]
    pub source: String,
    #[serde(default)]
    pub transform: AmTransform,
    #[serde(rename = "property", default)]
    pub properties: Vec<AmProperty>,
    #[serde(rename = "effect", default)]
    pub effects: Vec<AmEffect>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmLayerBase {
    #[serde(rename = "@id", default)]
    pub id: u64,
    #[serde(rename = "@label", default)]
    pub label: String,
    #[serde(rename = "@startTime", default)]
    pub start_time: i32,
    #[serde(rename = "@endTime", default)]
    pub end_time: i32,
    #[serde(rename = "@parent", default)]
    pub parent: u64,
    #[serde(
        rename = "@outTime",
        default,
        deserialize_with = "deserialize_i32_opt",
        serialize_with = "serialize_i32_opt"
    )]
    pub out_time: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmShape {
    #[serde(rename = "@id", default)]
    pub id: u64,
    #[serde(rename = "@label", default)]
    pub label: String,
    #[serde(rename = "@startTime", default)]
    pub start_time: i32,
    #[serde(rename = "@endTime", default)]
    pub end_time: i32,
    #[serde(rename = "@parent", default)]
    pub parent: u64,
    #[serde(rename = "@fillType", default)]
    pub fill_type: String,
    #[serde(rename = "@fillImage", default)]
    pub fill_image: String,
    #[serde(rename = "@s", default)]
    pub shape_type: String,
    #[serde(rename = "@blending", default)]
    pub blending: String,
    #[serde(rename = "@hidden", default)]
    pub hidden: bool,
    #[serde(rename = "@speed", default = "default_speed")]
    pub speed: f32,
    #[serde(default)]
    pub transform: AmTransform,
    #[serde(rename = "property", default)]
    pub properties: Vec<AmProperty>,
    #[serde(rename = "effect", default)]
    pub effects: Vec<AmEffect>,
    #[serde(rename = "fillColor", default)]
    pub fill_color: Option<AmFillColor>,
    #[serde(rename = "path-stroke", default)]
    pub stroke: Option<AmStroke>,
    #[serde(rename = "border", default)]
    pub borders: Vec<AmStroke>,
    #[serde(default)]
    pub gradient: Option<AmGradient>,
    #[serde(rename = "path", default)]
    pub path_element: Option<AmPathElement>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmStroke {
    #[serde(rename = "@direction", default)]
    pub direction: String,
    #[serde(
        rename = "@id",
        default,
        deserialize_with = "deserialize_i32_opt",
        serialize_with = "serialize_i32_opt"
    )]
    pub id: Option<i32>,
    #[serde(rename = "@cap", default)]
    pub cap: String,
    #[serde(rename = "@join", default)]
    pub join: String,
    #[serde(rename = "@end-size", default)]
    pub end_size: f32,
    #[serde(rename = "color", default)]
    pub color: Option<AmStrokeColor>,
    #[serde(rename = "size", default)]
    pub size: Option<AmStrokeSize>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmStrokeColor {
    #[serde(rename = "@value", default)]
    pub value: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmStrokeSize {
    #[serde(
        rename = "@value",
        default,
        deserialize_with = "deserialize_float_opt",
        serialize_with = "serialize_float_opt"
    )]
    pub value: Option<f32>,
    #[serde(rename = "kf", default)]
    pub keyframes: Vec<AmKeyframe>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmNullObj {
    #[serde(rename = "@id", default)]
    pub id: u64,
    #[serde(rename = "@label", default)]
    pub label: String,
    #[serde(rename = "@startTime", default)]
    pub start_time: i32,
    #[serde(rename = "@endTime", default)]
    pub end_time: i32,
    #[serde(rename = "@parent", default)]
    pub parent: u64,
    #[serde(rename = "@hidden", default)]
    pub hidden: bool,
    #[serde(rename = "@type", default)]
    pub obj_type: String,
    #[serde(default)]
    pub transform: AmTransform,
    #[serde(rename = "effect", default)]
    pub effects: Vec<AmEffect>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmEmbedScene {
    #[serde(rename = "@id", default)]
    pub id: u64,
    #[serde(rename = "@label", default)]
    pub label: String,
    #[serde(rename = "@startTime", default)]
    pub start_time: i32,
    #[serde(rename = "@endTime", default)]
    pub end_time: i32,
    #[serde(rename = "@parent", default)]
    pub parent: u64,
    #[serde(rename = "@hidden", default)]
    pub hidden: bool,
    #[serde(rename = "@fillType", default)]
    pub fill_type: String,
    #[serde(
        rename = "@inTime",
        default,
        deserialize_with = "deserialize_i32_opt",
        serialize_with = "serialize_i32_opt"
    )]
    pub in_time: Option<i32>,
    #[serde(
        rename = "@outTime",
        default,
        deserialize_with = "deserialize_i32_opt",
        serialize_with = "serialize_i32_opt"
    )]
    pub out_time: Option<i32>,
    #[serde(rename = "@speed", default = "default_speed")]
    pub speed: f32,
    #[serde(default)]
    pub transform: AmTransform,
    #[serde(rename = "fillColor", default)]
    pub fill_color: Option<AmFillColor>,
    #[serde(rename = "effect", default)]
    pub effects: Vec<AmEffect>,
    #[serde(default)]
    pub gradient: Option<AmGradient>,
    #[serde(rename = "@blending", default)]
    pub blending: String,
    pub scene: Box<AmScene>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmGradient {
    #[serde(rename = "@type", default)]
    pub gradient_type: String,
    #[serde(rename = "@startColor", default)]
    pub start_color: String,
    #[serde(rename = "@endColor", default)]
    pub end_color: String,
    #[serde(
        rename = "@start",
        default,
        deserialize_with = "deserialize_vec2_opt",
        serialize_with = "serialize_vec2_opt"
    )]
    pub start: Option<[f32; 2]>,
    #[serde(
        rename = "@end",
        default,
        deserialize_with = "deserialize_vec2_opt",
        serialize_with = "serialize_vec2_opt"
    )]
    pub end: Option<[f32; 2]>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmPathElement {
    #[serde(rename = "@d", default)]
    pub d: String,
}

fn default_volume() -> f32 {
    1.0
}

fn default_speed() -> f32 {
    1.0
}

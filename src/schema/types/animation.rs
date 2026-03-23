//! This file defines the reusable animation-oriented schema fragments shared by
//! many Alight Motion layer types. It covers animated scalars, vectors, colors,
//! keyframes, and effect/property payloads so higher-level layer structs can
//! compose timeline-capable fields without duplicating XML mapping logic.
//!
//! 这个文件定义了多个 Alight Motion 图层类型共用的动画 schema 片段。它包含
//! 带关键帧的标量、向量、颜色，以及 effect/property 的数据结构，让更高层的
//! 图层结构可以复用时间轴字段，而不必重复书写 XML 映射逻辑。

use bevy::prelude::Vec4;
use serde::{Deserialize, Serialize};

use super::serde_helpers::{
    deserialize_float_opt, deserialize_vec2_opt, deserialize_vec3_opt, serialize_float_opt,
    serialize_vec2_opt, serialize_vec3_opt,
};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmFillColor {
    #[serde(rename = "@value", default)]
    pub value: String,
    #[serde(rename = "kf", default)]
    pub keyframes: Vec<AmKeyframe>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmTransform {
    #[serde(rename = "@lockAspectRatio", default)]
    pub lock_aspect_ratio: bool,
    #[serde(default)]
    pub location: AmAnimatedVec3,
    #[serde(default)]
    pub pivot: AmAnimatedVec2,
    #[serde(default)]
    pub rotation: AmAnimatedFloat,
    #[serde(default)]
    pub scale: AmAnimatedVec2,
    #[serde(default)]
    pub opacity: AmAnimatedFloat,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmAnimatedVec3 {
    #[serde(
        rename = "@value",
        default,
        deserialize_with = "deserialize_vec3_opt",
        serialize_with = "serialize_vec3_opt"
    )]
    pub value: Option<[f32; 3]>,
    #[serde(rename = "kf", default)]
    pub keyframes: Vec<AmKeyframe>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmAnimatedVec2 {
    #[serde(
        rename = "@value",
        default,
        deserialize_with = "deserialize_vec2_opt",
        serialize_with = "serialize_vec2_opt"
    )]
    pub value: Option<[f32; 2]>,
    #[serde(rename = "kf", default)]
    pub keyframes: Vec<AmKeyframe>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmAnimatedFloat {
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AmAnimatedColor {
    pub value: Option<Vec4>,
    #[serde(default)]
    pub keyframes: Vec<AmKeyframe>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmKeyframe {
    #[serde(rename = "@t", default)]
    pub time: f32,
    #[serde(rename = "@v", default)]
    pub value: String,
    #[serde(rename = "@e", default)]
    pub easing: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmProperty {
    #[serde(rename = "@name", default)]
    pub name: String,
    #[serde(rename = "@type", default)]
    pub prop_type: String,
    #[serde(rename = "@value", default)]
    pub value: String,
    #[serde(rename = "kf", default)]
    pub keyframes: Vec<AmKeyframe>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AmEffect {
    #[serde(rename = "@id", default)]
    pub id: String,
    #[serde(rename = "@locallyApplied", default)]
    pub locally_applied: bool,
    #[serde(rename = "property", default)]
    pub properties: Vec<AmProperty>,
}

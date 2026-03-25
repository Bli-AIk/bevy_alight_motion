//! Contains the small serde adapters needed by the XML schema types.
//! Alight Motion stores many numeric vectors and optional values as comma-separated
//! strings, so these helpers centralize the parse/serialize rules instead of
//! scattering custom serde code across every schema struct.
//!
//! 存放 XML schema 类型需要的那组小型 serde 适配器。Alight Motion
//! 会把很多数值向量和可选字段编码成逗号分隔字符串，因此这些辅助函数把解析与
//! 序列化规则集中到一起，避免每个 schema 结构都各自写一份自定义 serde 代码。

use serde::{Deserialize, Deserializer, Serializer};

use crate::schema::parsing::{parse_vec2, parse_vec3};

pub(super) fn deserialize_vec3_opt<'de, D>(deserializer: D) -> Result<Option<[f32; 3]>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) if !s.is_empty() => parse_vec3(&s).map(Some).map_err(serde::de::Error::custom),
        _ => Ok(None),
    }
}

pub(super) fn serialize_vec3_opt<S>(
    value: &Option<[f32; 3]>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some([x, y, z]) => serializer.serialize_some(&format!("{},{},{}", x, y, z)),
        None => serializer.serialize_none(),
    }
}

pub(super) fn deserialize_vec2_opt<'de, D>(deserializer: D) -> Result<Option<[f32; 2]>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) if !s.is_empty() => parse_vec2(&s).map(Some).map_err(serde::de::Error::custom),
        _ => Ok(None),
    }
}

pub(super) fn serialize_vec2_opt<S>(
    value: &Option<[f32; 2]>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some([x, y]) => serializer.serialize_some(&format!("{},{}", x, y)),
        None => serializer.serialize_none(),
    }
}

pub(super) fn deserialize_float_opt<'de, D>(deserializer: D) -> Result<Option<f32>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) if !s.is_empty() => s.parse::<f32>().map(Some).map_err(serde::de::Error::custom),
        _ => Ok(None),
    }
}

pub(super) fn serialize_float_opt<S>(value: &Option<f32>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(v) => serializer.serialize_some(&v.to_string()),
        None => serializer.serialize_none(),
    }
}

pub(super) fn deserialize_i32_opt<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) if !s.is_empty() => s.parse::<i32>().map(Some).map_err(serde::de::Error::custom),
        _ => Ok(None),
    }
}

pub(super) fn serialize_i32_opt<S>(value: &Option<i32>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(v) => serializer.serialize_some(&v.to_string()),
        None => serializer.serialize_none(),
    }
}

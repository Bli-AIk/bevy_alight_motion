//! Defines identity and tagging components for scene layers.
//! These types give spawned entities stable ids, labels, and coarse element kinds
//! so later systems can find, debug, and reason about layers without reaching
//! back into the original XML structures.
//!
//! 定义了场景图层的身份与标记组件。它们会给已生成实体附上稳定的 id、
//! 标签和粗粒度元素类型，让后续系统无需回头依赖原始 XML 结构，也能定位、调试和
//! 识别各个图层。

use bevy::prelude::*;

#[derive(Component, Debug, Clone)]
pub struct AmLayerMarker {
    pub id: u64,
    pub label: String,
}

#[derive(Component, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AmLayerName(pub String);

impl AmLayerName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AmLayerName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Component, Debug, Clone, Default)]
pub struct AmElement;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AmElementType {
    Shape,
    Text,
    Image,
    Null,
    EmbedScene,
}

impl std::fmt::Display for AmElementType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AmElementType::Shape => write!(f, "Shape"),
            AmElementType::Text => write!(f, "Text"),
            AmElementType::Image => write!(f, "Image"),
            AmElementType::Null => write!(f, "Null"),
            AmElementType::EmbedScene => write!(f, "EmbedScene"),
        }
    }
}

#[derive(bevy::ecs::event::EntityEvent, Debug, Clone)]
pub struct AmEntitySpawned {
    #[event_target]
    pub entity: Entity,
    pub layer_name: String,
    pub layer_id: u64,
    pub element_type: AmElementType,
}

#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component)]
pub struct AmForceHidden;

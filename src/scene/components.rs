//! Scene-related components, bundles, and type definitions.

mod effects;
mod identity;
mod project;
mod spawn;

pub use effects::{AmBlendingMode, AmMaskEntry, AmMaskInfo, AmPaletteMapParams};
pub use identity::{
    AmElement, AmElementType, AmEntitySpawned, AmForceHidden, AmLayerMarker, AmLayerName,
};
pub use project::{
    AmEmbedContent, AmEmbedContentMarker, AmEmbedContentsContainer, AmLayersContainer,
    AmPendingLayers, AmProjectBundle, AmProjectRoot, AmRttCamerasContainer,
};
pub use spawn::{
    AmLayerSpec, AmSceneConfig, AmSpawnSettings, AmVisualSpawned, LayerFilter, PendingLayer,
};

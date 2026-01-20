//! # components.rs
//!
//! # 场景组件模块
//!
//! Scene-related components, bundles, and type definitions.
//! 场景相关的组件、Bundle 和类型定义。

use bevy::prelude::*;
use std::collections::HashMap;

use crate::animation::AmAnimated;
use crate::loader::AmProject;
use crate::schema::AmAnimatedFloat;

/// Component to track embed scene's content entities.
/// Content entities are spatially decoupled (not Bevy children) but logically belong to this embed.
/// This enables proper cleanup when the embed is despawned.
#[derive(Component, Debug, Clone, Default)]
pub struct AmEmbedContent {
    /// Entity IDs of content layers belonging to this embed.
    pub content_entities: Vec<Entity>,
}

/// Component marking an entity as content of an embed scene.
/// Used for lifecycle management - when the parent embed is despawned, these are too.
#[derive(Component, Debug, Clone)]
pub struct AmEmbedContentMarker {
    /// The embed entity this content belongs to.
    pub embed_entity: Entity,
    /// The embed's layer ID (for lookup in pending layers).
    pub embed_id: u64,
}

/// Component bundle for an AM project root.
#[derive(Bundle)]
pub struct AmProjectBundle {
    /// Transform for coordinate system conversion.
    pub transform: Transform,
    /// Global transform.
    pub global_transform: GlobalTransform,
    /// Visibility.
    pub visibility: Visibility,
    /// Inherited visibility.
    pub inherited_visibility: InheritedVisibility,
    /// View visibility.
    pub view_visibility: ViewVisibility,
    /// Marker component.
    pub marker: AmProjectRoot,
}

/// Marker component for the project root entity.
#[derive(Component, Debug, Clone)]
pub struct AmProjectRoot {
    /// Project handle.
    pub handle: Handle<AmProject>,
    /// Whether the scene has been spawned.
    pub spawned: bool,
}

/// Component storing pending layers for lazy entity spawning.
/// Attached to the project root, contains all layer definitions that haven't been spawned yet.
#[derive(Component, Debug, Clone, Default)]
pub struct AmPendingLayers {
    /// All layers in the project, stored as flat list with parent references.
    pub layers: Vec<PendingLayer>,
    /// Mapping from layer ID to entity (for spawned layers).
    pub spawned_entities: HashMap<u64, Entity>,
    /// Inverse fit scale for embed children coordinate adjustment.
    /// When the project is scaled to fit window, embed children need their coordinates
    /// scaled by 1/fit_scale to compensate for the root scaling.
    pub inv_fit_scale: f32,
    /// Entity of the layers container (parent for all top-level layers).
    /// None if container hasn't been created yet.
    pub layers_container: Option<Entity>,
    /// Entity of the embed contents container (parent for spatially decoupled embed content).
    /// None if container hasn't been created yet.
    pub embed_contents_container: Option<Entity>,
    /// Entity of the RTT cameras container (parent for EmbedSceneRttCamera entities).
    /// None if container hasn't been created yet.
    pub rtt_cameras_container: Option<Entity>,
}

/// Component marking an AM layer entity.
#[derive(Component, Debug, Clone)]
pub struct AmLayerMarker {
    /// Layer ID.
    pub id: u64,
    /// Layer label.
    pub label: String,
}

/// Marker component indicating the layer's visual has been spawned.
/// When present, the layer has active visual children that need to be despawned when out of time range.
#[derive(Component, Debug, Clone, Default)]
pub struct AmVisualSpawned;

/// Marker component for the layers container entity.
/// This entity is created under AmProjectRoot and serves as the parent for all AM visual layers.
/// Embed content (spatially decoupled) is NOT a Bevy child of this container, but is logically associated.
/// Users can query for this entity to manipulate all AM layers as a group.
#[derive(Component, Debug, Clone, Default)]
pub struct AmLayersContainer;

/// Marker component for the embed contents container entity.
/// This entity holds all embed content (spatially decoupled elements).
/// It has an identity Transform so embed content coordinates remain unchanged.
/// Embed content is added as Bevy children of this container for organization.
#[derive(Component, Debug, Clone, Default)]
pub struct AmEmbedContentsContainer;

/// Marker component for the RTT cameras container entity.
/// This entity holds all EmbedSceneRttCamera entities.
/// Organized for user convenience when inspecting the scene hierarchy.
#[derive(Component, Debug, Clone, Default)]
pub struct AmRttCamerasContainer;

/// Layer specification for lazy spawning. Contains all data needed to spawn the visual.
#[derive(Component, Debug, Clone)]
pub enum AmLayerSpec {
    /// Shape with sprite (media or color fill without stroke)
    SpriteShape {
        image_uri: String,
        is_media: bool,
        fill_color: Option<crate::schema::AmFillColor>,
        width: f32,
        height: f32,
        anchor: bevy::sprite::Anchor,
    },
    /// Shape with SDF rendering (has stroke)
    SdfShape {
        fill_color: Option<crate::schema::AmFillColor>,
        stroke_color_value: String,
        stroke_width: f32,
        stroke_join: String,
        width: f32,
        height: f32,
        pivot_x: f32,
        pivot_y: f32,
        shape_type: String,
    },
    /// Text layer
    Text {
        content: String,
        font_name: String,
        font_size: f32,
        align: String,
        fill_color: Option<crate::schema::AmFillColor>,
    },
    /// Image layer  
    Image {
        image_uri: String,
        width: f32,
        height: f32,
        anchor: bevy::sprite::Anchor,
    },
    /// Null object (no visual, always active within time range)
    Null,
    /// Embedded scene container (children managed separately)
    EmbedScene,
}

/// Blending mode for layers.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum AmBlendingMode {
    /// Normal rendering
    #[default]
    Normal,
    /// Mask layer - clips content below it to show only inside the mask (not rendered itself)
    Mask,
    /// Exclude layer - clips content below it to hide inside the mask (not rendered itself)
    Exclude,
}

/// Information about a single mask that can clip this layer.
#[derive(Debug, Clone, Default)]
pub struct AmMaskEntry {
    /// Center position of the mask in local coordinates
    pub center: Vec2,
    /// Half-size of the mask rectangle
    pub half_size: Vec2,
    /// Rotation of the mask in radians
    pub rotation: f32,
    /// Scale of the mask
    pub scale: Vec2,
    /// Whether this is a circle/ellipse mask (false = rectangle)
    pub is_circle: bool,
    /// Start time of the mask layer (ms)
    pub start_time: i32,
    /// End time of the mask layer (ms)
    pub end_time: i32,
    /// The ID of the mask layer
    pub mask_layer_id: u64,
    /// Whether this is an exclude mask (inverted - hide inside, show outside)
    pub is_exclude: bool,
}

/// Information about active masks that can clip this layer.
/// A layer can be affected by multiple masks at different times.
#[derive(Debug, Clone, Default, Component)]
pub struct AmMaskInfo {
    /// List of all masks that can affect this layer
    pub masks: Vec<AmMaskEntry>,
}

impl AmMaskInfo {
    /// Get the active mask for the given time (ms).
    /// Returns None if no mask is active at this time.
    pub fn get_active_mask(&self, time_ms: u64) -> Option<&AmMaskEntry> {
        self.masks
            .iter()
            .find(|m| time_ms >= m.start_time as u64 && time_ms < m.end_time as u64)
    }

    /// Get all active masks for the given time (ms).
    /// Returns masks sorted by z-order (lowest first).
    /// Multiple masks can be active simultaneously for composite effects.
    pub fn get_active_masks(&self, time_ms: u64) -> Vec<&AmMaskEntry> {
        self.masks
            .iter()
            .filter(|m| time_ms >= m.start_time as u64 && time_ms < m.end_time as u64)
            .collect()
    }
}

/// Complete layer definition for deferred spawning.
/// This stores all information needed to create an entity when the layer becomes active.
#[derive(Debug, Clone)]
pub struct PendingLayer {
    /// Layer ID
    pub id: u64,
    /// Layer label
    pub label: String,
    /// Parent layer ID (0 = root)
    pub parent: u64,
    /// Start time in ms
    pub start_time: i32,
    /// End time in ms  
    pub end_time: i32,
    /// Initial transform
    pub transform: Transform,
    /// Animation data
    pub animated: AmAnimated,
    /// Visual specification
    pub spec: AmLayerSpec,
    /// Z-order index
    pub z_index: f32,
    /// Child pending layers (for embed scenes)
    pub children: Vec<PendingLayer>,
    /// Blending mode (normal, mask, etc.)
    pub blending_mode: AmBlendingMode,
    /// Active mask info (if this layer is clipped by a mask)
    pub mask_info: Option<AmMaskInfo>,
    /// Palette map params (if this layer has palette map effect)
    pub palette_params: Option<AmPaletteMapParams>,
    /// For EmbedScene: internal scene dimensions for RTT clipping
    pub embed_scene_size: Option<(f32, f32)>,
    /// The embed layer ID this content belongs to (0 = not in embed, uses spatial decoupling).
    /// When set, this layer is rendered to the embed's RTT and not parented to embed entity.
    pub containing_embed_id: u64,
    /// Whether this layer came from a deeply nested scene (nesting_depth > 1).
    /// Layers from deeply nested scenes should not be spatially decoupled at outer levels
    /// because they need to be Bevy children so transforms of intermediate embeds propagate.
    pub from_deeply_nested_scene: bool,
}

/// Configuration for scene building.
#[derive(Debug, Clone)]
pub struct AmSceneConfig {
    /// Canvas width.
    pub canvas_width: f32,
    /// Canvas height.
    pub canvas_height: f32,
    /// Whether to flip Y axis (AM uses top-left origin).
    pub flip_y: bool,
    /// Z-spacing between layers at this nesting level.
    pub z_spacing: f32,
    /// Time offset from parent scene (for embedded scenes).
    /// Used for animation interpolation: local_time = (global - time_offset) * speed
    pub time_offset: i32,
    /// Lifecycle offset for visibility (not affected by speed).
    /// Used for spawn/despawn: lifecycle_time = global - lifecycle_offset
    pub lifecycle_offset: i32,
    /// Cumulative speed multiplier from parent scenes.
    /// Local time = (global_time - time_offset) * speed_multiplier
    pub speed_multiplier: f32,
    /// Nesting depth (0 = root scene, 1 = first level embed, etc.)
    pub nesting_depth: u32,
}

impl Default for AmSceneConfig {
    fn default() -> Self {
        Self {
            canvas_width: 1280.0,
            canvas_height: 960.0,
            flip_y: true,
            z_spacing: 0.1, // Base spacing for root scene
            time_offset: 0,
            lifecycle_offset: 0,
            speed_multiplier: 1.0,
            nesting_depth: 0,
        }
    }
}

/// Component to store palette map effect parameters for animation.
#[derive(Component, Debug, Clone)]
pub struct AmPaletteMapParams {
    /// Number of colors to use (1-8)
    pub count: u8,
    /// Whether to enable shade variations
    pub shades: bool,
    /// Palette colors (up to 8)
    pub colors: [Vec4; 8],
    /// Initial alpha value from the effect
    pub initial_alpha: f32,
}

impl AmPaletteMapParams {
    /// Create from extracted PaletteMapParams
    pub fn from_params(params: &super::effects::PaletteMapParams) -> Self {
        // Get initial alpha from keyframes if available, otherwise from static value
        let initial_alpha = if !params.alpha.keyframes.is_empty() {
            // Use the first keyframe's value as initial
            params.alpha.keyframes[0].value.parse().unwrap_or(0.0)
        } else {
            params.alpha.value.unwrap_or(1.0)
        };

        Self {
            count: params.count,
            shades: params.shades,
            colors: params.colors,
            initial_alpha,
        }
    }
}

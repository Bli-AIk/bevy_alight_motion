//! RTT (Render-to-Texture) Effect System for bevy_alight_motion.
//!
//! This module implements the Ping-Pong double buffering architecture for
//! stacking arbitrary effects on layers and groups.
//!
//! ## Architecture Overview
//!
//! Every visual layer in AM potentially has effects. Effects are processed in order:
//!
//! ```text
//! [Source Texture] -> [Effect 1] -> [Effect 2] -> ... -> [Final Output]
//! ```
//!
//! We use two RTT textures (Tex_A and Tex_B) that alternate as input/output:
//!
//! - Pass 1: Source -> Tex_A
//! - Pass 2: Tex_A -> Tex_B  
//! - Pass 3: Tex_B -> Tex_A
//! - Final: Display Tex_A (or Tex_B if odd number of passes)
//!
//! ## Design Decisions
//!
//! 1. **Single-Pass Optimization**: When a layer has only 1-3 basic effects (mask, wipe, stretch),
//!    we combine them in a single shader (`unified_effect.wgsl`) for performance.
//!
//! 2. **Multi-Pass for Complex Cases**: When effects exceed the unified shader's capabilities,
//!    or when groups have their own effects, we use the RTT pipeline.
//!
//! 3. **Always RTT-Ready**: All code paths assume RTT architecture. There is no "legacy mode".

use bevy::camera::RenderTarget;
use bevy::camera::ScalingMode;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};
use std::collections::HashMap;

// ============================================================================
// Effect Parameters
// ============================================================================

/// Parameters for Wipe effect
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WipeParams {
    /// Start position (0.0-1.0)
    pub start: f32,
    /// End position (0.0-1.0)
    pub end: f32,
    /// Angle in radians
    pub angle: f32,
    /// Edge feather amount
    pub feather: f32,
}

/// Parameters for Stretch Segment effect
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StretchSegmentParams {
    /// Stretch amount in pixels
    pub stretch: f32,
    /// Angle of split line in radians
    pub angle: f32,
    /// Offset of split line in pixels
    pub offset: f32,
    /// Smooth transition width
    pub smooth: f32,
}

/// Parameters for Mask effect (rectangular clip)
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MaskParams {
    /// Mask center X
    pub center_x: f32,
    /// Mask center Y
    pub center_y: f32,
    /// Mask half-width
    pub half_width: f32,
    /// Mask half-height
    pub half_height: f32,
}

/// All supported effect types
#[derive(Debug, Clone, PartialEq)]
pub enum EffectType {
    Wipe(WipeParams),
    StretchSegment(StretchSegmentParams),
    Mask(MaskParams),
    // Future: Blur, ColorAdjust, etc.
}

// ============================================================================
// Core Components
// ============================================================================

/// Component marking an entity that has effects applied.
///
/// This is the primary interface for effect processing. Add effects to the chain,
/// and the system will handle rendering them in order.
#[derive(Component, Debug, Clone, Default)]
pub struct EffectLayer {
    /// Ordered list of effects to apply
    pub effects: Vec<EffectType>,
    /// Source texture dimensions (used for RTT buffer sizing)
    pub source_size: Vec2,
    /// Dirty flag - set to true when effects need re-processing
    pub dirty: bool,
}

impl EffectLayer {
    /// Create a new effect layer with given source size
    pub fn new(source_size: Vec2) -> Self {
        Self {
            effects: Vec::new(),
            source_size,
            dirty: true,
        }
    }

    /// Add an effect to the chain
    pub fn with_effect(mut self, effect: EffectType) -> Self {
        self.effects.push(effect);
        self
    }

    pub fn has_effects(&self) -> bool {
        !self.effects.is_empty()
    }
    pub fn effect_count(&self) -> usize {
        self.effects.len()
    }
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    // Effect type checks
    pub fn has_wipe(&self) -> bool {
        self.effects
            .iter()
            .any(|e| matches!(e, EffectType::Wipe(_)))
    }
    pub fn has_stretch_segment(&self) -> bool {
        self.effects
            .iter()
            .any(|e| matches!(e, EffectType::StretchSegment(_)))
    }
    pub fn has_mask(&self) -> bool {
        self.effects
            .iter()
            .any(|e| matches!(e, EffectType::Mask(_)))
    }

    // Getters
    pub fn get_wipe(&self) -> Option<&WipeParams> {
        self.effects.iter().find_map(|e| match e {
            EffectType::Wipe(p) => Some(p),
            _ => None,
        })
    }
    pub fn get_stretch_segment(&self) -> Option<&StretchSegmentParams> {
        self.effects.iter().find_map(|e| match e {
            EffectType::StretchSegment(p) => Some(p),
            _ => None,
        })
    }
    pub fn get_mask(&self) -> Option<&MaskParams> {
        self.effects.iter().find_map(|e| match e {
            EffectType::Mask(p) => Some(p),
            _ => None,
        })
    }

    // Setters (create if not exists)
    pub fn set_wipe(&mut self, params: WipeParams) {
        if let Some(existing) = self.effects.iter_mut().find_map(|e| match e {
            EffectType::Wipe(p) => Some(p),
            _ => None,
        }) {
            *existing = params;
        } else {
            self.effects.push(EffectType::Wipe(params));
        }
        self.dirty = true;
    }

    pub fn set_stretch_segment(&mut self, params: StretchSegmentParams) {
        if let Some(existing) = self.effects.iter_mut().find_map(|e| match e {
            EffectType::StretchSegment(p) => Some(p),
            _ => None,
        }) {
            *existing = params;
        } else {
            self.effects.push(EffectType::StretchSegment(params));
        }
        self.dirty = true;
    }

    pub fn set_mask(&mut self, params: MaskParams) {
        if let Some(existing) = self.effects.iter_mut().find_map(|e| match e {
            EffectType::Mask(p) => Some(p),
            _ => None,
        }) {
            *existing = params;
        } else {
            self.effects.push(EffectType::Mask(params));
        }
        self.dirty = true;
    }
}

/// Component storing the original source texture for RTT processing
#[derive(Component, Debug, Clone)]
pub struct EffectSourceTexture(pub Handle<Image>);

/// Component storing the final output texture after all effects
#[derive(Component, Debug, Clone)]
pub struct EffectOutputTexture(pub Handle<Image>);

// ============================================================================
// Ping-Pong Buffer
// ============================================================================

/// Double buffer for effect pass chaining.
/// Alternates between two textures to avoid read-while-write conflicts.
#[derive(Component, Debug)]
pub struct PingPongBuffer {
    pub tex_a: Handle<Image>,
    pub tex_b: Handle<Image>,
    pub size: Vec2,
    /// 0 = read from A, write to B; 1 = read from B, write to A
    pub read_index: usize,
}

impl PingPongBuffer {
    pub fn new(images: &mut Assets<Image>, size: Vec2) -> Self {
        let tex_a = Self::create_rtt(images, size, "ping_pong_a");
        let tex_b = Self::create_rtt(images, size, "ping_pong_b");
        Self {
            tex_a,
            tex_b,
            size,
            read_index: 0,
        }
    }

    fn create_rtt(images: &mut Assets<Image>, size: Vec2, label: &'static str) -> Handle<Image> {
        let extent = Extent3d {
            width: size.x.max(1.0) as u32,
            height: size.y.max(1.0) as u32,
            depth_or_array_layers: 1,
        };

        let mut image = Image {
            texture_descriptor: TextureDescriptor {
                label: Some(label),
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            ..default()
        };
        image.resize(extent);
        images.add(image)
    }

    /// Get the current read (input) texture
    pub fn read_texture(&self) -> &Handle<Image> {
        if self.read_index == 0 {
            &self.tex_a
        } else {
            &self.tex_b
        }
    }

    /// Get the current write (output) texture
    pub fn write_texture(&self) -> &Handle<Image> {
        if self.read_index == 0 {
            &self.tex_b
        } else {
            &self.tex_a
        }
    }

    /// Swap after completing a pass
    pub fn swap(&mut self) {
        self.read_index = 1 - self.read_index;
    }

    /// Reset to initial state
    pub fn reset(&mut self) {
        self.read_index = 0;
    }

    /// Resize buffers if needed
    pub fn resize_if_needed(&mut self, images: &mut Assets<Image>, new_size: Vec2) {
        if (self.size - new_size).length_squared() < 0.01 {
            return;
        }

        let extent = Extent3d {
            width: new_size.x.max(1.0) as u32,
            height: new_size.y.max(1.0) as u32,
            depth_or_array_layers: 1,
        };

        if let Some(img) = images.get_mut(&self.tex_a) {
            img.resize(extent);
        }
        if let Some(img) = images.get_mut(&self.tex_b) {
            img.resize(extent);
        }
        self.size = new_size;
    }
}

// ============================================================================
// Systems
// ============================================================================

/// Automatically create ping-pong buffers for entities with effects
pub fn setup_effect_buffers_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    query: Query<(Entity, &EffectLayer), Without<PingPongBuffer>>,
) {
    for (entity, effect_layer) in query.iter() {
        if effect_layer.has_effects() {
            let buffer = PingPongBuffer::new(&mut images, effect_layer.source_size);
            commands.entity(entity).insert(buffer);
            bevy::log::debug!(
                "Created RTT buffer for {:?}, size {:?}",
                entity,
                effect_layer.source_size
            );
        }
    }
}

/// Update buffer sizes when layer size changes
pub fn update_effect_buffers_system(
    mut images: ResMut<Assets<Image>>,
    mut query: Query<(&EffectLayer, &mut PingPongBuffer), Changed<EffectLayer>>,
) {
    for (effect_layer, mut buffer) in query.iter_mut() {
        buffer.resize_if_needed(&mut images, effect_layer.source_size);
    }
}

/// Mark layers dirty when changed (triggers re-render)
pub fn mark_dirty_on_change_system(mut query: Query<&mut EffectLayer, Changed<EffectLayer>>) {
    for mut layer in query.iter_mut() {
        layer.dirty = true;
    }
}

// ============================================================================
// Conversion Helpers
// ============================================================================

pub fn vec4_to_wipe_params(v: Vec4) -> WipeParams {
    WipeParams {
        start: v.x,
        end: v.y,
        angle: v.z,
        feather: v.w,
    }
}

pub fn wipe_params_to_vec4(p: &WipeParams) -> Vec4 {
    Vec4::new(p.start, p.end, p.angle, p.feather)
}

pub fn vec4_to_stretch_params(v: Vec4) -> StretchSegmentParams {
    StretchSegmentParams {
        angle: v.x,
        stretch: v.y,
        offset: v.z,
        smooth: v.w,
    }
}

pub fn stretch_params_to_vec4(p: &StretchSegmentParams) -> Vec4 {
    Vec4::new(p.angle, p.stretch, p.offset, p.smooth)
}

pub fn vec4_to_mask_params(v: Vec4) -> MaskParams {
    MaskParams {
        center_x: v.x,
        center_y: v.y,
        half_width: v.z,
        half_height: v.w,
    }
}

pub fn mask_params_to_vec4(p: &MaskParams) -> Vec4 {
    Vec4::new(p.center_x, p.center_y, p.half_width, p.half_height)
}

// ============================================================================
// Plugin
// ============================================================================

/// Plugin for RTT effect rendering infrastructure
pub struct EffectRenderPlugin;

impl Plugin for EffectRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EmbedSceneRenderLayerPool>()
            .add_systems(
                Update,
                (
                    setup_effect_buffers_system,
                    update_effect_buffers_system,
                    mark_dirty_on_change_system,
                    setup_embed_scene_rtt_system,
                    debug_rtt_camera_projection_system,
                    propagate_render_layers_system,
                    cleanup_embed_scene_rtt_system,
                    cleanup_embed_content_system,
                ),
            );
    }
}

// ============================================================================
// EmbedScene RTT Architecture
// ============================================================================

/// Resource managing the pool of available RenderLayers for embedScenes.
/// Bevy supports up to 32 RenderLayers (0-31). Layer 0 is reserved for the main camera.
/// We use layers 1-31 for embedScene RTT rendering.
#[derive(Resource, Default)]
pub struct EmbedSceneRenderLayerPool {
    /// Bitset tracking which layers are in use (bit N = layer N+1)
    used_layers: u32,
}

impl EmbedSceneRenderLayerPool {
    /// Allocate a render layer. Returns None if all layers are in use.
    pub fn allocate(&mut self) -> Option<u8> {
        // Find first available layer (layers 1-31)
        for i in 0..31 {
            if (self.used_layers & (1 << i)) == 0 {
                self.used_layers |= 1 << i;
                return Some(i + 1); // Return layer index (1-31)
            }
        }
        None
    }

    /// Release a render layer back to the pool.
    pub fn release(&mut self, layer: u8) {
        if (1..=31).contains(&layer) {
            self.used_layers &= !(1 << (layer - 1));
        }
    }

    /// Check how many layers are currently in use.
    #[allow(dead_code)]
    pub fn used_count(&self) -> u32 {
        self.used_layers.count_ones()
    }
}

/// Component for embedScene entities that need RTT rendering.
/// Stores the render infrastructure for clipping content to scene bounds.
#[derive(Component)]
pub struct EmbedSceneRtt {
    /// The render target texture
    pub render_texture: Handle<Image>,
    /// The camera entity that renders to this RTT
    pub camera_entity: Entity,
    /// The render layer index (1-31)
    pub render_layer: u8,
    /// Scene dimensions (for camera orthographic projection)
    pub scene_width: f32,
    pub scene_height: f32,
}

/// Marker component for embedScene RTT cameras
#[derive(Component)]
pub struct EmbedSceneRttCamera {
    /// Reference to parent embedScene entity
    pub embed_entity: Entity,
    /// The render layer index (1-31) - stored here for cleanup when embed is despawned
    pub render_layer: u8,
}

/// Marker component indicating an entity needs RTT setup
#[derive(Component)]
pub struct NeedsEmbedSceneRtt {
    pub scene_width: f32,
    pub scene_height: f32,
}

/// System to set up RTT infrastructure for embedScenes marked with NeedsEmbedSceneRtt.
pub fn setup_embed_scene_rtt_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut layer_pool: ResMut<EmbedSceneRenderLayerPool>,
    query: Query<(Entity, &NeedsEmbedSceneRtt, &Transform), Without<EmbedSceneRtt>>,
    pending_query: Query<&crate::scene::AmPendingLayers>,
    parent_query: Query<&ChildOf>,
    embed_rtt_query: Query<&EmbedSceneRtt>,
) {
    // Get the RTT cameras container from AmPendingLayers
    let rtt_cameras_container = pending_query
        .iter()
        .next()
        .and_then(|p| p.rtt_cameras_container);

    bevy::log::trace!(
        "[RTT] setup_embed_scene_rtt_system: {} embeds need RTT setup",
        query.iter().count()
    );

    for (entity, needs_rtt, embed_transform) in query.iter() {
        // Log embed transform for debugging
        bevy::log::trace!(
            "[RTT] Embed {:?} transform: scale=({:.3},{:.3}), pos=({:.1},{:.1})",
            entity,
            embed_transform.scale.x,
            embed_transform.scale.y,
            embed_transform.translation.x,
            embed_transform.translation.y
        );

        // Try to allocate a render layer
        let Some(render_layer) = layer_pool.allocate() else {
            bevy::log::warn!(
                "No available render layers for embedScene {:?}. Max 31 concurrent embedScenes supported.",
                entity
            );
            continue;
        };

        // Create RTT texture
        let size = Extent3d {
            width: needs_rtt.scene_width.max(1.0) as u32,
            height: needs_rtt.scene_height.max(1.0) as u32,
            depth_or_array_layers: 1,
        };

        let mut render_texture = Image {
            texture_descriptor: TextureDescriptor {
                label: Some("embed_scene_rtt"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            ..default()
        };
        render_texture.resize(size);
        let render_texture_handle = images.add(render_texture);
        let render_layer_usize = render_layer as usize;

        // Create RTT camera with Fixed scaling mode to match embed's internal scene size
        // The camera is NOT added as a child of the embed to avoid inheriting scale/rotation.
        // Instead, sync_rtt_camera_position_system updates the camera's world position to follow
        // the embed's GlobalTransform translation.
        let camera_entity = commands
            .spawn((
                Name::new(format!("EmbedSceneRttCamera[layer={}]", render_layer)),
                EmbedSceneRttCamera {
                    embed_entity: entity,
                    render_layer, // Store for cleanup
                },
                Camera2d,
                Camera {
                    target: RenderTarget::Image(render_texture_handle.clone().into()),
                    clear_color: ClearColorConfig::Custom(Color::NONE),
                    order: -(render_layer as isize), // Render before main camera
                    ..default()
                },
                // Fixed scaling mode so projection area matches RTT texture size exactly
                Projection::Orthographic(OrthographicProjection {
                    scaling_mode: ScalingMode::Fixed {
                        width: needs_rtt.scene_width,
                        height: needs_rtt.scene_height,
                    },
                    near: -1000.0,
                    far: 1000.0,
                    ..OrthographicProjection::default_2d()
                }),
                // Camera only renders this specific layer
                RenderLayers::layer(render_layer_usize),
                // Camera positioned at center of scene - will be updated by sync_rtt_camera_position_system
                Transform::from_xyz(0.0, 0.0, 1000.0),
            ))
            .id();

        // Add RTT camera to the container if available
        if let Some(container) = rtt_cameras_container {
            commands.entity(container).add_child(camera_entity);
        }

        // Determine which RenderLayers to use for the embed's Sprite.
        // If this embed is a child of another embed (nested), it should render
        // to the parent embed's RTT layer so the parent embed can see it.
        // If it's a top-level embed, it renders to layer 0 (main camera).
        let sprite_render_layer = if let Ok(child_of) = parent_query.get(entity) {
            let parent = child_of.parent();
            // Check if parent has EmbedSceneRtt
            if let Ok(parent_rtt) = embed_rtt_query.get(parent) {
                bevy::log::trace!(
                    "[RTT] Embed {:?} is child of embed {:?} with RTT layer {}, using that layer for sprite",
                    entity,
                    parent,
                    parent_rtt.render_layer
                );
                RenderLayers::layer(parent_rtt.render_layer as usize)
            } else {
                // Parent is not an embed with RTT, use layer 0
                bevy::log::trace!(
                    "[RTT] Embed {:?} has parent {:?} but no RTT, using layer 0",
                    entity,
                    parent
                );
                RenderLayers::layer(0)
            }
        } else {
            // No parent, use layer 0
            bevy::log::trace!("[RTT] Embed {:?} has no parent, using layer 0", entity);
            RenderLayers::layer(0)
        };

        // Add EmbedSceneRtt component and remove the marker
        commands
            .entity(entity)
            .remove::<NeedsEmbedSceneRtt>()
            .insert((
                EmbedSceneRtt {
                    render_texture: render_texture_handle.clone(),
                    camera_entity,
                    render_layer,
                    scene_width: needs_rtt.scene_width,
                    scene_height: needs_rtt.scene_height,
                },
                // Add sprite to display RTT output
                Sprite {
                    image: render_texture_handle,
                    custom_size: Some(Vec2::new(needs_rtt.scene_width, needs_rtt.scene_height)),
                    ..default()
                },
                // Use appropriate RenderLayers based on nesting
                sprite_render_layer,
            ));

        bevy::log::trace!(
            "[RTT] Set up RTT for embedScene {:?}: layer={}, size={}x{}",
            entity,
            render_layer,
            needs_rtt.scene_width,
            needs_rtt.scene_height
        );
    }
}

/// System to fix RenderLayers for nested embeds.
/// Runs after setup_embed_scene_rtt_system and ApplyDeferred so all embeds have their RTT components.
pub fn fix_nested_embed_render_layers_system(
    mut commands: Commands,
    // Query embeds that have RTT but might need their RenderLayers fixed
    embed_query: Query<(Entity, &EmbedSceneRtt, &RenderLayers)>,
    parent_query: Query<&ChildOf>,
    embed_rtt_query: Query<&EmbedSceneRtt>,
) {
    for (entity, _rtt, current_layers) in embed_query.iter() {
        // Check if this embed is a child of another embed
        if let Ok(child_of) = parent_query.get(entity) {
            let parent = child_of.parent();
            if let Ok(parent_rtt) = embed_rtt_query.get(parent) {
                // This embed is nested inside another embed
                let expected_layer = RenderLayers::layer(parent_rtt.render_layer as usize);
                if *current_layers != expected_layer {
                    bevy::log::trace!(
                        "[RTT] Fixing nested embed {:?} RenderLayers: was {:?}, now layer {}",
                        entity,
                        current_layers,
                        parent_rtt.render_layer
                    );
                    commands.entity(entity).insert(expected_layer);
                }
            }
        }
    }
}

/// Debug system to verify RTT camera projection settings
pub fn debug_rtt_camera_projection_system(
    camera_query: Query<(Entity, &EmbedSceneRttCamera, &Projection)>,
) {
    static mut FRAME_COUNT: u32 = 0;
    unsafe {
        FRAME_COUNT += 1;
        if FRAME_COUNT != 5 {
            // Log on frame 5 only
            return;
        }
    }

    for (entity, _rtt_cam, projection) in camera_query.iter() {
        match projection {
            Projection::Orthographic(ortho) => {
                bevy::log::trace!(
                    "[RTT DEBUG] Camera {:?} projection: {:?}, area={}x{}",
                    entity,
                    ortho.scaling_mode,
                    ortho.area.width(),
                    ortho.area.height()
                );
            }
            _ => {
                bevy::log::warn!(
                    "[RTT DEBUG] Camera {:?} has non-orthographic projection!",
                    entity
                );
            }
        }
    }
}

/// System to propagate RenderLayers to embed content entities.
///
/// With spatial decoupling, embed content entities are NOT Bevy children of the embed entity.
/// Instead, they have `AmEmbedContentMarker` component that identifies which embed they belong to.
/// This system assigns the correct RenderLayers so content renders to the embed's RTT camera.
pub fn propagate_render_layers_system(
    mut commands: Commands,
    embed_query: Query<(Entity, &EmbedSceneRtt)>,
    content_query: Query<(Entity, &crate::scene::AmEmbedContentMarker), Without<EmbedSceneRtt>>,
    render_layers_query: Query<&RenderLayers>,
) {
    // Build a map of embed entity -> render layer
    let embed_layers: HashMap<Entity, u8> = embed_query
        .iter()
        .map(|(entity, rtt)| (entity, rtt.render_layer))
        .collect();

    // Debug: Log embed and content counts
    static mut FRAME_COUNT: u32 = 0;
    unsafe {
        FRAME_COUNT += 1;
        if FRAME_COUNT % 300 == 1 {
            bevy::log::trace!(
                "[RenderLayers] embeds with RTT: {}, content entities: {}",
                embed_layers.len(),
                content_query.iter().count()
            );
        }
    }

    // Assign RenderLayers to all embed content based on their marker
    for (content_entity, marker) in content_query.iter() {
        if let Some(&render_layer) = embed_layers.get(&marker.embed_entity) {
            let target_layer = RenderLayers::layer(render_layer as usize);

            // Check if already has correct RenderLayers
            let needs_update = match render_layers_query.get(content_entity) {
                Ok(current) => *current != target_layer,
                Err(_) => true,
            };

            if needs_update {
                // Insert RenderLayers and make visible (content starts Hidden until RTT is ready)
                commands.entity(content_entity).insert((
                    target_layer,
                    Visibility::Inherited, // Now safe to show - will render to RTT camera
                ));
                bevy::log::trace!(
                    "[RenderLayers] Assigned layer {} to embed content {:?} (embed {:?}), now visible",
                    render_layer,
                    content_entity,
                    marker.embed_entity
                );
            }
        } else {
            // Embed not found in query - may not have EmbedSceneRtt yet
            unsafe {
                if FRAME_COUNT % 300 == 1 {
                    bevy::log::warn!(
                        "[RenderLayers] Content {:?} belongs to embed {:?} which has no RTT yet",
                        content_entity,
                        marker.embed_entity
                    );
                }
            }
        }
    }
}

/// System to propagate RenderLayers to Bevy children of embeds (nested embed content).
///
/// When embeds are nested, their content becomes Bevy children (not spatially decoupled).
/// This system ensures all descendants of an embed get the correct RenderLayers so they
/// render to the embed's RTT camera.
///
/// This handles the case where:
/// 1. Embed A (with RTT layer X) contains Embed B (with RTT layer Y)
/// 2. Embed B's content is Bevy children of Embed B
/// 3. Embed B itself needs RenderLayers X to render into Embed A's RTT
/// 4. Embed B's children need RenderLayers Y to render into Embed B's RTT
pub fn propagate_render_layers_to_children_system(
    mut commands: Commands,
    embed_query: Query<(Entity, &EmbedSceneRtt)>,
    children_query: Query<&Children>,
    render_layers_query: Query<&RenderLayers>,
    // Query for entities that are NOT embeds (don't have EmbedSceneRtt)
    non_embed_query: Query<Entity, Without<EmbedSceneRtt>>,
) {
    // For each embed with RTT, propagate its render layer to DIRECT children.
    // This includes:
    // 1. Non-embed content - they render to this embed's RTT
    // 2. Nested embeds - their Sprite (displaying their RTT output) needs to render to this embed's RTT
    //
    // We do NOT recurse into nested embeds' children because nested embeds handle their own content.
    //
    // Note: We query EmbedSceneRtt separately from Children because Children may not exist
    // if the embed has no Bevy children yet.

    bevy::log::trace!(
        "[RenderLayers] propagate_render_layers_to_children_system: found {} embeds with RTT",
        embed_query.iter().count()
    );

    for (embed_entity, rtt) in embed_query.iter() {
        // Check if this embed has children
        let Ok(children) = children_query.get(embed_entity) else {
            bevy::log::trace!(
                "[RenderLayers] Embed {:?} has RTT layer {} but no Children",
                embed_entity,
                rtt.render_layer
            );
            continue;
        };

        let target_layer = RenderLayers::layer(rtt.render_layer as usize);

        bevy::log::trace!(
            "[RenderLayers] Processing embed {:?} with {} children, layer={}",
            embed_entity,
            children.len(),
            rtt.render_layer
        );

        // Process all direct children - both embeds and non-embeds need the parent's RenderLayers
        for child_entity in children.iter() {
            // Assign this parent embed's RenderLayers to the child
            let needs_update = match render_layers_query.get(child_entity) {
                Ok(current) => *current != target_layer,
                Err(_) => true,
            };

            if needs_update {
                commands.entity(child_entity).insert(target_layer.clone());
                bevy::log::trace!(
                    "[RenderLayers] Propagated layer {} to child {:?} of embed {:?}",
                    rtt.render_layer,
                    child_entity,
                    embed_entity
                );
            }

            // For non-embed children, also process their descendants (but stop at nested embeds)
            if non_embed_query.get(child_entity).is_ok() {
                // Recurse into non-embed children
                let mut to_process: Vec<Entity> = Vec::new();
                if let Ok(grandchildren) = children_query.get(child_entity) {
                    to_process.extend(grandchildren.to_vec());
                }

                while let Some(entity) = to_process.pop() {
                    // Only process non-embed descendants
                    if non_embed_query.get(entity).is_ok() {
                        let needs_update = match render_layers_query.get(entity) {
                            Ok(current) => *current != target_layer,
                            Err(_) => true,
                        };

                        if needs_update {
                            commands.entity(entity).insert(target_layer.clone());
                            bevy::log::trace!(
                                "[RenderLayers] Propagated layer {} to descendant {:?} of embed {:?}",
                                rtt.render_layer,
                                entity,
                                embed_entity
                            );
                        }

                        // Continue to grandchildren
                        if let Ok(grandchildren) = children_query.get(entity) {
                            to_process.extend(grandchildren.to_vec());
                        }
                    }
                    // If it's an embed, we still assigned it the parent's layer above,
                    // but we don't recurse into its children (it handles those itself)
                }
            }
        }
    }
}

/// System to sync RTT camera positions with their embed's GlobalTransform.
/// This ensures that RTT cameras follow their embed's world position, allowing them to
/// "see" the embed's Bevy children (which have world positions relative to the embed).
/// The camera only follows translation, not rotation or scale, to avoid distorting the RTT output.
pub fn sync_rtt_camera_position_system(
    embed_query: Query<(&EmbedSceneRtt, &GlobalTransform)>,
    mut camera_query: Query<(&EmbedSceneRttCamera, &mut Transform)>,
) {
    for (camera_marker, mut camera_transform) in camera_query.iter_mut() {
        if let Ok((_, embed_global)) = embed_query.get(camera_marker.embed_entity) {
            // Set camera position to follow embed's world position
            // Only copy translation, not rotation or scale
            let embed_translation = embed_global.translation();
            camera_transform.translation = Vec3::new(
                embed_translation.x,
                embed_translation.y,
                1000.0, // Keep camera at z=1000 for proper depth
            );
        }
    }
}

/// System to clean up embed content when embeds are despawned.
/// Since content entities are spatially decoupled (not Bevy children), they won't be
/// automatically despawned when the embed is despawned. This system handles cleanup.
pub fn cleanup_embed_content_system(
    mut commands: Commands,
    content_query: Query<(Entity, &crate::scene::AmEmbedContentMarker)>,
    embed_exists_query: Query<Entity>,
) {
    // Find content whose embed entity no longer exists at all
    // Note: We check if the entity exists, not if it has EmbedSceneRtt
    // This is because EmbedSceneRtt might be added asynchronously
    for (content_entity, marker) in content_query.iter() {
        if embed_exists_query.get(marker.embed_entity).is_err() {
            // Embed entity no longer exists (despawned), despawn content
            bevy::log::debug!(
                "Despawning orphaned embed content {:?} (embed entity {:?} no longer exists)",
                content_entity,
                marker.embed_entity
            );
            commands.entity(content_entity).despawn();
        }
    }
}

/// System to clean up RTT resources when embedScenes are despawned.
pub fn cleanup_embed_scene_rtt_system(
    mut commands: Commands,
    mut layer_pool: ResMut<EmbedSceneRenderLayerPool>,
    mut removed: RemovedComponents<EmbedSceneRtt>,
    rtt_query: Query<&EmbedSceneRtt>,
    camera_query: Query<(Entity, &EmbedSceneRttCamera)>,
) {
    for entity in removed.read() {
        // Log that RTT was removed - we'll clean up the camera in the next part
        bevy::log::debug!("EmbedSceneRtt removed from {:?}", entity);
    }

    // Clean up orphaned RTT cameras (their embed_entity no longer exists or has RTT)
    for (camera_entity, camera_marker) in camera_query.iter() {
        let should_cleanup = rtt_query.get(camera_marker.embed_entity).is_err();
        if should_cleanup {
            // Release render layer back to pool
            layer_pool.release(camera_marker.render_layer);
            bevy::log::debug!(
                "Released render layer {} from orphaned RTT camera {:?} for embed {:?}",
                camera_marker.render_layer,
                camera_entity,
                camera_marker.embed_entity
            );
            commands.entity(camera_entity).despawn();
        }
    }
}

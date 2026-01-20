//! # rtt.rs
//!
//! # RTT 渲染模块 - 混合渲染管线架构
//!
//! Hybrid Rendering Pipeline for embed scenes and effects.
//! 嵌入场景和效果的混合渲染管线。
//!
//! ## Architecture Philosophy
//! 
//! **Default Flat, Isolate on Demand** (默认扁平，按需隔离):
//! - By default, all content renders to Layer 0 (the main camera's layer)
//! - Only allocate separate RenderLayers when mathematically necessary
//! - Use Z-index sorting within shared layers for proper depth ordering
//!
//! ## Render Strategies
//!
//! 1. **Direct**: No isolation needed. Content inherits parent's layer.
//! 2. **Stencil**: Clipping via GPU stencil/scissor test, still on parent's layer.
//! 3. **Composite**: Full RTT isolation with dedicated RenderLayer.

use bevy::camera::RenderTarget;
use bevy::camera::ScalingMode;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};
use std::collections::HashMap;


use super::types::*;

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
                    // Hybrid Rendering Pipeline systems
                    evaluate_render_strategy_system,
                    setup_embed_scene_rtt_system,
                    debug_rtt_camera_projection_system,
                    propagate_render_layers_system,
                    propagate_render_layers_to_children_system, // NEW: for Bevy children of embeds
                    cleanup_embed_scene_rtt_system,
                    cleanup_embed_content_system,
                ),
            );
    }
}

// ============================================================================
// Hybrid Rendering Pipeline Architecture
// 混合渲染管线架构
// ============================================================================

/// Resource managing the pool of available RenderLayers for Composite strategy.
/// 
/// Bevy supports up to 32 RenderLayers (0-31). Layer 0 is reserved for the main camera.
/// We use layers 1-31 for embedScene RTT rendering (Composite strategy only).
/// 
/// With the Hybrid Pipeline, most embeds use Direct strategy and share Layer 0,
/// so we rarely exhaust the 31 available layers.
#[derive(Resource, Default)]
pub struct EmbedSceneRenderLayerPool {
    /// Bitset tracking which layers are in use (bit N = layer N+1)
    used_layers: u32,
    /// Count of embeds waiting for a layer (for diagnostics)
    waiting_count: u32,
}

impl EmbedSceneRenderLayerPool {
    /// Acquire a render layer for Composite strategy.
    /// Returns None if all layers are in use (pool exhausted).
    pub fn acquire(&mut self) -> Option<u8> {
        // Find first available layer (layers 1-31)
        for i in 0..31 {
            if (self.used_layers & (1 << i)) == 0 {
                self.used_layers |= 1 << i;
                return Some(i + 1); // Return layer index (1-31)
            }
        }
        self.waiting_count += 1;
        None
    }
    
    /// Legacy alias for acquire (for compatibility)
    pub fn allocate(&mut self) -> Option<u8> {
        self.acquire()
    }

    /// Release a render layer back to the pool.
    /// Called when: embed despawned, becomes hidden, or strategy changes to Direct.
    pub fn release(&mut self, layer: u8) {
        if (1..=31).contains(&layer) {
            self.used_layers &= !(1 << (layer - 1));
            if self.waiting_count > 0 {
                self.waiting_count -= 1;
            }
        }
    }

    /// Check how many layers are currently in use.
    #[allow(dead_code)]
    pub fn used_count(&self) -> u32 {
        self.used_layers.count_ones()
    }
    
    /// Check how many layers are available.
    #[allow(dead_code)]
    pub fn available_count(&self) -> u32 {
        31 - self.used_layers.count_ones()
    }
    
    /// Check if pool is exhausted.
    pub fn is_exhausted(&self) -> bool {
        self.used_layers == 0x7FFFFFFF // All 31 bits set
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

/// Marker component indicating an entity needs RTT setup (for Composite strategy only).
/// 
/// In the Hybrid Pipeline, this component is only added to embeds that have been
/// evaluated as requiring Composite strategy (full RTT isolation).
#[derive(Component)]
pub struct NeedsEmbedSceneRtt {
    pub scene_width: f32,
    pub scene_height: f32,
}

/// Marker component indicating an embed needs render strategy evaluation.
/// 
/// This is added to new EmbedScene entities during spawn.
/// The evaluate_render_strategy_system will analyze the embed and assign
/// a RenderStrategy, then remove this marker.
#[derive(Component)]
pub struct NeedsStrategyEvaluation {
    pub scene_width: f32,
    pub scene_height: f32,
}

// ============================================================================
// Render Strategy Evaluator
// 渲染策略评估器
// ============================================================================

/// System to evaluate render strategy for new embed scenes.
/// 
/// This is the "brain" of the Hybrid Rendering Pipeline.
/// It analyzes each embed and assigns one of three strategies:
/// - Direct: No RTT, content renders to parent's layer (90%+ of cases)
/// - Stencil: GPU stencil clipping, still on parent's layer
/// - Composite: Full RTT isolation with dedicated RenderLayer
/// 
/// Currently, we use a simple heuristic:
/// - All embeds start with Direct strategy
/// - Embeds with clipping enabled get Stencil (TODO: implement actual stencil rendering)
/// - Embeds with shader effects (blur, etc.) get Composite
/// 
/// Future enhancements:
/// - Detect shader effects and force Composite
/// - Detect complex blend modes and force Composite
/// - Implement actual stencil-based clipping for Stencil strategy
pub fn evaluate_render_strategy_system(
    mut commands: Commands,
    query: Query<(Entity, &NeedsStrategyEvaluation), Without<RenderStrategy>>,
    // Query to check if embed has any effects that require Composite
    // For now, we can check for specific components or use heuristics
) {
    // Log periodically to track query count
    use std::sync::atomic::{AtomicU32, Ordering};
    static FRAME_COUNT: AtomicU32 = AtomicU32::new(0);
    let frame = FRAME_COUNT.fetch_add(1, Ordering::Relaxed);
    if frame <= 10 || frame % 60 == 0 {
        let count = query.iter().count();
        bevy::log::trace!("[Strategy] Frame {}: query count = {}", frame, count);
    }
    
    for (entity, needs_eval) in query.iter() {
        // Determine render strategy based on embed properties
        //
        // Currently, we use Direct for ALL embeds by default.
        // This enables unlimited nesting without RenderLayer exhaustion.
        //
        // Future: Check for:
        // - Blur effects -> Composite
        // - Complex blend modes -> Composite
        // - Rectangular clipping -> Stencil
        // - Non-rectangular masks -> Composite
        
        let strategy = RenderStrategy::Direct;
        
        bevy::log::trace!(
            "[Strategy] Embed {:?} evaluated as {:?} (size={}x{})",
            entity,
            strategy,
            needs_eval.scene_width,
            needs_eval.scene_height
        );
        
        // Remove evaluation marker and assign strategy
        // For Direct strategy: set RenderLayers to layer 0 and make visible
        commands
            .entity(entity)
            .remove::<NeedsStrategyEvaluation>()
            .insert((
                strategy,
                RenderHierarchyInfo::default(),
                // Direct strategy: render to Layer 0 (main camera)
                RenderLayers::layer(0),
                // Make embed visible (it starts as Hidden)
                Visibility::Inherited,
            ));
        
        // For Composite strategy, we would add NeedsEmbedSceneRtt here.
        // But with Direct strategy, we DON'T need RTT at all!
        // Content will render directly to Layer 0 with proper Z-sorting.
    }
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
/// **Hybrid Rendering Pipeline Logic:**
/// 
/// With the new architecture, most embeds use Direct strategy and render to Layer 0.
/// Only embeds with Composite strategy have dedicated RenderLayers (1-31).
///
/// This system:
/// 1. For content belonging to Direct/Stencil embeds: Assign Layer 0, make visible
/// 2. For content belonging to Composite embeds: Assign the RTT layer, make visible
///
/// The key insight is that content WITHOUT an RTT embed now goes to Layer 0,
/// which is the opposite of the old behavior (where it stayed hidden).
pub fn propagate_render_layers_system(
    mut commands: Commands,
    // Query embeds with Composite strategy (have EmbedSceneRtt)
    composite_embed_query: Query<(Entity, &EmbedSceneRtt)>,
    // Query embeds with Direct strategy (have RenderStrategy but no EmbedSceneRtt)
    direct_embed_query: Query<(Entity, &RenderStrategy), Without<EmbedSceneRtt>>,
    // Query all embed content
    content_query: Query<(Entity, &crate::scene::AmEmbedContentMarker, Option<&RenderLayers>)>,
) {
    use std::sync::atomic::{AtomicU32, Ordering};
    static FRAME_COUNT: AtomicU32 = AtomicU32::new(0);
    let frame = FRAME_COUNT.fetch_add(1, Ordering::Relaxed);
    
    // Build a map of embed entity -> render layer for Composite embeds
    let composite_layers: HashMap<Entity, u8> = composite_embed_query
        .iter()
        .map(|(entity, rtt)| (entity, rtt.render_layer))
        .collect();
    
    // Build a set of Direct embed entities
    let direct_embeds: std::collections::HashSet<Entity> = direct_embed_query
        .iter()
        .filter(|(_, strategy)| **strategy == RenderStrategy::Direct)
        .map(|(entity, _)| entity)
        .collect();

    // Debug logging
    if frame <= 10 || frame % 60 == 0 {
        bevy::log::trace!(
            "[RenderLayers] Frame {}: Composite={}, Direct={}, content={}",
            frame,
            composite_layers.len(),
            direct_embeds.len(),
            content_query.iter().count()
        );
    }

    // Track how many updates we make
    let mut updates = 0;

    // Assign RenderLayers to all embed content
    for (content_entity, marker, current_layers) in content_query.iter() {
        // Determine target layer based on parent embed's strategy
        let target_layer = if let Some(&rtt_layer) = composite_layers.get(&marker.embed_entity) {
            // Parent embed uses Composite strategy - render to its RTT layer
            RenderLayers::layer(rtt_layer as usize)
        } else if direct_embeds.contains(&marker.embed_entity) {
            // Parent embed uses Direct strategy - render to Layer 0 (main camera)
            RenderLayers::layer(0)
        } else {
            // Parent embed hasn't been evaluated yet - skip for now
            // (This handles the case where strategy evaluation is still pending)
            continue;
        };

        // Check if update is needed
        let needs_update = match current_layers {
            Some(current) => *current != target_layer,
            None => true,
        };

        if needs_update {
            let layer_num = if composite_layers.contains_key(&marker.embed_entity) {
                composite_layers[&marker.embed_entity]
            } else {
                0
            };
            
            // Insert RenderLayers and make visible
            commands.entity(content_entity).insert((
                target_layer,
                Visibility::Inherited, // Safe to show - will render to correct layer
            ));
            
            updates += 1;
            bevy::log::trace!(
                "[RenderLayers] Assigned layer {} to content {:?} (embed {:?}), now visible",
                layer_num,
                content_entity,
                marker.embed_entity
            );
        }
    }
    
    // Log total updates made
    if updates > 0 {
        bevy::log::trace!("[RenderLayers] Made {} visibility updates this frame", updates);
    }
}

/// System to propagate RenderLayers to Bevy children of embeds.
///
/// **Hybrid Rendering Pipeline Logic:**
/// 
/// For Direct strategy embeds: propagate Layer 0 to all children and make them visible.
/// For Composite strategy embeds: propagate RTT layer to all children.
///
/// This handles nested embeds where content becomes Bevy children (not spatially decoupled).
pub fn propagate_render_layers_to_children_system(
    mut commands: Commands,
    // Composite strategy embeds (have RTT)
    composite_embed_query: Query<(Entity, &EmbedSceneRtt)>,
    // Direct strategy embeds (have RenderStrategy::Direct but no RTT)
    direct_embed_query: Query<(Entity, &RenderStrategy), Without<EmbedSceneRtt>>,
    children_query: Query<&Children>,
    render_layers_query: Query<&RenderLayers>,
    visibility_query: Query<&Visibility>,
    // Query for entities that are NOT embeds
    non_embed_query: Query<Entity, (Without<EmbedSceneRtt>, Without<RenderStrategy>)>,
) {
    use std::sync::atomic::{AtomicU32, Ordering};
    static FRAME_COUNT: AtomicU32 = AtomicU32::new(0);
    let frame = FRAME_COUNT.fetch_add(1, Ordering::Relaxed);
    
    let mut total_updates = 0;
    
    // Process Composite strategy embeds (propagate RTT layer)
    for (embed_entity, rtt) in composite_embed_query.iter() {
        let Ok(children) = children_query.get(embed_entity) else {
            continue;
        };

        let target_layer = RenderLayers::layer(rtt.render_layer as usize);
        total_updates += propagate_to_descendants(
            &mut commands,
            embed_entity,
            children,
            &target_layer,
            &children_query,
            &render_layers_query,
            &visibility_query,
            &non_embed_query,
        );
    }
    
    // Process Direct strategy embeds (propagate Layer 0)
    let layer_0 = RenderLayers::layer(0);
    let mut direct_with_children = 0;
    let mut direct_total_children = 0;
    for (embed_entity, strategy) in direct_embed_query.iter() {
        if *strategy != RenderStrategy::Direct {
            continue;
        }
        
        let Ok(children) = children_query.get(embed_entity) else {
            continue;
        };
        
        direct_with_children += 1;
        direct_total_children += children.len();

        total_updates += propagate_to_descendants(
            &mut commands,
            embed_entity,
            children,
            &layer_0,
            &children_query,
            &render_layers_query,
            &visibility_query,
            &non_embed_query,
        );
    }
    
    if frame <= 10 || frame % 60 == 0 || total_updates > 0 || direct_with_children > 0 {
        bevy::log::trace!(
            "[PropagateChildren] Frame {}: {} updates, Direct embeds with children: {} (total {} children)",
            frame,
            total_updates,
            direct_with_children,
            direct_total_children
        );
    }
}

/// Helper function to propagate RenderLayers to all descendants of an embed.
fn propagate_to_descendants(
    commands: &mut Commands,
    embed_entity: Entity,
    children: &Children,
    target_layer: &RenderLayers,
    children_query: &Query<&Children>,
    render_layers_query: &Query<&RenderLayers>,
    visibility_query: &Query<&Visibility>,
    non_embed_query: &Query<Entity, (Without<EmbedSceneRtt>, Without<RenderStrategy>)>,
) -> u32 {
    let mut updates = 0;
    
    // Process all direct children
    for child_entity in children.iter() {
        // Check if needs RenderLayers update
        let layer_needs_update = match render_layers_query.get(child_entity) {
            Ok(current) => current != target_layer,
            Err(_) => true,
        };
        
        // Check if needs Visibility update (make visible if currently hidden)
        let vis_needs_update = match visibility_query.get(child_entity) {
            Ok(Visibility::Hidden) => true,
            Err(_) => false, // No Visibility component, don't add one
            _ => false, // Already Inherited or Visible
        };
        
        if layer_needs_update || vis_needs_update {
            let mut entity_commands = commands.entity(child_entity);
            if layer_needs_update {
                entity_commands.insert(target_layer.clone());
            }
            if vis_needs_update {
                entity_commands.insert(Visibility::Inherited);
            }
            updates += 1;
            bevy::log::trace!(
                "[PropagateChildren] Updated child {:?} of embed {:?}",
                child_entity,
                embed_entity
            );
        }
        
        // Recurse into non-embed children
        if non_embed_query.get(child_entity).is_ok() {
            let mut to_process: Vec<Entity> = Vec::new();
            if let Ok(grandchildren) = children_query.get(child_entity) {
                to_process.extend(grandchildren.to_vec());
            }
            
            while let Some(entity) = to_process.pop() {
                // Only process non-embed descendants
                if non_embed_query.get(entity).is_ok() {
                    let layer_needs_update = match render_layers_query.get(entity) {
                        Ok(current) => current != target_layer,
                        Err(_) => true,
                    };
                    
                    let vis_needs_update = match visibility_query.get(entity) {
                        Ok(Visibility::Hidden) => true,
                        Err(_) => false,
                        _ => false,
                    };
                    
                    if layer_needs_update || vis_needs_update {
                        let mut entity_commands = commands.entity(entity);
                        if layer_needs_update {
                            entity_commands.insert(target_layer.clone());
                        }
                        if vis_needs_update {
                            entity_commands.insert(Visibility::Inherited);
                        }
                        updates += 1;
                    }
                    
                    // Continue to grandchildren
                    if let Ok(grandchildren) = children_query.get(entity) {
                        to_process.extend(grandchildren.to_vec());
                    }
                }
            }
        }
    }
    
    updates
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

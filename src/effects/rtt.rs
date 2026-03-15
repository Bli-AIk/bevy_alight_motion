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
use bevy::render::render_resource::TextureFormat;
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
                    // Embed boundary clipping for Direct strategy
                    apply_embed_bounds_clipping_system,
                    cleanup_embed_scene_rtt_system,
                    cleanup_embed_content_system,
                ),
            );
        // NOTE: The following systems are registered in plugin.rs with explicit
        // ordering via .chain() and ApplyDeferred. Do NOT register them here:
        // - evaluate_render_strategy_system
        // - setup_embed_scene_rtt_system
        // - fix_nested_embed_render_layers_system
        // - propagate_render_layers_system
        // - propagate_render_layers_to_children_system (disabled, causes transform issues)
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

/// Component storing embed scene bounds for content clipping.
/// Added to all embed entities regardless of render strategy.
/// Used by child content to clip rendering to the embed's bounds.
#[derive(Component, Debug, Clone)]
pub struct EmbedSceneBounds {
    /// Scene width in project coordinates
    pub width: f32,
    /// Scene height in project coordinates
    pub height: f32,
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
    /// Whether this embed has scale animation (requires bounds clipping)
    pub has_scale_animation: bool,
}

/// Marker component for embedScene layers used as masks (blending="mask"/"exclude").
/// These need Composite strategy to render their content to a texture,
/// which is then sampled by content layers as a mask.
#[derive(Component, Debug, Clone)]
pub struct AmEmbedMask;

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
    query: Query<
        (
            Entity,
            &NeedsStrategyEvaluation,
            Option<&AmGroupFill>,
            Option<&AmEmbedMask>,
        ),
        Without<RenderStrategy>,
    >,
) {
    for (entity, needs_eval, group_fill, embed_mask) in query.iter() {
        let needs_fill = group_fill.is_some();
        let is_mask = embed_mask.is_some();

        let strategy = if needs_fill || is_mask {
            RenderStrategy::Composite
        } else if needs_eval.has_scale_animation {
            RenderStrategy::Stencil
        } else {
            RenderStrategy::Direct
        };

        bevy::log::warn!(
            "[Strategy-DBG] Embed {:?} → {:?} (fill={}, mask={})",
            entity,
            strategy,
            needs_fill,
            is_mask,
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
                // Store embed bounds for content clipping
                EmbedSceneBounds {
                    width: needs_eval.scene_width,
                    height: needs_eval.scene_height,
                },
            ));

        // For Composite strategy, trigger RTT setup
        if strategy == RenderStrategy::Composite {
            commands.entity(entity).insert(NeedsEmbedSceneRtt {
                scene_width: needs_eval.scene_width,
                scene_height: needs_eval.scene_height,
            });
        }

        // Handle fillType="none" - make group invisible
        if let Some(fill) = group_fill
            && fill.fill_type == GroupFillType::None
        {
            commands.entity(entity).insert(Visibility::Hidden);
        }
    }
}

/// System to set up RTT infrastructure for embedScenes marked with NeedsEmbedSceneRtt.
pub fn setup_embed_scene_rtt_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut layer_pool: ResMut<EmbedSceneRenderLayerPool>,
    mut fill_materials: ResMut<Assets<crate::group_fill::GroupFillMaterial>>,
    mut unified_materials: ResMut<Assets<crate::masked_sprite::UnifiedEffectMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    _color_materials: ResMut<Assets<ColorMaterial>>,
    query: Query<
        (
            Entity,
            &NeedsEmbedSceneRtt,
            &Transform,
            &GlobalTransform,
            Option<&AmGroupFill>,
            &crate::animation::AmAnimated,
            Option<&AmEmbedMask>,
        ),
        Without<EmbedSceneRtt>,
    >,
    parent_query: Query<&ChildOf>,
    embed_rtt_query: Query<&EmbedSceneRtt>,
) {
    for (entity, needs_rtt, _embed_transform, embed_global, group_fill, animated, embed_mask) in query.iter() {
        bevy::log::warn!("[RTT-SETUP-DBG] Setting up RTT for {:?}, is_mask={}", entity, embed_mask.is_some());
        // Try to allocate a render layer
        let Some(render_layer) = layer_pool.allocate() else {
            bevy::log::warn!(
                "No available render layers for embedScene {:?}. Max 31 concurrent embedScenes supported.",
                entity
            );
            continue;
        };

        // Create RTT texture with sRGB format for correct color space handling.
        // Using Rgba8UnormSrgb ensures linear→sRGB conversion on write and sRGB→linear on read.
        let render_texture = Image::new_target_texture(
            needs_rtt.scene_width.max(1.0) as u32,
            needs_rtt.scene_height.max(1.0) as u32,
            TextureFormat::Rgba8UnormSrgb,
            None,
        );
        let render_texture_handle = images.add(render_texture);
        let render_layer_usize = render_layer as usize;

        // Create RTT camera with Fixed scaling mode to match embed's internal scene size
        // The camera is NOT added as a child of the embed to avoid inheriting scale/rotation.
        // Instead, sync_rtt_camera_position_system updates the camera's world position to follow
        // the embed's GlobalTransform translation.
        //
        // IMPORTANT: The embed's children inherit the scene entity's scale (fit_scale) via
        // GlobalTransform. The RTT camera must compensate for this by scaling its projection
        // area by the embed's global scale. Otherwise, shapes would be rendered too small
        // because the global scale is applied both in the RTT rendering AND when displaying
        // the RTT output on screen.
        let global_scale = embed_global.to_scale_rotation_translation().0;
        let effective_width = needs_rtt.scene_width * global_scale.x.abs();
        let effective_height = needs_rtt.scene_height * global_scale.y.abs();
        let camera_entity = commands
            .spawn((
                Name::new(format!("EmbedSceneRttCamera[layer={}]", render_layer)),
                EmbedSceneRttCamera {
                    embed_entity: entity,
                    render_layer, // Store for cleanup
                },
                Camera2d,
                Camera {
                    clear_color: ClearColorConfig::Custom(Color::NONE),
                    order: -(render_layer as isize), // Render before main camera
                    ..default()
                },
                // In Bevy 0.18, RenderTarget is a separate component
                RenderTarget::Image(render_texture_handle.clone().into()),
                // Fixed scaling mode compensated for the embed's inherited global scale
                Projection::Orthographic(OrthographicProjection {
                    scaling_mode: ScalingMode::Fixed {
                        width: effective_width,
                        height: effective_height,
                    },
                    near: -1000.0,
                    far: 1000.0,
                    ..OrthographicProjection::default_2d()
                }),
                // Camera only renders this specific layer
                RenderLayers::layer(render_layer_usize),
                // Camera must match embed's world position, rotation, AND scale sign.
                // Content entities are Bevy children of the embed, so they inherit
                // the embed's GlobalTransform (including rotation and scale sign).
                // The camera must share rotation and scale sign to cancel them out
                // in RTT space:
                // - Rotation: prevents double-rotation (RTT + mesh display)
                // - Scale sign: prevents double-flip when embed has negative scale
                //   (e.g., scale=(-1,1) for horizontal mirroring). Without this,
                //   content is flipped in RTT texture AND on the mesh, canceling
                //   the intended mirror effect.
                {
                    let (_, embed_rotation, embed_translation) =
                        embed_global.to_scale_rotation_translation();
                    Transform {
                        translation: Vec3::new(embed_translation.x, embed_translation.y, 1000.0),
                        rotation: embed_rotation,
                        scale: Vec3::new(global_scale.x.signum(), global_scale.y.signum(), 1.0),
                        ..default()
                    }
                },
            ))
            .id();

        // Determine which RenderLayers to use for the embed's Sprite.
        // If this embed is a child of another embed (nested), it should render
        // to the parent embed's RTT layer so the parent embed can see it.
        // If it's a top-level embed, it renders to layer 0 (main camera).
        let sprite_render_layer = if let Ok(child_of) = parent_query.get(entity) {
            let parent = child_of.parent();
            if let Ok(parent_rtt) = embed_rtt_query.get(parent) {
                RenderLayers::layer(parent_rtt.render_layer as usize)
            } else {
                RenderLayers::layer(0)
            }
        } else {
            RenderLayers::layer(0)
        };

        // Add EmbedSceneRtt component and remove the marker.
        // For group fill, use Mesh2d + GroupFillMaterial instead of Sprite.
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
                // Use appropriate RenderLayers based on nesting
                sprite_render_layer,
            ));

        // For mask embeds, don't add any visual sprite - the RTT texture is only
        // used as a mask by content layers. The embed must still be "visible" for
        // its children to render to RTT, but it won't display anything itself.
        if embed_mask.is_some() {
            bevy::log::debug!(
                "[RTT] Mask embed {:?}: RTT setup without sprite (layer={})",
                entity, render_layer
            );
        } else if let Some(fill) = group_fill {
            if fill.fill_type != GroupFillType::None {
                // Create GroupFillMaterial for color/gradient fill
                use crate::group_fill::{GroupFillMaterial, GroupFillUniform};
                let uniform = match &fill.fill_type {
                    GroupFillType::Color => GroupFillUniform {
                        fill_color: fill.fill_color,
                        gradient_config: Vec4::ZERO, // 0 = solid color
                        ..default()
                    },
                    GroupFillType::Gradient {
                        gradient_type,
                        start_color,
                        end_color,
                        points,
                    } => GroupFillUniform {
                        fill_color: Vec4::ONE,
                        gradient_config: Vec4::new(*gradient_type as f32, 0.0, 0.0, 0.0),
                        gradient_start_color: *start_color,
                        gradient_end_color: *end_color,
                        gradient_points: *points,
                    },
                    GroupFillType::None => unreachable!(),
                };
                let material = fill_materials.add(GroupFillMaterial {
                    uniform_data: uniform,
                    texture: Some(render_texture_handle),
                });
                let mesh = meshes.add(Rectangle::new(
                    needs_rtt.scene_width,
                    needs_rtt.scene_height,
                ));
                commands
                    .entity(entity)
                    .insert((Mesh2d(mesh), MeshMaterial2d(material)));
            }
        } else {
            // Check if embed has any effects that need UnifiedEffectMaterial
            let needs_unified = animated.exposure_has_effect
                || animated.wavewarp2_has_effect
                || animated.mirror_has_effect
                || animated.lift_has_effect
                || animated.rays_has_effect
                || animated.rgb_split_enabled
                || animated.chromakey_enabled;

            if needs_unified {
                // Use UnifiedEffectMaterial so effects can be applied to RTT output
                let width = needs_rtt.scene_width;
                let height = needs_rtt.scene_height;
                let material = unified_materials.add(crate::masked_sprite::UnifiedEffectMaterial {
                    uniform_data: crate::masked_sprite::UnifiedEffectUniform {
                        color: Vec4::new(1.0, 1.0, 1.0, 1.0),
                        original_size: Vec4::new(width, height, width, height),
                        ..default()
                    },
                    texture: Some(render_texture_handle),
                    lift_comp_texture: None,
                    mask_texture: None,
                });
                let mesh = meshes.add(Rectangle::new(width, height));
                commands.entity(entity).insert((
                    Mesh2d(mesh),
                    MeshMaterial2d(material),
                    crate::masked_sprite::UnifiedEffectMarker,
                ));
            } else {
                // No effects - use plain Sprite to display RTT output
                commands.entity(entity).insert(Sprite {
                    image: render_texture_handle,
                    custom_size: Some(Vec2::new(needs_rtt.scene_width, needs_rtt.scene_height)),
                    ..default()
                });
            }
        }

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
        let Ok(child_of) = parent_query.get(entity) else {
            continue;
        };
        let parent = child_of.parent();
        let Ok(parent_rtt) = embed_rtt_query.get(parent) else {
            continue;
        };
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
    content_query: Query<(
        Entity,
        &crate::scene::AmEmbedContentMarker,
        Option<&RenderLayers>,
        Option<&Visibility>,
    )>,
    // For propagating to Bevy children (e.g., SDF mesh entities)
    children_query: Query<&Children>,
    render_layers_query: Query<&RenderLayers>,
) {
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

    let mut updates = 0;

    // Assign RenderLayers to all embed content and their Bevy children
    for (content_entity, marker, current_layers, current_visibility) in content_query.iter() {
        // Determine target layer based on parent embed's strategy
        let target_layer = if let Some(&rtt_layer) = composite_layers.get(&marker.embed_entity) {
            RenderLayers::layer(rtt_layer as usize)
        } else if direct_embeds.contains(&marker.embed_entity) {
            RenderLayers::layer(0)
        } else {
            continue;
        };

        // Check if update is needed for the content entity itself
        let needs_update = match current_layers {
            Some(current) => *current != target_layer,
            None => true,
        };

        if needs_update {
            let target_visibility = match current_visibility {
                Some(Visibility::Hidden) => Visibility::Hidden,
                _ => Visibility::Inherited,
            };

            commands
                .entity(content_entity)
                .insert((target_layer.clone(), target_visibility));
            updates += 1;
        }

        // Propagate RenderLayers to ALL Bevy descendants (e.g., SDF mesh child entities).
        // SDF shapes are spawned as children with Mesh2d + MeshMaterial2d but WITHOUT
        // RenderLayers. Without propagation, they default to layer 0 and are invisible
        // to the RTT camera.
        let mut to_visit = Vec::new();
        if let Ok(children) = children_query.get(content_entity) {
            to_visit.extend(children.iter());
        }
        while let Some(child) = to_visit.pop() {
            let child_needs_update = match render_layers_query.get(child) {
                Ok(current) => *current != target_layer,
                Err(_) => true,
            };
            if child_needs_update {
                commands.entity(child).insert(target_layer.clone());
                updates += 1;
            }
            if let Ok(grandchildren) = children_query.get(child) {
                to_visit.extend(grandchildren.iter());
            }
        }
    }

    if updates > 0 {
        bevy::log::trace!(
            "[RenderLayers] Made {} updates this frame (content + descendants)",
            updates
        );
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

    if total_updates > 0 {
        bevy::log::trace!(
            "[PropagateChildren] {} updates, Direct embeds with children: {} (total {} children)",
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
            _ => false,      // Already Inherited or Visible
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
        let mut to_process: Vec<Entity> = Vec::new();
        if non_embed_query.get(child_entity).is_ok()
            && let Ok(grandchildren) = children_query.get(child_entity)
        {
            to_process.extend(grandchildren.to_vec());
        }

        while let Some(entity) = to_process.pop() {
            // Only process non-embed descendants
            if non_embed_query.get(entity).is_err() {
                continue;
            }

            let layer_needs_update = match render_layers_query.get(entity) {
                Ok(current) => current != target_layer,
                Err(_) => true,
            };

            let vis_needs_update = match visibility_query.get(entity) {
                Ok(Visibility::Hidden) => true,
                Err(_) => false,
                _ => false,
            };

            if layer_needs_update {
                commands.entity(entity).insert(target_layer.clone());
            }
            if vis_needs_update {
                commands.entity(entity).insert(Visibility::Inherited);
            }
            if layer_needs_update || vis_needs_update {
                updates += 1;
            }

            // Continue to grandchildren
            if let Ok(grandchildren) = children_query.get(entity) {
                to_process.extend(grandchildren.to_vec());
            }
        }
    }

    updates
}

/// System to sync RTT camera positions and projection with their embed's GlobalTransform.
/// This ensures that RTT cameras follow their embed's world position, allowing them to
/// "see" the embed's Bevy children (which have world positions relative to the embed).
/// Also syncs the projection's Fixed scaling to account for changes in global scale
/// (e.g., animated scale on the embed or its parents).
pub fn sync_rtt_camera_position_system(
    embed_query: Query<(&EmbedSceneRtt, &GlobalTransform)>,
    mut camera_query: Query<(&EmbedSceneRttCamera, &mut Transform, &mut Projection)>,
) {
    for (camera_marker, mut camera_transform, mut projection) in camera_query.iter_mut() {
        if let Ok((rtt, embed_global)) = embed_query.get(camera_marker.embed_entity) {
            // Sync position, rotation, and scale sign to match embed's world transform.
            // Rotation must match so content (which inherits embed's rotation via
            // Bevy hierarchy) appears unrotated in RTT space.
            // Scale sign must match so content (which inherits embed's scale sign
            // via Bevy hierarchy) appears unflipped in RTT space.
            let (global_scale, embed_rotation, embed_translation) =
                embed_global.to_scale_rotation_translation();
            camera_transform.translation =
                Vec3::new(embed_translation.x, embed_translation.y, 1000.0);
            camera_transform.rotation = embed_rotation;
            camera_transform.scale =
                Vec3::new(global_scale.x.signum(), global_scale.y.signum(), 1.0);

            // Sync projection scale to compensate for inherited global scale
            let effective_width = rtt.scene_width * global_scale.x.abs();
            let effective_height = rtt.scene_height * global_scale.y.abs();
            if let Projection::Orthographic(ref mut ortho) = *projection {
                ortho.scaling_mode = ScalingMode::Fixed {
                    width: effective_width,
                    height: effective_height,
                };
            }
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

/// System to apply embed boundary clipping to embed content using shader masks.
///
/// For Direct render strategy, content is rendered directly to Layer 0 without RTT.
/// To achieve proper clipping, we set mask_params on content materials to clip
/// pixels outside the embed's bounds.
///
/// Note: This only applies to embeds with Stencil or Composite strategy.
/// Direct strategy embeds don't need bounds clipping as content renders
/// directly to the main canvas without any composition.
///
/// Note: This only works correctly for non-rotated embeds. Rotated embeds would
/// need a more complex solution (e.g., rotated rectangle SDF in shader).
pub fn apply_embed_bounds_clipping_system(
    embed_query: Query<(&EmbedSceneBounds, &GlobalTransform, Option<&RenderStrategy>)>,
    content_query: Query<(
        Entity,
        &crate::scene::AmEmbedContentMarker,
        &MeshMaterial2d<crate::masked_sprite::UnifiedEffectMaterial>,
        Option<&crate::scene::AmMaskInfo>,
    )>,
    playback: Res<crate::animation::AmPlayback>,
    mut materials: ResMut<Assets<crate::masked_sprite::UnifiedEffectMaterial>>,
) {
    // Get current playback time for mask layer checks
    let global_time = playback.current_time_ms as u64;

    for (_entity, marker, material_handle, mask_info) in content_query.iter() {
        // Get the embed's bounds, transform, and render strategy
        let Ok((bounds, embed_gt, strategy)) = embed_query.get(marker.embed_entity) else {
            continue;
        };

        // Skip clipping for Direct strategy embeds
        // Direct embeds render content directly to main canvas without composition.
        // For embeds that need bounds clipping (e.g., those with scale animation),
        // Stencil or Composite strategy should be used instead.
        if strategy.is_some_and(|s| *s == RenderStrategy::Direct) {
            continue;
        }

        // Skip if material doesn't exist
        let Some(material) = materials.get_mut(&material_handle.0) else {
            continue;
        };

        // Check if this content has active masks from mask layers
        // If it does, mask layers take priority over embed bounds
        let has_active_mask = mask_info
            .map(|info| !info.get_active_masks(global_time).is_empty())
            .unwrap_or(false);

        if has_active_mask {
            // Let the mask layer system handle this content
            continue;
        }

        // Extract embed's world position, scale, and rotation from GlobalTransform
        // Note: GlobalTransform already includes fit_scale from parent chain,
        // so we DON'T need to multiply by fit_scale again!
        let (embed_scale, embed_rotation, embed_pos) = embed_gt.to_scale_rotation_translation();

        // Calculate embed bounds in world coordinates
        // bounds.width/height are in project coordinates (e.g., 1440x1080)
        // embed_scale already includes fit_scale (e.g., 0.8889)
        let half_width = bounds.width * 0.5 * embed_scale.x.abs();
        let half_height = bounds.height * 0.5 * embed_scale.y.abs();
        let center_x = embed_pos.x;
        let center_y = embed_pos.y;

        // Extract Z rotation angle for rotated rectangle clipping
        let rotation_z = embed_rotation.to_euler(bevy::math::EulerRot::XYZ).2;

        // Set mask params for rectangular clipping
        // mask_type 1.0 = rectangle include (only show pixels inside)
        material.uniform_data.effect_flags.x = 1.0; // Rectangle mask
        material.uniform_data.mask_params =
            bevy::math::Vec4::new(center_x, center_y, half_width, half_height);
        // mask_blend: fill_alpha=1.0 (opaque fill), opacity=1.0 (full strength), sw=0
        material.uniform_data.mask_blend = bevy::math::Vec4::new(1.0, 1.0, 0.0, 0.0);
        // mask1 rotation stored in mask2_flags.y
        material.uniform_data.mask2_flags.y = rotation_z;

        bevy::log::trace!(
            "[EmbedClip] Content {:?} clipped to embed bounds: center=({:.1},{:.1}), half=({:.1},{:.1}), rot={:.3}, embed_scale=({:.3},{:.3})",
            _entity,
            center_x,
            center_y,
            half_width,
            half_height,
            rotation_z,
            embed_scale.x,
            embed_scale.y
        );
    }
}

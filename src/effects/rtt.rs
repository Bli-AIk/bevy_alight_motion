//! # rtt.rs
//!
//! # RTT 渲染模块
//!
//! Render-to-Texture (RTT) systems for embed scenes and effects.
//! 嵌入场景和效果的渲染到纹理 (RTT) 系统。

use bevy::camera::RenderTarget;
use bevy::camera::ScalingMode;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};
use std::collections::HashMap;

use crate::scene::{AmEmbedContent, AmEmbedContentMarker, AmLayerMarker, AmPendingLayers};

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

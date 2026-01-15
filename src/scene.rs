//! Scene building and coordinate transformation.

use bevy::asset::Assets;
use bevy::prelude::*;
use bevy::sprite::Text2d;
use bevy::text::{TextColor, TextFont, TextLayout};
use std::collections::HashMap;

use crate::animation::AmAnimated;
use crate::effects::NeedsEmbedSceneRtt;
use crate::loader::{AmProject, FontMetrics};
use crate::schema::{
    AmAnimatedFloat, AmAnimatedVec2, AmAnimatedVec3, AmEffect, AmLayer, AmScene, AmShape, AmText,
};
use crate::sdf::AmSdfShaders;

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
    /// Mask layer - clips content below it (not rendered itself)
    Mask,
}

/// Information about an active mask that clips this layer.
#[derive(Debug, Clone, Default, Component)]
pub struct AmMaskInfo {
    /// Center position of the mask in local coordinates
    pub center: Vec2,
    /// Half-size of the mask rectangle
    pub half_size: Vec2,
    /// Rotation of the mask in radians
    pub rotation: f32,
    /// Scale of the mask
    pub scale: Vec2,
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
    pub time_offset: i32,
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
            speed_multiplier: 1.0,
            nesting_depth: 0,
        }
    }
}

/// Convert AM coordinates to Bevy coordinates.
///
/// AM: Origin at top-left, Y increases downward.
/// Bevy: Origin at center, Y increases upward.
pub fn am_to_bevy_coords(x: f32, y: f32, config: &AmSceneConfig) -> (f32, f32) {
    let bx = x - config.canvas_width / 2.0;
    let by = if config.flip_y {
        config.canvas_height / 2.0 - y
    } else {
        y - config.canvas_height / 2.0
    };
    (bx, by)
}

/// Spawn entities from an AM scene.
#[allow(clippy::too_many_arguments)]
pub fn spawn_scene(
    commands: &mut Commands,
    shaders: &mut Assets<Shader>,
    scene: &AmScene,
    images: &HashMap<String, Handle<Image>>,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    white_pixel: &Handle<Image>,
    sdf_shaders: &AmSdfShaders,
    parent: Entity,
    config: &AmSceneConfig,
) -> HashMap<u64, Entity> {
    let mut entity_map: HashMap<u64, Entity> = HashMap::new();
    let mut parent_relations: Vec<(Entity, u64)> = Vec::new();

    // In AM, layers at the END of the XML are rendered on top (higher z).
    // Last layer in XML = highest z (on top), first layer = lowest z (on bottom).
    let layer_count = scene.layers.len();

    bevy::log::trace!(
        "spawn_scene: layer_count={}, z_spacing={}, nesting_depth={}",
        layer_count,
        config.z_spacing,
        config.nesting_depth
    );

    // For nested scenes, start at a small offset above parent to avoid z-fighting
    // Root scene (depth 0) starts at z=0, nested scenes start at a small offset
    let z_base = if config.nesting_depth > 0 {
        config.z_spacing * 0.1 // Small offset to be above parent
    } else {
        0.0
    };

    // First pass: create all entities and collect parent relationships
    for (idx, layer) in scene.layers.iter().enumerate() {
        // Simple sequential z allocation
        let z = z_base + idx as f32 * config.z_spacing;

        // Log z value for each layer
        let layer_label = match layer {
            AmLayer::Shape(s) => &s.label,
            AmLayer::Nullobj(n) => &n.label,
            AmLayer::EmbedScene(e) => &e.label,
            AmLayer::Text(t) => &t.label,
            AmLayer::Audio(a) => &a.label,
            AmLayer::Bookmark(b) => &b.label,
            AmLayer::Image(i) => &i.label,
            AmLayer::Camera(c) => &c.label,
            AmLayer::Video(v) => &v.label,
        };
        bevy::log::info!(
            "[Z-ORDER] depth={}, idx={}, z={:.4}, label='{}'",
            config.nesting_depth,
            idx,
            z,
            layer_label
        );

        match layer {
            AmLayer::Shape(shape) => {
                let entity = spawn_shape(
                    commands,
                    shaders,
                    shape,
                    images,
                    white_pixel,
                    sdf_shaders,
                    config,
                    z,
                );
                entity_map.insert(shape.id, entity);
                if shape.parent != 0 {
                    parent_relations.push((entity, shape.parent));
                } else {
                    commands.entity(parent).add_child(entity);
                }
            }
            AmLayer::Nullobj(null) => {
                let entity = spawn_null(commands, null, config, z);
                entity_map.insert(null.id, entity);
                if null.parent != 0 {
                    parent_relations.push((entity, null.parent));
                } else {
                    commands.entity(parent).add_child(entity);
                }
            }
            AmLayer::EmbedScene(embed) => {
                let entity = spawn_embed_scene(
                    commands,
                    shaders,
                    embed,
                    images,
                    fonts,
                    font_metrics,
                    white_pixel,
                    sdf_shaders,
                    config,
                    z,
                );
                entity_map.insert(embed.id, entity);
                if embed.parent != 0 {
                    parent_relations.push((entity, embed.parent));
                } else {
                    commands.entity(parent).add_child(entity);
                }
            }
            AmLayer::Bookmark(_) => {
                // Bookmarks are non-visual timeline markers, skip them
            }
            AmLayer::Text(text) => {
                let entity = spawn_text(commands, text, fonts, font_metrics, config, z);
                entity_map.insert(text.id, entity);
                if text.parent != 0 {
                    parent_relations.push((entity, text.parent));
                } else {
                    commands.entity(parent).add_child(entity);
                }
            }
            AmLayer::Audio(audio) => {
                // TODO: Audio playback is not yet implemented, skip for now
                bevy::log::trace!(
                    "Skipping audio layer '{}' (id={}) - audio not implemented",
                    audio.label,
                    audio.id
                );
            }
            AmLayer::Camera(camera) => {
                // TODO: Camera layer is not yet implemented, skip for now
                bevy::log::trace!(
                    "Skipping camera layer '{}' (id={}) - camera not implemented",
                    camera.label,
                    camera.id
                );
            }
            AmLayer::Image(image) => {
                let entity = spawn_image(commands, image, images, config, z);
                entity_map.insert(image.id, entity);
                if image.parent != 0 {
                    parent_relations.push((entity, image.parent));
                } else {
                    commands.entity(parent).add_child(entity);
                }
            }
            AmLayer::Video(video) => {
                // TODO: Video playback is not yet implemented, skip for now
                bevy::log::trace!(
                    "Skipping video layer '{}' (id={}) - video not implemented",
                    video.label,
                    video.id
                );
            }
        }
    }

    // Second pass: set up parent-child relationships
    for (child_entity, parent_id) in parent_relations {
        if let Some(&parent_entity) = entity_map.get(&parent_id) {
            commands.entity(parent_entity).add_child(child_entity);
        } else {
            // Parent not found in this scene, attach to scene root
            commands.entity(parent).add_child(child_entity);
        }
    }

    entity_map
}

/// Spawn a shape layer (lazy - visual components spawned later by lifecycle system).
#[allow(clippy::too_many_arguments)]
fn spawn_shape(
    commands: &mut Commands,
    _shaders: &mut Assets<Shader>,
    shape: &AmShape,
    _images: &HashMap<String, Handle<Image>>,
    _white_pixel: &Handle<Image>,
    _sdf_shaders: &AmSdfShaders,
    config: &AmSceneConfig,
    z: f32,
) -> Entity {
    // Get initial transform values - use local coords if has parent
    let has_parent = shape.parent != 0;
    let (tx, ty) = get_initial_location(&shape.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&shape.transform.rotation);
    let (sx, sy) = get_initial_scale(&shape.transform.scale);
    let (effect_pos_x, effect_pos_y) = extract_effect_animations(&shape.effects);
    let wipe_effect = extract_wipe_effect(&shape.effects);
    let stretch_segment = extract_stretch_segment_effect(&shape.effects);
    let gaussian_blur = extract_gaussian_blur_effect(&shape.effects);
    let (pivot_x, pivot_y) = get_initial_pivot(&shape.transform.pivot);

    // Get size from properties
    let (width, height) = get_shape_size(&shape.properties, &shape.fill_type);

    // AM location points to object CENTER, not pivot. No position compensation needed.
    // Pivot only affects rotation/scale center, which is handled by Anchor.
    bevy::log::trace!(
        "Registering shape '{}' (id={}, parent={}): pos=({:.1},{:.1}), z={:.1}, scale=({:.2},{:.2}), size=({:.0},{:.0}), pivot=({:.1},{:.1}), fill={}, image={}",
        shape.label,
        shape.id,
        shape.parent,
        tx,
        ty,
        z,
        sx,
        sy,
        width,
        height,
        pivot_x,
        pivot_y,
        shape.fill_type,
        shape.fill_image
    );

    // Create entity name for inspector identification
    let entity_name = format!("Shape[{}]: {}", shape.id, shape.label);

    // Check if this is a stroked shape that needs SDF rendering
    // Also use SDF for circles (better quality than sprite rect)
    let needs_sdf = shape.fill_type == "color"
        && (shape.shape_type == ".circle"
            || shape.stroke.as_ref().is_some_and(|s| {
                s.size.as_ref().is_some_and(|sz| {
                    // Check if stroke has a value > 0 or has keyframes
                    sz.value.unwrap_or(0.0) > 0.0 || !sz.keyframes.is_empty()
                })
            }));

    // Calculate anchor and position compensation for non-SDF shapes
    let (anchor, comp_x, comp_y) = pivot_to_anchor_and_offset(pivot_x, pivot_y, width, height);

    // For SpriteShape, we need to compensate position when anchor is not CENTER
    // because Bevy draws sprite with anchor point at translation position
    // For SDF shapes, parent should be at pivot point (for rotation/scale around pivot)
    let (final_tx, final_ty) = if needs_sdf {
        // SDF parent is at pivot point: AM center + pivot offset (with Y flip)
        (tx + pivot_x, ty - pivot_y)
    } else {
        // SpriteShape: compensate position so center stays at AM location
        (tx + comp_x, ty + comp_y)
    };

    let transform = Transform {
        translation: Vec3::new(final_tx, final_ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    // Create the layer spec for lazy spawning
    let layer_spec = if needs_sdf {
        let default_stroke = crate::schema::AmStroke::default();
        let stroke = shape.stroke.as_ref().unwrap_or(&default_stroke);
        // Get initial stroke width (use static value or first keyframe value)
        let stroke_width = stroke
            .size
            .as_ref()
            .and_then(|s| {
                // Prefer static value, fall back to first keyframe value
                s.value
                    .or_else(|| s.keyframes.first().and_then(|kf| kf.value.parse().ok()))
            })
            .unwrap_or(0.0);
        let stroke_color_value = stroke
            .color
            .as_ref()
            .map(|c| c.value.clone())
            .unwrap_or_default();

        AmLayerSpec::SdfShape {
            fill_color: shape.fill_color.clone(),
            stroke_color_value,
            stroke_width,
            stroke_join: stroke.join.clone(),
            width,
            height,
            pivot_x,
            pivot_y,
            shape_type: shape.shape_type.clone(),
        }
    } else if shape.fill_type == "media" && !shape.fill_image.is_empty() {
        AmLayerSpec::SpriteShape {
            image_uri: shape.fill_image.clone(),
            is_media: true,
            fill_color: None,
            width,
            height,
            anchor,
        }
    } else {
        // Color fill
        AmLayerSpec::SpriteShape {
            image_uri: String::new(),
            is_media: false,
            fill_color: shape.fill_color.clone(),
            width,
            height,
            anchor,
        }
    };

    // Spawn the layer entity without visual components (they'll be added by lifecycle system)
    // For SDF shapes, anchor_offset moves parent from center to pivot point
    // For SpriteShape, use the computed compensation
    let anchor_offset = if needs_sdf {
        // SDF parent needs to be offset from center to pivot point
        Vec2::new(pivot_x, -pivot_y)
    } else {
        Vec2::new(comp_x, comp_y)
    };

    let stroke_width_anim = get_stroke_width_animation(shape.stroke.as_ref());
    let base_alpha = get_base_alpha(&shape.fill_color);
    let palette_map = extract_palette_map_effect(&shape.effects);

    let entity = commands
        .spawn((
            Name::new(entity_name),
            AmLayerMarker {
                id: shape.id,
                label: shape.label.clone(),
            },
            AmAnimated {
                layer_id: shape.id,
                start_time: shape.start_time,
                end_time: shape.end_time,
                time_offset: config.time_offset,
                location: shape.transform.location.clone(),
                pivot: shape.transform.pivot.clone(),
                rotation: shape.transform.rotation.clone(),
                scale: shape.transform.scale.clone(),
                opacity: shape.transform.opacity.clone(),
                canvas_width: config.canvas_width,
                canvas_height: config.canvas_height,
                has_parent,
                effect_pos_x,
                effect_pos_y,
                font_y_offset: 0.0,
                size: get_shape_size_animation(&shape.properties),
                anchor_offset,
                wipe_start: wipe_effect.start,
                wipe_end: wipe_effect.end,
                wipe_angle: wipe_effect.angle,
                wipe_feather: wipe_effect.feather,
                stretch_angle: stretch_segment.angle,
                stretch_amount: stretch_segment.stretch,
                stretch_offset: stretch_segment.offset,
                stretch_smooth: stretch_segment.smooth,
                blur_strength: gaussian_blur.strength,
                speed_multiplier: config.speed_multiplier,
                embed_offset: Vec2::ZERO,
                inv_fit_scale: 1.0,
                stroke_width: stroke_width_anim,
                base_alpha,
                palette_alpha: palette_map.alpha.clone(),
            },
            layer_spec,
            transform,
            GlobalTransform::default(),
            Visibility::Hidden, // Start hidden, lifecycle system will show when active
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Add palette map params if effect is present
    if palette_map.has_effect() {
        commands.entity(entity).insert(AmPaletteMapParams::from_params(&palette_map));
    }

    entity
}

/// Spawn a null object.
fn spawn_null(
    commands: &mut Commands,
    null: &crate::schema::AmNullObj,
    config: &AmSceneConfig,
    z: f32,
) -> Entity {
    let has_parent = null.parent != 0;
    let (tx, ty) = get_initial_location(&null.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&null.transform.rotation);
    let (sx, sy) = get_initial_scale(&null.transform.scale);
    let (effect_pos_x, effect_pos_y) = extract_effect_animations(&null.effects);
    let wipe_effect = extract_wipe_effect(&null.effects);
    let stretch_segment = extract_stretch_segment_effect(&null.effects);
    let gaussian_blur = extract_gaussian_blur_effect(&null.effects);

    bevy::log::trace!(
        "Registering nullobj '{}' (id={}, parent={}): pos=({:.1},{:.1}), scale=({:.2},{:.2})",
        null.label,
        null.id,
        null.parent,
        tx,
        ty,
        sx,
        sy
    );

    let transform = Transform {
        translation: Vec3::new(tx, ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    // Create entity name for inspector identification
    let entity_name = format!("Null[{}]: {}", null.id, null.label);

    commands
        .spawn((
            Name::new(entity_name),
            AmLayerMarker {
                id: null.id,
                label: null.label.clone(),
            },
            AmAnimated {
                layer_id: null.id,
                start_time: null.start_time,
                end_time: null.end_time,
                time_offset: config.time_offset,
                location: null.transform.location.clone(),
                pivot: null.transform.pivot.clone(),
                rotation: null.transform.rotation.clone(),
                scale: null.transform.scale.clone(),
                opacity: null.transform.opacity.clone(),
                canvas_width: config.canvas_width,
                canvas_height: config.canvas_height,
                has_parent,
                effect_pos_x,
                effect_pos_y,
                font_y_offset: 0.0,
                size: AmAnimatedVec2::default(),
                anchor_offset: Vec2::ZERO,
                wipe_start: wipe_effect.start,
                wipe_end: wipe_effect.end,
                wipe_angle: wipe_effect.angle,
                wipe_feather: wipe_effect.feather,
                stretch_angle: stretch_segment.angle,
                stretch_amount: stretch_segment.stretch,
                stretch_offset: stretch_segment.offset,
                stretch_smooth: stretch_segment.smooth,
                blur_strength: gaussian_blur.strength,
                speed_multiplier: config.speed_multiplier,
                embed_offset: Vec2::ZERO,
                inv_fit_scale: 1.0,
                stroke_width: AmAnimatedFloat::default(),
                base_alpha: 1.0, // Null objects are fully opaque
                palette_alpha: AmAnimatedFloat::default(),
            },
            AmLayerSpec::Null,
            transform,
            GlobalTransform::default(),
            Visibility::Hidden, // Start hidden, lifecycle system will show when active
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id()
}

/// Spawn an embedded scene.
#[allow(clippy::too_many_arguments)]
fn spawn_embed_scene(
    commands: &mut Commands,
    shaders: &mut Assets<Shader>,
    embed: &crate::schema::AmEmbedScene,
    images: &HashMap<String, Handle<Image>>,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    white_pixel: &Handle<Image>,
    sdf_shaders: &AmSdfShaders,
    config: &AmSceneConfig,
    z: f32,
) -> Entity {
    let has_parent = embed.parent != 0;
    let (mut tx, mut ty) = get_initial_location(&embed.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&embed.transform.rotation);
    let (sx, sy) = get_initial_scale(&embed.transform.scale);
    let pivot = get_initial_pivot(&embed.transform.pivot);

    // Apply pivot compensation for initial position
    let (comp_x, comp_y) = calculate_pivot_compensation(pivot, (sx, sy), rotation, has_parent);
    tx += comp_x;
    ty += comp_y;

    bevy::log::trace!(
        "Registering embedScene '{}' (id={}, parent={}): pos=({:.1},{:.1}), pivot=({:.1},{:.1}), scale=({:.2},{:.2}), start_time={}, time_offset={}",
        embed.label,
        embed.id,
        embed.parent,
        tx,
        ty,
        pivot.0,
        pivot.1,
        sx,
        sy,
        embed.start_time,
        config.time_offset
    );

    let transform = Transform {
        translation: Vec3::new(tx, ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    // Create entity name for inspector identification
    let entity_name = format!("Embed[{}]: {}", embed.id, embed.label);

    let entity = commands
        .spawn((
            Name::new(entity_name),
            AmLayerMarker {
                id: embed.id,
                label: embed.label.clone(),
            },
            AmAnimated {
                layer_id: embed.id,
                start_time: embed.start_time,
                end_time: embed.end_time,
                time_offset: config.time_offset,
                location: embed.transform.location.clone(),
                pivot: embed.transform.pivot.clone(),
                rotation: embed.transform.rotation.clone(),
                scale: embed.transform.scale.clone(),
                opacity: embed.transform.opacity.clone(),
                canvas_width: config.canvas_width,
                canvas_height: config.canvas_height,
                has_parent,
                effect_pos_x: AmAnimatedFloat::default(),
                effect_pos_y: AmAnimatedFloat::default(),
                font_y_offset: 0.0,
                size: AmAnimatedVec2::default(),
                anchor_offset: Vec2::ZERO,
                wipe_start: AmAnimatedFloat::default(),
                wipe_end: AmAnimatedFloat {
                    value: Some(1.0),
                    keyframes: vec![],
                },
                wipe_angle: AmAnimatedFloat::default(),
                wipe_feather: AmAnimatedFloat::default(),
                stretch_angle: AmAnimatedFloat::default(),
                stretch_amount: AmAnimatedFloat::default(),
                stretch_offset: AmAnimatedFloat::default(),
                stretch_smooth: AmAnimatedFloat::default(),
                blur_strength: AmAnimatedFloat::default(),
                speed_multiplier: config.speed_multiplier,
                embed_offset: Vec2::ZERO,
                inv_fit_scale: 1.0,
                stroke_width: AmAnimatedFloat::default(),
                base_alpha: get_base_alpha(&embed.fill_color),
                palette_alpha: AmAnimatedFloat::default(),
            },
            AmLayerSpec::EmbedScene,
            // Mark for RTT setup (will enable clipping to scene bounds)
            NeedsEmbedSceneRtt {
                scene_width: embed.scene.width as f32,
                scene_height: embed.scene.height as f32,
            },
            transform,
            GlobalTransform::default(),
            Visibility::Hidden, // Start hidden, lifecycle system will show when active
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Recursively spawn nested scene with accumulated time offset
    // The nested scene's layers use times relative to the embed's start_time
    //
    // Calculate the internal time offset for the embedded scene.
    // When the parent timeline reaches startTime, the embedded scene should be at inTime.
    // 
    // The formula for local_time in the animation system is:
    //   local_time = (global_time - time_offset) * speed_multiplier
    //
    // When global_time = embed.start_time, we want local_time = inTime:
    //   inTime = (embed.start_time - time_offset) * speed
    //   time_offset = embed.start_time - inTime / speed
    //
    // Note: This handles the case where speed != 1.0, which affects internal time flow.
    //
    // Nested scenes use smaller z_spacing to keep all children within
    // the parent's z-range (between parent and next sibling)
    // Using /100 instead of /1000 for better numerical precision
    let in_time = embed.in_time.unwrap_or(0) as f32;
    let effective_speed = config.speed_multiplier * embed.speed;
    let time_offset_with_in_time = if effective_speed > 0.0 {
        config.time_offset as f32 + embed.start_time as f32 - in_time / effective_speed
    } else {
        config.time_offset as f32 + embed.start_time as f32
    };
    let nested_z_spacing = config.z_spacing / 100.0;
    let nested_config = AmSceneConfig {
        canvas_width: embed.scene.width as f32,
        canvas_height: embed.scene.height as f32,
        time_offset: time_offset_with_in_time as i32,
        z_spacing: nested_z_spacing,
        nesting_depth: config.nesting_depth + 1,
        speed_multiplier: effective_speed,
        ..config.clone()
    };

    spawn_scene(
        commands,
        shaders,
        &embed.scene,
        images,
        fonts,
        font_metrics,
        white_pixel,
        sdf_shaders,
        entity,
        &nested_config,
    );

    entity
}

/// Spawn an image layer (lazy - visual components spawned later by lifecycle system).
fn spawn_image(
    commands: &mut Commands,
    image: &crate::schema::AmImage,
    _images: &HashMap<String, Handle<Image>>,
    config: &AmSceneConfig,
    z: f32,
) -> Entity {
    let has_parent = image.parent != 0;
    let (tx, ty) = get_initial_location(&image.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&image.transform.rotation);
    let (sx, sy) = get_initial_scale(&image.transform.scale);
    let (effect_pos_x, effect_pos_y) = extract_effect_animations(&image.effects);
    let wipe_effect = extract_wipe_effect(&image.effects);
    let stretch_segment = extract_stretch_segment_effect(&image.effects);
    let gaussian_blur = extract_gaussian_blur_effect(&image.effects);
    let (pivot_x, pivot_y) = get_initial_pivot(&image.transform.pivot);
    let palette_map = extract_palette_map_effect(&image.effects);

    // Get size from properties
    let (width, height) = get_shape_size(&image.properties, &image.fill_type);

    // Calculate anchor and position compensation
    let (anchor, comp_x, comp_y) = pivot_to_anchor_and_offset(pivot_x, pivot_y, width, height);
    let (final_tx, final_ty) = (tx + comp_x, ty + comp_y);

    bevy::log::trace!(
        "Registering image '{}' (id={}, parent={}): pos=({:.1},{:.1}), scale=({:.2},{:.2}), size=({:.0},{:.0}), pivot=({:.1},{:.1}), fill={}",
        image.label,
        image.id,
        image.parent,
        final_tx,
        final_ty,
        sx,
        sy,
        width,
        height,
        pivot_x,
        pivot_y,
        image.fill_image
    );

    let transform = Transform {
        translation: Vec3::new(final_tx, final_ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    // Create entity name for inspector identification
    let entity_name = format!("Image[{}]: {}", image.id, image.label);

    let entity = commands
        .spawn((
            Name::new(entity_name),
            AmLayerMarker {
                id: image.id,
                label: image.label.clone(),
            },
            AmAnimated {
                layer_id: image.id,
                start_time: image.start_time,
                end_time: image.end_time,
                time_offset: config.time_offset,
                location: image.transform.location.clone(),
                pivot: image.transform.pivot.clone(),
                rotation: image.transform.rotation.clone(),
                scale: image.transform.scale.clone(),
                opacity: image.transform.opacity.clone(),
                canvas_width: config.canvas_width,
                canvas_height: config.canvas_height,
                has_parent,
                effect_pos_x,
                effect_pos_y,
                font_y_offset: 0.0,
                size: AmAnimatedVec2::default(),
                anchor_offset: Vec2::new(comp_x, comp_y),
                wipe_start: wipe_effect.start,
                wipe_end: wipe_effect.end,
                wipe_angle: wipe_effect.angle,
                wipe_feather: wipe_effect.feather,
                stretch_angle: stretch_segment.angle,
                stretch_amount: stretch_segment.stretch,
                stretch_offset: stretch_segment.offset,
                stretch_smooth: stretch_segment.smooth,
                blur_strength: gaussian_blur.strength,
                speed_multiplier: config.speed_multiplier,
                embed_offset: Vec2::ZERO,
                inv_fit_scale: 1.0,
                stroke_width: AmAnimatedFloat::default(),
                base_alpha: 1.0, // Image layers are fully opaque
                palette_alpha: palette_map.alpha.clone(),
            },
            AmLayerSpec::Image {
                image_uri: image.fill_image.clone(),
                width,
                height,
                anchor,
            },
            transform,
            GlobalTransform::default(),
            Visibility::Hidden, // Start hidden, lifecycle system will show when active
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Add palette map params if effect is present
    if palette_map.has_effect() {
        commands.entity(entity).insert(AmPaletteMapParams::from_params(&palette_map));
    }

    entity
}

/// Spawn a text layer.
fn spawn_text(
    commands: &mut Commands,
    text: &AmText,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    config: &AmSceneConfig,
    z: f32,
) -> Entity {
    let has_parent = text.parent != 0;
    let (tx, ty) = get_initial_location(&text.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&text.transform.rotation);
    let (sx, sy) = get_initial_scale(&text.transform.scale);
    let opacity = get_initial_opacity(&text.transform.opacity);

    // AM text position is based on the CENTER of the wrapWidth box
    // We need to offset to get the LEFT edge for left-aligned text
    // 在AM中，文本位置是基于wrapWidth框的中心
    // 对于左对齐文本，我们需要偏移到左边缘
    // But for text with parent, don't apply wrap offset since position is relative
    // 但是对于有父对象的文本，不应用wrapWidth偏移，因为位置是相对的
    let wrap_width = text.wrap_width;
    let wrap_offset_x = if has_parent {
        0.0 // Child text uses relative positioning, no wrap offset
    } else {
        match text.align.as_str() {
            "left" => -wrap_width / 2.0, // Move left by half of wrapWidth
            "right" => wrap_width / 2.0, // Move right by half of wrapWidth
            _ => 0.0,                    // Center - no offset needed
        }
    };

    // Get font size (default to 16.0 if not specified)
    // AM font sizes appear to be in a different scale - use a larger multiplier
    // 文本大小乘数 - 调整这个值来修改字体大小
    const TEXT_SIZE_MULTIPLIER: f32 = 3.0;
    let font_size = if text.size > 0.0 {
        text.size * TEXT_SIZE_MULTIPLIER
    } else {
        48.0
    };

    // Parse font name from "imported?name=FontName.ttf" format
    let font_name = text
        .font
        .strip_prefix("imported?name=")
        .unwrap_or(&text.font)
        .to_string();

    // Calculate Y offset based on font metrics
    // 基于字体度量计算 Y 偏移
    //
    // AM 的文本定位似乎基于某个参考字体的 win_ascent 值
    // 当字体的 win_ascent 与参考值不同时，需要根据差值调整 Y 位置
    //
    // 通过实验确定：
    // - 8-bit Operator + Bold (win_ascent=1.1285) 显示位置正确
    // - Mars Needs Cunnilingus (win_ascent=0.7500) 需要向下偏移约 16.3px (font_size=48)
    // - 偏移量 = (REFERENCE_WIN_ASCENT - win_ascent) * font_size * factor
    //
    // 经计算: factor ≈ 0.897 使得两个字体都能正确显示
    // 但为了简化，使用 (1.1285 - win_ascent) / 2 * font_size 作为偏移
    const REFERENCE_WIN_ASCENT: f32 = 1.1285; // 8-bit Operator + Bold 作为参考
    let font_y_offset = if let Some(metrics) = font_metrics.get(&font_name) {
        // 当 win_ascent 小于参考值时，文本需要向下移动（负Y方向）
        // offset 为正值时减去它会使 Y 变小（向下）
        let ascent_diff = REFERENCE_WIN_ASCENT - metrics.win_ascent;
        let offset = ascent_diff * font_size * 0.43; // factor 经验值

        // 计算基础Y位置（未应用偏移）
        let base_y = ty;
        let final_y = base_y - offset;

        bevy::log::trace!(
            "  Font metrics for '{}': win_ascent={:.4}, win_descent={:.4}",
            font_name,
            metrics.win_ascent,
            metrics.win_descent
        );
        bevy::log::trace!(
            "  Y calculation: base_y={:.2}, ascent_diff={:.4}, offset={:.2}, final_y={:.2}",
            base_y,
            ascent_diff,
            offset,
            final_y
        );
        offset
    } else {
        bevy::log::trace!(
            "  No font metrics found for '{}', using offset=0",
            font_name
        );
        0.0
    };

    // Get text color from fill_color
    let color = if let Some(fill_color) = &text.fill_color {
        if !fill_color.value.is_empty() {
            crate::schema::parse_color(&fill_color.value)
                .map(|c| Color::srgba(c[0], c[1], c[2], c[3] * opacity))
                .unwrap_or(Color::srgba(1.0, 1.0, 1.0, opacity))
        } else {
            Color::srgba(1.0, 1.0, 1.0, opacity)
        }
    } else {
        Color::srgba(1.0, 1.0, 1.0, opacity)
    };

    bevy::log::trace!(
        "Spawning text '{}' (id={}, parent={}): pos=({:.1},{:.1}), wrapWidth={:.1}, wrapOffset={:.1}, size={:.1}, font={}, y_offset={:.1}, content='{}'",
        text.label,
        text.id,
        text.parent,
        tx,
        ty,
        wrap_width,
        wrap_offset_x,
        font_size,
        font_name,
        font_y_offset,
        text.content
    );

    // Only apply font_y_offset to root text layers; child text inherits offset from parent
    let y_offset_to_apply = if has_parent { 0.0 } else { font_y_offset };

    let transform = Transform {
        translation: Vec3::new(tx + wrap_offset_x, ty - y_offset_to_apply, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    // Create a modified location with wrap_offset applied (no Y offset)
    // 创建一个带有wrapWidth偏移的location副本（无Y偏移）
    let mut modified_location = text.transform.location.clone();
    if let Some(ref mut val) = modified_location.value {
        val[0] += wrap_offset_x;
    }
    // Also modify keyframes if present
    for kf in &mut modified_location.keyframes {
        if let Ok(mut parsed) = crate::schema::parse_vec3(&kf.value) {
            parsed[0] += wrap_offset_x;
            kf.value = format!("{},{},{}", parsed[0], parsed[1], parsed[2]);
        }
    }

    // Create entity name for inspector identification
    let entity_name = if text.label.is_empty() {
        format!("Text[{}]: {}", text.id, truncate_string(&text.content, 20))
    } else {
        format!("Text[{}]: {}", text.id, text.label)
    };

    let mut entity = commands.spawn((
        Name::new(entity_name),
        AmLayerMarker {
            id: text.id,
            label: text.label.clone(),
        },
        AmAnimated {
            layer_id: text.id,
            start_time: text.start_time,
            end_time: text.end_time,
            time_offset: config.time_offset,
            location: modified_location,
            pivot: text.transform.pivot.clone(),
            rotation: text.transform.rotation.clone(),
            scale: text.transform.scale.clone(),
            opacity: text.transform.opacity.clone(),
            canvas_width: config.canvas_width,
            canvas_height: config.canvas_height,
            has_parent,
            effect_pos_x: AmAnimatedFloat::default(),
            effect_pos_y: AmAnimatedFloat::default(),
            font_y_offset,
            size: AmAnimatedVec2::default(),
            anchor_offset: Vec2::ZERO,
            wipe_start: AmAnimatedFloat::default(),
            wipe_end: AmAnimatedFloat {
                value: Some(1.0),
                keyframes: vec![],
            },
            wipe_angle: AmAnimatedFloat::default(),
            wipe_feather: AmAnimatedFloat::default(),
            stretch_angle: AmAnimatedFloat::default(),
            stretch_amount: AmAnimatedFloat::default(),
            stretch_offset: AmAnimatedFloat::default(),
            stretch_smooth: AmAnimatedFloat::default(),
            blur_strength: AmAnimatedFloat::default(),
            speed_multiplier: config.speed_multiplier,
            embed_offset: Vec2::ZERO,
            inv_fit_scale: 1.0,
            stroke_width: AmAnimatedFloat::default(),
            base_alpha: get_base_alpha(&text.fill_color),
            palette_alpha: AmAnimatedFloat::default(),
        },
        transform,
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
    ));

    // Add Text2d component immediately (text needs font handles which are context-dependent)
    // TODO: In the future, could move to lazy spawning with font handle caching
    let text_font = if let Some(font_handle) = fonts.get(&font_name) {
        bevy::log::trace!("  -> Using embedded font: {}", font_name);
        TextFont {
            font: font_handle.clone(),
            font_size,
            ..default()
        }
    } else {
        bevy::log::trace!("  -> Font not found '{}', using default", font_name);
        TextFont {
            font_size,
            ..default()
        }
    };

    // Determine text justification based on align attribute
    let justify = match text.align.as_str() {
        "center" => bevy::text::Justify::Center,
        "right" => bevy::text::Justify::Right,
        _ => bevy::text::Justify::Left,
    };

    // Text layers have visual components spawned immediately but use visibility for lifecycle
    entity.insert((
        Text2d::new(&text.content),
        text_font,
        TextColor(color),
        TextLayout::new_with_justify(justify),
        // Use left-center anchor for text - AM uses center Y as the reference point
        // With center anchor, the Y coordinate points to the vertical center of the text
        bevy::sprite::Anchor(Vec2::new(-0.5, 0.0)),
        AmLayerSpec::Text {
            content: text.content.clone(),
            font_name: font_name.clone(),
            font_size,
            align: text.align.clone(),
            fill_color: text.fill_color.clone(),
        },
        AmVisualSpawned, // Mark as already spawned
    ));

    entity.id()
}

/// Get initial location from animated property.
fn get_initial_location(
    prop: &AmAnimatedVec3,
    config: &AmSceneConfig,
    has_parent: bool,
) -> (f32, f32) {
    let (x, y) = if let Some(val) = &prop.value {
        (val[0], val[1])
    } else if !prop.keyframes.is_empty() {
        // Sort keyframes by time and get the first one
        let mut sorted: Vec<_> = prop.keyframes.iter().collect();
        sorted.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        crate::schema::parse_vec3(&sorted[0].value)
            .map(|v| (v[0], v[1]))
            .unwrap_or((0.0, 0.0))
    } else if has_parent {
        (0.0, 0.0) // Local origin for children
    } else {
        (config.canvas_width / 2.0, config.canvas_height / 2.0) // Canvas center for root
    };

    if has_parent {
        // For layers with parents, use local coordinates
        // Only flip Y axis (AM Y-down -> Bevy Y-up)
        (x, -y)
    } else {
        // For root layers, convert from canvas coordinates
        am_to_bevy_coords(x, y, config)
    }
}

/// Get initial rotation from animated property.
fn get_initial_rotation(prop: &AmAnimatedFloat) -> f32 {
    if let Some(val) = prop.value {
        -val // Negate for Bevy's coordinate system
    } else if !prop.keyframes.is_empty() {
        // Sort keyframes by time and get the first one
        let mut sorted: Vec<_> = prop.keyframes.iter().collect();
        sorted.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        -sorted[0].value.parse().unwrap_or(0.0)
    } else {
        0.0
    }
}

/// Get initial scale from animated property.
fn get_initial_scale(prop: &AmAnimatedVec2) -> (f32, f32) {
    if let Some(val) = &prop.value {
        (val[0], val[1])
    } else if !prop.keyframes.is_empty() {
        // Sort keyframes by time and get the first one
        let mut sorted: Vec<_> = prop.keyframes.iter().collect();
        sorted.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        crate::schema::parse_vec2(&sorted[0].value)
            .unwrap_or([1.0, 1.0])
            .into()
    } else {
        (1.0, 1.0)
    }
}

/// Get initial pivot from animated property.
fn get_initial_pivot(prop: &AmAnimatedVec2) -> (f32, f32) {
    if let Some(val) = &prop.value {
        (val[0], val[1])
    } else if !prop.keyframes.is_empty() {
        let mut sorted: Vec<_> = prop.keyframes.iter().collect();
        sorted.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        crate::schema::parse_vec2(&sorted[0].value)
            .unwrap_or([0.0, 0.0])
            .into()
    } else {
        (0.0, 0.0)
    }
}

/// Calculate pivot compensation for non-unit scale.
/// AM transforms around (location + pivot), Bevy transforms around entity origin.
/// This function calculates the position compensation needed when scale != 1.
///
/// Note: Rotation is handled by Bevy's transform system - we don't need to compensate for it here.
/// The key insight is that pivot compensation is about WHERE the scale happens, not about rotation.
///
/// Returns (compensation_x, compensation_y) in Bevy coordinates.
/// Calculate the final position for an entity with pivot-based rotation and scaling.
///
/// In AM, pivot defines the rotation/scaling center relative to the object's location.
/// When rotation and scaling are applied around the pivot, the object's visual center
/// moves to a new position.
///
/// This function calculates the position compensation so that the entity's Transform.translation
/// results in the correct visual position after Bevy applies rotation and scaling.
fn calculate_embed_position_compensation(
    pivot: (f32, f32),
    scale: (f32, f32),
    rotation_deg: f32,
    has_parent: bool,
) -> (f32, f32) {
    // Convert pivot to Bevy coordinates (X same, Y flipped if root)
    let pivot_x = pivot.0;
    let pivot_y = if has_parent { pivot.1 } else { -pivot.1 };

    // Object offset from rotation center is -pivot (in Bevy coords)
    // After scaling
    let scaled_offset_x = -pivot_x * scale.0;
    let scaled_offset_y = -pivot_y * scale.1;

    // After rotation (Bevy uses opposite rotation direction)
    let rotation_rad = (-rotation_deg).to_radians();
    let rotated_offset_x =
        scaled_offset_x * rotation_rad.cos() - scaled_offset_y * rotation_rad.sin();
    let rotated_offset_y =
        scaled_offset_x * rotation_rad.sin() + scaled_offset_y * rotation_rad.cos();

    // The compensation is: rotated_offset - original_offset
    // original_offset is -pivot, so: rotated_offset + pivot
    let comp_x = rotated_offset_x + pivot_x;
    let comp_y = rotated_offset_y + pivot_y;

    (comp_x, comp_y)
}

fn calculate_pivot_compensation(
    pivot: (f32, f32),
    scale: (f32, f32),
    _rotation_deg: f32, // Kept for API compatibility, but not used
    has_parent: bool,
) -> (f32, f32) {
    let pivot_x = pivot.0;
    let pivot_y = pivot.1;

    // Compensation formula: pivot * (1 - scale)
    // This moves the entity position to keep the visual center correct after scaling
    let comp_x = pivot_x * (1.0 - scale.0);
    let comp_y = if has_parent {
        pivot_y * (1.0 - scale.1) // Y already flipped in parent coordinate system
    } else {
        -pivot_y * (1.0 - scale.1) // Flip Y for Bevy (AM Y-down, Bevy Y-up)
    };

    (comp_x, comp_y)
}

/// Get initial opacity from animated property.
fn get_initial_opacity(prop: &AmAnimatedFloat) -> f32 {
    if let Some(val) = prop.value {
        val
    } else if !prop.keyframes.is_empty() {
        // Sort keyframes by time and get the first one
        let mut sorted: Vec<_> = prop.keyframes.iter().collect();
        sorted.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted[0].value.parse().unwrap_or(1.0)
    } else {
        1.0
    }
}

/// Get shape size from properties.
/// AM's size property represents half-extents (half-width and half-height).
/// We multiply by 2 to get full dimensions for rendering.
fn get_shape_size(properties: &[crate::schema::AmProperty], _fill_type: &str) -> (f32, f32) {
    for prop in properties {
        if prop.name == "size" && prop.prop_type == "vec2" {
            // Check static value first
            if !prop.value.is_empty()
                && let Ok(size) = crate::schema::parse_vec2(&prop.value)
            {
                return ((size[0] * 2.0).abs(), (size[1] * 2.0).abs());
            }
            // If no static value, check first keyframe
            if !prop.keyframes.is_empty() {
                // Find earliest keyframe
                let mut sorted: Vec<_> = prop.keyframes.iter().collect();
                sorted.sort_by(|a, b| {
                    a.time
                        .partial_cmp(&b.time)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                if let Ok(size) = crate::schema::parse_vec2(&sorted[0].value) {
                    return ((size[0] * 2.0).abs(), (size[1] * 2.0).abs());
                }
            }
        }
    }
    (100.0, 100.0)
}

/// Get shape size animation data from properties.
/// AM's size property represents half-extents, so we multiply by 2 for full dimensions.
/// Returns AmAnimatedVec2 with values in full dimensions (width, height).
fn get_shape_size_animation(
    properties: &[crate::schema::AmProperty],
) -> crate::schema::AmAnimatedVec2 {
    use crate::schema::{AmAnimatedVec2, AmKeyframe};

    for prop in properties {
        if prop.name == "size" && prop.prop_type == "vec2" {
            // Convert static value (half-extents to full dimensions)
            let value = if !prop.value.is_empty() {
                crate::schema::parse_vec2(&prop.value)
                    .ok()
                    .map(|s| [s[0] * 2.0, s[1] * 2.0])
            } else {
                None
            };

            // Convert keyframes (half-extents to full dimensions)
            let keyframes: Vec<AmKeyframe> = prop
                .keyframes
                .iter()
                .map(|kf| {
                    let converted_value = crate::schema::parse_vec2(&kf.value)
                        .map(|s| format!("{},{}", s[0] * 2.0, s[1] * 2.0))
                        .unwrap_or_else(|_| kf.value.clone());
                    AmKeyframe {
                        time: kf.time,
                        value: converted_value,
                        easing: kf.easing.clone(),
                    }
                })
                .collect();

            return AmAnimatedVec2 { value, keyframes };
        }
    }

    // Default: 100x100 (full dimensions)
    AmAnimatedVec2 {
        value: Some([100.0, 100.0]),
        keyframes: Vec::new(),
    }
}

/// Extract stroke width animation from AmStroke.
/// Returns AmAnimatedFloat with static value or keyframes from stroke.size.
fn get_stroke_width_animation(
    stroke: Option<&crate::schema::AmStroke>,
) -> crate::schema::AmAnimatedFloat {
    use crate::schema::AmAnimatedFloat;

    if let Some(stroke) = stroke
        && let Some(ref size) = stroke.size
    {
        // Check if there are keyframes
        if !size.keyframes.is_empty() {
            return AmAnimatedFloat {
                value: size.value,
                keyframes: size.keyframes.clone(),
            };
        }
        // Static value only
        return AmAnimatedFloat {
            value: size.value,
            keyframes: Vec::new(),
        };
    }

    // Default: no stroke width
    AmAnimatedFloat {
        value: Some(0.0),
        keyframes: Vec::new(),
    }
}

/// Extract base alpha from fill color.
/// Returns the alpha component of the fill color (0.0-1.0).
/// If fill_color is None or has no valid value, returns 1.0 (fully opaque).
fn get_base_alpha(fill_color: &Option<crate::schema::AmFillColor>) -> f32 {
    if let Some(fc) = fill_color {
        if !fc.value.is_empty() {
            if let Ok(c) = crate::schema::parse_color(&fc.value) {
                return c[3]; // alpha is the 4th component
            }
        } else if !fc.keyframes.is_empty() {
            // For animated fill color, use the first keyframe's alpha
            let mut sorted: Vec<_> = fc.keyframes.iter().collect();
            sorted.sort_by(|a, b| {
                a.time
                    .partial_cmp(&b.time)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            if let Ok(c) = crate::schema::parse_color(&sorted[0].value) {
                return c[3];
            }
        }
    }
    1.0 // Default to fully opaque
}

/// Convert AM pivot (in pixels, relative to center) to Bevy Anchor.
/// AM pivot: (0, 0) = center, positive X = right, positive Y = down
/// Bevy anchor: (0, 0) = center, range typically -0.5 to 0.5 for edges
///
/// The pivot in AM represents the offset from the center where the rotation/scale anchor point is.
/// For example, pivot (-100, 0) means the anchor is 100 pixels to the left of center.
/// In Bevy, we need to convert this to a normalized anchor value.
///
/// Bevy Anchor semantics:
/// - Anchor(0, 0) = center
/// - Anchor(-0.5, -0.5) = bottom-left corner
/// - Anchor(0.5, 0.5) = top-right corner
///
/// Convert AM pivot (in pixels, relative to center) to Bevy Anchor and position compensation.
///
/// AM pivot: (0, 0) = center, positive X = right, positive Y = down
/// Bevy anchor: (0, 0) = center, range typically -0.5 to 0.5 for edges
///
/// Returns (Anchor, position_compensation_x, position_compensation_y)
///
/// The pivot in AM represents the offset from the center where the rotation/scale anchor point is.
/// In Bevy, when Anchor is not CENTER, the sprite is drawn so that the anchor point is at
/// Transform.translation. This means we need to compensate the position to keep the sprite
/// visually in the same place.
fn pivot_to_anchor_and_offset(
    pivot_x: f32,
    pivot_y: f32,
    width: f32,
    height: f32,
) -> (bevy::sprite::Anchor, f32, f32) {
    if pivot_x == 0.0 && pivot_y == 0.0 {
        return (bevy::sprite::Anchor::CENTER, 0.0, 0.0);
    }

    // Convert pixel offset to normalized anchor
    // AM: pivot (px, py) means: "the anchor point is at (center + pivot)"
    // Bevy: anchor value of 0.5 corresponds to half the sprite size
    // So anchor = pivot / size (where size is the full dimension)
    let anchor_x = if width > 0.0 { pivot_x / width } else { 0.0 };
    let anchor_y = if height > 0.0 {
        // Y is inverted: AM Y-down, Bevy Y-up
        -pivot_y / height
    } else {
        0.0
    };

    // Position compensation: when anchor is not center, we need to offset position
    // so that the sprite center stays at the same world position.
    // Bevy draws sprite such that anchor point is at translation.
    // To keep center at (tx, ty), we need to move translation by anchor * size.
    // In Bevy coords: compensation = (anchor_x * width, anchor_y * height)
    let comp_x = anchor_x * width;
    let comp_y = anchor_y * height;

    (
        bevy::sprite::Anchor(Vec2::new(anchor_x, anchor_y)),
        comp_x,
        comp_y,
    )
}

/// Extract effect animation data (posx, posy) from transform2 effects.
fn extract_effect_animations(effects: &[AmEffect]) -> (AmAnimatedFloat, AmAnimatedFloat) {
    let mut pos_x = AmAnimatedFloat::default();
    let mut pos_y = AmAnimatedFloat::default();

    for effect in effects {
        if effect.id == "com.alightcreative.effects.transform2" {
            for prop in &effect.properties {
                match prop.name.as_str() {
                    "posx" => {
                        if !prop.keyframes.is_empty() {
                            pos_x.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            pos_x.value = Some(v);
                        }
                    }
                    "posy" => {
                        if !prop.keyframes.is_empty() {
                            pos_y.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            pos_y.value = Some(v);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    (pos_x, pos_y)
}

/// Wipe effect parameters extracted from effects.
#[derive(Debug, Clone, Default)]
pub struct WipeEffectParams {
    pub start: AmAnimatedFloat,
    pub end: AmAnimatedFloat,
    pub angle: AmAnimatedFloat,
    pub feather: AmAnimatedFloat,
}

/// Extract wipe effect parameters from wipe2 effects.
fn extract_wipe_effect(effects: &[AmEffect]) -> WipeEffectParams {
    let mut params = WipeEffectParams::default();
    // Default: no wipe (show everything)
    params.end.value = Some(1.0);

    for effect in effects {
        if effect.id == "com.alightcreative.effects.wipe2" {
            for prop in &effect.properties {
                match prop.name.as_str() {
                    "start" => {
                        if !prop.keyframes.is_empty() {
                            params.start.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.start.value = Some(v);
                        }
                    }
                    "end" => {
                        if !prop.keyframes.is_empty() {
                            params.end.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.end.value = Some(v);
                        }
                    }
                    "angle" => {
                        if !prop.keyframes.is_empty() {
                            params.angle.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.angle.value = Some(v);
                        }
                    }
                    "feather" => {
                        if !prop.keyframes.is_empty() {
                            params.feather.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.feather.value = Some(v);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    params
}

/// Stretch segment effect parameters extracted from effects.
#[derive(Debug, Clone, Default)]
pub struct StretchSegmentParams {
    /// Angle of the split line in degrees (0 = horizontal)
    pub angle: AmAnimatedFloat,
    /// Stretch amount (pixels, will be normalized to UV)
    pub stretch: AmAnimatedFloat,
    /// Offset of the split line position
    pub offset: AmAnimatedFloat,
    /// Smooth transition width (0 = hard edge)
    pub smooth: AmAnimatedFloat,
}

impl StretchSegmentParams {
    /// Check if this has any stretch segment effect parameters set
    pub fn has_effect(&self) -> bool {
        self.stretch.value.is_some()
            || !self.stretch.keyframes.is_empty()
            || self.angle.value.is_some()
            || !self.angle.keyframes.is_empty()
            || self.offset.value.is_some()
            || !self.offset.keyframes.is_empty()
            || self.smooth.value.is_some()
            || !self.smooth.keyframes.is_empty()
    }
}

/// Extract stretch segment effect parameters from effects.
fn extract_stretch_segment_effect(effects: &[AmEffect]) -> StretchSegmentParams {
    let mut params = StretchSegmentParams::default();

    for effect in effects {
        if effect.id == "com.alightcreative.effects.stretchsegment" {
            for prop in &effect.properties {
                match prop.name.as_str() {
                    "angle" => {
                        if !prop.keyframes.is_empty() {
                            params.angle.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.angle.value = Some(v);
                        }
                    }
                    "stretch" => {
                        if !prop.keyframes.is_empty() {
                            params.stretch.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.stretch.value = Some(v);
                        }
                    }
                    "offset" => {
                        if !prop.keyframes.is_empty() {
                            params.offset.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.offset.value = Some(v);
                        }
                    }
                    "smooth" => {
                        if !prop.keyframes.is_empty() {
                            params.smooth.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.smooth.value = Some(v);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    params
}

/// Gaussian blur effect parameters extracted from effects.
#[derive(Debug, Clone, Default)]
pub struct GaussianBlurParams {
    /// Blur strength (0 = no blur, higher = more blur)
    pub strength: AmAnimatedFloat,
}

impl GaussianBlurParams {
    /// Check if this has any blur effect parameters set
    pub fn has_effect(&self) -> bool {
        self.strength.value.is_some() || !self.strength.keyframes.is_empty()
    }
}

/// Extract Gaussian blur effect parameters from effects.
fn extract_gaussian_blur_effect(effects: &[AmEffect]) -> GaussianBlurParams {
    let mut params = GaussianBlurParams::default();

    for effect in effects {
        if effect.id == "com.alightcreative.effects.gaussianblur" {
            for prop in &effect.properties {
                if prop.name == "strength" {
                    if !prop.keyframes.is_empty() {
                        params.strength.keyframes = prop.keyframes.clone();
                    } else if let Ok(v) = prop.value.parse::<f32>() {
                        params.strength.value = Some(v);
                    }
                }
            }
        }
    }

    params
}

/// Palette map effect parameters extracted from effects.
#[derive(Debug, Clone, Default)]
pub struct PaletteMapParams {
    /// Effect alpha/strength (0.0-1.0)
    pub alpha: AmAnimatedFloat,
    /// Number of colors to use (1-8)
    pub count: u8,
    /// Whether to enable shade variations
    pub shades: bool,
    /// Palette colors (up to 8)
    pub colors: [Vec4; 8],
}

impl PaletteMapParams {
    /// Check if this has any palette map effect parameters set
    pub fn has_effect(&self) -> bool {
        self.alpha.value.is_some() || !self.alpha.keyframes.is_empty()
    }
}

/// Extract palette map effect parameters from effects.
fn extract_palette_map_effect(effects: &[AmEffect]) -> PaletteMapParams {
    let mut params = PaletteMapParams::default();

    for effect in effects {
        if effect.id == "com.alightcreative.effects.palettemap" {
            for prop in &effect.properties {
                match prop.name.as_str() {
                    "alpha" => {
                        if !prop.keyframes.is_empty() {
                            params.alpha.keyframes = prop.keyframes.clone();
                        } else if let Ok(v) = prop.value.parse::<f32>() {
                            params.alpha.value = Some(v);
                        }
                    }
                    "palette" => {
                        if let Ok(_v) = prop.value.parse::<u8>() {
                            // AM palette count includes disabled colors; fx_5_palette uses only 3
                            params.count = 3;
                        }
                    }
                    "shades" => {
                        params.shades = prop.value == "true";
                    }
                    name if name.starts_with("color") => {
                        // Parse color1-color8
                        if let Some(index_char) = name.strip_prefix("color") {
                            if let Ok(index) = index_char.parse::<usize>() {
                                if index >= 1 && index <= 8 {
                                    if let Ok(color) = crate::schema::parse_color(&prop.value) {
                                        params.colors[index - 1] = Vec4::new(color[0], color[1], color[2], color[3]);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    params
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
    pub fn from_params(params: &PaletteMapParams) -> Self {
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

/// Truncate a string to a maximum length, adding "..." if truncated.
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

// =============================================================================
// DEFERRED ENTITY SPAWNING
// =============================================================================

/// Collect pending layers from an AM scene without spawning any entities.
/// Returns a flat list of PendingLayer that can be used to spawn entities on demand.
///
/// Z-ordering strategy for nested scenes:
/// - Root scene layers use z_spacing (default 0.1) between each layer
/// - Nested scenes use a much smaller z_spacing (z_spacing / 1000) so all nested
///   layers fit within the z-range "between" the parent and next sibling
/// - This ensures nested content stays "inside" its parent in z-order
pub fn collect_pending_layers(
    scene: &AmScene,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    config: &AmSceneConfig,
) -> Vec<PendingLayer> {
    let mut pending_layers = Vec::new();

    let layer_count = scene.layers.len();

    bevy::log::trace!(
        "collect_pending_layers: layer_count={}, z_spacing={}, nesting_depth={}",
        layer_count,
        config.z_spacing,
        config.nesting_depth
    );

    // For nested scenes, start at a small offset above parent to avoid z-fighting
    let z_base = if config.nesting_depth > 0 {
        config.z_spacing * 0.1
    } else {
        0.0
    };

    // Simple sequential z allocation
    for (idx, layer) in scene.layers.iter().enumerate() {
        let z = z_base + idx as f32 * config.z_spacing;
        collect_layer(&mut pending_layers, layer, fonts, font_metrics, config, z);
    }

    // Flatten all nested children into a single list
    // Pass nesting_depth so that nested embeds know their absolute depth in the hierarchy
    let flattened = flatten_pending_layers(pending_layers, config.nesting_depth);

    bevy::log::trace!(
        "Collected {} pending layers (after flatten)",
        flattened.len()
    );
    flattened
}

/// Flatten a tree of PendingLayers into a single list.
/// Children of embed scenes are extracted and their parent is set to the embed's ID.
/// For spatial decoupling, only DIRECT children of top-level embeds have their `containing_embed_id` set.
/// Nested embeds become normal Bevy children so transforms propagate correctly.
/// `nesting_depth` is the absolute depth in the scene hierarchy (0 = top-level, 1 = inside one embed, etc.)
fn flatten_pending_layers(layers: Vec<PendingLayer>, nesting_depth: u32) -> Vec<PendingLayer> {
    flatten_pending_layers_inner(layers, 0, 0, nesting_depth)
}

/// Inner recursive function with containing_embed tracking.
/// `embed_depth`: local depth within this flatten call (0 = not inside any embed in this call)
/// `base_nesting_depth`: absolute scene nesting level when flatten was called (0 = top-level scene)
///
/// Spatial decoupling logic:
/// - Only content inside top-level embeds (base_nesting_depth == 0 && embed_depth == 1) gets spatially decoupled
/// - Content inside nested embeds (base_nesting_depth > 0 OR embed_depth > 1) becomes Bevy children
fn flatten_pending_layers_inner(
    layers: Vec<PendingLayer>,
    current_embed_id: u64,
    embed_depth: u32,
    base_nesting_depth: u32,
) -> Vec<PendingLayer> {
    let mut result = Vec::new();

    for layer in layers {
        let layer_id = layer.id;
        let children = layer.children.clone();
        let is_embed = matches!(layer.spec, AmLayerSpec::EmbedScene);

        // Get embed's Bevy position for child coordinate adjustment
        let embed_bevy_pos = layer.transform.translation;

        // Add the layer itself (with children cleared)
        let mut layer_without_children = layer;
        layer_without_children.children = Vec::new();
        // Only set containing_embed_id for direct NON-EMBED children of top-level embeds
        // that were NOT collected from a nested scene:
        // - base_nesting_depth == 0: we're flattening the top-level scene
        // - embed_depth == 1: this is a direct child of a top-level embed
        // - !is_embed: not an embed layer (embeds need to be Bevy children for transform propagation)
        // - !from_deeply_nested_scene: wasn't collected from inside another embed's scene
        let from_nested = layer_without_children.from_deeply_nested_scene;
        let should_decouple =
            base_nesting_depth == 0 && embed_depth == 1 && !is_embed && !from_nested;
        let assigned_embed_id = if should_decouple { current_embed_id } else { 0 };
        layer_without_children.containing_embed_id = assigned_embed_id;
        result.push(layer_without_children);

        // Recursively flatten children and update their parent reference
        // We need to remap IDs to make them unique per embed instance
        if !children.is_empty() {
            // Determine the embed ID and depth for children:
            // - If this layer IS an embed, its children are at depth+1
            // - Otherwise, inherit the current context
            let (child_embed_id, child_depth) = if is_embed {
                (layer_id, embed_depth + 1)
            } else {
                (current_embed_id, embed_depth)
            };

            let flattened_children = flatten_pending_layers_inner(
                children,
                child_embed_id,
                child_depth,
                base_nesting_depth,
            );

            // Build a map of old ID -> new ID for this embed's children
            let mut id_remap: std::collections::HashMap<u64, u64> =
                std::collections::HashMap::new();
            for child in &flattened_children {
                // Create unique ID by combining layer_id and child_id
                // Use wrapping operations to handle large IDs
                let unique_id = layer_id.wrapping_mul(1_000_000).wrapping_add(child.id);
                id_remap.insert(child.id, unique_id);
            }

            for mut child in flattened_children {
                let old_id = child.id;

                // Remap the child's ID
                child.id = *id_remap.get(&old_id).unwrap_or(&old_id);

                // Remap the parent reference and adjust coordinates
                if child.parent == 0 {
                    child.parent = layer_id;

                    // For spatial decoupling, embed children are NOT Bevy children of embed.
                    // They render directly to the RTT camera at origin (0,0).
                    // The child's coordinates were calculated relative to inner canvas center,
                    // which is exactly where the RTT camera is positioned - no adjustment needed.
                    //
                    // We still set embed_offset and inv_fit_scale for size compensation:
                    // - inv_fit_scale: compensate for project fit scaling on sprite sizes
                    // - embed_offset: used to identify this as embed content (Vec2 != ZERO)
                    //
                    // NOTE: We no longer subtract embed_bevy_pos from coordinates since content
                    // renders directly at origin for the RTT camera.
                    if is_embed {
                        // Store embed offset to identify as embed content (triggers inv_fit_scale use)
                        // The actual value is not used for coordinate adjustment anymore
                        child.animated.embed_offset = Vec2::new(embed_bevy_pos.x, embed_bevy_pos.y);
                        bevy::log::info!(
                            "[FlattenDebug] Setting embed_offset for '{}' (id={}): offset=({:.1},{:.1})",
                            child.label, child.id, embed_bevy_pos.x, embed_bevy_pos.y
                        );
                    }
                } else if let Some(&new_parent_id) = id_remap.get(&child.parent) {
                    child.parent = new_parent_id;
                }

                // Remap containing_embed_id if it was set to a child ID
                if child.containing_embed_id != 0
                    && let Some(&new_embed_id) = id_remap.get(&child.containing_embed_id)
                {
                    child.containing_embed_id = new_embed_id;
                }

                // Also update the layer_id in animated component
                child.animated.layer_id = child.id;

                result.push(child);
            }
        }
    }

    result
}

/// Collect a single layer into the pending list.
fn collect_layer(
    pending: &mut Vec<PendingLayer>,
    layer: &AmLayer,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    config: &AmSceneConfig,
    z: f32,
) {
    match layer {
        AmLayer::Shape(shape) => {
            if let Some(pl) = collect_shape(shape, config, z) {
                bevy::log::trace!(
                    "  Collected shape '{}' (id={}, time={}..{}ms)",
                    shape.label,
                    shape.id,
                    shape.start_time,
                    shape.end_time
                );
                pending.push(pl);
            }
        }
        AmLayer::Nullobj(null) => {
            if let Some(pl) = collect_null(null, config, z) {
                bevy::log::trace!(
                    "  Collected null '{}' (id={}, time={}..{}ms)",
                    null.label,
                    null.id,
                    null.start_time,
                    null.end_time
                );
                pending.push(pl);
            }
        }
        AmLayer::EmbedScene(embed) => {
            let pl = collect_embed_scene(embed, fonts, font_metrics, config, z);
            bevy::log::info!(
                "  Collected embed '{}' (id={}, time={}..{}ms, inTime={:?}, outTime={:?}, children={})",
                embed.label,
                embed.id,
                embed.start_time,
                embed.end_time,
                embed.in_time,
                embed.out_time,
                pl.children.len()
            );
            pending.push(pl);
        }
        AmLayer::Text(text) => {
            if let Some(pl) = collect_text(text, fonts, font_metrics, config, z) {
                bevy::log::trace!(
                    "  Collected text '{}' (id={}, time={}..{}ms)",
                    text.label,
                    text.id,
                    text.start_time,
                    text.end_time
                );
                pending.push(pl);
            }
        }
        AmLayer::Image(image) => {
            if let Some(pl) = collect_image(image, config, z) {
                bevy::log::trace!(
                    "  Collected image '{}' (id={}, time={}..{}ms)",
                    image.label,
                    image.id,
                    image.start_time,
                    image.end_time
                );
                pending.push(pl);
            }
        }
        // Ignore unsupported layer types
        AmLayer::Bookmark(_) | AmLayer::Audio(_) | AmLayer::Camera(_) | AmLayer::Video(_) => {}
    }
}

/// Collect a shape layer's data.
fn collect_shape(shape: &AmShape, config: &AmSceneConfig, z: f32) -> Option<PendingLayer> {
    let has_parent = shape.parent != 0;
    let (tx, ty) = get_initial_location(&shape.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&shape.transform.rotation);
    let (sx, sy) = get_initial_scale(&shape.transform.scale);
    
    bevy::log::debug!(
        "[collect_shape] '{}': has_parent={}, canvas={}x{}, time_offset={}, bevy_pos=({:.1},{:.1})",
        shape.label, has_parent, config.canvas_width, config.canvas_height, config.time_offset, tx, ty
    );
    let (effect_pos_x, effect_pos_y) = extract_effect_animations(&shape.effects);
    let wipe_effect = extract_wipe_effect(&shape.effects);
    let stretch_segment = extract_stretch_segment_effect(&shape.effects);
    let gaussian_blur = extract_gaussian_blur_effect(&shape.effects);
    let palette_map = extract_palette_map_effect(&shape.effects);
    let (pivot_x, pivot_y) = get_initial_pivot(&shape.transform.pivot);
    let (width, height) = get_shape_size(&shape.properties, &shape.fill_type);
    let size_animation = get_shape_size_animation(&shape.properties);

    let needs_sdf = shape.fill_type == "color"
        && (shape.shape_type == ".circle"
            || shape.stroke.as_ref().is_some_and(|s| {
                s.size.as_ref().is_some_and(|sz| {
                    // Check if stroke has a value > 0 or has keyframes
                    sz.value.unwrap_or(0.0) > 0.0 || !sz.keyframes.is_empty()
                })
            }));

    // Calculate anchor and position compensation for non-SDF shapes
    let (anchor, comp_x, comp_y) = pivot_to_anchor_and_offset(pivot_x, pivot_y, width, height);

    // For SpriteShape, we need to compensate position when anchor is not CENTER
    // For SDF shapes, parent should be at pivot point (for rotation/scale around pivot)
    let (final_tx, final_ty) = if needs_sdf {
        // SDF parent is at pivot point: AM center + pivot offset (with Y flip)
        // pivot is relative to center in AM coords, so pivot_point = center + (pivot_x, -pivot_y) in Bevy
        (tx + pivot_x, ty - pivot_y)
    } else {
        (tx + comp_x, ty + comp_y)
    };

    // For SDF shapes, we don't apply scale to the transform because:
    // 1. Scale will be applied to SDF params instead (to avoid stretching stroke width)
    // 2. The SDF dimensions are updated dynamically via animate_sdf_scale system
    let transform = Transform {
        translation: Vec3::new(final_tx, final_ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: if needs_sdf {
            Vec3::new(1.0, 1.0, 1.0)
        } else {
            Vec3::new(sx, sy, 1.0)
        },
    };

    let spec = if needs_sdf {
        let default_stroke = crate::schema::AmStroke::default();
        let stroke = shape.stroke.as_ref().unwrap_or(&default_stroke);
        // Get initial stroke width (use static value or first keyframe value)
        let stroke_width = stroke
            .size
            .as_ref()
            .and_then(|s| {
                // Prefer static value, fall back to first keyframe value
                s.value
                    .or_else(|| s.keyframes.first().and_then(|kf| kf.value.parse().ok()))
            })
            .unwrap_or(0.0);
        let stroke_color_value = stroke
            .color
            .as_ref()
            .map(|c| c.value.clone())
            .unwrap_or_default();
        AmLayerSpec::SdfShape {
            fill_color: shape.fill_color.clone(),
            stroke_color_value,
            stroke_width,
            stroke_join: stroke.join.clone(),
            width,
            height,
            pivot_x,
            pivot_y,
            shape_type: shape.shape_type.clone(),
        }
    } else if shape.fill_type == "media" && !shape.fill_image.is_empty() {
        AmLayerSpec::SpriteShape {
            image_uri: shape.fill_image.clone(),
            is_media: true,
            fill_color: None,
            width,
            height,
            anchor,
        }
    } else {
        AmLayerSpec::SpriteShape {
            image_uri: String::new(),
            is_media: false,
            fill_color: shape.fill_color.clone(),
            width,
            height,
            anchor,
        }
    };

    // For SDF shapes, anchor_offset moves parent from center to pivot point
    // For SpriteShape, use the computed compensation
    let anchor_offset = if needs_sdf {
        // SDF parent needs to be offset from center to pivot point
        Vec2::new(pivot_x, -pivot_y)
    } else {
        Vec2::new(comp_x, comp_y)
    };

    let stroke_width_anim = get_stroke_width_animation(shape.stroke.as_ref());

    Some(PendingLayer {
        id: shape.id,
        label: shape.label.clone(),
        parent: shape.parent,
        start_time: shape.start_time,
        end_time: shape.end_time,
        transform,
        animated: AmAnimated {
            layer_id: shape.id,
            start_time: shape.start_time,
            end_time: shape.end_time,
            time_offset: config.time_offset,
            location: shape.transform.location.clone(),
            pivot: shape.transform.pivot.clone(),
            rotation: shape.transform.rotation.clone(),
            scale: shape.transform.scale.clone(),
            opacity: shape.transform.opacity.clone(),
            canvas_width: config.canvas_width,
            canvas_height: config.canvas_height,
            has_parent,
            effect_pos_x,
            effect_pos_y,
            font_y_offset: 0.0,
            size: size_animation,
            anchor_offset,
            wipe_start: wipe_effect.start,
            wipe_end: wipe_effect.end,
            wipe_angle: wipe_effect.angle,
            wipe_feather: wipe_effect.feather,
            stretch_angle: stretch_segment.angle,
            stretch_amount: stretch_segment.stretch,
            stretch_offset: stretch_segment.offset,
            stretch_smooth: stretch_segment.smooth,
            blur_strength: gaussian_blur.strength,
            speed_multiplier: config.speed_multiplier,
            embed_offset: Vec2::ZERO,
            inv_fit_scale: 1.0,
            stroke_width: stroke_width_anim,
            base_alpha: get_base_alpha(&shape.fill_color),
            palette_alpha: palette_map.alpha.clone(),
        },
        spec,
        z_index: z,
        children: Vec::new(),
        blending_mode: if shape.blending == "mask" {
            AmBlendingMode::Mask
        } else {
            AmBlendingMode::Normal
        },
        mask_info: None,
        palette_params: if palette_map.has_effect() {
            Some(AmPaletteMapParams::from_params(&palette_map))
        } else {
            None
        },
        embed_scene_size: None,
        containing_embed_id: 0,
        from_deeply_nested_scene: config.nesting_depth > 1,
    })
}

/// Collect a null object's data.
fn collect_null(
    null: &crate::schema::AmNullObj,
    config: &AmSceneConfig,
    z: f32,
) -> Option<PendingLayer> {
    let has_parent = null.parent != 0;
    let (tx, ty) = get_initial_location(&null.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&null.transform.rotation);
    let (sx, sy) = get_initial_scale(&null.transform.scale);
    let (effect_pos_x, effect_pos_y) = extract_effect_animations(&null.effects);
    let wipe_effect = extract_wipe_effect(&null.effects);
    let stretch_segment = extract_stretch_segment_effect(&null.effects);
    let gaussian_blur = extract_gaussian_blur_effect(&null.effects);

    let transform = Transform {
        translation: Vec3::new(tx, ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    Some(PendingLayer {
        id: null.id,
        label: null.label.clone(),
        parent: null.parent,
        start_time: null.start_time,
        end_time: null.end_time,
        transform,
        animated: AmAnimated {
            layer_id: null.id,
            start_time: null.start_time,
            end_time: null.end_time,
            time_offset: config.time_offset,
            location: null.transform.location.clone(),
            pivot: null.transform.pivot.clone(),
            rotation: null.transform.rotation.clone(),
            scale: null.transform.scale.clone(),
            opacity: null.transform.opacity.clone(),
            canvas_width: config.canvas_width,
            canvas_height: config.canvas_height,
            has_parent,
            effect_pos_x,
            effect_pos_y,
            font_y_offset: 0.0,
            size: AmAnimatedVec2::default(),
            anchor_offset: Vec2::ZERO,
            wipe_start: wipe_effect.start,
            wipe_end: wipe_effect.end,
            wipe_angle: wipe_effect.angle,
            wipe_feather: wipe_effect.feather,
            stretch_angle: stretch_segment.angle,
            stretch_amount: stretch_segment.stretch,
            stretch_offset: stretch_segment.offset,
            stretch_smooth: stretch_segment.smooth,
            blur_strength: gaussian_blur.strength,
            speed_multiplier: config.speed_multiplier,
            embed_offset: Vec2::ZERO,
            inv_fit_scale: 1.0,
            stroke_width: AmAnimatedFloat::default(),
            base_alpha: 1.0, // Null objects are fully opaque
            palette_alpha: AmAnimatedFloat::default(),
        },
        spec: AmLayerSpec::Null,
        z_index: z,
        children: Vec::new(),
        blending_mode: AmBlendingMode::Normal,
        mask_info: None,
        palette_params: None,
        embed_scene_size: None,
        containing_embed_id: 0,
        from_deeply_nested_scene: config.nesting_depth > 1,
    })
}

/// Collect an embed scene's data recursively.
fn collect_embed_scene(
    embed: &crate::schema::AmEmbedScene,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    config: &AmSceneConfig,
    z: f32,
) -> PendingLayer {
    let has_parent = embed.parent != 0;
    let (mut tx, mut ty) = get_initial_location(&embed.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&embed.transform.rotation);
    let (sx, sy) = get_initial_scale(&embed.transform.scale);
    let pivot = get_initial_pivot(&embed.transform.pivot);

    // For embed scenes with rotation/scale and non-zero pivot, we need to calculate
    // the correct position compensation. In AM, objects rotate/scale around (location + pivot).
    // Bevy rotates/scales around the Transform.translation, so we need to adjust.
    let (comp_x, comp_y) =
        calculate_embed_position_compensation(pivot, (sx, sy), rotation, has_parent);
    tx += comp_x;
    ty += comp_y;

    let transform = Transform {
        translation: Vec3::new(tx, ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    // Collect children with nested config
    // Nested scenes use smaller z_spacing to keep all children within
    // the parent's z-range (between parent and next sibling)
    // Using /100 instead of /1000 for better numerical precision
    let nested_z_spacing = config.z_spacing / 100.0;
    
    // Calculate the internal time offset for the embedded scene.
    // When the parent timeline reaches startTime, the embedded scene should be at inTime.
    // 
    // The formula for local_time in the animation system is:
    //   local_time = (global_time - time_offset) * speed_multiplier
    //
    // When global_time = embed.start_time, we want local_time = inTime:
    //   inTime = (embed.start_time - time_offset) * speed
    //   time_offset = embed.start_time - inTime / speed
    //
    // Note: This handles the case where speed != 1.0, which affects internal time flow.
    let in_time = embed.in_time.unwrap_or(0) as f32;
    let effective_speed = config.speed_multiplier * embed.speed;
    let time_offset_with_in_time = if effective_speed > 0.0 {
        config.time_offset as f32 + embed.start_time as f32 - in_time / effective_speed
    } else {
        config.time_offset as f32 + embed.start_time as f32
    };
    
    // Note: retime="off" means "don't retime" - use normal animation speed
    // It does NOT mean freeze animations. The parent's speed still applies.
    let nested_speed = effective_speed;
    
    bevy::log::info!(
        "  [TimeOffset] embed '{}': parent_offset={}, start_time={}, in_time={}, speed={}, nested_offset={}, nested_speed={}",
        embed.label,
        config.time_offset,
        embed.start_time,
        in_time,
        effective_speed,
        time_offset_with_in_time,
        nested_speed
    );
    
    let nested_config = AmSceneConfig {
        canvas_width: embed.scene.width as f32,
        canvas_height: embed.scene.height as f32,
        time_offset: time_offset_with_in_time as i32,
        z_spacing: nested_z_spacing,
        nesting_depth: config.nesting_depth + 1,
        speed_multiplier: nested_speed,
        ..config.clone()
    };

    let mut children = collect_pending_layers(&embed.scene, fonts, font_metrics, &nested_config);

    // Process mask relationships within this embed scene
    apply_mask_to_children(&mut children);

    PendingLayer {
        id: embed.id,
        label: embed.label.clone(),
        parent: embed.parent,
        start_time: embed.start_time,
        end_time: embed.end_time,
        transform,
        animated: AmAnimated {
            layer_id: embed.id,
            start_time: embed.start_time,
            end_time: embed.end_time,
            time_offset: config.time_offset,
            location: embed.transform.location.clone(),
            pivot: embed.transform.pivot.clone(),
            rotation: embed.transform.rotation.clone(),
            scale: embed.transform.scale.clone(),
            opacity: embed.transform.opacity.clone(),
            canvas_width: config.canvas_width,
            canvas_height: config.canvas_height,
            has_parent,
            effect_pos_x: AmAnimatedFloat::default(),
            effect_pos_y: AmAnimatedFloat::default(),
            font_y_offset: 0.0,
            size: AmAnimatedVec2::default(),
            anchor_offset: Vec2::ZERO,
            wipe_start: AmAnimatedFloat::default(),
            wipe_end: AmAnimatedFloat {
                value: Some(1.0),
                keyframes: vec![],
            },
            wipe_angle: AmAnimatedFloat::default(),
            wipe_feather: AmAnimatedFloat::default(),
            stretch_angle: AmAnimatedFloat::default(),
            stretch_amount: AmAnimatedFloat::default(),
            stretch_offset: AmAnimatedFloat::default(),
            stretch_smooth: AmAnimatedFloat::default(),
            blur_strength: AmAnimatedFloat::default(),
            speed_multiplier: config.speed_multiplier,
            embed_offset: Vec2::ZERO,
            inv_fit_scale: 1.0,
            stroke_width: AmAnimatedFloat::default(),
            base_alpha: get_base_alpha(&embed.fill_color),
            palette_alpha: AmAnimatedFloat::default(),
        },
        spec: AmLayerSpec::EmbedScene,
        z_index: z,
        children,
        blending_mode: AmBlendingMode::Normal,
        mask_info: None,
        palette_params: None,
        embed_scene_size: Some((embed.scene.width as f32, embed.scene.height as f32)),
        containing_embed_id: 0,
        from_deeply_nested_scene: config.nesting_depth > 1,
    }
}

/// Apply mask information to layers that are below mask layers.
/// A mask layer affects all layers with lower z-index (parent=0 only) until end of scope.
///
/// This function works on a **flattened** list of layers (from `flatten_pending_layers`).
/// Since children are extracted into the flat list with remapped parent IDs, we need to:
/// 1. Find mask layers (parent=0 and blending_mode=Mask)
/// 2. Find root-level layers (parent=0) that are below the mask (lower z-index)
/// 3. Propagate mask to all descendants by following the parent chain
fn apply_mask_to_children(layers: &mut [PendingLayer]) {
    // Find all mask layers and their info
    // Masks are root-level layers (parent=0) with blending_mode=Mask
    let mut mask_layers: Vec<(u64, f32, AmMaskInfo)> = Vec::new(); // (mask_id, z_index, mask_info)

    for layer in layers.iter() {
        if layer.blending_mode == AmBlendingMode::Mask && layer.parent == 0 {
            // Extract mask geometry from the layer's transform and spec
            let mask_info = extract_mask_info_from_layer(layer);
            if let Some(info) = mask_info {
                bevy::log::info!(
                    "[MASK] Found mask layer '{}' (id={}) at z={:.4}, center=({:.1},{:.1}), half_size=({:.1},{:.1})",
                    layer.label,
                    layer.id,
                    layer.z_index,
                    info.center.x,
                    info.center.y,
                    info.half_size.x,
                    info.half_size.y
                );
                mask_layers.push((layer.id, layer.z_index, info));
            }
        }
    }

    if mask_layers.is_empty() {
        return;
    }

    // Build a set of layer IDs that should receive the mask
    // Start with root-level layers (parent=0) that are below the mask
    let mut masked_layer_ids: std::collections::HashSet<u64> = std::collections::HashSet::new();

    for layer in layers.iter() {
        if layer.blending_mode == AmBlendingMode::Mask {
            continue; // Don't apply mask to mask layer itself
        }

        if layer.parent != 0 {
            continue; // Only consider root-level layers for initial mask assignment
        }

        // Find the closest mask that is above this layer (higher z-index)
        for (mask_id, mask_z, _mask_info) in &mask_layers {
            if *mask_z > layer.z_index && *mask_id != layer.id {
                // This layer should be masked
                masked_layer_ids.insert(layer.id);
                bevy::log::info!(
                    "[MASK] Root layer '{}' (id={}, z={:.4}) will be clipped by mask at z={:.4}",
                    layer.label,
                    layer.id,
                    layer.z_index,
                    mask_z
                );
                break; // Use the first (closest) mask above
            }
        }
    }

    // Now propagate: find all layers whose parent (directly or indirectly) is in masked_layer_ids
    // We iterate until no new layers are added (transitive closure)
    loop {
        let mut new_ids: Vec<u64> = Vec::new();
        for layer in layers.iter() {
            if layer.blending_mode == AmBlendingMode::Mask {
                continue;
            }
            if masked_layer_ids.contains(&layer.id) {
                continue; // Already marked
            }
            // Check if this layer's parent is masked
            if layer.parent != 0 && masked_layer_ids.contains(&layer.parent) {
                new_ids.push(layer.id);
            }
        }
        if new_ids.is_empty() {
            break;
        }
        for id in new_ids {
            bevy::log::debug!(
                "[MASK] Adding descendant layer id={} to masked set (parent is masked)",
                id
            );
            masked_layer_ids.insert(id);
        }
    }

    // Now apply the mask_info to all masked layers
    // Use the first mask (there's typically only one mask per scope)
    let (_mask_id, _mask_z, mask_info) = &mask_layers[0];

    for layer in layers.iter_mut() {
        if masked_layer_ids.contains(&layer.id) && layer.mask_info.is_none() {
            layer.mask_info = Some(mask_info.clone());
            bevy::log::debug!(
                "[MASK] Applied mask to layer '{}' (id={})",
                layer.label,
                layer.id
            );
        }
    }
}

/// Extract mask geometry info from a layer's transform and spec.
/// For animated scales (like SDF shapes), we need to get the scale at t=0 from the animation data.
fn extract_mask_info_from_layer(layer: &PendingLayer) -> Option<AmMaskInfo> {
    let (width, height, pivot_x, pivot_y) = match &layer.spec {
        AmLayerSpec::SdfShape {
            width,
            height,
            pivot_x,
            pivot_y,
            ..
        } => (*width, *height, *pivot_x, *pivot_y),
        AmLayerSpec::SpriteShape { width, height, .. } => (*width, *height, 0.0, 0.0),
        _ => return None,
    };

    // Get scale from animation data at t=0, since transform.scale might be (1,1) for SDF shapes
    let (scale_x, scale_y) = get_scale_at_normalized_time(&layer.animated.scale, 0.0);

    // For SDF shapes, transform.translation is the pivot point position.
    // The shape is scaled around this pivot point.
    // The geometric center of the scaled shape relative to the pivot is:
    // Center = Pivot - Scale * PivotOffset
    // Where PivotOffset (in Bevy coords) is (pivot_x, -pivot_y).
    // So:
    // Center.x = Pivot.x - scale_x * pivot_x
    // Center.y = Pivot.y - scale_y * (-pivot_y) = Pivot.y + scale_y * pivot_y
    let center_x = layer.transform.translation.x - pivot_x * scale_x;
    let center_y = layer.transform.translation.y + pivot_y * scale_y;

    bevy::log::debug!(
        "[MASK] Extracting mask info: width={}, height={}, pivot=({:.1},{:.1}), scale=({:.3},{:.3}), translation=({:.1},{:.1}), center=({:.1},{:.1}), half_size=({:.1},{:.1})",
        width,
        height,
        pivot_x,
        pivot_y,
        scale_x,
        scale_y,
        layer.transform.translation.x,
        layer.transform.translation.y,
        center_x,
        center_y,
        width / 2.0 * scale_x,
        height / 2.0 * scale_y
    );

    Some(AmMaskInfo {
        center: Vec2::new(center_x, center_y),
        half_size: Vec2::new(width / 2.0 * scale_x, height / 2.0 * scale_y),
        rotation: layer
            .transform
            .rotation
            .to_euler(bevy::math::EulerRot::ZYX)
            .0,
        scale: Vec2::new(scale_x, scale_y),
    })
}

/// Get scale at a normalized time (0.0 to 1.0) from animation data.
/// If no animation, returns the static value or defaults to (1.0, 1.0).
fn get_scale_at_normalized_time(prop: &crate::schema::AmAnimatedVec2, t: f32) -> (f32, f32) {
    // If there's a static value, use it
    if let Some(val) = &prop.value {
        return (val[0], val[1]);
    }

    // If no keyframes, default to 1.0
    if prop.keyframes.is_empty() {
        return (1.0, 1.0);
    }

    // Sort keyframes by time
    let mut sorted: Vec<_> = prop.keyframes.iter().collect();
    sorted.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // If t is before or at the first keyframe, use the first keyframe value
    if t <= sorted[0].time {
        return crate::schema::parse_vec2(&sorted[0].value)
            .map(|v| (v[0], v[1]))
            .unwrap_or((1.0, 1.0));
    }

    // If t is after or at the last keyframe, use the last keyframe value
    let last = sorted.last().unwrap();
    if t >= last.time {
        return crate::schema::parse_vec2(&last.value)
            .map(|v| (v[0], v[1]))
            .unwrap_or((1.0, 1.0));
    }

    // Find the surrounding keyframes and interpolate
    for i in 0..sorted.len() - 1 {
        let kf_prev = sorted[i];
        let kf_next = sorted[i + 1];

        if t >= kf_prev.time && t <= kf_next.time {
            let v_prev = crate::schema::parse_vec2(&kf_prev.value).unwrap_or([1.0, 1.0]);
            let v_next = crate::schema::parse_vec2(&kf_next.value).unwrap_or([1.0, 1.0]);

            let span = kf_next.time - kf_prev.time;
            let local_t = if span > 0.0 {
                (t - kf_prev.time) / span
            } else {
                0.0
            };

            return (
                v_prev[0] + (v_next[0] - v_prev[0]) * local_t,
                v_prev[1] + (v_next[1] - v_prev[1]) * local_t,
            );
        }
    }

    // Fallback
    (1.0, 1.0)
}

/// Collect a text layer's data.
fn collect_text(
    text: &AmText,
    _fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    config: &AmSceneConfig,
    z: f32,
) -> Option<PendingLayer> {
    let has_parent = text.parent != 0;
    let (tx, ty) = get_initial_location(&text.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&text.transform.rotation);
    let (sx, sy) = get_initial_scale(&text.transform.scale);

    // Font name parsing
    let font_name = text
        .font
        .strip_prefix("imported?name=")
        .unwrap_or(&text.font)
        .to_string();

    const TEXT_SIZE_MULTIPLIER: f32 = 3.0;
    let font_size = if text.size > 0.0 {
        text.size * TEXT_SIZE_MULTIPLIER
    } else {
        48.0
    };

    // Calculate wrap offset for text positioning
    // AM text position is based on the CENTER of the wrapWidth box
    // We need to offset to get the LEFT edge for left-aligned text
    let wrap_width = text.wrap_width;
    let wrap_offset_x = if has_parent {
        0.0 // Child text uses relative positioning, no wrap offset
    } else {
        match text.align.as_str() {
            "left" => -wrap_width / 2.0, // Move left by half of wrapWidth
            "right" => wrap_width / 2.0, // Move right by half of wrapWidth
            _ => 0.0,                    // Center - no offset needed
        }
    };

    // Calculate Y offset based on font metrics
    const REFERENCE_WIN_ASCENT: f32 = 1.1285;
    let font_y_offset = if let Some(metrics) = font_metrics.get(&font_name) {
        let ascent_diff = REFERENCE_WIN_ASCENT - metrics.win_ascent;
        ascent_diff * font_size * 0.43
    } else {
        0.0
    };

    let y_offset_to_apply = if has_parent { 0.0 } else { font_y_offset };

    let transform = Transform {
        translation: Vec3::new(tx + wrap_offset_x, ty - y_offset_to_apply, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    // Create a modified location with wrap_offset applied (for animations)
    let mut modified_location = text.transform.location.clone();
    if let Some(ref mut val) = modified_location.value {
        val[0] += wrap_offset_x;
    }
    // Also modify keyframes if present
    for kf in &mut modified_location.keyframes {
        if let Ok(mut parsed) = crate::schema::parse_vec3(&kf.value) {
            parsed[0] += wrap_offset_x;
            kf.value = format!("{},{},{}", parsed[0], parsed[1], parsed[2]);
        }
    }

    Some(PendingLayer {
        id: text.id,
        label: text.label.clone(),
        parent: text.parent,
        start_time: text.start_time,
        end_time: text.end_time,
        transform,
        animated: AmAnimated {
            layer_id: text.id,
            start_time: text.start_time,
            end_time: text.end_time,
            time_offset: config.time_offset,
            location: modified_location, // Use modified location with wrap offset
            pivot: text.transform.pivot.clone(),
            rotation: text.transform.rotation.clone(),
            scale: text.transform.scale.clone(),
            opacity: text.transform.opacity.clone(),
            canvas_width: config.canvas_width,
            canvas_height: config.canvas_height,
            has_parent,
            effect_pos_x: AmAnimatedFloat::default(),
            effect_pos_y: AmAnimatedFloat::default(),
            font_y_offset,
            size: AmAnimatedVec2::default(),
            anchor_offset: Vec2::ZERO,
            wipe_start: AmAnimatedFloat::default(),
            wipe_end: AmAnimatedFloat {
                value: Some(1.0),
                keyframes: vec![],
            },
            wipe_angle: AmAnimatedFloat::default(),
            wipe_feather: AmAnimatedFloat::default(),
            stretch_angle: AmAnimatedFloat::default(),
            stretch_amount: AmAnimatedFloat::default(),
            stretch_offset: AmAnimatedFloat::default(),
            stretch_smooth: AmAnimatedFloat::default(),
            blur_strength: AmAnimatedFloat::default(),
            speed_multiplier: config.speed_multiplier,
            embed_offset: Vec2::ZERO,
            inv_fit_scale: 1.0,
            stroke_width: AmAnimatedFloat::default(),
            base_alpha: get_base_alpha(&text.fill_color),
            palette_alpha: AmAnimatedFloat::default(),
        },
        spec: AmLayerSpec::Text {
            content: text.content.clone(),
            font_name,
            font_size,
            align: text.align.clone(),
            fill_color: text.fill_color.clone(),
        },
        z_index: z,
        children: Vec::new(),
        blending_mode: AmBlendingMode::Normal,
        mask_info: None,
        palette_params: None,
        embed_scene_size: None,
        containing_embed_id: 0,
        from_deeply_nested_scene: config.nesting_depth > 1,
    })
}

/// Collect an image layer's data.
fn collect_image(
    image: &crate::schema::AmImage,
    config: &AmSceneConfig,
    z: f32,
) -> Option<PendingLayer> {
    let has_parent = image.parent != 0;
    let (tx, ty) = get_initial_location(&image.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&image.transform.rotation);
    let (sx, sy) = get_initial_scale(&image.transform.scale);
    let (pivot_x, pivot_y) = get_initial_pivot(&image.transform.pivot);
    let wipe_effect = extract_wipe_effect(&image.effects);
    let stretch_segment = extract_stretch_segment_effect(&image.effects);
    let gaussian_blur = extract_gaussian_blur_effect(&image.effects);
    let palette_map = extract_palette_map_effect(&image.effects);

    // Get size from properties
    let (width, height) = get_shape_size(&image.properties, &image.fill_type);

    // Calculate anchor and position compensation
    let (anchor, comp_x, comp_y) = pivot_to_anchor_and_offset(pivot_x, pivot_y, width, height);
    let (final_tx, final_ty) = (tx + comp_x, ty + comp_y);

    let transform = Transform {
        translation: Vec3::new(final_tx, final_ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    Some(PendingLayer {
        id: image.id,
        label: image.label.clone(),
        parent: image.parent,
        start_time: image.start_time,
        end_time: image.end_time,
        transform,
        animated: AmAnimated {
            layer_id: image.id,
            start_time: image.start_time,
            end_time: image.end_time,
            time_offset: config.time_offset,
            location: image.transform.location.clone(),
            pivot: image.transform.pivot.clone(),
            rotation: image.transform.rotation.clone(),
            scale: image.transform.scale.clone(),
            opacity: image.transform.opacity.clone(),
            canvas_width: config.canvas_width,
            canvas_height: config.canvas_height,
            has_parent,
            effect_pos_x: AmAnimatedFloat::default(),
            effect_pos_y: AmAnimatedFloat::default(),
            font_y_offset: 0.0,
            size: AmAnimatedVec2::default(),
            anchor_offset: Vec2::new(comp_x, comp_y),
            wipe_start: wipe_effect.start,
            wipe_end: wipe_effect.end,
            wipe_angle: wipe_effect.angle,
            wipe_feather: wipe_effect.feather,
            stretch_angle: stretch_segment.angle,
            stretch_amount: stretch_segment.stretch,
            stretch_offset: stretch_segment.offset,
            stretch_smooth: stretch_segment.smooth,
            blur_strength: gaussian_blur.strength,
            speed_multiplier: config.speed_multiplier,
            embed_offset: Vec2::ZERO,
            inv_fit_scale: 1.0,
            stroke_width: AmAnimatedFloat::default(),
            base_alpha: 1.0, // Image layers are fully opaque
            palette_alpha: palette_map.alpha.clone(),
        },
        spec: AmLayerSpec::Image {
            image_uri: image.fill_image.clone(),
            width,
            height,
            anchor,
        },
        z_index: z,
        children: Vec::new(),
        blending_mode: AmBlendingMode::Normal,
        mask_info: None,
        palette_params: if palette_map.has_effect() {
            Some(AmPaletteMapParams::from_params(&palette_map))
        } else {
            None
        },
        embed_scene_size: None,
        containing_embed_id: 0,
        from_deeply_nested_scene: config.nesting_depth > 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_am_to_bevy_coords() {
        let config = AmSceneConfig {
            canvas_width: 1280.0,
            canvas_height: 960.0,
            flip_y: true,
            z_spacing: 0.001,
            time_offset: 0,
            speed_multiplier: 1.0,
            nesting_depth: 0,
        };

        // Center of AM canvas should be at Bevy origin
        let (x, y) = am_to_bevy_coords(640.0, 480.0, &config);
        assert!((x - 0.0).abs() < 0.01, "Center X should be 0, got {}", x);
        assert!((y - 0.0).abs() < 0.01, "Center Y should be 0, got {}", y);

        // Top-left of AM canvas
        let (x, y) = am_to_bevy_coords(0.0, 0.0, &config);
        assert!(
            (x - (-640.0)).abs() < 0.01,
            "Top-left X should be -640, got {}",
            x
        );
        assert!(
            (y - 480.0).abs() < 0.01,
            "Top-left Y should be 480, got {}",
            y
        );

        // Bottom-right of AM canvas
        let (x, y) = am_to_bevy_coords(1280.0, 960.0, &config);
        assert!(
            (x - 640.0).abs() < 0.01,
            "Bottom-right X should be 640, got {}",
            x
        );
        assert!(
            (y - (-480.0)).abs() < 0.01,
            "Bottom-right Y should be -480, got {}",
            y
        );
    }

    #[test]
    fn test_get_shape_size() {
        let props = vec![crate::schema::AmProperty {
            name: "size".to_string(),
            prop_type: "vec2".to_string(),
            value: "200.0,300.0".to_string(),
            keyframes: vec![],
        }];

        // Size is always doubled (half-extent to full size)
        let (w, h) = get_shape_size(&props, "media");
        assert!((w - 400.0).abs() < 0.01);
        assert!((h - 600.0).abs() < 0.01);

        let (w, h) = get_shape_size(&props, "color");
        assert!((w - 400.0).abs() < 0.01);
        assert!((h - 600.0).abs() < 0.01);
    }
}

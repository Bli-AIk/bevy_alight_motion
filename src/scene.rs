//! Scene building and coordinate transformation.

use bevy::asset::Assets;
use bevy::prelude::*;
use bevy::sprite::{Anchor, Text2d};
use bevy::text::{TextColor, TextFont, TextLayout};
use bevy_smud::prelude::*;
use std::collections::HashMap;

use crate::animation::AmAnimated;
use crate::loader::{AmProject, FontMetrics};
use crate::schema::{
    AmAnimatedFloat, AmAnimatedVec2, AmAnimatedVec3, AmEffect, AmLayer, AmScene, AmShape, AmText,
};
use crate::sdf::AmSdfShaders;

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
        width: f32,
        height: f32,
        pivot_x: f32,
        pivot_y: f32,
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
    /// Z-spacing between layers.
    pub z_spacing: f32,
    /// Time offset from parent scene (for embedded scenes).
    pub time_offset: i32,
}

impl Default for AmSceneConfig {
    fn default() -> Self {
        Self {
            canvas_width: 1280.0,
            canvas_height: 960.0,
            flip_y: true,
            z_spacing: 0.001,
            time_offset: 0,
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
    println!(
        "spawn_scene: layer_count={}, z_spacing={}",
        layer_count, config.z_spacing
    );

    // First pass: create all entities and collect parent relationships
    for (idx, layer) in scene.layers.iter().enumerate() {
        // Direct z order: last layer in XML = highest z (on top)
        let z = idx as f32 * config.z_spacing;

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
                println!(
                    "Skipping audio layer '{}' (id={}) - audio not implemented",
                    audio.label, audio.id
                );
            }
            AmLayer::Camera(camera) => {
                // TODO: Camera layer is not yet implemented, skip for now
                println!(
                    "Skipping camera layer '{}' (id={}) - camera not implemented",
                    camera.label, camera.id
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
                println!(
                    "Skipping video layer '{}' (id={}) - video not implemented",
                    video.label, video.id
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
    let (pivot_x, pivot_y) = get_initial_pivot(&shape.transform.pivot);

    // Get size from properties
    let (width, height) = get_shape_size(&shape.properties, &shape.fill_type);

    // Convert pivot to Bevy anchor
    let anchor = pivot_to_anchor(pivot_x, pivot_y, width, height);

    println!(
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

    let transform = Transform {
        translation: Vec3::new(tx, ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    // Create entity name for inspector identification
    let entity_name = format!("Shape[{}]: {}", shape.id, shape.label);

    // Check if this is a stroked shape that needs SDF rendering
    let needs_sdf = shape.fill_type == "color"
        && shape.stroke.as_ref().map_or(false, |s| {
            s.size.as_ref().map_or(false, |sz| sz.value > 0.0)
        });

    // Create the layer spec for lazy spawning
    let layer_spec = if needs_sdf {
        let stroke = shape.stroke.as_ref().unwrap();
        let stroke_width = stroke.size.as_ref().map(|s| s.value).unwrap_or(0.0);
        let stroke_color_value = stroke
            .color
            .as_ref()
            .map(|c| c.value.clone())
            .unwrap_or_default();

        AmLayerSpec::SdfShape {
            fill_color: shape.fill_color.clone(),
            stroke_color_value,
            stroke_width,
            width,
            height,
            pivot_x,
            pivot_y,
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
    commands
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
            },
            layer_spec,
            transform,
            GlobalTransform::default(),
            Visibility::Hidden, // Start hidden, lifecycle system will show when active
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id()
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

    println!(
        "Registering nullobj '{}' (id={}, parent={}): pos=({:.1},{:.1}), scale=({:.2},{:.2})",
        null.label, null.id, null.parent, tx, ty, sx, sy
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
    let (tx, ty) = get_initial_location(&embed.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&embed.transform.rotation);
    let (sx, sy) = get_initial_scale(&embed.transform.scale);

    println!(
        "Registering embedScene '{}' (id={}, parent={}): pos=({:.1},{:.1}), start_time={}, time_offset={}",
        embed.label, embed.id, embed.parent, tx, ty, embed.start_time, config.time_offset
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
            },
            AmLayerSpec::EmbedScene,
            transform,
            GlobalTransform::default(),
            Visibility::Hidden, // Start hidden, lifecycle system will show when active
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();

    // Recursively spawn nested scene with accumulated time offset
    // The nested scene's layers use times relative to the embed's start_time
    let nested_config = AmSceneConfig {
        canvas_width: embed.scene.width as f32,
        canvas_height: embed.scene.height as f32,
        time_offset: config.time_offset + embed.start_time,
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
    let (pivot_x, pivot_y) = get_initial_pivot(&image.transform.pivot);

    // Get size from properties
    let (width, height) = get_shape_size(&image.properties, &image.fill_type);

    // Convert pivot to Bevy anchor
    let anchor = pivot_to_anchor(pivot_x, pivot_y, width, height);

    println!(
        "Registering image '{}' (id={}, parent={}): pos=({:.1},{:.1}), scale=({:.2},{:.2}), size=({:.0},{:.0}), pivot=({:.1},{:.1}), fill={}",
        image.label,
        image.id,
        image.parent,
        tx,
        ty,
        sx,
        sy,
        width,
        height,
        pivot_x,
        pivot_y,
        image.fill_image
    );

    let transform = Transform {
        translation: Vec3::new(tx, ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };

    // Create entity name for inspector identification
    let entity_name = format!("Image[{}]: {}", image.id, image.label);

    commands
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
        .id()
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

        println!(
            "  Font metrics for '{}': win_ascent={:.4}, win_descent={:.4}",
            font_name, metrics.win_ascent, metrics.win_descent
        );
        println!(
            "  Y calculation: base_y={:.2}, ascent_diff={:.4}, offset={:.2}, final_y={:.2}",
            base_y, ascent_diff, offset, final_y
        );
        offset
    } else {
        println!(
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

    println!(
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
        println!("  -> Using embedded font: {}", font_name);
        TextFont {
            font: font_handle.clone(),
            font_size,
            ..default()
        }
    } else {
        println!("  -> Font not found '{}', using default", font_name);
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
        Anchor(Vec2::new(-0.5, 0.0)),
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
/// Note: AM stores size as half-extents (like radius), so we double them to get full dimensions.
fn get_shape_size(properties: &[crate::schema::AmProperty], _fill_type: &str) -> (f32, f32) {
    for prop in properties {
        if prop.name == "size"
            && prop.prop_type == "vec2"
            && let Ok(size) = crate::schema::parse_vec2(&prop.value)
        {
            // AM size is half-extent for all shape types, double it for full size
            return (size[0] * 2.0, size[1] * 2.0);
        }
    }
    (100.0, 100.0)
}

/// Get initial pivot from animated property.
/// Returns (pivot_x, pivot_y) in pixels. Default is (0, 0) which means center.
fn get_initial_pivot(prop: &AmAnimatedVec2) -> (f32, f32) {
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
            .unwrap_or([0.0, 0.0])
            .into()
    } else {
        (0.0, 0.0)
    }
}

/// Convert AM pivot (in pixels, relative to center) to Bevy Anchor.
/// AM pivot: (0, 0) = center, positive X = right, positive Y = down
/// Bevy anchor: (0, 0) = center, range typically -0.5 to 0.5 for edges
///
/// The pivot in AM represents the offset from the center where the anchor point is.
/// For example, pivot (-100, 0) means the anchor is 100 pixels to the left of center.
/// In Bevy, we need to convert this to a normalized anchor value.
///
/// Bevy Anchor semantics:
/// - Anchor(0, 0) = center
/// - Anchor(-0.5, -0.5) = bottom-left corner
/// - Anchor(0.5, 0.5) = top-right corner
/// So Anchor.x = 0.5 means the anchor is at the right edge (half the width from center).
fn pivot_to_anchor(pivot_x: f32, pivot_y: f32, width: f32, height: f32) -> bevy::sprite::Anchor {
    if pivot_x == 0.0 && pivot_y == 0.0 {
        return bevy::sprite::Anchor::CENTER;
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

    bevy::sprite::Anchor(Vec2::new(anchor_x, anchor_y))
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
pub fn collect_pending_layers(
    scene: &AmScene,
    fonts: &HashMap<String, Handle<Font>>,
    font_metrics: &HashMap<String, FontMetrics>,
    config: &AmSceneConfig,
) -> Vec<PendingLayer> {
    let mut pending_layers = Vec::new();
    
    let layer_count = scene.layers.len();
    println!(
        "collect_pending_layers: layer_count={}, z_spacing={}",
        layer_count, config.z_spacing
    );
    
    for (idx, layer) in scene.layers.iter().enumerate() {
        let z = idx as f32 * config.z_spacing;
        collect_layer(&mut pending_layers, layer, fonts, font_metrics, config, z);
    }
    
    println!("Collected {} pending layers", pending_layers.len());
    pending_layers
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
                println!(
                    "  Collected shape '{}' (id={}, time={}..{}ms)",
                    shape.label, shape.id, shape.start_time, shape.end_time
                );
                pending.push(pl);
            }
        }
        AmLayer::Nullobj(null) => {
            if let Some(pl) = collect_null(null, config, z) {
                println!(
                    "  Collected null '{}' (id={}, time={}..{}ms)",
                    null.label, null.id, null.start_time, null.end_time
                );
                pending.push(pl);
            }
        }
        AmLayer::EmbedScene(embed) => {
            let pl = collect_embed_scene(embed, fonts, font_metrics, config, z);
            println!(
                "  Collected embed '{}' (id={}, time={}..{}ms, children={})",
                embed.label, embed.id, embed.start_time, embed.end_time, pl.children.len()
            );
            pending.push(pl);
        }
        AmLayer::Text(text) => {
            if let Some(pl) = collect_text(text, fonts, font_metrics, config, z) {
                println!(
                    "  Collected text '{}' (id={}, time={}..{}ms)",
                    text.label, text.id, text.start_time, text.end_time
                );
                pending.push(pl);
            }
        }
        AmLayer::Image(image) => {
            if let Some(pl) = collect_image(image, config, z) {
                println!(
                    "  Collected image '{}' (id={}, time={}..{}ms)",
                    image.label, image.id, image.start_time, image.end_time
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
    let (effect_pos_x, effect_pos_y) = extract_effect_animations(&shape.effects);
    let (pivot_x, pivot_y) = get_initial_pivot(&shape.transform.pivot);
    let (width, height) = get_shape_size(&shape.properties, &shape.fill_type);
    let anchor = pivot_to_anchor(pivot_x, pivot_y, width, height);
    
    let transform = Transform {
        translation: Vec3::new(tx, ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };
    
    let needs_sdf = shape.fill_type == "color"
        && shape.stroke.as_ref().map_or(false, |s| {
            s.size.as_ref().map_or(false, |sz| sz.value > 0.0)
        });
    
    let spec = if needs_sdf {
        let stroke = shape.stroke.as_ref().unwrap();
        let stroke_width = stroke.size.as_ref().map(|s| s.value).unwrap_or(0.0);
        let stroke_color_value = stroke
            .color
            .as_ref()
            .map(|c| c.value.clone())
            .unwrap_or_default();
        AmLayerSpec::SdfShape {
            fill_color: shape.fill_color.clone(),
            stroke_color_value,
            stroke_width,
            width,
            height,
            pivot_x,
            pivot_y,
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
        },
        spec,
        z_index: z,
        children: Vec::new(),
    })
}

/// Collect a null object's data.
fn collect_null(null: &crate::schema::AmNullObj, config: &AmSceneConfig, z: f32) -> Option<PendingLayer> {
    let has_parent = null.parent != 0;
    let (tx, ty) = get_initial_location(&null.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&null.transform.rotation);
    let (sx, sy) = get_initial_scale(&null.transform.scale);
    let (effect_pos_x, effect_pos_y) = extract_effect_animations(&null.effects);
    
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
        },
        spec: AmLayerSpec::Null,
        z_index: z,
        children: Vec::new(),
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
    let (tx, ty) = get_initial_location(&embed.transform.location, config, has_parent);
    let rotation = get_initial_rotation(&embed.transform.rotation);
    let (sx, sy) = get_initial_scale(&embed.transform.scale);
    
    let transform = Transform {
        translation: Vec3::new(tx, ty, z),
        rotation: Quat::from_rotation_z(rotation.to_radians()),
        scale: Vec3::new(sx, sy, 1.0),
    };
    
    // Collect children with nested config
    let nested_config = AmSceneConfig {
        canvas_width: embed.scene.width as f32,
        canvas_height: embed.scene.height as f32,
        time_offset: config.time_offset + embed.start_time,
        ..config.clone()
    };
    
    let children = collect_pending_layers(&embed.scene, fonts, font_metrics, &nested_config);
    
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
        },
        spec: AmLayerSpec::EmbedScene,
        z_index: z,
        children,
    }
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
    
    // Get size from properties
    let (width, height) = get_shape_size(&image.properties, &image.fill_type);
    let anchor = pivot_to_anchor(pivot_x, pivot_y, width, height);
    
    let transform = Transform {
        translation: Vec3::new(tx, ty, z),
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
        },
        spec: AmLayerSpec::Image {
            image_uri: image.fill_image.clone(),
            width,
            height,
            anchor,
        },
        z_index: z,
        children: Vec::new(),
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

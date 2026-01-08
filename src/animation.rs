//! Animation systems for interpolating keyframes.

use bevy::prelude::*;

use crate::scene::AmLayerMarker;
use crate::schema::{AmAnimatedFloat, AmAnimatedVec2, AmAnimatedVec3, AmKeyframe, Easing};

/// Component marking an entity as part of an AM animation.
#[derive(Component, Debug, Clone)]
pub struct AmAnimated {
    /// Unique layer ID from AM.
    pub layer_id: u64,
    /// Start time in milliseconds (relative to time_offset).
    pub start_time: i32,
    /// End time in milliseconds (relative to time_offset).
    pub end_time: i32,
    /// Time offset from parent scene (for embedded scenes).
    pub time_offset: i32,
    /// Location animation data.
    pub location: AmAnimatedVec3,
    /// Pivot/anchor point animation data.
    pub pivot: AmAnimatedVec2,
    /// Rotation animation data.
    pub rotation: AmAnimatedFloat,
    /// Scale animation data.
    pub scale: AmAnimatedVec2,
    /// Opacity animation data.
    pub opacity: AmAnimatedFloat,
    /// Canvas width for coordinate conversion.
    pub canvas_width: f32,
    /// Canvas height for coordinate conversion.
    pub canvas_height: f32,
    /// Whether this layer has a parent (uses local coordinates).
    pub has_parent: bool,
    /// Effect position X offset (from transform2 effect).
    pub effect_pos_x: AmAnimatedFloat,
    /// Effect position Y offset (from transform2 effect).
    pub effect_pos_y: AmAnimatedFloat,
    /// Font Y offset for text layers (to compensate for different font metrics).
    pub font_y_offset: f32,
}

/// Resource to control animation playback.
#[derive(Resource, Debug, Clone)]
pub struct AmPlayback {
    /// Current time in milliseconds.
    pub current_time_ms: f32,
    /// Total duration in milliseconds.
    pub total_time_ms: f32,
    /// Is playing.
    pub playing: bool,
    /// Playback speed (1.0 = normal).
    pub speed: f32,
    /// Loop playback.
    pub looping: bool,
    /// Force stopped - when true, animation systems won't update transforms.
    /// Use this for debugging/inspector editing. Normal pause still updates animations.
    pub force_stopped: bool,
}

impl Default for AmPlayback {
    fn default() -> Self {
        Self {
            current_time_ms: 0.0,
            total_time_ms: 2000.0,
            playing: true,
            speed: 1.0,
            looping: true,
            force_stopped: false,
        }
    }
}

impl AmPlayback {
    /// Create with specific duration.
    pub fn with_duration(total_time_ms: f32) -> Self {
        Self {
            total_time_ms,
            ..Default::default()
        }
    }

    /// Reset to beginning.
    pub fn reset(&mut self) {
        self.current_time_ms = 0.0;
    }

    /// Toggle play/pause.
    pub fn toggle(&mut self) {
        self.playing = !self.playing;
    }

    /// Toggle force stop - freezes all animation updates for inspector editing.
    pub fn toggle_force_stop(&mut self) {
        self.force_stopped = !self.force_stopped;
    }
}

/// System to advance playback time.
pub fn advance_playback(time: Res<Time>, mut playback: ResMut<AmPlayback>) {
    if !playback.playing {
        return;
    }

    playback.current_time_ms += time.delta_secs() * 1000.0 * playback.speed;

    if playback.current_time_ms >= playback.total_time_ms {
        if playback.looping {
            playback.current_time_ms %= playback.total_time_ms;
        } else {
            playback.current_time_ms = playback.total_time_ms;
            playback.playing = false;
        }
    }
}

/// System to animate transforms based on keyframes.
/// Only skips updates when force_stopped is true (for inspector editing).
/// Normal pause still updates animations based on current time.
pub fn animate_transform(
    playback: Res<AmPlayback>,
    mut query: Query<(&AmAnimated, &mut Transform, &AmLayerMarker)>,
) {
    // Skip animation only when force stopped (for inspector editing)
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (animated, mut transform, _marker) in query.iter_mut() {
        // Calculate local time (accounting for time offset from parent scene)
        let local_time = global_time - animated.time_offset as f32;

        // Check if layer is active at current local time
        if local_time < animated.start_time as f32 || local_time > animated.end_time as f32 {
            continue;
        }

        // Calculate normalized time within layer duration
        let layer_duration = (animated.end_time - animated.start_time) as f32;
        let layer_time = (local_time - animated.start_time as f32) / layer_duration;

        // Interpolate location and convert from AM to Bevy coordinates
        if let Some(loc) = interpolate_vec3(&animated.location, layer_time) {
            let (mut bx, mut by) = if animated.has_parent {
                // For layers with parents, use local coordinates
                // Only flip Y axis (AM Y-down -> Bevy Y-up)
                (loc[0], -loc[1])
            } else {
                // For root layers, convert from canvas coordinates
                // AM: Origin at top-left, Y increases downward
                // Bevy: Origin at center, Y increases upward
                (
                    loc[0] - animated.canvas_width / 2.0,
                    animated.canvas_height / 2.0 - loc[1],
                )
            };

            // Apply effect position offsets (transform2 effect)
            if let Some(effect_x) = interpolate_float(&animated.effect_pos_x, layer_time) {
                bx += effect_x;
            }
            if let Some(effect_y) = interpolate_float(&animated.effect_pos_y, layer_time) {
                by -= effect_y; // Y is inverted
            }

            // Apply font Y offset for text layers (to compensate for different font metrics)
            // Only apply to root text layers; child text inherits offset from parent
            if !animated.has_parent {
                by -= animated.font_y_offset;
            }

            transform.translation = Vec3::new(bx, by, transform.translation.z);
        }

        // Interpolate rotation (negate for Bevy's coordinate system)
        if let Some(rot) = interpolate_float(&animated.rotation, layer_time) {
            transform.rotation = Quat::from_rotation_z((-rot).to_radians());
        }

        // Interpolate scale
        if let Some(scale) = interpolate_vec2(&animated.scale, layer_time) {
            transform.scale = Vec3::new(scale[0], scale[1], 1.0);
        }
    }
}

/// System to animate sprite opacity.
/// Only skips updates when force_stopped is true (for inspector editing).
pub fn animate_opacity(playback: Res<AmPlayback>, mut query: Query<(&AmAnimated, &mut Sprite)>) {
    // Skip animation only when force stopped (for inspector editing)
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (animated, mut sprite) in query.iter_mut() {
        // Calculate local time (accounting for time offset from parent scene)
        let local_time = global_time - animated.time_offset as f32;

        // Check if layer is active
        if local_time < animated.start_time as f32 || local_time > animated.end_time as f32 {
            sprite.color.set_alpha(0.0);
            continue;
        }

        let layer_duration = (animated.end_time - animated.start_time) as f32;
        let layer_time = (local_time - animated.start_time as f32) / layer_duration;

        // Get opacity from animation data, default to 1.0 if not specified
        let opacity = interpolate_float(&animated.opacity, layer_time).unwrap_or(1.0);
        sprite.color.set_alpha(opacity.clamp(0.0, 1.0));
    }
}

/// System to animate text opacity (handles Text2d entities).
/// Uses Visibility component for proper show/hide behavior and TextColor alpha for opacity animation.
/// Only skips updates when force_stopped is true (for inspector editing).
pub fn animate_text_opacity(
    playback: Res<AmPlayback>,
    mut query: Query<
        (
            &AmAnimated,
            &mut bevy::text::TextColor,
            &mut Visibility,
            &AmLayerMarker,
        ),
        With<Text2d>,
    >,
) {
    // Skip animation only when force stopped (for inspector editing)
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;
    let text_count = query.iter().count();

    // Debug: log count of text entities being processed (only occasionally to avoid spam)
    static mut FRAME_COUNT: u32 = 0;
    unsafe {
        FRAME_COUNT += 1;
        if FRAME_COUNT % 300 == 1 {
            println!(
                "[TEXT] Processing {} text entities at time={:.0}",
                text_count, global_time
            );
        }
    }

    for (animated, mut text_color, mut visibility, marker) in query.iter_mut() {
        // Calculate local time (accounting for time offset from parent scene)
        let local_time = global_time - animated.time_offset as f32;

        // Check if layer is active
        if local_time < animated.start_time as f32 || local_time > animated.end_time as f32 {
            // Hide text when outside its time range
            if *visibility != Visibility::Hidden {
                println!(
                    "[TEXT] Hiding '{}' (id={}): time={:.0}, range=[{}, {}]",
                    marker.label, marker.id, local_time, animated.start_time, animated.end_time
                );
            }
            *visibility = Visibility::Hidden;
            text_color.0.set_alpha(0.0);
            continue;
        }

        // Show text when within its time range
        if *visibility == Visibility::Hidden {
            println!(
                "[TEXT] Showing '{}' (id={}): time={:.0}, range=[{}, {}]",
                marker.label, marker.id, local_time, animated.start_time, animated.end_time
            );
        }
        *visibility = Visibility::Inherited;

        let layer_duration = (animated.end_time - animated.start_time) as f32;
        let layer_time = (local_time - animated.start_time as f32) / layer_duration;

        // Get opacity from keyframes, or default to 1.0 if no opacity animation
        let opacity = interpolate_float(&animated.opacity, layer_time).unwrap_or(1.0);
        text_color.0.set_alpha(opacity.clamp(0.0, 1.0));
    }
}

/// System to animate SDF shape opacity (handles SmudShape entities).
/// Uses Visibility component for proper show/hide behavior and SmudShape color alpha for opacity animation.
/// Only skips updates when force_stopped is true (for inspector editing).
pub fn animate_sdf_opacity(
    playback: Res<AmPlayback>,
    mut query: Query<(
        &AmAnimated,
        &mut bevy_smud::SmudShape,
        &mut Visibility,
        &AmLayerMarker,
    )>,
) {
    // Skip animation only when force stopped (for inspector editing)
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    for (animated, mut smud_shape, mut visibility, marker) in query.iter_mut() {
        // Calculate local time (accounting for time offset from parent scene)
        let local_time = global_time - animated.time_offset as f32;

        // Check if layer is active
        if local_time < animated.start_time as f32 || local_time > animated.end_time as f32 {
            // Hide shape when outside its time range
            *visibility = Visibility::Hidden;
            smud_shape.color.set_alpha(0.0);
            continue;
        }

        // Show shape when within its time range
        *visibility = Visibility::Inherited;

        let layer_duration = (animated.end_time - animated.start_time) as f32;
        let layer_time = (local_time - animated.start_time as f32) / layer_duration;

        // Get opacity from keyframes, or default to 1.0 if no opacity animation
        let opacity = interpolate_float(&animated.opacity, layer_time).unwrap_or(1.0);
        smud_shape.color.set_alpha(opacity.clamp(0.0, 1.0));
    }
}

/// Interpolate a Vec3 property at normalized time t.
pub fn interpolate_vec3(prop: &AmAnimatedVec3, t: f32) -> Option<[f32; 3]> {
    if prop.keyframes.is_empty() {
        return prop.value;
    }

    let (kf_prev, kf_next, local_t) = find_keyframes(&prop.keyframes, t);

    let v_prev = parse_keyframe_vec3(&kf_prev.value).unwrap_or([0.0, 0.0, 0.0]);
    let v_next = parse_keyframe_vec3(&kf_next.value).unwrap_or(v_prev);

    // Easing is defined on the "target" keyframe (describes how to arrive at it)
    let easing = kf_next
        .easing
        .as_ref()
        .map(|e| Easing::parse(e))
        .unwrap_or_default();
    let eased_t = easing.evaluate(local_t);

    Some([
        lerp(v_prev[0], v_next[0], eased_t),
        lerp(v_prev[1], v_next[1], eased_t),
        lerp(v_prev[2], v_next[2], eased_t),
    ])
}

/// Interpolate a Vec2 property at normalized time t.
pub fn interpolate_vec2(prop: &AmAnimatedVec2, t: f32) -> Option<[f32; 2]> {
    if prop.keyframes.is_empty() {
        return prop.value;
    }

    let (kf_prev, kf_next, local_t) = find_keyframes(&prop.keyframes, t);

    let v_prev = parse_keyframe_vec2(&kf_prev.value).unwrap_or([1.0, 1.0]);
    let v_next = parse_keyframe_vec2(&kf_next.value).unwrap_or(v_prev);

    // Easing is defined on the "target" keyframe (describes how to arrive at it)
    let easing = kf_next
        .easing
        .as_ref()
        .map(|e| Easing::parse(e))
        .unwrap_or_default();
    let eased_t = easing.evaluate(local_t);

    Some([
        lerp(v_prev[0], v_next[0], eased_t),
        lerp(v_prev[1], v_next[1], eased_t),
    ])
}

/// Interpolate a float property at normalized time t.
pub fn interpolate_float(prop: &AmAnimatedFloat, t: f32) -> Option<f32> {
    if prop.keyframes.is_empty() {
        return prop.value;
    }

    let (kf_prev, kf_next, local_t) = find_keyframes(&prop.keyframes, t);

    let v_prev: f32 = kf_prev.value.parse().unwrap_or(0.0);
    let v_next: f32 = kf_next.value.parse().unwrap_or(v_prev);

    // Easing is defined on the "target" keyframe (describes how to arrive at it)
    let easing = kf_next
        .easing
        .as_ref()
        .map(|e| Easing::parse(e))
        .unwrap_or_default();
    let eased_t = easing.evaluate(local_t);

    Some(lerp(v_prev, v_next, eased_t))
}

/// Find the surrounding keyframes for a given time.
fn find_keyframes(keyframes: &[AmKeyframe], t: f32) -> (&AmKeyframe, &AmKeyframe, f32) {
    // Sort keyframes by time (in case they're not sorted)
    let mut sorted: Vec<_> = keyframes.iter().collect();
    sorted.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Handle edge cases
    if sorted.len() == 1 {
        return (sorted[0], sorted[0], 0.0);
    }

    // Find surrounding keyframes
    for i in 0..sorted.len() - 1 {
        let kf_prev = sorted[i];
        let kf_next = sorted[i + 1];

        if t >= kf_prev.time && t <= kf_next.time {
            let span = kf_next.time - kf_prev.time;
            let local_t = if span > 0.0 {
                (t - kf_prev.time) / span
            } else {
                0.0
            };
            return (kf_prev, kf_next, local_t);
        }
    }

    // Before first keyframe
    if t < sorted[0].time {
        return (sorted[0], sorted[0], 0.0);
    }

    // After last keyframe
    let last = sorted.last().unwrap();
    (last, last, 0.0)
}

/// Linear interpolation.
#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Parse Vec3 from keyframe value string.
fn parse_keyframe_vec3(s: &str) -> Option<[f32; 3]> {
    crate::schema::parse_vec3(s).ok()
}

/// Parse Vec2 from keyframe value string.
fn parse_keyframe_vec2(s: &str) -> Option<[f32; 2]> {
    crate::schema::parse_vec2(s).ok()
}

// ============================================================================
// Layer Lifecycle Management System
// ============================================================================

use crate::loader::AmProject;
use crate::plugin::AmWhitePixel;
use crate::scene::{AmLayerSpec, AmPendingLayers, AmVisualSpawned, PendingLayer};
use bevy::asset::Assets;
use bevy_smud::prelude::*;
use std::collections::HashMap;

/// System to manage layer lifecycle based on playback time.
/// - Creates entities when layers enter their time range
/// - Destroys entities when layers exit their time range
/// - Implements true lazy spawning where no entities exist until needed
pub fn manage_layer_lifecycle(
    mut commands: Commands,
    playback: Res<AmPlayback>,
    mut shaders: ResMut<Assets<Shader>>,
    white_pixel: Option<Res<AmWhitePixel>>,
    projects: Res<Assets<AmProject>>,
    mut project_query: Query<(Entity, &crate::scene::AmProjectRoot, &mut AmPendingLayers)>,
) {
    // Skip if force stopped
    if playback.force_stopped {
        return;
    }

    let global_time = playback.current_time_ms;

    // Debug logging
    static mut FRAME_COUNT: u32 = 0;
    unsafe {
        FRAME_COUNT += 1;
    }

    for (project_entity, root, mut pending) in project_query.iter_mut() {
        let Some(project) = projects.get(&root.handle) else {
            continue;
        };

        let images = &project.images;
        let fonts = &project.fonts;
        let white_pixel_handle = white_pixel.as_ref().map(|wp| wp.0.clone());

        // Process all pending layers (including nested ones)
        process_pending_layers(
            &mut commands,
            &mut shaders,
            &mut pending,
            &project.images,
            &project.fonts,
            white_pixel_handle.as_ref(),
            global_time,
            project_entity,
            0, // root time offset
        );

        // Log stats occasionally
        unsafe {
            if FRAME_COUNT % 300 == 1 {
                let spawned_count = pending.spawned_entities.len();
                let total_layers = count_total_layers(&pending.layers);
                println!(
                    "[Lifecycle] time={:.0}ms | spawned={}/{} entities",
                    global_time, spawned_count, total_layers
                );
            }
        }
    }
}

/// Count total layers including nested ones.
fn count_total_layers(layers: &[PendingLayer]) -> usize {
    layers.iter().map(|l| 1 + count_total_layers(&l.children)).sum()
}

/// Process pending layers recursively.
fn process_pending_layers(
    commands: &mut Commands,
    shaders: &mut Assets<Shader>,
    pending: &mut AmPendingLayers,
    images: &HashMap<String, Handle<Image>>,
    fonts: &HashMap<String, Handle<Font>>,
    white_pixel: Option<&Handle<Image>>,
    global_time: f32,
    parent_entity: Entity,
    time_offset: i32,
) {
    // We need to collect actions to avoid borrowing issues
    let mut to_spawn: Vec<usize> = Vec::new(); // indices of layers to spawn
    let mut to_despawn: Vec<u64> = Vec::new(); // layer_id

    for (idx, layer) in pending.layers.iter().enumerate() {
        // Calculate local time
        let local_time = global_time - (layer.animated.time_offset + time_offset) as f32;
        
        // Check if layer should be active
        let should_be_active = local_time >= layer.start_time as f32 
            && local_time <= layer.end_time as f32;

        let is_spawned = pending.spawned_entities.contains_key(&layer.id);

        if should_be_active && !is_spawned {
            to_spawn.push(idx);
        } else if !should_be_active && is_spawned {
            to_despawn.push(layer.id);
        }
    }

    // Despawn entities that are no longer active
    for layer_id in to_despawn {
        if let Some(entity) = pending.spawned_entities.remove(&layer_id) {
            // Find layer info for logging
            if let Some(layer) = pending.layers.iter().find(|l| l.id == layer_id) {
                println!("  [Lifecycle] Despawning '{}' (id={})", layer.label, layer_id);
            }
            // Despawn entity (Bevy will handle orphaned children)
            commands.entity(entity).despawn();
        }
    }

    // Sort layers to spawn by dependency (parents before children)
    // Build a set of layer IDs being spawned this frame
    let spawning_ids: std::collections::HashSet<u64> = to_spawn.iter()
        .map(|&idx| pending.layers[idx].id)
        .collect();
    
    // Sort: layers without parents or with already-spawned parents come first
    to_spawn.sort_by(|&a_idx, &b_idx| {
        let a = &pending.layers[a_idx];
        let b = &pending.layers[b_idx];
        
        // Check if parent is also being spawned this frame
        let a_needs_wait = a.parent != 0 && spawning_ids.contains(&a.parent);
        let b_needs_wait = b.parent != 0 && spawning_ids.contains(&b.parent);
        
        match (a_needs_wait, b_needs_wait) {
            (false, true) => std::cmp::Ordering::Less,    // a comes before b
            (true, false) => std::cmp::Ordering::Greater, // b comes before a
            _ => std::cmp::Ordering::Equal,               // same priority
        }
    });

    // Spawn new entities in dependency order
    for idx in to_spawn {
        let layer = &pending.layers[idx];
        
        // Determine parent for this entity
        let actual_parent = if layer.parent != 0 {
            pending.spawned_entities.get(&layer.parent).copied().unwrap_or(parent_entity)
        } else {
            parent_entity
        };
        
        let entity = spawn_layer_entity(
            commands,
            shaders,
            layer,
            images,
            fonts,
            white_pixel,
            actual_parent,
        );
        
        println!(
            "  [Lifecycle] Spawning '{}' (id={}, parent={}, time={}..{}ms)",
            layer.label, layer.id, layer.parent, layer.start_time, layer.end_time
        );
        
        pending.spawned_entities.insert(layer.id, entity);
    }
}

/// Spawn a complete entity from a PendingLayer.
fn spawn_layer_entity(
    commands: &mut Commands,
    shaders: &mut Assets<Shader>,
    layer: &PendingLayer,
    images: &HashMap<String, Handle<Image>>,
    fonts: &HashMap<String, Handle<Font>>,
    white_pixel: Option<&Handle<Image>>,
    parent_entity: Entity,
) -> Entity {
    let entity_name = format!("Layer[{}]: {}", layer.id, layer.label);
    
    // Create base entity with common components
    let entity = commands.spawn((
        Name::new(entity_name),
        AmLayerMarker {
            id: layer.id,
            label: layer.label.clone(),
        },
        layer.animated.clone(),
        layer.spec.clone(),
        layer.transform,
        GlobalTransform::default(),
        Visibility::Inherited,
        InheritedVisibility::default(),
        ViewVisibility::default(),
    )).id();

    // Add visual components based on spec
    add_visual_components(
        commands,
        shaders,
        entity,
        &layer.spec,
        images,
        fonts,
        white_pixel,
        &layer.label,
        layer.id,
    );

    // Add as child of parent
    commands.entity(parent_entity).add_child(entity);

    entity
}

/// Add visual components to an entity based on layer spec.
fn add_visual_components(
    commands: &mut Commands,
    shaders: &mut Assets<Shader>,
    entity: Entity,
    spec: &AmLayerSpec,
    images: &HashMap<String, Handle<Image>>,
    fonts: &HashMap<String, Handle<Font>>,
    white_pixel: Option<&Handle<Image>>,
    label: &str,
    id: u64,
) {
    match spec {
        AmLayerSpec::SpriteShape {
            image_uri,
            is_media,
            fill_color,
            width,
            height,
            anchor,
        } => {
            if *is_media && !image_uri.is_empty() {
                if let Some(handle) = images.get(image_uri) {
                    commands.entity(entity).insert((
                        Sprite {
                            image: handle.clone(),
                            color: Color::WHITE,
                            custom_size: Some(Vec2::new(*width, *height)),
                            ..default()
                        },
                        anchor.clone(),
                        AmVisualSpawned,
                    ));
                }
            } else if let Some(wp) = white_pixel {
                let color = extract_fill_color(fill_color);
                commands.entity(entity).insert((
                    Sprite {
                        image: wp.clone(),
                        color,
                        custom_size: Some(Vec2::new(*width, *height)),
                        ..default()
                    },
                    anchor.clone(),
                    AmVisualSpawned,
                ));
            }
        }
        AmLayerSpec::SdfShape {
            fill_color,
            stroke_color_value,
            stroke_width,
            width,
            height,
            pivot_x,
            pivot_y,
        } => {
            spawn_sdf_visual(
                commands,
                shaders,
                entity,
                fill_color,
                stroke_color_value,
                *stroke_width,
                *width,
                *height,
                *pivot_x,
                *pivot_y,
                &AmLayerMarker { id, label: label.to_string() },
            );
        }
        AmLayerSpec::Image {
            image_uri,
            width,
            height,
            anchor,
        } => {
            if let Some(handle) = images.get(image_uri) {
                commands.entity(entity).insert((
                    Sprite {
                        image: handle.clone(),
                        color: Color::WHITE,
                        custom_size: Some(Vec2::new(*width, *height)),
                        ..default()
                    },
                    anchor.clone(),
                    AmVisualSpawned,
                ));
            }
        }
        AmLayerSpec::Text {
            content,
            font_name,
            font_size,
            align,
            fill_color,
        } => {
            // For text, we need font handles
            if let Some(font_handle) = fonts.get(font_name) {
                use bevy::sprite::{Anchor, Text2d};
                use bevy::text::{Justify, TextColor, TextFont, TextLayout};
                
                let color = extract_fill_color(fill_color);
                let justify = match align.as_str() {
                    "center" => Justify::Center,
                    "right" => Justify::Right,
                    _ => Justify::Left,
                };
                
                commands.entity(entity).insert((
                    Text2d::new(content),
                    TextFont {
                        font: font_handle.clone(),
                        font_size: *font_size,
                        ..default()
                    },
                    TextColor(color),
                    TextLayout::new_with_justify(justify),
                    Anchor(Vec2::new(-0.5, 0.0)), // Left-center anchor
                    AmVisualSpawned,
                ));
            }
        }
        AmLayerSpec::Null | AmLayerSpec::EmbedScene => {
            // No visual components needed
            commands.entity(entity).insert(AmVisualSpawned);
        }
    }
}

/// Extract fill color from AmFillColor.
fn extract_fill_color(fill_color: &Option<crate::schema::AmFillColor>) -> Color {
    if let Some(fc) = fill_color {
        if !fc.value.is_empty() {
            if let Ok(c) = crate::schema::parse_color(&fc.value) {
                return Color::srgba(c[0], c[1], c[2], c[3]);
            }
        } else if !fc.keyframes.is_empty() {
            let mut sorted: Vec<_> = fc.keyframes.iter().collect();
            sorted.sort_by(|a, b| {
                a.time
                    .partial_cmp(&b.time)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            if let Ok(c) = crate::schema::parse_color(&sorted[0].value) {
                return Color::srgba(c[0], c[1], c[2], c[3]);
            }
        }
    }
    Color::WHITE
}

/// Spawn SDF visual components as children of the layer entity.
fn spawn_sdf_visual(
    commands: &mut Commands,
    shaders: &mut Assets<Shader>,
    parent_entity: Entity,
    fill_color: &Option<crate::schema::AmFillColor>,
    stroke_color_value: &str,
    stroke_width: f32,
    width: f32,
    height: f32,
    _pivot_x: f32,
    _pivot_y: f32,
    marker: &AmLayerMarker,
) {
    let fill = extract_fill_color(fill_color);
    let stroke = if !stroke_color_value.is_empty() {
        crate::schema::parse_color(stroke_color_value)
            .map(|c| Color::srgba(c[0], c[1], c[2], c[3]))
            .unwrap_or(Color::WHITE)
    } else {
        Color::WHITE
    };

    let base_half_width = width / 2.0;
    let base_half_height = height / 2.0;
    let base_stroke_width = stroke_width;

    let fill_half_width = (base_half_width - base_stroke_width / 2.0).max(0.0);
    let fill_half_height = (base_half_height - base_stroke_width / 2.0).max(0.0);
    let stroke_outer_half_width = base_half_width + base_stroke_width / 2.0;
    let stroke_outer_half_height = base_half_height + base_stroke_width / 2.0;

    let fill_sdf = crate::sdf::create_box_sdf(shaders, fill_half_width, fill_half_height);
    let stroke_sdf =
        crate::sdf::create_box_sdf(shaders, stroke_outer_half_width, stroke_outer_half_height);

    let fill_frame_size = fill_half_width.max(fill_half_height) + 10.0;
    let stroke_frame_size = stroke_outer_half_width.max(stroke_outer_half_height) + 10.0;

    // Spawn fill child
    let fill_entity = commands
        .spawn((
            Name::new(format!("SdfFill[{}]: {}", marker.id, marker.label)),
            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0001)),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            SmudShape {
                color: fill,
                sdf: fill_sdf,
                frame: Frame::Quad(fill_frame_size),
                fill: SIMPLE_FILL_HANDLE,
                ..default()
            },
        ))
        .id();

    // Spawn stroke child
    let stroke_entity = commands
        .spawn((
            Name::new(format!("SdfStroke[{}]: {}", marker.id, marker.label)),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            SmudShape {
                color: stroke,
                sdf: stroke_sdf,
                frame: Frame::Quad(stroke_frame_size),
                fill: SIMPLE_FILL_HANDLE,
                ..default()
            },
        ))
        .id();

    // Add as children and mark parent as spawned
    commands
        .entity(parent_entity)
        .add_child(fill_entity)
        .add_child(stroke_entity)
        .insert(AmVisualSpawned);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_keyframe(t: f32, v: &str, e: Option<&str>) -> AmKeyframe {
        AmKeyframe {
            time: t,
            value: v.to_string(),
            easing: e.map(String::from),
        }
    }

    #[test]
    fn test_interpolate_float_static() {
        let prop = AmAnimatedFloat {
            value: Some(0.5),
            keyframes: vec![],
        };
        assert_eq!(interpolate_float(&prop, 0.0), Some(0.5));
        assert_eq!(interpolate_float(&prop, 0.5), Some(0.5));
        assert_eq!(interpolate_float(&prop, 1.0), Some(0.5));
    }

    #[test]
    fn test_interpolate_float_linear() {
        let prop = AmAnimatedFloat {
            value: None,
            keyframes: vec![
                make_keyframe(0.0, "0.0", None),
                make_keyframe(1.0, "1.0", None),
            ],
        };

        let v = interpolate_float(&prop, 0.0).unwrap();
        assert!((v - 0.0).abs() < 0.001);

        let v = interpolate_float(&prop, 0.5).unwrap();
        assert!((v - 0.5).abs() < 0.001);

        let v = interpolate_float(&prop, 1.0).unwrap();
        assert!((v - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_interpolate_float_step() {
        // Easing is on the target keyframe (describes how to arrive at it)
        let prop = AmAnimatedFloat {
            value: None,
            keyframes: vec![
                make_keyframe(0.0, "1.0", None),
                make_keyframe(1.0, "0.0", Some("step 1.0 0.0")),
            ],
        };

        let v = interpolate_float(&prop, 0.0).unwrap();
        assert!((v - 1.0).abs() < 0.001, "At t=0.0, expected 1.0, got {}", v);

        let v = interpolate_float(&prop, 0.5).unwrap();
        assert!(
            (v - 1.0).abs() < 0.001,
            "At t=0.5, expected 1.0 (step), got {}",
            v
        );

        let v = interpolate_float(&prop, 0.99).unwrap();
        assert!(
            (v - 1.0).abs() < 0.001,
            "At t=0.99, expected 1.0 (step), got {}",
            v
        );
    }

    #[test]
    fn test_interpolate_vec3_linear() {
        let prop = AmAnimatedVec3 {
            value: None,
            keyframes: vec![
                make_keyframe(0.0, "0.0,0.0,0.0", None),
                make_keyframe(1.0, "100.0,200.0,0.0", None),
            ],
        };

        let v = interpolate_vec3(&prop, 0.5).unwrap();
        assert!((v[0] - 50.0).abs() < 0.1);
        assert!((v[1] - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_interpolate_boundary() {
        let prop = AmAnimatedFloat {
            value: None,
            keyframes: vec![
                make_keyframe(0.2, "0.0", None),
                make_keyframe(0.8, "1.0", None),
            ],
        };

        // Before first keyframe
        let v = interpolate_float(&prop, 0.0).unwrap();
        assert!((v - 0.0).abs() < 0.001);

        // After last keyframe
        let v = interpolate_float(&prop, 1.0).unwrap();
        assert!((v - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_interpolate_cubic_bezier() {
        // Easing is on the target keyframe (describes how to arrive at it)
        let prop = AmAnimatedFloat {
            value: None,
            keyframes: vec![
                make_keyframe(0.0, "0.0", None),
                make_keyframe(1.0, "100.0", Some("cubicBezier 0.0 0.0 0.58 1.0")),
            ],
        };

        let v_mid = interpolate_float(&prop, 0.5).unwrap();
        // ease-out should be faster at the start, so at t=0.5, value should be > 50
        assert!(v_mid > 50.0, "Expected > 50.0 for ease-out, got {}", v_mid);
    }
}

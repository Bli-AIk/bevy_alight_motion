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

use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

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

    pub fn has_effects(&self) -> bool { !self.effects.is_empty() }
    pub fn effect_count(&self) -> usize { self.effects.len() }
    pub fn mark_dirty(&mut self) { self.dirty = true; }

    // Effect type checks
    pub fn has_wipe(&self) -> bool {
        self.effects.iter().any(|e| matches!(e, EffectType::Wipe(_)))
    }
    pub fn has_stretch_segment(&self) -> bool {
        self.effects.iter().any(|e| matches!(e, EffectType::StretchSegment(_)))
    }
    pub fn has_mask(&self) -> bool {
        self.effects.iter().any(|e| matches!(e, EffectType::Mask(_)))
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
        Self { tex_a, tex_b, size, read_index: 0 }
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
        if self.read_index == 0 { &self.tex_a } else { &self.tex_b }
    }

    /// Get the current write (output) texture
    pub fn write_texture(&self) -> &Handle<Image> {
        if self.read_index == 0 { &self.tex_b } else { &self.tex_a }
    }

    /// Swap after completing a pass
    pub fn swap(&mut self) { self.read_index = 1 - self.read_index; }

    /// Reset to initial state
    pub fn reset(&mut self) { self.read_index = 0; }

    /// Resize buffers if needed
    pub fn resize_if_needed(&mut self, images: &mut Assets<Image>, new_size: Vec2) {
        if (self.size - new_size).length_squared() < 0.01 { return; }

        let extent = Extent3d {
            width: new_size.x.max(1.0) as u32,
            height: new_size.y.max(1.0) as u32,
            depth_or_array_layers: 1,
        };

        if let Some(img) = images.get_mut(&self.tex_a) { img.resize(extent); }
        if let Some(img) = images.get_mut(&self.tex_b) { img.resize(extent); }
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
                entity, effect_layer.source_size
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
pub fn mark_dirty_on_change_system(
    mut query: Query<&mut EffectLayer, Changed<EffectLayer>>
) {
    for mut layer in query.iter_mut() {
        layer.dirty = true;
    }
}

// ============================================================================
// Conversion Helpers
// ============================================================================

pub fn vec4_to_wipe_params(v: Vec4) -> WipeParams {
    WipeParams { start: v.x, end: v.y, angle: v.z, feather: v.w }
}

pub fn wipe_params_to_vec4(p: &WipeParams) -> Vec4 {
    Vec4::new(p.start, p.end, p.angle, p.feather)
}

pub fn vec4_to_stretch_params(v: Vec4) -> StretchSegmentParams {
    StretchSegmentParams { angle: v.x, stretch: v.y, offset: v.z, smooth: v.w }
}

pub fn stretch_params_to_vec4(p: &StretchSegmentParams) -> Vec4 {
    Vec4::new(p.angle, p.stretch, p.offset, p.smooth)
}

pub fn vec4_to_mask_params(v: Vec4) -> MaskParams {
    MaskParams { center_x: v.x, center_y: v.y, half_width: v.z, half_height: v.w }
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
        app.add_systems(
            Update,
            (
                setup_effect_buffers_system,
                update_effect_buffers_system,
                mark_dirty_on_change_system,
            ),
        );
    }
}

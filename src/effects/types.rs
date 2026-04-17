//! # types.rs
//!
//! # 效果类型模块
//!
//! Effect parameter types and data structures.
//! 效果参数类型和数据结构。

use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

/// Create a render-target `Image` with `data: None` so Bevy allocates GPU
/// memory without an expensive DMA zero-fill upload (~18 ms per 1080p RGBA8).
/// The camera will overwrite every pixel on first render anyway.
///
/// 创建 `data: None` 的渲染目标纹理。跳过 DMA 零填充上传，GPU 仅分配显存。
pub fn create_rtt_image(w: u32, h: u32, format: TextureFormat) -> Image {
    Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size: Extent3d {
                width: w.max(1),
                height: h.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        data: None,
        ..default()
    }
}

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

    fn create_rtt(images: &mut Assets<Image>, size: Vec2, _label: &'static str) -> Handle<Image> {
        images.add(create_rtt_image(
            size.x.max(1.0) as u32,
            size.y.max(1.0) as u32,
            TextureFormat::Rgba8UnormSrgb,
        ))
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
// Render Strategy Types for Hybrid Rendering Pipeline
// 混合渲染管线的渲染策略类型
// ============================================================================

/// Render strategy for an EmbedScene.
///
/// Determines how an EmbedScene should be rendered based on its requirements.
/// This is the core of the "Hybrid Rendering Pipeline" architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Component)]
pub enum RenderStrategy {
    /// Direct rendering - no isolation needed.
    ///
    /// The embed's children render directly to the parent's RenderLayer.
    /// This is the default for 90%+ of embeds (simple groups with only transform).
    ///
    /// - No RTT allocated
    /// - No separate RenderLayer
    /// - Children inherit parent's layer via Z-sorting
    #[default]
    Direct,

    /// Stencil-based clipping.
    ///
    /// Content is clipped to embed bounds using GPU stencil/scissor test.
    /// Still renders to parent's layer - no RTT overhead.
    ///
    /// - No RTT allocated
    /// - No separate RenderLayer
    /// - Uses stencil test for rectangular clipping
    Stencil,

    /// Full composition with RTT.
    ///
    /// Requires a dedicated RenderLayer and render-to-texture.
    /// Used only when mathematically necessary:
    /// - Has shader effects (blur, distortion)
    /// - Has complex blend modes with overlapping content
    /// - Has non-rectangular masks
    ///
    /// - Allocates RTT from pool
    /// - Gets dedicated RenderLayer (1-31)
    /// - Content renders to RTT, result composites to parent
    Composite,
}

/// Group fill type for embed scenes.
/// Determines how the group's RTT output is rendered.
#[derive(Debug, Clone, PartialEq)]
pub enum GroupFillType {
    /// No fill - group is invisible (fillType="none").
    None,
    /// Solid color fill (fillType="color").
    Color,
    /// Gradient fill (fillType="gradient").
    Gradient {
        /// Gradient type: 1=linear, 2=radial, 3=sweep
        gradient_type: u8,
        /// Start color (linear RGBA)
        start_color: Vec4,
        /// End color (linear RGBA)
        end_color: Vec4,
        /// Start/end points in UV space: (start_x, start_y, end_x, end_y)
        points: Vec4,
    },
}

/// Component for embed scenes with fill applied.
/// When attached, the group uses Composite (RTT) strategy and applies fill to the output.
#[derive(Component, Debug, Clone)]
pub struct AmGroupFill {
    /// Fill type and parameters.
    pub fill_type: GroupFillType,
    /// Fill color (linear RGBA) - used for Color fill type.
    pub fill_color: Vec4,
}

/// Component storing the computed render hierarchy info.
/// Used by the propagation system to determine RenderLayers.
#[derive(Component, Debug, Clone)]
pub struct RenderHierarchyInfo {
    /// The effective RenderLayer for this entity's content.
    /// For Direct/Stencil, this is inherited from parent.
    /// For Composite, this is the allocated layer from pool.
    pub effective_layer: u8,

    /// Computed global Z value for sorting within the same layer.
    pub global_z: f32,

    /// Whether this entity requires RTT (Composite strategy).
    pub requires_rtt: bool,
}

impl Default for RenderHierarchyInfo {
    fn default() -> Self {
        Self {
            effective_layer: 0,
            global_z: 0.0,
            requires_rtt: false,
        }
    }
}

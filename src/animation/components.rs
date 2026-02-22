//! # components.rs
//!
//! # 组件模块
//!
//! Animation components and resources for Alight Motion projects.
//! Contains AmAnimated, AmPlayback, AmSdfParams and related component definitions.
//!
//! Alight Motion 项目的动画组件和资源。
//! 包含 AmAnimated、AmPlayback、AmSdfParams 及相关组件定义。

use bevy::prelude::*;

use crate::scene::effects::PathRepeatParams;
use crate::schema::{AmAnimatedFloat, AmAnimatedVec2, AmAnimatedVec3};

/// DEBUG: 拉伸效果乘数，用于调试编组内图片的拉伸计算
/// 当前问题："编组 2 Copy" 内的图片拉伸效果过大
/// 调整此值直到编组内图片的拉伸效果与AM一致
/// 然后报告该值，用于推导正确的计算公式
///
/// 负height元素使用对角线公式：base_size = sqrt(w^2 + h^2) * SCALE_FACTOR
/// 当前测试表明需要的修正因子是 1/0.615 = 1.626
/// 而纯对角线公式给出 1.634
/// 所以需要额外的缩放因子 = 1.626 / 1.634 = 0.995
///
/// 默认值 1.0 = 纯对角线公式
/// 尝试 0.99, 0.98 等值来增大拉伸
pub const DEBUG_NEGATIVE_HEIGHT_SCALE: f32 = 1.05;

/// Component marking an entity as part of an AM animation.
///
/// 标记实体为 AM 动画一部分的组件。
#[derive(Component, Debug, Clone, Default)]
pub struct AmAnimated {
    /// Unique layer ID from AM.
    ///
    /// AM 中的唯一图层 ID。
    pub layer_id: u64,
    /// Start time in milliseconds (relative to time_offset).
    ///
    /// 开始时间（毫秒，相对于时间偏移）。
    pub start_time: i32,
    /// End time in milliseconds (relative to time_offset).
    pub end_time: i32,
    /// Time offset from parent scene (for embedded scenes).
    /// Used for animation interpolation: local_time = (global - time_offset) * speed
    pub time_offset: i32,
    /// Lifecycle offset for visibility calculation (not affected by speed).
    /// Used for spawn/despawn: lifecycle_time = global - lifecycle_offset
    /// For embeds: lifecycle_offset = embed_start - in_time
    pub lifecycle_offset: i32,
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
    /// Parent layer's ID (0 if no parent). Used for AM-style transform computation.
    pub parent_layer_id: u64,
    /// Effect position X offset (from transform2 effect).
    pub effect_pos_x: AmAnimatedFloat,
    /// Effect position Y offset (from transform2 effect).
    pub effect_pos_y: AmAnimatedFloat,
    /// Effect scale (from transform2 posz, default 1.0).
    pub effect_posz: AmAnimatedFloat,
    /// Effect rotation angle in degrees (from transform2 angle).
    pub effect_angle: AmAnimatedFloat,
    /// Transform2 X inversion flag.
    pub effect_xinv: bool,
    /// Transform2 Y inversion flag.
    pub effect_yinv: bool,
    /// Transform2 Z (scale) inversion flag.
    pub effect_zinv: bool,
    /// Transform2 angle inversion flag.
    pub effect_ainv: bool,
    /// Additional stacked transform2 effects (beyond the first).
    pub extra_transform2: Vec<crate::scene::effects::Transform2Params>,
    /// Font Y offset for text layers (to compensate for different font metrics).
    pub font_y_offset: f32,
    /// Size animation data (for shapes). AM size is half-extents, stored as full dimensions.
    pub size: AmAnimatedVec2,
    /// Position compensation for anchor offset (Bevy coords).
    /// When anchor is not CENTER, sprite position needs adjustment to keep center at AM location.
    pub anchor_offset: Vec2,
    /// Wipe effect start (0.0-1.0 percentage, default 0.0).
    pub wipe_start: AmAnimatedFloat,
    /// Wipe effect end (0.0-1.0 percentage, default 1.0).
    pub wipe_end: AmAnimatedFloat,
    /// Wipe effect angle in radians (0 = left-to-right).
    pub wipe_angle: AmAnimatedFloat,
    /// Wipe effect feather (softness of edge, 0.0 = sharp).
    pub wipe_feather: AmAnimatedFloat,
    /// Stretch segment effect angle in degrees (0 = horizontal split).
    pub stretch_angle: AmAnimatedFloat,
    /// Stretch segment effect stretch amount (pixels, normalized to UV).
    pub stretch_amount: AmAnimatedFloat,
    /// Stretch segment effect offset (position of split line).
    pub stretch_offset: AmAnimatedFloat,
    /// Stretch segment effect smooth width (0 = hard edge).
    pub stretch_smooth: AmAnimatedFloat,
    /// Gaussian blur effect strength (0 = no blur).
    pub blur_strength: AmAnimatedFloat,
    /// Speed multiplier from parent embed scenes.
    /// Local time = (global_time - time_offset) * speed_multiplier
    pub speed_multiplier: f32,
    /// Element-level speed (from shape/nullobj `speed` attribute, default 1.0).
    /// Affects keyframe interpolation rate: layer_time = raw_layer_time * element_speed.
    /// Does NOT affect visibility timing (start/end).
    pub element_speed: f32,
    /// Embed parent offset (Bevy coords) for coordinate adjustment.
    /// When this layer is a child of an embed scene, this stores the embed's
    /// Bevy position so the animation system can compensate for it.
    pub embed_offset: Vec2,
    /// Inverse fit scale for embed children coordinate adjustment.
    /// When the project is scaled to fit window, embed children need their coordinates
    /// scaled by 1/fit_scale to compensate for the root scaling.
    pub inv_fit_scale: f32,
    /// Stroke width animation data (for SDF shapes with stroke).
    pub stroke_width: AmAnimatedFloat,
    /// Base alpha from fill color (0.0-1.0).
    /// Opacity animation is multiplied by this value to preserve original fill transparency.
    pub base_alpha: f32,
    /// Palette map effect alpha (effect strength, 0.0-1.0).
    pub palette_alpha: AmAnimatedFloat,
    /// Scale assist effect scale multiplier (animated).
    pub scale_assist: AmAnimatedFloat,
    /// Scale assist effect damp factor (animated).
    pub scale_assist_damp: AmAnimatedFloat,
    /// Scale assist effect axis (1=X, 2=Y, 3=XY).
    pub scale_assist_axis: i32,
    /// Stretch2 effect scale (animated).
    pub stretch2_scale: AmAnimatedFloat,
    /// Stretch2 effect angle in degrees (animated).
    pub stretch2_angle: AmAnimatedFloat,
    /// Stretch2 contentOnly flag.
    pub stretch2_content_only: bool,
    /// Replace color effect: original color to replace (RGBA)
    pub replace_old_color: Vec4,
    /// Replace color effect: new color (animated RGBA)
    pub replace_new_color: crate::schema::AmAnimatedColor,
    /// Replace color effect: threshold (0.0-1.0)
    pub replace_threshold: AmAnimatedFloat,
    /// Replace color effect: feather (0.0-1.0)
    pub replace_feather: AmAnimatedFloat,
    /// Replace color effect: alpha/strength (0.0-1.0)
    pub replace_alpha: AmAnimatedFloat,
    /// Replace color effect: lock luminance
    pub replace_lock_luminance: bool,
    /// Repeat effect: number of copies (0 = no effect)
    pub repeat_count: AmAnimatedFloat,
    /// Repeat effect: X,Y offset per copy (pixels)
    pub repeat_offset: AmAnimatedVec2,
    /// Repeat effect: rotation angle per copy (degrees)
    pub repeat_angle: AmAnimatedFloat,
    /// Repeat effect: scale multiplier per copy
    pub repeat_scale: AmAnimatedFloat,
    /// Repeat effect: alpha multiplier per copy
    pub repeat_alpha: AmAnimatedFloat,
    // Linear Repeat effect (com.alightcreative.effects.repeat.line)
    /// Linear repeat effect: number of copies
    pub linear_repeat_count: AmAnimatedFloat,
    /// Linear repeat effect: position offset for the repeat line (pixels)
    pub linear_repeat_position: AmAnimatedVec2,
    /// Linear repeat effect: additional offset per copy (pixels)
    pub linear_repeat_offset: AmAnimatedVec2,
    /// Linear repeat effect: rotation angle per copy (degrees)
    pub linear_repeat_angle: AmAnimatedFloat,
    /// Linear repeat effect: scale multiplier per copy
    pub linear_repeat_scale: AmAnimatedFloat,
    /// Linear repeat effect: alpha multiplier per copy
    pub linear_repeat_alpha: AmAnimatedFloat,
    /// Linear repeat effect: fill color for copies (animated)
    pub linear_repeat_fill_color: crate::schema::AmAnimatedColor,
    /// Linear repeat effect: color blend factor
    pub linear_repeat_blend: AmAnimatedFloat,
    /// Linear repeat effect: color alt copies flag
    pub linear_repeat_color_alt_copies: bool,
    /// Linear repeat effect: start of visible range (0.0-1.0)
    pub linear_repeat_start: AmAnimatedFloat,
    /// Linear repeat effect: end of visible range (0.0-1.0)
    pub linear_repeat_end: AmAnimatedFloat,
    /// Linear repeat effect: phase shift
    pub linear_repeat_phase: AmAnimatedFloat,
    /// Linear repeat effect: ease-in factor
    pub linear_repeat_ease_in: AmAnimatedFloat,
    /// Linear repeat effect: ease-out factor
    pub linear_repeat_ease_out: AmAnimatedFloat,
    /// Linear repeat effect: overlap factor
    pub linear_repeat_overlap: AmAnimatedFloat,
    /// Linear repeat effect: distribution shape (0 = linear)
    pub linear_repeat_shape: i32,
    /// Linear repeat effect: invert flag
    pub linear_repeat_invert: bool,
    /// Linear repeat effect: random order flag
    pub linear_repeat_random_order: bool,
    /// Linear repeat effect: random seed
    pub linear_repeat_seed: f32,
    /// Second linear repeat effect (for stacked/dual effects)
    pub linear_repeat2: Option<Box<crate::scene::effects::LinearRepeatParams>>,
    // Radial Repeat effect (com.alightcreative.effects.repeat.radial)
    pub radial_repeat_count: AmAnimatedFloat,
    pub radial_repeat_radius: AmAnimatedFloat,
    pub radial_repeat_orientation: AmAnimatedFloat,
    pub radial_repeat_start_angle: AmAnimatedFloat,
    pub radial_repeat_sweep: AmAnimatedFloat,
    pub radial_repeat_base_scale: AmAnimatedFloat,
    pub radial_repeat_offset: AmAnimatedVec2,
    pub radial_repeat_angle: AmAnimatedFloat,
    pub radial_repeat_scale: AmAnimatedFloat,
    pub radial_repeat_alpha: AmAnimatedFloat,
    pub radial_repeat_fill_color: crate::schema::AmAnimatedColor,
    pub radial_repeat_blend: AmAnimatedFloat,
    pub radial_repeat_color_alt_copies: bool,
    pub radial_repeat_start: AmAnimatedFloat,
    pub radial_repeat_end: AmAnimatedFloat,
    pub radial_repeat_phase: AmAnimatedFloat,
    pub radial_repeat_ease_in: AmAnimatedFloat,
    pub radial_repeat_ease_out: AmAnimatedFloat,
    pub radial_repeat_overlap: AmAnimatedFloat,
    pub radial_repeat_shape: i32,
    pub radial_repeat_invert: bool,
    pub radial_repeat_random_order: bool,
    pub radial_repeat_seed: f32,
    // Oscillate effect (com.alightcreative.effects.oscillate3)
    /// Oscillate effect: direction mode (0=angle, 1=depth/z, 2=orbit)
    pub oscillate_direction: i32,
    /// Oscillate effect: movement angle (degrees)
    pub oscillate_angle: AmAnimatedFloat,
    /// Oscillate effect: frequency (Hz)
    pub oscillate_freq: AmAnimatedFloat,
    /// Oscillate effect: magnitude (pixels)
    pub oscillate_mag: AmAnimatedFloat,
    /// Oscillate effect: wave type (0=sine, 1=triangle)
    pub oscillate_wave_type: i32,
    /// Oscillate effect: phase offset
    pub oscillate_phase: AmAnimatedFloat,
    // Swing effect (com.alightcreative.effects.swing2)
    /// Swing effect: oscillation frequency (per second)
    pub swing_freq: AmAnimatedFloat,
    /// Swing effect: minimum angle (degrees)
    pub swing_a1: AmAnimatedFloat,
    /// Swing effect: maximum angle (degrees)
    pub swing_a2: AmAnimatedFloat,
    /// Swing effect: phase offset (0.0-1.0)
    pub swing_phase: AmAnimatedFloat,
    /// Swing effect: swing type (0 = sine, 1 = triangle)
    pub swing_type: i32,
    // Spin effect (com.alightcreative.effects.spin)
    /// Spin effect: RPM (revolutions per minute)
    pub spin_rpm: AmAnimatedFloat,
    // Threshold effect (com.alightcreative.effects.threshold)
    /// Threshold effect: threshold value (0.0-1.0)
    pub threshold_value: AmAnimatedFloat,
    /// Threshold effect: feather/softness (0.0-1.0)
    pub threshold_feather: AmAnimatedFloat,
    /// Threshold effect: invert flag
    pub threshold_invert: bool,
    /// Threshold effect: blend mode
    pub threshold_blend_mode: i32,
    // Grid effect (com.alightcreative.effects.grid2)
    /// Grid effect: position offset
    pub grid_position: AmAnimatedVec2,
    /// Grid effect: spacing (0.0-1.0)
    pub grid_spacing: AmAnimatedFloat,
    /// Grid effect: line width (0.0-1.0)
    pub grid_width: AmAnimatedFloat,
    /// Grid effect: color
    pub grid_color: crate::schema::AmAnimatedColor,
    /// Grid effect: punchout mode
    pub grid_punchout: bool,
    /// Grid effect: smoothing
    pub grid_smoothing: AmAnimatedFloat,
    /// Grid effect: screen space mode
    pub grid_screen_space: bool,
    // Pixelate effect (com.alightcreative.effects.pixelate2)
    /// Pixelate effect: pixel block size
    pub pixelate_size: AmAnimatedFloat,
    /// Pixelate effect: stretch factor (x, y)
    pub pixelate_stretch: AmAnimatedVec2,
    /// Pixelate effect: rotation angle (degrees)
    pub pixelate_angle: AmAnimatedFloat,
    /// Pixelate effect: vignette darkening
    pub pixelate_vignette: AmAnimatedFloat,
    /// Pixelate effect: threshold for color posterization
    pub pixelate_threshold: AmAnimatedFloat,
    /// Pixelate effect: saturation adjustment
    pub pixelate_saturation: AmAnimatedFloat,
    /// Pixelate effect: use screen space coordinates
    pub pixelate_screen_space: bool,
    // Solid color effect (com.alightcreative.solidcolor)
    /// Solid color: overlay color (animated)
    pub solid_color: crate::schema::AmAnimatedColor,
    /// Solid color: blend alpha (0.0-1.0)
    pub solid_color_alpha: AmAnimatedFloat,
    /// Solid color: blend mode (0=normal, 1=multiply, 2=screen)
    pub solid_color_blend_mode: i32,
    /// Base fill color (stored for solidcolor mixing)
    pub base_fill_color: [f32; 4],
    // Path Repeat effect (com.alightcreative.effects.repeat.path)
    /// Path repeat params (None = no effect)
    pub path_repeat: Option<PathRepeatParams>,
    // Text Spacing effect (com.alightcreative.effects.textspacing)
    /// Letter spacing in em units (0.0 = default)
    pub textspacing_letter: AmAnimatedFloat,
    /// Line spacing multiplier (1.0 = default)
    pub textspacing_line: AmAnimatedFloat,
    // Text Progress effect (com.alightcreative.effects.textprogress)
    /// Text progress start (0.0-1.0)
    pub textprogress_start: AmAnimatedFloat,
    /// Text progress end (0.0-1.0)
    pub textprogress_end: AmAnimatedFloat,
    /// Text progress cursor style (0-3)
    pub textprogress_cursor: i32,
    /// Text progress blink enabled
    pub textprogress_blink: bool,
    // Shape-specific animated properties
    /// Generic shape float properties (up to 4).
    /// Meaning depends on shape_type:
    /// RoundRect: [cornerRadius, _, _, _]
    /// Polygon: [sideCount, radius, offsetAngle, _]
    /// Star/Multifoil: [pointCount, outerRadius, innerRadius, offsetAngle]
    /// Pie/Arc: [startAngle, endAngle, radius, _]
    /// Plus: [stemSize, _, _, _]
    pub shape_props: [AmAnimatedFloat; 4],
    /// Generic shape vec2 properties (up to 5 points).
    /// Used by Line, Triangle, Quad, Penta for vertex animation.
    pub shape_points: [AmAnimatedVec2; 5],
}

impl AmAnimated {
    /// Calculate local time considering speed_multiplier (for animation interpolation).
    pub fn calc_local_time(&self, global_time: f32) -> f32 {
        (global_time - self.time_offset as f32) * self.speed_multiplier
    }

    /// Calculate lifecycle time (for visibility/spawn decisions, not affected by speed).
    pub fn calc_lifecycle_time(&self, global_time: f32) -> f32 {
        global_time - self.lifecycle_offset as f32
    }

    /// Check if layer is active at the given local time.
    pub fn is_active(&self, local_time: f32) -> bool {
        local_time >= self.start_time as f32 && local_time <= self.end_time as f32
    }

    /// Calculate normalized layer time (0.0 to 1.0) from local time.
    pub fn calc_layer_time(&self, local_time: f32) -> f32 {
        let duration = (self.end_time - self.start_time) as f32;
        if duration > 0.0 {
            (local_time - self.start_time as f32) / duration
        } else {
            0.0
        }
    }
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

/// Component to store SDF shape parameters for animation.
/// Used by animate_sdf_scale to update SdfMaterial.params based on animation scale.
#[derive(Component, Debug, Clone)]
pub struct AmSdfParams {
    /// Base half width of the shape (before animation scale)
    pub base_half_width: f32,
    /// Base half height of the shape (before animation scale)
    pub base_half_height: f32,
    /// Stroke width in pixels (constant, not scaled)
    pub stroke_width: f32,
    /// Packed stroke color (stored to preserve during updates)
    pub packed_stroke: f32,
    /// Base stroke alpha (0.0-1.0) from original stroke color
    pub base_stroke_alpha: f32,
    /// Base pivot X in pixels
    pub base_pivot_x: f32,
    /// Base pivot Y in pixels
    pub base_pivot_y: f32,
    /// Border 2 width (static, 0 if no second border)
    pub border2_width: f32,
    /// Border 2 packed color
    pub border2_packed_color: f32,
    /// Border 2 mode (0=centered, 1=inside, -1=outside)
    pub border2_mode: f32,
}

// Keep legacy types for now to avoid breaking changes in case they're referenced elsewhere
/// Component to store original SDF fill parameters for animation.
/// @deprecated Use AmSdfParams instead
#[derive(Component, Debug, Clone)]
pub struct AmSdfFillParams {
    /// Base half width of the shape (without scale)
    pub base_half_width: f32,
    /// Base half height of the shape (without scale)
    pub base_half_height: f32,
    /// Half of the stroke width (used to inset the fill)
    pub stroke_half_width: f32,
}

/// Component to store original SDF stroke parameters for animation.
/// @deprecated Use AmSdfParams instead
#[derive(Component, Debug, Clone)]
pub struct AmSdfStrokeParams {
    /// Base half width of the shape (without scale)
    pub base_half_width: f32,
    /// Base half height of the shape (without scale)
    pub base_half_height: f32,
    /// Half of the stroke width (used to offset the stroke)
    pub stroke_half_width: f32,
}

/// Marker component to identify entities that are SDF shape parents.
/// Used to skip scale animation in animate_transform (scale is handled by animate_sdf_scale).
#[derive(Component, Debug, Clone, Default)]
pub struct AmSdfShapeParent;

/// Camera layer data for AM perspective camera animation.
/// Stores FOV animation and base parameters needed to compute 2D pan/zoom from 3D camera.
#[derive(Component, Debug, Clone)]
pub struct AmCameraLayer {
    /// FOV animation in degrees.
    pub fov: AmAnimatedFloat,
    /// Initial Z distance (negative, e.g. -1247).
    pub base_z: f32,
    /// Scene width in pixels.
    pub scene_width: f32,
    /// Scene height in pixels.
    pub scene_height: f32,
}

/// Component linking a path-repeat entity to its path source (previous layer).
/// The source entity provides the shape outline along which copies are placed.
/// Source animation data is stored directly so it remains available even after
/// the source entity is despawned.
#[derive(Component, Debug)]
pub struct AmPathRepeat {
    /// The entity whose shape outline defines the path.
    pub source_entity: Entity,
    /// Entities spawned as copies (managed by the path-repeat system).
    pub copy_entities: Vec<Entity>,
    /// Shape type of source (e.g. ".rect")
    pub source_shape_type: String,
    /// Layer ID of the source (for logging)
    pub source_layer_id: u64,
    /// Cloned source animation data so we can compute path positions
    /// even when the source entity has been despawned.
    pub source_animated: AmAnimated,
}

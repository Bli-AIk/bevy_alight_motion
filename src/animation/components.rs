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

use crate::scene::AmBlendingMode;
use crate::scene::effects::PathRepeatParams;
use crate::schema::{AmAnimatedFloat, AmAnimatedVec2, AmAnimatedVec3};

/// Retime mode for embedded scenes.
/// Controls how nested scene time is mapped when container duration differs from content duration.
///
/// 嵌入场景的重定时模式。
/// 控制容器时长与内容时长不同时，嵌套场景时间的映射方式。
#[derive(Debug, Clone, Default, PartialEq)]
pub enum RetimeMode {
    /// No retiming - content plays at normal speed, may be cut off.
    #[default]
    Off,
    /// Stretch content to fit container duration.
    Stretch,
    /// Freeze on last frame when content ends.
    Freeze,
    /// Loop content when it ends.
    Loop,
    /// Loop with integer stride so loops fit evenly.
    LoopStretch,
    /// Content goes blank when it ends.
    Blank,
}

impl RetimeMode {
    /// Parse retime mode from AM XML attribute value.
    pub fn parse(s: &str) -> Self {
        match s {
            "stretch" => Self::Stretch,
            "freeze" => Self::Freeze,
            "loop" => Self::Loop,
            "loop-stretch" => Self::LoopStretch,
            "blank" => Self::Blank,
            _ => Self::Off,
        }
    }
}

/// Retime parameters for children of a retimed embed scene.
/// Stored on each child layer's AmAnimated to transform global time into retimed time.
///
/// 重定时参数，用于重定时嵌入场景的子图层。
#[derive(Debug, Clone)]
pub struct AmRetimeInfo {
    pub mode: RetimeMode,
    /// Global time when the embed container starts playing.
    pub embed_global_start: f32,
    /// Duration of the embed container in ms (endTime - startTime).
    pub container_duration_ms: f32,
    /// totalTime of the nested scene in ms.
    pub nested_total_time_ms: f32,
    /// Combined speed up to the embed level (parent speed * embed speed).
    pub embed_speed: f32,
}

/// Runtime echokf data for dynamically updating echo entities each frame.
/// Attached to echo entities that need per-frame count/seconds/alpha updates.
#[derive(Component, Debug, Clone)]
pub struct AmEchoRuntime {
    /// This echo's index (0-based)
    pub echo_index: u32,
    /// Max echo count (total spawned)
    pub max_count: u32,
    /// Echo mode: 0=atop, 1=behind
    pub mode: i32,
    /// Keyframed count parameter
    pub count_kf: crate::schema::AmAnimatedFloat,
    /// Keyframed seconds parameter
    pub seconds_kf: crate::schema::AmAnimatedFloat,
    /// Keyframed alpha parameter
    pub alpha_kf: crate::schema::AmAnimatedFloat,
    /// Embed element start time (ms) for fractional time computation
    pub embed_start: f32,
    /// Embed element end time (ms)
    pub embed_end: f32,
    /// Embed element time_offset (for computing global-to-local time)
    pub embed_time_offset: f32,
    /// Parent speed multiplier
    pub embed_speed: f32,
}

/// Echo alpha config for entities in an echokf echo subtree.
/// Contains everything needed to evaluate per-frame alpha.
#[derive(Debug, Clone)]
pub struct EchoAlphaConfig {
    /// Alpha keyframes from echokf effect
    pub alpha_keyframes: crate::schema::AmAnimatedFloat,
    /// Fraction (0..1) for mixing: mix(alpha(t), 1.0, fraction)
    pub fraction: f32,
    /// Parent element start time (ms)
    pub parent_start: i32,
    /// Parent element end time (ms)
    pub parent_end: i32,
    /// Parent element time_offset
    pub parent_time_offset: f32,
    /// Parent element speed_multiplier
    pub parent_speed: f32,
}

/// Marker for unified-material visuals whose size should come from Transform.scale
/// instead of per-frame mesh resizing.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct AmUnifiedUsesTransformScale;

impl EchoAlphaConfig {
    /// Evaluate echo alpha at the given global time.
    /// Returns the multiplier for opacity (0.0 = invisible, 1.0 = fully opaque).
    pub fn evaluate(&self, global_time: f32) -> f32 {
        let parent_local = (global_time - self.parent_time_offset) * self.parent_speed;
        let parent_duration = (self.parent_end - self.parent_start) as f32;
        let parent_layer_time = if parent_duration > 0.0 {
            (parent_local - self.parent_start as f32) / parent_duration
        } else {
            0.0
        };
        let alpha_at_time =
            super::interpolation::interpolate_float(&self.alpha_keyframes, parent_layer_time)
                .unwrap_or(1.0);
        // mix(alpha, 1.0, fraction) = alpha * (1 - fraction) + fraction
        alpha_at_time * (1.0 - self.fraction) + self.fraction
    }
}

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
    pub time_offset: f32,
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
    /// Second stretch segment effect angle in degrees.
    pub stretch_seg2_angle: AmAnimatedFloat,
    /// Second stretch segment effect stretch amount.
    pub stretch_seg2_amount: AmAnimatedFloat,
    /// Second stretch segment effect offset.
    pub stretch_seg2_offset: AmAnimatedFloat,
    /// Second stretch segment effect smooth width.
    pub stretch_seg2_smooth: AmAnimatedFloat,
    /// Gaussian blur effect strength (0 = no blur).
    pub blur_strength: AmAnimatedFloat,
    /// Speed multiplier from parent embed scenes.
    /// Local time = (global_time - time_offset) * speed_multiplier
    pub speed_multiplier: f32,
    /// Element-level speed (from shape/nullobj `speed` attribute, default 1.0).
    /// Affects keyframe interpolation rate: layer_time = raw_layer_time * element_speed.
    /// Does NOT affect visibility timing (start/end).
    pub element_speed: f32,
    /// Scene FPS for timing calculations (from the scene's fps attribute).
    pub scene_fps: f32,
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
    /// Fade-in duration in seconds (animated). / 渐入持续时间（秒）。
    pub fade_in_time: AmAnimatedFloat,
    /// Fade-out duration in seconds (animated). / 渐出持续时间（秒）。
    pub fade_out_time: AmAnimatedFloat,
    /// Layer duration in milliseconds (used by fade effect for time normalization).
    /// 图层持续时间（毫秒，用于渐入渐出效果的时间归一化）。
    pub fade_layer_duration_ms: f32,
    /// Palette map effect alpha (effect strength, 0.0-1.0).
    pub palette_alpha: AmAnimatedFloat,
    /// Scale assist effect scale multiplier (animated).
    pub scale_assist: AmAnimatedFloat,
    /// Scale assist effect damp factor (animated).
    pub scale_assist_damp: AmAnimatedFloat,
    /// Scale assist effect axis (1=X, 2=Y, 3=XY).
    pub scale_assist_axis: i32,
    /// Parenthelper scale mode (0=normal, 1=locked, 2=weighted).
    pub parenthelper_scale_mode: i32,
    /// Parenthelper rotation mode (0=normal, 1=locked, 2=weighted).
    pub parenthelper_rotate_mode: i32,
    /// Parenthelper scale inheritance weight.
    pub parenthelper_scale_weight: AmAnimatedFloat,
    /// Parenthelper rotation inheritance weight.
    pub parenthelper_rotate_weight: AmAnimatedFloat,
    /// Parenthelper auto-rotation mode (0=off, 1=X, 2=Y).
    pub parenthelper_auto_rotate: i32,
    /// Parenthelper radius adjustment for auto-rotation.
    pub parenthelper_radius_adjust: AmAnimatedFloat,
    /// Whether parenthelper is present on this layer.
    pub parenthelper_has_effect: bool,
    /// Stretch2 effect scale (animated).
    pub stretch2_scale: AmAnimatedFloat,
    /// Stretch2 effect angle in degrees (animated).
    pub stretch2_angle: AmAnimatedFloat,
    /// Stretch2 contentOnly flag.
    pub stretch2_content_only: bool,
    // Wavewarp2 effect (波浪歪曲)
    /// Wave phase offset. / 波浪相位偏移。
    pub wavewarp2_phase: AmAnimatedFloat,
    /// Wave direction angle (degrees). / 波浪方向角度。
    pub wavewarp2_a1d: AmAnimatedFloat,
    /// Wave spacing/frequency. / 波浪间距。
    pub wavewarp2_m1: AmAnimatedFloat,
    /// Wave displacement magnitude. / 波浪幅度。
    pub wavewarp2_m2: AmAnimatedFloat,
    /// Warp direction angle offset (degrees). / 翘曲角度偏移。
    pub wavewarp2_a2d: AmAnimatedFloat,
    /// Magnitude damping. / 幅度阻尼。
    pub wavewarp2_damping: AmAnimatedFloat,
    /// Spacing damping. / 间距阻尼。
    pub wavewarp2_damping_space: AmAnimatedFloat,
    /// Damping origin. / 阻尼原点。
    pub wavewarp2_damping_origin: AmAnimatedFloat,
    /// Use screen-space coordinates. / 屏幕空间坐标。
    pub wavewarp2_screen_space: bool,
    /// Whether wavewarp2 is present on this layer.
    pub wavewarp2_has_effect: bool,
    /// Mirror type: 0=horizontal, 1=vertical. / 镜像方向。
    pub mirror_type: i32,
    /// Mirror blend mode. / 镜子混合模式。
    pub mirror_blend_mode: i32,
    /// Mirror alpha. / 镜子透明度。
    pub mirror_alpha: AmAnimatedFloat,
    /// Mirror offset. / 镜子偏移。
    pub mirror_offset: AmAnimatedFloat,
    /// Whether mirror is present on this layer.
    pub mirror_has_effect: bool,
    /// Lift (copy background) fill amount: 0=background, 1=original content. / 复制背景填充量。
    pub lift_fill: AmAnimatedFloat,
    /// Whether lift effect is present on this layer.
    pub lift_has_effect: bool,
    // Rays effect (com.alightcreative.effects.rays) / 射线效果
    /// Rays center X (AM coords, ±500). / 射线中心X。
    pub rays_center_x: AmAnimatedFloat,
    /// Rays center Y (AM coords, ±500). / 射线中心Y。
    pub rays_center_y: AmAnimatedFloat,
    /// Rays strength/length (0.0-4.0). / 射线长度。
    pub rays_strength: AmAnimatedFloat,
    /// Rays intensity (0.0-5.0). / 射线强度。
    pub rays_intensity: AmAnimatedFloat,
    /// Rays threshold (0.0-1.0). / 射线阈值。
    pub rays_threshold: AmAnimatedFloat,
    /// Rays threshold color (linear RGBA). / 阈值颜色。
    pub rays_threshold_color: Vec4,
    /// Rays fill color (linear RGBA). / 射线颜色。
    pub rays_fill_color: Vec4,
    /// Rays blend (0.0-1.0). / 混合比例。
    pub rays_blend: AmAnimatedFloat,
    /// Rays quality / sample count (10-800). / 采样数量。
    pub rays_quality: AmAnimatedFloat,
    /// Whether rays effect is present on this layer.
    pub rays_has_effect: bool,
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
    pub linear_repeat_seed: AmAnimatedFloat,
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
    /// Animated fill color keyframes (for runtime fill color animation)
    pub fill_color: crate::schema::AmAnimatedColor,
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
    /// Counter effect offset (added to numeric values in text)
    pub counter_offset: AmAnimatedFloat,
    /// Counter effect scale (multiplied with numeric values in text)
    pub counter_scale: AmAnimatedFloat,
    // Shape-specific animated properties
    /// Generic shape float properties (up to 4).
    /// Meaning depends on shape_type:
    /// RoundRect: [cornerRadius, _, _, _]
    /// Polygon: [sideCount, radius, offsetAngle, _]
    /// Star/Multifoil: [pointCount, outerRadius, innerRadius, offsetAngle]
    /// Pie/Arc: [startAngle, endAngle, radius, _]
    /// Plus: [stemSize, _, _, _]
    /// Arrow: [lineWidth, headWidth, headLength, _]
    pub shape_props: [AmAnimatedFloat; 4],
    /// Generic shape vec2 properties (up to 5 points).
    /// Used by Line, Arrow, Triangle, Quad, Penta for vertex animation.
    pub shape_points: [AmAnimatedVec2; 5],
    // Jitter effect (com.alightcreative.effects.jitter)
    /// Jitter effect: whether it is enabled
    pub jitter_enabled: bool,
    /// Jitter effect: movement angle (degrees) - may be keyframed
    pub jitter_angle: AmAnimatedFloat,
    /// Jitter effect: quantization frequency (steps/sec) - may be keyframed
    pub jitter_freq: AmAnimatedFloat,
    /// Jitter effect: displacement magnitude (pixels) - may be keyframed
    pub jitter_mag: AmAnimatedFloat,
    /// Jitter effect: noise seed - may be keyframed
    pub jitter_seed: AmAnimatedFloat,
    /// Jitter effect: perpendicular slack (0.0-1.0) - may be keyframed
    pub jitter_slack: AmAnimatedFloat,
    /// Jitter effect: z-axis jitter magnitude - may be keyframed
    pub jitter_zjitter: AmAnimatedFloat,
    // Simplex displace effect (com.alightcreative.effects.randomdisplace)
    /// Whether simplex displace is enabled
    pub sd_enabled: bool,
    /// Simplex displace: displacement magnitude (pixels) - may be keyframed
    pub sd_mag: AmAnimatedFloat,
    /// Simplex displace: noise evolution (temporal) - may be keyframed
    pub sd_evolution: AmAnimatedFloat,
    /// Simplex displace: noise seed - may be keyframed
    pub sd_seed: AmAnimatedFloat,
    /// Simplex displace: spatial frequency (0.0-2.0) - may be keyframed
    pub sd_scatter: AmAnimatedFloat,
    // RGB split effect (com.alightcreative.effects.rgbsep)
    /// Whether RGB split is enabled
    pub rgb_split_enabled: bool,
    /// RGB split: channel offset strength - may be keyframed
    pub rgb_split_strength: AmAnimatedFloat,
    /// RGB split: separation angle (degrees) - may be keyframed
    pub rgb_split_angle: AmAnimatedFloat,
    /// RGB split: center channel (0=R, 1=G, 2=B)
    pub rgb_split_center: i32,
    /// RGB split: compositing mode (0=Mask, 1=Luma, 2=Light, 3=Dark)
    pub rgb_split_mode: i32,
    // Exposure / Gamma effect (com.alightcreative.effects.exposure)
    /// Exposure adjustment value
    pub exposure_value: AmAnimatedFloat,
    /// Gamma curve value
    pub exposure_gamma: AmAnimatedFloat,
    /// Brightness offset value
    pub exposure_offset: AmAnimatedFloat,
    /// Whether exposure/gamma effect is present
    pub exposure_has_effect: bool,
    // ChromaKey effect (com.alightcreative.effects.chromakey)
    /// Whether chromakey effect is enabled
    pub chromakey_enabled: bool,
    /// Key color to remove (animated RGBA)
    pub chromakey_key_color: crate::schema::AmAnimatedColor,
    /// Color matching tolerance (0.0-1.0, animated)
    pub chromakey_threshold: AmAnimatedFloat,
    /// Edge transition softness (0.0-1.0, animated)
    pub chromakey_feather: AmAnimatedFloat,
    /// Remove edge color spill
    pub chromakey_defringe: bool,
    /// Invert keying result (keep key color areas)
    pub chromakey_invert: bool,
    /// Layer blend mode (Normal, Multiply, Screen, etc.)
    pub blend_mode: AmBlendingMode,
    /// Retime info for children of retimed embed scenes.
    /// When present, overrides linear time mapping with retime mode.
    pub retime: Option<AmRetimeInfo>,
    /// Echo time shift in milliseconds (for echokf effect).
    /// Applied in calc_layer_time to show animation at a past time.
    pub echo_time_shift_ms: f32,
    /// Echo alpha config (for echokf effect). If present, entity is an echo copy.
    /// Contains alpha keyframes, fraction, and parent timing for per-frame alpha evaluation.
    pub echo_alpha_config: Option<EchoAlphaConfig>,
    /// Accumulated repeat rotation offset in degrees (for group repeat copies).
    /// Applied additively to the final rotation in animate_transform_system.
    pub repeat_rotation_offset_deg: f32,
    /// Accumulated repeat scale factor (for group repeat copies).
    /// Applied multiplicatively to the final scale in animate_transform_system.
    pub repeat_scale_factor: f32,
    /// Accumulated repeat position offset in Bevy coords (for group repeat copies).
    /// Applied additively to the final position in animate_transform_system.
    pub repeat_position_offset: Vec2,
    /// For embed children: the inner scene's totalTime in ms.
    /// When local_time exceeds this, it gets clamped to freeze content at the last frame.
    /// This matches AM behavior where inner content stays visible when the embed
    /// plays longer than its inner scene duration.
    pub embed_inner_total_time: Option<f32>,
}

impl AmAnimated {
    /// Apply retime transformation to get nested scene time from global time.
    /// Returns None if no retime is active (use normal linear mapping).
    fn apply_retime(&self, global_time: f32) -> Option<f32> {
        let rt = self.retime.as_ref()?;
        if rt.mode == RetimeMode::Off {
            return None;
        }
        let embed_elapsed = (global_time - rt.embed_global_start) * rt.embed_speed;
        if embed_elapsed < 0.0 {
            return Some(0.0);
        }
        let total = rt.nested_total_time_ms;
        if total <= 0.0 {
            return Some(0.0);
        }
        let nested_time = match rt.mode {
            RetimeMode::Off => return None,
            RetimeMode::Stretch => {
                let container = rt.container_duration_ms;
                if container > 0.0 {
                    (embed_elapsed / container) * total
                } else {
                    embed_elapsed
                }
            }
            RetimeMode::Freeze => embed_elapsed.min(total),
            RetimeMode::Loop => embed_elapsed.rem_euclid(total),
            RetimeMode::LoopStretch => {
                Self::calc_loop_stretch_time(rt.container_duration_ms, total, embed_elapsed)
            }
            RetimeMode::Blank => {
                if embed_elapsed > total {
                    // Return a time that won't match any layer's range.
                    return Some(-1.0);
                }
                embed_elapsed
            }
        };
        Some(nested_time)
    }

    fn calc_loop_stretch_time(container: f32, total: f32, embed_elapsed: f32) -> f32 {
        if container <= 0.0 {
            return embed_elapsed;
        }
        let loops = (container / total).ceil().max(1.0);
        let stride = container / loops;
        if stride > 0.0 {
            ((embed_elapsed % stride) / stride) * total
        } else {
            embed_elapsed
        }
    }

    /// Calculate local time considering speed_multiplier (for animation interpolation).
    ///
    /// Note: The embed_inner_total_time clamp was intentionally removed.
    /// For retime=freeze, apply_retime() already clamps via `embed_elapsed.min(total)`.
    /// For retime=off, elements should naturally expire (is_active returns false),
    /// not freeze at the last frame.
    pub fn calc_local_time(&self, global_time: f32) -> f32 {
        if let Some(nested_time) = self.apply_retime(global_time) {
            return nested_time;
        }
        (global_time - self.time_offset) * self.speed_multiplier
    }

    /// Calculate lifecycle time (for visibility/spawn decisions, not affected by speed).
    pub fn calc_lifecycle_time(&self, global_time: f32) -> f32 {
        if let Some(nested_time) = self.apply_retime(global_time) {
            return nested_time;
        }
        global_time - self.lifecycle_offset as f32
    }

    /// Check if layer is active at the given local time.
    /// For echo/repeat entities with echo_time_shift_ms > 0, extends the active window
    /// so the shifted animation has time to play out fully.
    pub fn is_active(&self, local_time: f32) -> bool {
        local_time >= self.start_time as f32
            && local_time <= self.end_time as f32 + self.echo_time_shift_ms
    }

    /// Calculate normalized layer time (0.0 to 1.0) from local time.
    /// Applies element_speed: with speed=0.5, animation plays at half rate.
    /// For echo entities, shifts time backward by echo_time_shift_ms to show past state.
    pub fn calc_layer_time(&self, local_time: f32) -> f32 {
        let effective_time = local_time - self.echo_time_shift_ms;
        let duration = (self.end_time - self.start_time) as f32;
        if duration > 0.0 {
            (effective_time - self.start_time as f32) / duration * self.element_speed
        } else {
            0.0
        }
    }

    /// Calculate fade alpha multiplier at the given normalized layer time.
    /// Uses easeInOutQuad easing to smoothly fade in at the beginning and out at the end.
    ///
    /// 计算给定归一化图层时间下的渐入渐出透明度乘数。
    /// 使用 easeInOutQuad 缓动函数在开头平滑淡入，在结尾平滑淡出。
    pub fn calc_fade_alpha(&self, layer_time: f32) -> f32 {
        use super::interpolation::interpolate_float;

        let has_fade = self.fade_in_time.value.is_some()
            || !self.fade_in_time.keyframes.is_empty()
            || self.fade_out_time.value.is_some()
            || !self.fade_out_time.keyframes.is_empty();

        if !has_fade {
            return 1.0;
        }

        // Duration in seconds
        let duration_secs = self.fade_layer_duration_ms / 1000.0;
        if duration_secs <= 0.0 {
            return 1.0;
        }

        let in_time_secs = interpolate_float(&self.fade_in_time, layer_time).unwrap_or(0.0);
        let out_time_secs = interpolate_float(&self.fade_out_time, layer_time).unwrap_or(0.0);

        // Normalize to 0.0-1.0 range
        let in_time = in_time_secs / duration_secs;
        let out_time = out_time_secs / duration_secs;

        let t = layer_time;
        let mut alpha = 1.0_f32;

        // Fade in: during the first `in_time` portion
        if in_time > 0.0 && t < in_time {
            let progress = t / in_time;
            alpha *= ease_in_out_quad(progress);
        }

        // Fade out: during the last `out_time` portion
        if out_time > 0.0 && t > 1.0 - out_time {
            let progress = (1.0 - t) / out_time;
            alpha *= ease_in_out_quad(progress);
        }

        alpha.clamp(0.0, 1.0)
    }
}

/// Quadratic ease-in-out function (matches AM's EasingFunctions.easeInOutQuad).
/// easeInOutQuad: t < 0.5 ? 2*t*t : -1+(4-2*t)*t
fn ease_in_out_quad(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        2.0 * t * t
    } else {
        -1.0 + (4.0 - 2.0 * t) * t
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
    /// Frame half at spawn time, used to compute mesh scale ratio for parent-child scale inheritance
    pub spawn_frame_half: f32,
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

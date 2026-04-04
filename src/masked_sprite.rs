//! Unified Effect Material for the RTT effect system.
//!
//! This material combines basic effects (mask, wipe, stretch, blur) in a single shader pass.
//! It is part of the RTT architecture and optimized for common single-layer effect chains.
//!
//! For complex multi-pass scenarios (e.g., group effects), the RTT ping-pong buffer
//! system in `effects.rs` handles chaining multiple effect passes.

use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, BlendState, ShaderType},
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dKey},
};

/// Packed uniform data for unified effect material.
/// All effect parameters are combined into a single ShaderType struct to minimize binding count.
/// This reduces uniform buffer bindings from 23 to 1, ensuring compatibility with hardware
/// that limits uniform bindings to 15 per shader stage.
#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct UnifiedEffectUniform {
    /// Tint color (LinearRgba as vec4)
    pub color: Vec4,

    /// Effect flags: (mask1_type, wipe, stretch, blur)
    /// mask1_type: 0=disabled, 1=rect, 2=ellipse, 3=rect exclude, 4=ellipse exclude
    pub effect_flags: Vec4,

    /// Mask 1: (center_x, center_y, half_width, half_height)
    pub mask_params: Vec4,

    /// Wipe: (start, end, angle, feather)
    pub wipe_params: Vec4,

    /// Stretch: (angle_rad, stretch_px, offset_px, smooth)
    pub stretch_params: Vec4,

    /// Size: (orig_w, orig_h, mesh_w, mesh_h)
    pub original_size: Vec4,

    /// Offset: (transform_rotation_rad, 0, scene_width, scene_height)
    pub mesh_offset: Vec4,

    /// Blur: (strength, 0, 0, 0)
    pub blur_params: Vec4,

    /// Palette map flags: (enabled, count, shades, alpha)
    pub palette_flags: Vec4,

    /// Palette colors 1-8
    pub palette_color1: Vec4,
    pub palette_color2: Vec4,
    pub palette_color3: Vec4,
    pub palette_color4: Vec4,
    pub palette_color5: Vec4,
    pub palette_color6: Vec4,
    pub palette_color7: Vec4,
    pub palette_color8: Vec4,

    /// Mask 2: (center_x, center_y, half_width, half_height)
    pub mask2_params: Vec4,

    /// Mask 2 flags: (mask2_type, mask1_rotation, mask2_rotation, 0)
    pub mask2_flags: Vec4,

    /// Replace color flags: (enabled, lock_luminance, 0, 0)
    pub replace_color_flags: Vec4,

    /// Replace color: old color to replace (r, g, b, a)
    pub replace_old_color: Vec4,

    /// Replace color: new replacement color (r, g, b, a)
    pub replace_new_color: Vec4,

    /// Replace color params: (threshold, feather, alpha, 0)
    pub replace_color_params: Vec4,

    /// Repeat effect params: (count, offset_x, offset_y, angle_deg)
    pub repeat_params1: Vec4,

    /// Repeat effect params: (scale, alpha, 0, 0)
    pub repeat_params2: Vec4,

    /// Linear repeat params1: (count, position_x, position_y, angle_deg)
    pub linear_repeat_params1: Vec4,

    /// Linear repeat params2: (offset_x, offset_y, scale, alpha)
    pub linear_repeat_params2: Vec4,

    /// Linear repeat params3: (start, end, phase, overlap)
    pub linear_repeat_params3: Vec4,

    /// Linear repeat params4: (ease_in, ease_out, blend, shape_invert_alt)
    /// shape_invert_alt packs: shape*100 + invert*10 + color_alt_copies
    pub linear_repeat_params4: Vec4,

    /// Linear repeat params5: (random_order, perm_packed0/seed_lo, perm_packed1/seed_hi, stretch_before_repeat)
    /// random_order: 0.0=off, 2.0=precomputed perm, 1.0=GPU-side shuffle fallback
    pub linear_repeat_params5: Vec4,

    /// Precomputed shuffle permutation for linear repeat (indices packed 4-per-u32
    /// in 8-bit lanes, supports count ≤ 24).
    pub linear_repeat_perm: Vec4,

    /// Linear repeat source size: (source_width, source_height, layer_scale_x, layer_scale_y)
    pub linear_repeat_source_size: Vec4,

    /// Linear repeat fill color (r, g, b, a)
    pub linear_repeat_fill_color: Vec4,

    /// Second linear repeat params1: (count, position_x, position_y, angle_deg)
    pub linear_repeat2_params1: Vec4,

    /// Second linear repeat params2: (offset_x, offset_y, scale, alpha)
    pub linear_repeat2_params2: Vec4,

    /// Second linear repeat params3: (start, end, phase, overlap)
    pub linear_repeat2_params3: Vec4,

    /// Second linear repeat params4: (ease_in, ease_out, blend, shape_invert_alt)
    pub linear_repeat2_params4: Vec4,

    /// Second linear repeat params5: (random_order, perm_packed0/seed_lo, perm_packed1/seed_hi, 0)
    pub linear_repeat2_params5: Vec4,

    /// Precomputed shuffle permutation for second linear repeat.
    pub linear_repeat2_perm: Vec4,

    /// Second linear repeat fill color (r, g, b, a)
    pub linear_repeat2_fill_color: Vec4,

    /// Radial repeat params1: (count, radius, orientation_deg, startAngle_deg)
    pub radial_repeat_params1: Vec4,

    /// Radial repeat params2: (sweep_deg, baseScale, angle_deg, scale)
    pub radial_repeat_params2: Vec4,

    /// Radial repeat params3: (alpha, offset_x, offset_y, blend)
    pub radial_repeat_params3: Vec4,

    /// Radial repeat params4: (start, end, phase, overlap)
    pub radial_repeat_params4: Vec4,

    /// Radial repeat params5: (ease_in, ease_out, shape_invert_alt, seed)
    pub radial_repeat_params5: Vec4,

    /// Radial repeat params6: (pivot_x, pivot_y, 0, 0)
    pub radial_repeat_params6: Vec4,

    /// Radial repeat fill color (r, g, b, a)
    pub radial_repeat_fill_color: Vec4,

    /// Threshold effect params: (threshold, feather, invert, blendMode)
    pub threshold_params: Vec4,

    /// Grid flags: (enabled, punchout, screen_space, 0)
    pub grid_flags: Vec4,

    /// Grid params1: (pos_x, pos_y, spacing, width)
    pub grid_params1: Vec4,

    /// Grid params2: (smoothing, 0, 0, 0)
    pub grid_params2: Vec4,

    /// Grid color (r, g, b, a)
    pub grid_color: Vec4,

    /// Pixelate flags: (enabled, screen_space, 0, 0)
    pub pixelate_flags: Vec4,

    /// Pixelate params1: (size, stretch_x, stretch_y, angle)
    pub pixelate_params1: Vec4,

    /// Pixelate params2: (vignette, threshold, saturation, 0)
    pub pixelate_params2: Vec4,

    /// Mask 1 blend params: (fill_alpha, opacity, stroke_width, 0)
    pub mask_blend: Vec4,

    /// Mask 2 blend params: (fill_alpha, opacity, stroke_width, 0)
    pub mask2_blend: Vec4,

    /// Stretch2 params: (scale, angle_radians, content_only, 0)
    pub stretch2_params: Vec4,

    /// Solidcolor params: (r, g, b, blend_mode)
    pub solid_color_params: Vec4,

    /// Solidcolor alpha: (alpha, layer_scale_x, layer_scale_y, 0)
    pub solid_color_alpha: Vec4,

    /// Second stretch segment params: (angle_rad, stretch_px, offset_px, smooth)
    pub stretch_seg2_params: Vec4,

    /// Mask1 stretch-segment params: (angle_rad, adj_stretch, offset, smooth)
    pub mask1_stretch1_params: Vec4,
    /// Mask1 second stretch-segment params: (angle_rad, adj_stretch, offset, smooth)
    pub mask1_stretch2_params: Vec4,
    /// Mask1 stretch aspect info: (aspect_w, aspect_h, orig_half_w, orig_half_h)
    pub mask1_stretch_info: Vec4,

    // Wavewarp2 effect (波浪歪曲)
    /// Wavewarp2 params1: (phase, a1_rad, m1_spacing, m2_magnitude)
    pub wavewarp2_params1: Vec4,
    /// Wavewarp2 params2: (a2_rad, damping, damping_space, damping_origin)
    pub wavewarp2_params2: Vec4,
    /// Wavewarp2 flags: (screen_space, enabled, 0, 0)
    pub wavewarp2_flags: Vec4,
    /// Mirror params: (type_plus_1, blend_mode, alpha, offset)
    /// type_plus_1 = 0 → disabled, 1 → horizontal, 2 → vertical
    pub mirror_params: Vec4,
    /// Lift (copy background) params: (fill, canvas_width, canvas_height, enabled)
    pub lift_params: Vec4,
    // Rays (volumetric light rays) effect / 射线效果
    /// Rays params1: (strength, intensity, threshold, quality)
    pub rays_params1: Vec4,
    /// Rays params2: (blend, center_x_norm, center_y_norm, enabled)
    pub rays_params2: Vec4,
    /// Rays threshold color (linear RGBA)
    pub rays_threshold_color: Vec4,
    /// Rays fill color (linear RGBA)
    pub rays_fill_color: Vec4,
    // RGB split (chromatic aberration) effect / RGB 分离效果
    /// RGB split params: (offset_x, offset_y, center_channel, mode)
    pub rgb_split_params: Vec4,
    // Exposure / Gamma effect / 曝光/伽马效果
    /// Exposure/gamma params: (exposure, gamma, offset, enabled)
    pub exposure_gamma_params: Vec4,
    // Blend mode / 混合模式
    /// Blend mode params: (mode_id, canvas_w, canvas_h, enabled)
    pub blend_mode_params: Vec4,
    // ChromaKey (chroma keying) effect / 色度键效果
    /// ChromaKey params: (threshold, feather, defringe, invert)
    pub chromakey_params: Vec4,
    /// ChromaKey key color (linear RGBA)
    pub chromakey_key_color: Vec4,
    // Mask 1 linear repeat effect / 蒙版1线性重复效果
    /// Mask1 linear repeat params1: (count, position_x, position_y, angle_deg)
    pub mask1_lr_params1: Vec4,
    /// Mask1 linear repeat params2: (offset_x, offset_y, scale, alpha)
    pub mask1_lr_params2: Vec4,
    /// Mask1 linear repeat params3: (start, end, phase, overlap)
    pub mask1_lr_params3: Vec4,
    /// Mask1 linear repeat params4: (ease_in, ease_out, 0, shape_invert_alt)
    pub mask1_lr_params4: Vec4,
    /// Mask1 linear repeat params5: (random_order, seed_lo, seed_hi, 0)
    pub mask1_lr_params5: Vec4,
    // Mask 1 second linear repeat effect (dual repeat) / 蒙版1第二线性重复效果
    /// Mask1 linear repeat2 params1: (count, position_x, position_y, angle_deg)
    pub mask1_lr2_params1: Vec4,
    /// Mask1 linear repeat2 params2: (offset_x, offset_y, scale, alpha)
    pub mask1_lr2_params2: Vec4,
    /// Mask1 linear repeat2 params3: (start, end, phase, overlap)
    pub mask1_lr2_params3: Vec4,
    /// Mask1 linear repeat2 params4: (ease_in, ease_out, 0, shape_invert_alt)
    pub mask1_lr2_params4: Vec4,
    /// Mask1 linear repeat2 params5: (random_order, seed_lo, seed_hi, 0)
    pub mask1_lr2_params5: Vec4,
    /// Mask1 basic repeat params1: (count, offset_x_world, offset_y_world, angle_deg)
    pub mask1_repeat_params1: Vec4,
    /// Mask1 basic repeat params2: (scale, alpha, 0, 0)
    pub mask1_repeat_params2: Vec4,
    /// Mask1 radial repeat params1: (count, radius, orientation_deg, start_angle_deg)
    pub mask1_rr_params1: Vec4,
    /// Mask1 radial repeat params2: (sweep_deg, base_scale, angle_deg, scale)
    pub mask1_rr_params2: Vec4,
    /// Mask1 radial repeat params3: (alpha, offset_x, offset_y, 0)
    pub mask1_rr_params3: Vec4,
    /// Mask1 radial repeat params4: (start, end, phase, overlap)
    pub mask1_rr_params4: Vec4,
    /// Mask1 radial repeat params5: (ease_in, ease_out, shape_invert_alt, seed+random)
    pub mask1_rr_params5: Vec4,
    /// Source flags: (sampled_from_offscreen, premultiplied_alpha, source_kind, 0)
    pub source_flags: Vec4,
    /// Embed bounds clip: (center_x, center_y, half_width, half_height)
    /// Applied independently of mask evaluation so stretched content inside embeds
    /// is clipped even when a real mask is also active.  Disabled when half_width ≤ 0.
    pub embed_clip_params: Vec4,
    /// Embed clip rotation (radians) stored separately to keep embed_clip_params a
    /// simple AABB-like rect.  Only the z-rotation component is used.
    pub embed_clip_rotation: Vec4,
}

/// Unified material supporting mask, wipe, stretch segment, and blur effects.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
pub struct UnifiedEffectMaterial {
    /// All uniform data packed into a single binding
    #[uniform(0)]
    pub uniform_data: UnifiedEffectUniform,

    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,

    /// Lift (copy background) composite texture - background rendered to RTT
    #[texture(3)]
    #[sampler(4)]
    pub lift_comp_texture: Option<Handle<Image>>,

    /// Mask RTT texture - embedScene (group) mask rendered to texture
    #[texture(5)]
    #[sampler(6)]
    pub mask_texture: Option<Handle<Image>>,
}

// Convenience accessors for uniform-backed fields
impl UnifiedEffectMaterial {
    /// Get color
    pub fn color(&self) -> LinearRgba {
        LinearRgba::new(
            self.uniform_data.color.x,
            self.uniform_data.color.y,
            self.uniform_data.color.z,
            self.uniform_data.color.w,
        )
    }

    /// Set color
    pub fn set_color(&mut self, color: LinearRgba) {
        self.uniform_data.color = Vec4::new(color.red, color.green, color.blue, color.alpha);
    }

    pub fn effect_flags(&self) -> Vec4 {
        self.uniform_data.effect_flags
    }

    pub fn mask_params(&self) -> Vec4 {
        self.uniform_data.mask_params
    }

    pub fn wipe_params(&self) -> Vec4 {
        self.uniform_data.wipe_params
    }

    pub fn stretch_params(&self) -> Vec4 {
        self.uniform_data.stretch_params
    }

    pub fn original_size(&self) -> Vec4 {
        self.uniform_data.original_size
    }

    pub fn mesh_offset(&self) -> Vec4 {
        self.uniform_data.mesh_offset
    }

    pub fn blur_params(&self) -> Vec4 {
        self.uniform_data.blur_params
    }

    pub fn palette_flags(&self) -> Vec4 {
        self.uniform_data.palette_flags
    }

    pub fn mask2_params(&self) -> Vec4 {
        self.uniform_data.mask2_params
    }

    pub fn mask2_flags(&self) -> Vec4 {
        self.uniform_data.mask2_flags
    }

    pub fn replace_color_flags(&self) -> Vec4 {
        self.uniform_data.replace_color_flags
    }

    pub fn replace_old_color(&self) -> Vec4 {
        self.uniform_data.replace_old_color
    }

    pub fn replace_new_color(&self) -> Vec4 {
        self.uniform_data.replace_new_color
    }

    pub fn replace_color_params(&self) -> Vec4 {
        self.uniform_data.replace_color_params
    }

    pub fn set_mask_enabled(&mut self, enabled: bool) {
        self.uniform_data.set_mask_enabled(enabled);
    }

    pub fn set_wipe_enabled(&mut self, enabled: bool) {
        self.uniform_data.set_wipe_enabled(enabled);
    }

    pub fn is_stretch_enabled(&self) -> bool {
        self.uniform_data.is_stretch_enabled()
    }

    pub fn set_stretch_enabled(&mut self, enabled: bool) {
        self.uniform_data.set_stretch_enabled(enabled);
    }

    pub fn set_blur_enabled(&mut self, enabled: bool) {
        self.uniform_data.set_blur_enabled(enabled);
    }

    pub fn set_palette_enabled(&mut self, enabled: bool) {
        self.uniform_data.set_palette_enabled(enabled);
    }

    pub fn set_palette_alpha(&mut self, alpha: f32) {
        self.uniform_data.set_palette_alpha(alpha);
    }

    pub fn is_palette_enabled(&self) -> bool {
        self.uniform_data.is_palette_enabled()
    }

    pub fn set_threshold(
        &mut self,
        enabled: bool,
        threshold: f32,
        feather: f32,
        invert: bool,
        blend_mode: i32,
    ) {
        self.uniform_data
            .set_threshold(enabled, threshold, feather, invert, blend_mode);
    }

    pub fn set_grid(
        &mut self,
        enabled: bool,
        punchout: bool,
        screen_space: bool,
        pos_x: f32,
        pos_y: f32,
        spacing: f32,
        width: f32,
        smoothing: f32,
        color: Vec4,
    ) {
        self.uniform_data.set_grid(
            enabled,
            punchout,
            screen_space,
            pos_x,
            pos_y,
            spacing,
            width,
            smoothing,
            color,
        );
    }

    pub fn set_pixelate(
        &mut self,
        enabled: bool,
        screen_space: bool,
        size: f32,
        stretch_x: f32,
        stretch_y: f32,
        angle: f32,
        vignette: f32,
        threshold: f32,
        saturation: f32,
    ) {
        self.uniform_data.set_pixelate(
            enabled,
            screen_space,
            size,
            stretch_x,
            stretch_y,
            angle,
            vignette,
            threshold,
            saturation,
        );
    }

    pub fn set_replace_color(
        &mut self,
        old_color: Vec4,
        new_color: Vec4,
        threshold: f32,
        feather: f32,
        alpha: f32,
        lock_luminance: bool,
    ) {
        self.uniform_data.set_replace_color(
            old_color,
            new_color,
            threshold,
            feather,
            alpha,
            lock_luminance,
        );
    }

    pub fn set_exposure_gamma(&mut self, exposure: f32, gamma: f32, offset: f32, enabled: bool) {
        self.uniform_data
            .set_exposure_gamma(exposure, gamma, offset, enabled);
    }

    /// Set blend mode: (mode_id, canvas_w, canvas_h, enabled)
    pub fn set_blend_mode(&mut self, mode_id: f32, canvas_w: f32, canvas_h: f32) {
        self.uniform_data
            .set_blend_mode(mode_id, canvas_w, canvas_h);
    }

    /// Set chromakey params: (threshold, feather, defringe, invert) + key_color
    pub fn set_chromakey(
        &mut self,
        key_color: Vec4,
        threshold: f32,
        feather: f32,
        defringe: bool,
        invert: bool,
    ) {
        self.uniform_data
            .set_chromakey(key_color, threshold, feather, defringe, invert);
    }
}

// Setter/getter helpers and Default impl live in `uniform_setters.rs`
// to keep this file within the tokei line limit.

impl Material2d for UnifiedEffectMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/unified_effect.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
    fn specialize(
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        // Override blend state to premultiplied alpha (ONE, ONE_MINUS_SRC_ALPHA).
        // AM composites layers with premultiplied blending. This is required for
        // RGB split (chromatic aberration) where the effect outputs non-premultiplied
        // RGB with mode-specific alpha — producing additive color fringes at
        // transparent regions. The fragment shader premultiplies all non-RGB-split
        // outputs manually so other rendering is unchanged.
        if let Some(fragment) = &mut descriptor.fragment {
            for target_state in fragment.targets.iter_mut().flatten() {
                target_state.blend = Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING);
            }
        }
        Ok(())
    }
}

#[derive(Component, Default)]
pub struct UnifiedEffectMarker;

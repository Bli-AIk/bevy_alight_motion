//! Setter/getter helpers and `Default` impl for [`UnifiedEffectUniform`].
//!
//! Extracted from `masked_sprite.rs` to keep file sizes within the tokei limit.
//! These convenience methods let update functions operate on a standalone
//! `&mut UnifiedEffectUniform` without requiring a full `&mut UnifiedEffectMaterial`,
//! which avoids spurious `AssetEvent::Modified` GPU re-uploads.

use bevy::prelude::*;

use crate::masked_sprite::UnifiedEffectUniform;

impl UnifiedEffectUniform {
    pub fn set_mask_enabled(&mut self, enabled: bool) {
        self.effect_flags.x = if enabled { 1.0 } else { 0.0 };
    }

    pub fn set_wipe_enabled(&mut self, enabled: bool) {
        self.effect_flags.y = if enabled { 1.0 } else { 0.0 };
    }

    pub fn is_stretch_enabled(&self) -> bool {
        self.effect_flags.z != 0.0
    }

    pub fn set_stretch_enabled(&mut self, enabled: bool) {
        self.effect_flags.z = if enabled { 1.0 } else { 0.0 };
    }

    pub fn set_blur_enabled(&mut self, enabled: bool) {
        self.effect_flags.w = if enabled { 1.0 } else { 0.0 };
    }

    pub fn set_palette_enabled(&mut self, enabled: bool) {
        self.palette_flags.x = if enabled { 1.0 } else { 0.0 };
    }

    pub fn set_palette_alpha(&mut self, alpha: f32) {
        self.palette_flags.w = alpha;
    }

    pub fn is_palette_enabled(&self) -> bool {
        self.palette_flags.x > 0.5
    }

    pub fn set_threshold(
        &mut self,
        enabled: bool,
        threshold: f32,
        feather: f32,
        invert: bool,
        blend_mode: i32,
    ) {
        self.replace_color_flags.z = if enabled { 1.0 } else { 0.0 };
        self.threshold_params = Vec4::new(
            threshold,
            feather,
            if invert { 1.0 } else { 0.0 },
            blend_mode as f32,
        );
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
        self.grid_flags = Vec4::new(
            if enabled { 1.0 } else { 0.0 },
            if punchout { 1.0 } else { 0.0 },
            if screen_space { 1.0 } else { 0.0 },
            0.0,
        );
        self.grid_params1 = Vec4::new(pos_x, pos_y, spacing, width);
        self.grid_params2 = Vec4::new(smoothing, 0.0, 0.0, 0.0);
        self.grid_color = color;
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
        self.pixelate_flags = Vec4::new(
            if enabled { 1.0 } else { 0.0 },
            if screen_space { 1.0 } else { 0.0 },
            0.0,
            0.0,
        );
        self.pixelate_params1 = Vec4::new(size, stretch_x, stretch_y, angle);
        self.pixelate_params2 = Vec4::new(vignette, threshold, saturation, 0.0);
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
        self.replace_color_flags = Vec4::new(1.0, if lock_luminance { 1.0 } else { 0.0 }, 0.0, 0.0);
        self.replace_old_color = old_color;
        self.replace_new_color = new_color;
        self.replace_color_params = Vec4::new(threshold, feather, alpha, 0.0);
    }

    pub fn set_exposure_gamma(&mut self, exposure: f32, gamma: f32, offset: f32, enabled: bool) {
        self.exposure_gamma_params =
            Vec4::new(exposure, gamma, offset, if enabled { 1.0 } else { 0.0 });
    }

    pub fn set_blend_mode(&mut self, mode_id: f32, canvas_w: f32, canvas_h: f32) {
        self.blend_mode_params = Vec4::new(
            mode_id,
            canvas_w,
            canvas_h,
            if mode_id > 0.5 { 1.0 } else { 0.0 },
        );
    }

    pub fn set_chromakey(
        &mut self,
        key_color: Vec4,
        threshold: f32,
        feather: f32,
        defringe: bool,
        invert: bool,
    ) {
        self.chromakey_params = Vec4::new(
            threshold,
            feather,
            if defringe { 1.0 } else { 0.0 },
            if invert { 1.0 } else { 0.0 },
        );
        self.chromakey_key_color = key_color;
    }
}

impl Default for UnifiedEffectUniform {
    fn default() -> Self {
        Self {
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            effect_flags: Vec4::ZERO,
            mask_params: Vec4::new(0.0, 0.0, 10000.0, 10000.0),
            wipe_params: Vec4::new(0.0, 1.0, 0.0, 0.0),
            stretch_params: Vec4::ZERO,
            original_size: Vec4::new(100.0, 100.0, 100.0, 100.0),
            mesh_offset: Vec4::ZERO,
            blur_params: Vec4::ZERO,
            palette_flags: Vec4::ZERO,
            palette_color1: Vec4::ZERO,
            palette_color2: Vec4::ZERO,
            palette_color3: Vec4::ZERO,
            palette_color4: Vec4::ZERO,
            palette_color5: Vec4::ZERO,
            palette_color6: Vec4::ZERO,
            palette_color7: Vec4::ZERO,
            palette_color8: Vec4::ZERO,
            mask2_params: Vec4::new(0.0, 0.0, 10000.0, 10000.0),
            mask2_flags: Vec4::ZERO,
            replace_color_flags: Vec4::ZERO,
            replace_old_color: Vec4::ZERO,
            replace_new_color: Vec4::ZERO,
            replace_color_params: Vec4::ZERO,
            repeat_params1: Vec4::ZERO,
            repeat_params2: Vec4::new(1.0, 1.0, 0.0, 0.0),
            linear_repeat_params1: Vec4::ZERO,
            linear_repeat_params2: Vec4::new(0.0, 0.0, 1.0, 1.0),
            linear_repeat_params3: Vec4::new(0.0, 1.0, 0.0, 0.0),
            linear_repeat_params4: Vec4::ZERO,
            linear_repeat_params5: Vec4::ZERO,
            linear_repeat_perm: Vec4::ZERO,
            linear_repeat_source_size: Vec4::ZERO,
            linear_repeat_fill_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            linear_repeat2_params1: Vec4::new(-1.0, 0.0, 0.0, 0.0),
            linear_repeat2_params2: Vec4::new(0.0, 0.0, 1.0, 1.0),
            linear_repeat2_params3: Vec4::new(0.0, 1.0, 0.0, 0.0),
            linear_repeat2_params4: Vec4::ZERO,
            linear_repeat2_params5: Vec4::ZERO,
            linear_repeat2_perm: Vec4::ZERO,
            linear_repeat2_fill_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            radial_repeat_params1: Vec4::ZERO,
            radial_repeat_params2: Vec4::new(360.0, 1.0, 0.0, 1.0),
            radial_repeat_params3: Vec4::new(1.0, 0.0, 0.0, 0.0),
            radial_repeat_params4: Vec4::new(0.0, 1.0, 0.0, 0.0),
            radial_repeat_params5: Vec4::ZERO,
            radial_repeat_params6: Vec4::ZERO,
            radial_repeat_fill_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            threshold_params: Vec4::ZERO,
            grid_flags: Vec4::ZERO,
            grid_params1: Vec4::ZERO,
            grid_params2: Vec4::ZERO,
            grid_color: Vec4::new(0.0, 0.0, 0.0, 1.0),
            pixelate_flags: Vec4::ZERO,
            pixelate_params1: Vec4::ZERO,
            pixelate_params2: Vec4::ZERO,
            mask_blend: Vec4::ZERO,
            mask2_blend: Vec4::ZERO,
            stretch2_params: Vec4::ZERO,
            solid_color_params: Vec4::ZERO,
            solid_color_alpha: Vec4::ZERO,
            stretch_seg2_params: Vec4::ZERO,
            mask1_stretch1_params: Vec4::ZERO,
            mask1_stretch2_params: Vec4::ZERO,
            mask1_stretch_info: Vec4::ZERO,
            wavewarp2_params1: Vec4::ZERO,
            wavewarp2_params2: Vec4::ZERO,
            wavewarp2_flags: Vec4::ZERO,
            mirror_params: Vec4::ZERO,
            lift_params: Vec4::ZERO,
            rays_params1: Vec4::ZERO,
            rays_params2: Vec4::ZERO,
            rays_threshold_color: Vec4::ZERO,
            rays_fill_color: Vec4::ZERO,
            rgb_split_params: Vec4::new(0.0, 0.0, 0.0, -1.0),
            exposure_gamma_params: Vec4::ZERO,
            blend_mode_params: Vec4::ZERO,
            chromakey_params: Vec4::ZERO,
            chromakey_key_color: Vec4::ZERO,
            mask1_lr_params1: Vec4::new(-1.0, 0.0, 0.0, 0.0),
            mask1_lr_params2: Vec4::ZERO,
            mask1_lr_params3: Vec4::ZERO,
            mask1_lr_params4: Vec4::ZERO,
            mask1_lr_params5: Vec4::ZERO,
            mask1_lr2_params1: Vec4::new(-1.0, 0.0, 0.0, 0.0),
            mask1_lr2_params2: Vec4::ZERO,
            mask1_lr2_params3: Vec4::ZERO,
            mask1_lr2_params4: Vec4::ZERO,
            mask1_lr2_params5: Vec4::ZERO,
            mask1_repeat_params1: Vec4::ZERO,
            mask1_repeat_params2: Vec4::new(1.0, 1.0, 0.0, 0.0),
            mask1_rr_params1: Vec4::ZERO,
            mask1_rr_params2: Vec4::ZERO,
            mask1_rr_params3: Vec4::ZERO,
            mask1_rr_params4: Vec4::ZERO,
            mask1_rr_params5: Vec4::ZERO,
            source_flags: crate::effects::TextureSourceContract::default().to_uniform_flags(),
            embed_clip_params: Vec4::ZERO,
            embed_clip_rotation: Vec4::ZERO,
        }
    }
}

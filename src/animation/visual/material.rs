use bevy::asset::Assets;
use bevy::prelude::*;

use crate::scene::{AmMaskInfo, AmPaletteMapParams};

use super::super::visual_helpers::compute_initial_mask_params;

#[expect(clippy::too_many_arguments)] // reason: unified material creation mirrors runtime uniforms
pub(super) fn create_unified_material(
    unified_materials: &mut Assets<crate::masked_sprite::UnifiedEffectMaterial>,
    texture: Handle<Image>,
    color: LinearRgba,
    width: f32,
    height: f32,
    mask_info: &Option<AmMaskInfo>,
    wipe_params: Option<Vec4>,
    stretch_params: Option<Vec4>,
    blur_params: Option<Vec4>,
    palette_params: Option<&AmPaletteMapParams>,
    mesh_offset: Option<Vec4>,
    mesh_size: Option<(f32, f32)>,
    fit_scale: f32,
    global_time_ms: u64,
    replace_color_params: Option<(Vec4, Vec4, Vec4, Vec4)>,
) -> Handle<crate::masked_sprite::UnifiedEffectMaterial> {
    use crate::masked_sprite::{UnifiedEffectMaterial, UnifiedEffectUniform};

    let (mesh_width, mesh_height) = mesh_size.unwrap_or((width, height));

    let (initial_effect_flags_x, initial_mask_params, initial_mask2_flags_x, initial_mask2_params) =
        compute_initial_mask_params(mask_info, fit_scale, global_time_ms);

    let mut material = UnifiedEffectMaterial {
        uniform_data: UnifiedEffectUniform {
            color: Vec4::new(color.red, color.green, color.blue, color.alpha),
            effect_flags: Vec4::new(initial_effect_flags_x, 0.0, 0.0, 0.0),
            mask_params: initial_mask_params,
            original_size: Vec4::new(width, height, mesh_width, mesh_height),
            mesh_offset: mesh_offset.unwrap_or(Vec4::ZERO),
            mask2_params: initial_mask2_params,
            mask2_flags: Vec4::new(initial_mask2_flags_x, 0.0, 0.0, 0.0),
            ..default()
        },
        texture: Some(texture),
        lift_comp_texture: None,
        mask_texture: None,
    };

    if let Some(wp) = wipe_params {
        material.uniform_data.effect_flags.y = 1.0;
        material.uniform_data.wipe_params = wp;
    }

    if let Some(sp) = stretch_params {
        material.uniform_data.effect_flags.z = 1.0;
        material.uniform_data.stretch_params = sp;
    }

    if let Some(bp) = blur_params {
        material.uniform_data.effect_flags.w = 1.0;
        material.uniform_data.blur_params = bp;
    }

    if let Some(palette) = palette_params {
        material.uniform_data.palette_flags.x = 1.0;
        material.uniform_data.palette_flags.y = palette.count as f32;
        material.uniform_data.palette_flags.z = 0.0;
        material.uniform_data.palette_flags.w = palette.initial_alpha;
        material.uniform_data.palette_color1 = palette.colors[0];
        material.uniform_data.palette_color2 = palette.colors[1];
        material.uniform_data.palette_color3 = palette.colors[2];
        material.uniform_data.palette_color4 = palette.colors[3];
        material.uniform_data.palette_color5 = palette.colors[4];
        material.uniform_data.palette_color6 = palette.colors[5];
        material.uniform_data.palette_color7 = palette.colors[6];
        material.uniform_data.palette_color8 = palette.colors[7];
    }

    if let Some((flags, old_color, new_color, params)) = replace_color_params {
        material.uniform_data.replace_color_flags = flags;
        material.uniform_data.replace_old_color = old_color;
        material.uniform_data.replace_new_color = new_color;
        material.uniform_data.replace_color_params = params;
    }

    unified_materials.add(material)
}

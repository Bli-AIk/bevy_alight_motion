use bevy::prelude::*;

use crate::animation::AmAnimated;

use super::effects::{AmBlendingMode, AmMaskInfo, AmPaletteMapParams};

#[derive(Debug, Clone, Default)]
pub enum LayerFilter {
    #[default]
    None,
    AllowList(Vec<String>),
    BlockList(Vec<String>),
}

impl LayerFilter {
    pub fn should_spawn(&self, layer_name: &str) -> bool {
        match self {
            LayerFilter::None => true,
            LayerFilter::AllowList(list) => list.iter().any(|name| name == layer_name),
            LayerFilter::BlockList(list) => !list.iter().any(|name| name == layer_name),
        }
    }
}

#[derive(Component, Debug, Clone, Default)]
pub struct AmSpawnSettings {
    pub filter: LayerFilter,
}

#[derive(Component, Debug, Clone, Default)]
pub struct AmVisualSpawned;

#[derive(Component, Debug, Clone)]
#[expect(clippy::large_enum_variant)]
pub enum AmLayerSpec {
    SpriteShape {
        image_uri: String,
        is_media: bool,
        fill_color: Option<crate::schema::AmFillColor>,
        width: f32,
        height: f32,
        anchor: bevy::sprite::Anchor,
    },
    SdfShape {
        fill_color: Option<crate::schema::AmFillColor>,
        stroke_color_value: String,
        stroke_width: f32,
        stroke_join: String,
        stroke_direction: String,
        border2_color_value: String,
        border2_width: f32,
        border2_direction: String,
        width: f32,
        height: f32,
        pivot_x: f32,
        pivot_y: f32,
        shape_type: String,
        no_fill: bool,
        shape_extra: bevy::math::Vec4,
        shape_extra2: bevy::math::Vec4,
        shape_extra3: bevy::math::Vec4,
        shape_extra4: bevy::math::Vec4,
        shape_extra5: bevy::math::Vec4,
        shape_extra6: bevy::math::Vec4,
        shape_extra7: bevy::math::Vec4,
        gradient_type: u8,
        gradient_start_color: bevy::math::Vec4,
        gradient_end_color: bevy::math::Vec4,
        gradient_points: bevy::math::Vec4,
    },
    Text {
        content: String,
        font_name: String,
        font_size: f32,
        align: String,
        fill_color: Option<crate::schema::AmFillColor>,
        wrap_width: f32,
        line_height_ratio: f32,
    },
    Image {
        image_uri: String,
        width: f32,
        height: f32,
        anchor: bevy::sprite::Anchor,
    },
    Null,
    EmbedScene,
    Camera {
        fov: crate::schema::AmAnimatedFloat,
        base_z: f32,
    },
}

#[derive(Debug, Clone)]
pub struct PendingLayer {
    pub id: u64,
    pub label: String,
    pub parent: u64,
    pub start_time: i32,
    pub end_time: i32,
    pub transform: Transform,
    pub animated: AmAnimated,
    pub spec: AmLayerSpec,
    pub z_index: f32,
    pub children: Vec<PendingLayer>,
    pub blending_mode: AmBlendingMode,
    pub mask_info: Option<AmMaskInfo>,
    pub palette_params: Option<AmPaletteMapParams>,
    pub embed_scene_size: Option<(f32, f32)>,
    pub containing_embed_id: u64,
    pub from_deeply_nested_scene: bool,
    pub echo_runtime: Option<crate::animation::AmEchoRuntime>,
    pub group_fill: Option<crate::effects::AmGroupFill>,
    pub embed_requires_composite: bool,
    pub embed_dynamic_resolution: bool,
    pub embed_inner_total_time: Option<f32>,
    pub hidden: bool,
}

#[derive(Debug, Clone)]
pub struct AmSceneConfig {
    pub canvas_width: f32,
    pub canvas_height: f32,
    pub flip_y: bool,
    pub z_spacing: f32,
    pub time_offset: f32,
    pub lifecycle_offset: i32,
    pub speed_multiplier: f32,
    pub nesting_depth: u32,
    pub scene_fps: f32,
    pub scene_total_time: f32,
    pub retime: Option<crate::animation::AmRetimeInfo>,
    pub echo_time_shift_ms: f32,
    pub echo_alpha_config: Option<crate::animation::EchoAlphaConfig>,
    pub repeat_alpha_factor: f32,
    pub repeat_offset: Vec2,
    pub repeat_rotation_deg: f32,
    pub repeat_scale_factor: f32,
    pub render_fps: f32,
}

impl Default for AmSceneConfig {
    fn default() -> Self {
        Self {
            canvas_width: 1280.0,
            canvas_height: 960.0,
            flip_y: true,
            z_spacing: 0.1,
            time_offset: 0.0,
            lifecycle_offset: 0,
            speed_multiplier: 1.0,
            nesting_depth: 0,
            scene_fps: 30.0,
            scene_total_time: 0.0,
            retime: None,
            echo_time_shift_ms: 0.0,
            echo_alpha_config: None,
            repeat_alpha_factor: 1.0,
            repeat_offset: Vec2::ZERO,
            repeat_rotation_deg: 0.0,
            repeat_scale_factor: 1.0,
            render_fps: 30.0,
        }
    }
}

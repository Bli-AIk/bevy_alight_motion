//! This file builds the `AmLayerSpec` for shape layers.
//! It chooses between SDF and sprite-shape representations, derives stroke and
//! gradient information, and packages the resulting geometry metadata into the
//! spec that later spawn and animation code will use.
//!
//! 这个文件负责为形状图层构建 `AmLayerSpec`。它会在 SDF 和 sprite-shape 表示之间
//! 做选择，推导描边与渐变信息，并把最终几何元数据打包进后续生成和动画代码要使用的
//! spec 结构里。

use crate::schema::{AmShape, AmStroke};

use super::super::components::{AmLayerSpec, AmSceneConfig};
use super::super::helpers::extract_gradient_data;
use super::shape_extras::extract_shape_extras;

pub(super) fn build_shape_spec(
    shape: &AmShape,
    config: &AmSceneConfig,
    needs_sdf: bool,
    width: f32,
    height: f32,
    pivot_x: f32,
    pivot_y: f32,
    anchor: bevy::sprite::Anchor,
) -> AmLayerSpec {
    if needs_sdf {
        let default_stroke = AmStroke::default();
        let has_path_stroke = shape.stroke.is_some();
        let stroke = shape
            .stroke
            .as_ref()
            .unwrap_or_else(|| shape.borders.first().unwrap_or(&default_stroke));

        let border_scale = if has_path_stroke {
            1.0
        } else {
            let direction = shape
                .borders
                .first()
                .map(|b| b.direction.as_str())
                .unwrap_or("centered");
            if direction == "centered" {
                1.0
            } else {
                config.canvas_width / 2048.0
            }
        };

        let has_any_stroke = has_path_stroke || !shape.borders.is_empty();
        let stroke_width = if has_any_stroke {
            stroke
                .size
                .as_ref()
                .and_then(|s| {
                    s.value
                        .or_else(|| s.keyframes.first().and_then(|kf| kf.value.parse().ok()))
                })
                .unwrap_or(4.0)
                * border_scale
        } else {
            0.0
        };
        let stroke_color_value = stroke
            .color
            .as_ref()
            .map(|c| c.value.clone())
            .unwrap_or_default();
        let no_fill = shape.fill_type == "none";

        let border2 = shape.borders.get(1);
        let border2_scale = config.canvas_width / 2048.0;
        let border2_width = border2
            .and_then(|b| {
                b.size.as_ref().and_then(|s| {
                    s.value
                        .or_else(|| s.keyframes.first().and_then(|kf| kf.value.parse().ok()))
                })
            })
            .unwrap_or(0.0)
            * border2_scale;
        let border2_color_value = border2
            .and_then(|b| b.color.as_ref().map(|c| c.value.clone()))
            .unwrap_or_default();
        let border2_direction = border2.map(|b| b.direction.clone()).unwrap_or_default();

        let (
            shape_extra,
            shape_extra2,
            shape_extra3,
            shape_extra4,
            shape_extra5,
            shape_extra6,
            shape_extra7,
        ) = extract_shape_extras(
            &shape.shape_type,
            &shape.properties,
            shape
                .path_element
                .as_ref()
                .map(|p| p.d.as_str())
                .unwrap_or(""),
        );

        let (gradient_type, gradient_start_color, gradient_end_color, gradient_points) =
            extract_gradient_data(&shape.gradient);

        AmLayerSpec::SdfShape {
            fill_color: shape.fill_color.clone(),
            stroke_color_value,
            stroke_width,
            stroke_join: stroke.join.clone(),
            stroke_direction: stroke.direction.clone(),
            border2_color_value,
            border2_width,
            border2_direction,
            width,
            height,
            pivot_x,
            pivot_y,
            shape_type: shape.shape_type.clone(),
            no_fill,
            shape_extra,
            shape_extra2,
            shape_extra3,
            shape_extra4,
            shape_extra5,
            shape_extra6,
            shape_extra7,
            gradient_type,
            gradient_start_color,
            gradient_end_color,
            gradient_points,
        }
    } else if !shape.fill_image.is_empty()
        && (shape.fill_type == "media" || shape.fill_type == "color")
    {
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
    }
}

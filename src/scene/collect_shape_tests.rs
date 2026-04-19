//! Regression tests for shape collection edge cases.
//! 覆盖 shape 收集边界行为的回归测试。
//!
//! These tests focus on shape-specific collection rules such as honoring fill-mode priority even
//! when a stale fill-image reference is present. They protect the translation from
//! imported XML shape data into `AmLayerSpec::SpriteShape`.
//! 这些测试专门覆盖 shape 收集阶段的边界规则，例如在作者仍保留 fill-image 引用时，是否仍然以
//! fill mode 为准，而不是被残留 sprite 误导。
//! 引用。它们用于保护从导入 XML shape 数据到 `AmLayerSpec::SpriteShape` 的这段转换逻辑。

use super::collect_shape::collect_shape;
use crate::scene::{AmLayerSpec, AmSceneConfig};
use crate::schema::{AmProperty, AmShape};

fn rect_shape(fill_type: &str, fill_image: &str) -> AmShape {
    AmShape {
        id: 42,
        label: "rect".to_string(),
        start_time: 0,
        end_time: 1000,
        parent: 0,
        fill_type: fill_type.to_string(),
        fill_image: fill_image.to_string(),
        shape_type: ".rect".to_string(),
        blending: String::new(),
        hidden: false,
        speed: 1.0,
        transform: Default::default(),
        properties: vec![AmProperty {
            name: "size".to_string(),
            prop_type: "vec2".to_string(),
            value: "10,20".to_string(),
            keyframes: Vec::new(),
        }],
        effects: Vec::new(),
        fill_color: None,
        stroke: None,
        borders: Vec::new(),
        gradient: None,
        path_element: None,
    }
}

#[test]
fn collect_shape_color_fill_ignores_stale_fill_image() {
    let mut shape = rect_shape("color", "amproj:spr_s_boneloop_0.png");
    shape.fill_color = Some(crate::schema::AmFillColor {
        value: "#ff3d4cf5".to_string(),
        keyframes: Vec::new(),
    });

    let pending =
        collect_shape(&shape, &AmSceneConfig::default(), 0.0).expect("shape should collect");

    match pending.spec {
        AmLayerSpec::SpriteShape {
            image_uri,
            is_media,
            fill_color,
            width,
            height,
            ..
        } => {
            assert!(!is_media);
            assert_eq!(image_uri, "");
            assert!(fill_color.is_some());
            assert_eq!((width, height), (20.0, 40.0));
        }
        other => panic!("expected SpriteShape, got {other:?}"),
    }
}

#[test]
fn collect_shape_media_fill_keeps_fill_image() {
    let shape = rect_shape("media", "amproj:spr_s_boneloop_0.png");

    let pending =
        collect_shape(&shape, &AmSceneConfig::default(), 0.0).expect("shape should collect");

    match pending.spec {
        AmLayerSpec::SpriteShape {
            image_uri,
            is_media,
            fill_color,
            ..
        } => {
            assert!(is_media);
            assert_eq!(image_uri, "amproj:spr_s_boneloop_0.png");
            assert!(fill_color.is_none());
        }
        other => panic!("expected SpriteShape, got {other:?}"),
    }
}

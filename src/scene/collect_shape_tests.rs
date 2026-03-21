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
fn collect_shape_preserves_fill_image_for_color_fill() {
    let shape = rect_shape("color", "amproj:spr_s_boneloop_0.png");

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
            assert!(is_media);
            assert_eq!(image_uri, "amproj:spr_s_boneloop_0.png");
            assert!(fill_color.is_none());
            assert_eq!((width, height), (20.0, 40.0));
        }
        other => panic!("expected SpriteShape, got {other:?}"),
    }
}

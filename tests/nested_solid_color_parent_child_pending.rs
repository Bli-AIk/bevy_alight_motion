mod common;

use bevy::prelude::Handle;
use bevy_alight_motion::loader::FontMetrics;
use bevy_alight_motion::scene::{AmLayerSpec, AmSceneConfig, collect_pending_layers};
use bevy_alight_motion::schema::AmScene;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read};
use std::path::Path;

fn load_scene_from_amproj(path: impl AsRef<Path>) -> AmScene {
    let bytes = std::fs::read(path).expect("failed to read amproj");
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).expect("failed to open amproj zip");

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).expect("zip entry");
        if !file.name().ends_with(".xml") {
            continue;
        }

        let mut xml = String::new();
        file.read_to_string(&mut xml).expect("read xml");
        return quick_xml::de::from_str(&xml).expect("parse xml");
    }

    panic!("no xml found in amproj");
}

#[test]
fn nested_solid_color_parent_child_pending_ids_remain_unique() {
    let scene = load_scene_from_amproj(common::fixture_path(
        "assets/projects/basic/fill/nested_solid_color_parent_child.amproj",
    ));
    let config = AmSceneConfig {
        canvas_width: scene.width as f32,
        canvas_height: scene.height as f32,
        scene_fps: scene.fps as f32,
        scene_total_time: scene.total_time as f32,
        render_fps: scene.fps as f32,
        ..Default::default()
    };

    let fonts: HashMap<String, Handle<bevy::text::Font>> = HashMap::new();
    let font_metrics: HashMap<String, FontMetrics> = HashMap::new();
    let pending = collect_pending_layers(&scene, &fonts, &font_metrics, &config);

    let mut unique_ids = HashSet::new();
    let duplicate_ids: Vec<u64> = pending
        .iter()
        .filter_map(|layer| (!unique_ids.insert(layer.id)).then_some(layer.id))
        .collect();

    assert!(
        duplicate_ids.is_empty(),
        "flattened pending layer ids must be unique, duplicates: {duplicate_ids:?}"
    );

    let embeds: Vec<_> = pending
        .iter()
        .filter(|layer| matches!(layer.spec, AmLayerSpec::EmbedScene))
        .map(|layer| (layer.id, layer.label.as_str()))
        .collect();

    let left_embed_id = embeds
        .iter()
        .find(|(_, label)| *label == "编组 3")
        .map(|(id, _)| *id)
        .expect("left embed id");
    let right_embed_id = embeds
        .iter()
        .find(|(_, label)| *label == "编组 3 Copy")
        .map(|(id, _)| *id)
        .expect("right embed id");

    let inner_shapes: Vec<_> = pending
        .iter()
        .filter(|layer| layer.label == "spr_s_boneloop_0.png Copy")
        .map(|layer| {
            (
                layer.id,
                layer.parent,
                layer.containing_embed_id,
                layer.animated.parent_layer_id,
                layer.animated.has_parent,
            )
        })
        .collect();

    assert_eq!(
        inner_shapes.len(),
        2,
        "expected two flattened inner shapes for the sibling embeds"
    );
    assert_ne!(
        inner_shapes[0].0, inner_shapes[1].0,
        "sibling embed children must not share the same flattened runtime id"
    );

    let left_shape = inner_shapes
        .iter()
        .find(|(_, parent, containing_embed_id, ..)| {
            *parent == left_embed_id || *containing_embed_id == left_embed_id
        })
        .expect("left inner shape");
    let right_shape = inner_shapes
        .iter()
        .find(|(_, parent, containing_embed_id, ..)| {
            *parent == right_embed_id || *containing_embed_id == right_embed_id
        })
        .expect("right inner shape");

    assert_eq!(left_shape.2, left_embed_id);
    assert_eq!(right_shape.2, right_embed_id);
}

use bevy::prelude::Handle;
use bevy_alight_motion::loader::FontMetrics;
use bevy_alight_motion::scene::{AmLayerSpec, AmSceneConfig, collect_pending_layers};
use bevy_alight_motion::schema::AmScene;
use std::collections::HashMap;
use std::io::{Cursor, Read};

fn load_scene_from_amproj(path: &str) -> AmScene {
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
fn inspect_mortis_split1_nested_embed_pending_layers() {
    let scene = load_scene_from_amproj("assets/projects/private/USER_mortis/revenge/split1.amproj");
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

    let interesting: Vec<_> = pending
        .iter()
        .filter(|layer| {
            matches!(layer.label.as_str(), "编组 4" | "编组 3 Copy" | "长方形 2")
                || layer
                    .label
                    .starts_with("Untitled 05-18-2024 05-46-18 (24).png")
        })
        .map(|layer| {
            (
                layer.id,
                layer.label.clone(),
                layer.parent,
                layer.containing_embed_id,
                layer.start_time,
                layer.end_time,
                matches!(layer.spec, AmLayerSpec::EmbedScene),
            )
        })
        .collect::<Vec<_>>();

    assert!(
        interesting
            .iter()
            .any(|(_, label, ..)| label == "编组 3 Copy"),
        "expected split1 to contain 编组 3 Copy in collected pending layers"
    );

    println!("\n=== split1 interesting pending layers ===");
    for (id, label, parent, containing_embed_id, start_time, end_time, is_embed) in &interesting {
        println!(
            "id={id} label='{label}' parent={parent} containing_embed_id={containing_embed_id} time={start_time}..{end_time} embed={is_embed}"
        );
    }

    let nested_embed_id = interesting
        .iter()
        .find(|(_, label, ..)| label == "编组 3 Copy")
        .map(|(id, ..)| *id)
        .expect("nested embed id");

    let nested_image = interesting
        .iter()
        .find(|(_, label, _parent, containing_embed_id, start_time, ..)| {
            label == "Untitled 05-18-2024 05-46-18 (24).png"
                && *start_time == 267
                && *containing_embed_id == nested_embed_id
        })
        .expect("nested cyan image under 编组 3 Copy");

    assert_eq!(
        nested_image.3, nested_embed_id,
        "nested image should stay owned by 编组 3 Copy after flatten"
    );
}

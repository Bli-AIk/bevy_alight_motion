//! Integration tests against real Alight Motion project archives.
//! 针对真实 Alight Motion 工程归档的集成测试。
//!
//! These tests open actual `.amproj` files, inspect parsed scene structure, and print hierarchy data
//! for debugging. They exist to catch importer regressions that only appear when the full archive
//! format, XML parser, and schema model interact together.
//! 这些测试会打开真实的 `.amproj` 文件，检查解析后的 scene 结构，并输出层级信息辅助调试。
//! 它们用于捕获那些只有在完整归档格式、XML 解析器和 schema 模型共同作用时才会暴露出来的导入回归。

mod common;

use std::io::{Cursor, Read};

#[test]
fn test_6ex_layer_hierarchy() {
    let amproj_path = common::fixture_path("assets/projects/private/USER_chen_pi/6_ex.amproj");

    // Skip if file doesn't exist
    if !amproj_path.exists() {
        eprintln!(
            "Skipping test: amproj file not found at {}",
            amproj_path.display()
        );
        return;
    }

    let bytes = std::fs::read(&amproj_path).expect("Failed to read amproj");
    let cursor = Cursor::new(&bytes);
    let mut archive = zip::ZipArchive::new(cursor).expect("Failed to open ZIP");

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().to_string();

        if !name.ends_with(".xml") {
            continue;
        }

        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();

        let scene: bevy_alight_motion::schema::AmScene =
            quick_xml::de::from_str(&content).expect("Failed to parse XML");

        println!("\n=== 6_ex Layer Structure ===\n");
        println!("Canvas: {}x{}", scene.width, scene.height);

        // Find all nullobj layers (空)
        for layer in &scene.layers {
            if let bevy_alight_motion::schema::AmLayer::Nullobj(null) = layer {
                let rot_val = null.transform.rotation.value.unwrap_or(0.0);
                let loc_val = null.transform.location.value.unwrap_or_default();
                let loc_kf_count = null.transform.location.keyframes.len();
                println!(
                    "NULLOBJ: id={}, label='{}', parent={}, rot={:.1}°, loc={:?} ({} keyframes)",
                    null.id, null.label, null.parent, rot_val, loc_val, loc_kf_count
                );
            } else if let bevy_alight_motion::schema::AmLayer::Shape(shape) = layer
                && shape.label.contains("Image_")
            {
                let rot_val = shape.transform.rotation.value.unwrap_or(0.0);
                println!(
                    "IMAGE: id={}, label='{}', parent={}, rot={:.1}°",
                    shape.id, shape.label, shape.parent, rot_val
                );
            }
        }

        // Verify that 空2 layers have 90 degree rotation
        for layer in &scene.layers {
            if let bevy_alight_motion::schema::AmLayer::Nullobj(null) = layer
                && null.label.contains("空 2")
            {
                let rot = null.transform.rotation.value.unwrap_or(0.0);
                assert!(
                    (rot - 90.0).abs() < 0.01,
                    "空2 should have 90° rotation, got {}°",
                    rot
                );
                println!("\n✓ Verified: '{}' has correct 90° rotation", null.label);
            }
        }
    }
}

#[test]
fn test_parse_real_amproj() {
    let amproj_path =
        std::path::Path::new("/home/aik/Downloads/am/新项目 24 20260105_182938.amproj");

    // Skip if file doesn't exist
    if !amproj_path.exists() {
        eprintln!("Skipping test: amproj file not found");
        return;
    }

    let bytes = std::fs::read(amproj_path).expect("Failed to read amproj");
    let cursor = Cursor::new(&bytes);
    let mut archive = zip::ZipArchive::new(cursor).expect("Failed to open ZIP");

    let mut xml_found = false;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().to_string();

        if name.ends_with(".xml") {
            xml_found = true;
            let mut content = String::new();
            file.read_to_string(&mut content).unwrap();

            let scene: bevy_alight_motion::schema::AmScene =
                quick_xml::de::from_str(&content).expect("Failed to parse XML");

            assert_eq!(scene.title, "新项目 24");
            assert_eq!(scene.width, 1280);
            assert_eq!(scene.height, 960);
            assert_eq!(scene.fps, 60);
            assert!(!scene.media.is_empty(), "Should have media");
            assert!(!scene.layers.is_empty(), "Should have layers");

            // Check media URIs
            for media in &scene.media {
                assert!(
                    media.uri.starts_with("amproj:"),
                    "Media URI should start with amproj:"
                );
            }
        }
    }

    assert!(xml_found, "Should find XML in amproj");
}

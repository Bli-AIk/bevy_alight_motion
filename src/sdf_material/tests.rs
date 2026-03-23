use super::*;

const SDF_MATERIAL_UNIFORM_MIN_SIZE_BYTES: u64 = 592;

#[test]
fn test_pack_color() {
    let white = Color::WHITE;
    let packed = pack_color(white);
    let bits = packed.to_bits();
    assert!((bits >> 24) >= 254, "R should be ~255, got {}", bits >> 24);
    assert!(((bits >> 16) & 0xFF) >= 254, "G should be ~255");
    assert!(((bits >> 8) & 0xFF) >= 254, "B should be ~255");
    assert!((bits & 0xFF) >= 254, "A should be ~255");

    let red = Color::srgba(1.0, 0.0, 0.0, 1.0);
    let packed = pack_color(red);
    let bits = packed.to_bits();
    assert!((bits >> 24) >= 254, "R should be ~255");
    assert_eq!((bits >> 16) & 0xFF, 0);
    assert_eq!((bits >> 8) & 0xFF, 0);
    assert!((bits & 0xFF) >= 254, "A should be ~255");
}

#[test]
fn test_sdf_uniform_size() {
    use bevy::render::render_resource::ShaderType;
    let size = SdfMaterialUniform::min_size();
    println!("SdfMaterialUniform min_size = {}", size);
    assert_eq!(
        size.get(),
        SDF_MATERIAL_UNIFORM_MIN_SIZE_BYTES,
        "SdfMaterialUniform size mismatch! Expected {SDF_MATERIAL_UNIFORM_MIN_SIZE_BYTES} bytes"
    );
}

#[test]
fn test_repack_with_alpha() {
    let white = Color::WHITE;
    let packed = pack_color(white);
    let repacked = repack_with_alpha(packed, 0.5);
    let bits = repacked.to_bits();
    assert!((bits >> 24) >= 254, "R should be ~255");
    assert!(((bits >> 16) & 0xFF) >= 254, "G should be ~255");
    assert!(((bits >> 8) & 0xFF) >= 254, "B should be ~255");
    assert!(
        (bits & 0xFF) >= 126 && (bits & 0xFF) <= 129,
        "A should be ~127, got {}",
        bits & 0xFF
    );
}

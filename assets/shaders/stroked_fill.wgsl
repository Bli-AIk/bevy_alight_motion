// Stroked fill shader for AM shapes
// This shader renders both fill and stroke in a single pass
// 
// params.x = half_width of the base shape (100x100 base, so typically 50)
// params.y = half_height of the base shape (100x100 base, so typically 50)
// params.z = stroke_width
// params.w = encoded stroke color (packed RGBA)
//
// The shape is rendered with:
// - Fill color from input.color when distance < 0
// - Stroke color when 0 <= distance < stroke_width
// - Transparent when distance >= stroke_width

// Unpack RGBA color from a float (each channel 0-255 packed into u32 bits)
fn unpack_color(packed: f32) -> vec4<f32> {
    let bits = bitcast<u32>(packed);
    let r = f32((bits >> 24u) & 0xFFu) / 255.0;
    let g = f32((bits >> 16u) & 0xFFu) / 255.0;
    let b = f32((bits >> 8u) & 0xFFu) / 255.0;
    let a = f32(bits & 0xFFu) / 255.0;
    return vec4<f32>(r, g, b, a);
}

// Main fill function
// input.distance: SDF distance value
// input.color: Fill color
// input.params: Shape parameters (half_width, half_height, stroke_width, packed_stroke_color)
let stroke_width = input.params.z;
let stroke_color = unpack_color(input.params.w);

// Inside fill region
if input.distance < 0.0 {
    return input.color;
}

// Inside stroke region (0 to stroke_width)
if input.distance < stroke_width {
    // Smooth edge at the outer boundary of stroke
    let edge_smoothness = 1.0; // pixels for anti-aliasing
    let alpha = 1.0 - smoothstep(stroke_width - edge_smoothness, stroke_width, input.distance);
    return vec4<f32>(stroke_color.rgb, stroke_color.a * alpha);
}

// Outside - transparent
return vec4<f32>(0.0, 0.0, 0.0, 0.0);

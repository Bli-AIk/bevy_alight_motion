// Stroked fill shader for Box (Miter/Square corners)
// Uses Chebyshev distance (L-infinity norm) for sharp corners

let stroke_width = input.params.z;
let half_width = input.params.x;
let half_height = input.params.y;

// Calculate signed distance for sharp box
// d = max(|x| - w, |y| - h)
let d = abs(input.pos) - vec2<f32>(half_width, half_height);
let dist = max(d.x, d.y);

// Unpack stroke color from params.w
let stroke_bits = bitcast<u32>(input.params.w);
let stroke_r = f32((stroke_bits >> 24u) & 0xFFu) / 255.0;
let stroke_g = f32((stroke_bits >> 16u) & 0xFFu) / 255.0;
let stroke_b = f32((stroke_bits >> 8u) & 0xFFu) / 255.0;
let stroke_a = f32(stroke_bits & 0xFFu) / 255.0;
let stroke_color = vec4<f32>(stroke_r, stroke_g, stroke_b, stroke_a);

// Stroke logic (Centered)
// Stroke band covers distance [-half_stroke, +half_stroke]
let half_stroke = stroke_width * 0.5;
let dist_from_center_line = abs(dist);
let stroke_alpha = 1.0 - smoothstep(half_stroke - 0.5, half_stroke + 0.5, dist_from_center_line);
let stroke_col = vec4<f32>(stroke_color.rgb, stroke_color.a * stroke_alpha);

// Fill logic
// Fill exists when dist < 0
let fill_alpha = 1.0 - smoothstep(-0.5, 0.5, dist);
let fill_col = vec4<f32>(input.color.rgb, input.color.a * fill_alpha);

// Composite: Stroke Over Fill
let out_a = stroke_col.a + fill_col.a * (1.0 - stroke_col.a);

if (out_a <= 0.0) {
    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}

let out_rgb = (stroke_col.rgb * stroke_col.a + fill_col.rgb * fill_col.a * (1.0 - stroke_col.a)) / out_a;

return vec4<f32>(out_rgb, out_a);

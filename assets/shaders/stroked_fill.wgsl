// Stroked fill shader for AM shapes (Square corners version)
// This shader renders both fill and stroke in a single pass with SHARP corners
// 
// == Params ==
// params.x = half_width of the shape
// params.y = half_height of the shape
// params.z = stroke_width
// params.w = encoded stroke color (packed RGBA as u32 bits in f32)
//
// == Position ==
// input.pos = pixel position relative to shape center
//
// == How it works ==
// For sharp corners, we use Chebyshev distance (L∞ norm) instead of Euclidean.
// Chebyshev distance produces rectangular iso-distance curves.
//
// Standard SDF: distance = sqrt(dx² + dy²) → rounded corners
// Chebyshev:    distance = max(|dx|, |dy|) → sharp corners

// ============================================
// ADJUSTMENT VARIABLES - Modify these to tune
// ============================================
let offset_x = 0.0;        // Horizontal offset (positive = right)
let offset_y = 34.0;        // Vertical offset (positive = up)
let size_scale_x = 0.99;    // Width multiplier (1.0 = no change)
let size_scale_y = 0.96;    // Height multiplier (1.0 = no change)
let stroke_offset = 1.5;   // Stroke width adjustment (positive = thicker)
// ============================================

let half_width = input.params.x * size_scale_x;
let half_height = input.params.y * size_scale_y;
let stroke_width = input.params.z + stroke_offset;

// Apply position offset
let pos_x = input.pos.x - offset_x;
let pos_y = input.pos.y - offset_y;

// Unpack stroke color from params.w
let stroke_bits = bitcast<u32>(input.params.w);
let stroke_r = f32((stroke_bits >> 24u) & 0xFFu) / 255.0;
let stroke_g = f32((stroke_bits >> 16u) & 0xFFu) / 255.0;
let stroke_b = f32((stroke_bits >> 8u) & 0xFFu) / 255.0;
let stroke_a = f32(stroke_bits & 0xFFu) / 255.0;
let stroke_color = vec4<f32>(stroke_r, stroke_g, stroke_b, stroke_a);

// Calculate signed distance to box edges (using offset position)
let dx = abs(pos_x) - half_width;
let dy = abs(pos_y) - half_height;

// Inside fill region: both dx and dy are negative
let inside_fill = dx < 0.0 && dy < 0.0;

// Chebyshev distance for sharp-cornered stroke
let chebyshev_dist = max(dx, dy);

// Inside fill region
if inside_fill {
    return input.color;
}

// Inside stroke region (using Chebyshev distance)
if chebyshev_dist < stroke_width {
    // Anti-alias only the outer edge
    let edge_smoothness = 1.0;
    let alpha = 1.0 - smoothstep(stroke_width - edge_smoothness, stroke_width, chebyshev_dist);
    return vec4<f32>(stroke_color.rgb, stroke_color.a * alpha);
}

// Outside - transparent
return vec4<f32>(0.0, 0.0, 0.0, 0.0);

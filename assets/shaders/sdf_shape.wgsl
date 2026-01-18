// SDF Shape Shader - Custom implementation replacing bevy_smud
//
// Renders rectangles (round/miter/bevel corners) and circles/ellipses
// with optional strokes using signed distance field techniques.
//
// Uniforms (in SdfMaterialUniform struct):
// - color: Fill color (vec4<f32>)
// - params: (half_width, half_height, stroke_width, packed_stroke_color)
// - mask_params: (mask_center_x, mask_center_y, mask_half_width, mask_half_height)
// - shape_type: 0=BoxRound, 1=BoxMiter, 2=BoxBevel, 3=Circle
// - mask_type: 0=disabled, 1=rectangle, 2=ellipse
// - _padding: alignment padding

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct SdfMaterialUniform {
    color: vec4<f32>,
    params: vec4<f32>,
    mask_params: vec4<f32>,
    shape_type: f32,
    mask_type: f32,
    _padding2: f32,
    _padding3: f32,
};

@group(2) @binding(0) var<uniform> material: SdfMaterialUniform;

// SDF for rounded box (Euclidean distance)
fn sd_box(p: vec2<f32>, half_size: vec2<f32>) -> f32 {
    let d = abs(p) - half_size;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0);
}

// SDF for sharp box with miter corners (Chebyshev/L-infinity)
fn sd_box_miter(p: vec2<f32>, half_size: vec2<f32>) -> f32 {
    let d = abs(p) - half_size;
    return max(d.x, d.y);
}

// SDF for box with bevel corners
fn sd_box_bevel(p: vec2<f32>, half_size: vec2<f32>) -> f32 {
    let d = abs(p) - half_size;
    let dist_miter = max(d.x, d.y);
    let dist_bevel = d.x + d.y;
    return max(dist_miter, dist_bevel);
}

// SDF for circle
fn sd_circle(p: vec2<f32>, r: f32) -> f32 {
    return length(p) - r;
}

// SDF for ellipse (approximate)
fn sd_ellipse(p: vec2<f32>, a: f32, b: f32) -> f32 {
    // Normalize to unit circle space, compute distance, scale back
    let q = p / vec2<f32>(a, b);
    let d = length(q) - 1.0;
    // Scale distance back (approximate)
    let scale = min(a, b);
    return d * scale;
}

// Unpack RGBA from u32 bits stored in f32
fn unpack_color(packed: f32) -> vec4<f32> {
    let bits = bitcast<u32>(packed);
    let r_srgb = f32((bits >> 24u) & 0xFFu) / 255.0;
    let g_srgb = f32((bits >> 16u) & 0xFFu) / 255.0;
    let b_srgb = f32((bits >> 8u) & 0xFFu) / 255.0;
    let a = f32(bits & 0xFFu) / 255.0;
    
    // Convert sRGB to linear (gamma 2.2 approximation)
    let r = pow(r_srgb, 2.2);
    let g = pow(g_srgb, 2.2);
    let b = pow(b_srgb, 2.2);
    
    return vec4<f32>(r, g, b, a);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Check mask first - use world position for mask testing
    let mask_type = material.mask_type;
    if mask_type > 0.5 {
        let mask_center = material.mask_params.xy;
        let mask_half_size = material.mask_params.zw;
        
        // Only apply mask if half_size is reasonable (< 5000)
        if mask_half_size.x < 5000.0 {
            let world_pos = in.world_position.xy;
            let rel_pos = world_pos - mask_center;
            
            // Ellipse mask (mask_type >= 1.5)
            if mask_type > 1.5 {
                let normalized = rel_pos / mask_half_size;
                if dot(normalized, normalized) > 1.0 {
                    discard;
                }
            } else {
                // Rectangle mask
                if abs(rel_pos.x) > mask_half_size.x || abs(rel_pos.y) > mask_half_size.y {
                    discard;
                }
            }
        }
    }

    let half_width = material.params.x;
    let half_height = material.params.y;
    let stroke_width = material.params.z;
    let packed_stroke = material.params.w;
    
    // The mesh is created with size = frame_size x frame_size
    // where frame_size = max(half_width, half_height) * 2.0 * max_scale_factor + stroke_width * 2.0
    // UV (0,0) is bottom-left, (1,1) is top-right
    // We need position relative to center
    
    // Calculate the actual frame size that was used to create the mesh
    // Note: max_scale_factor is 10.0 in spawn_sdf_visual
    let max_scale_factor = 10.0;
    let frame_half = max(half_width, half_height) * max_scale_factor + stroke_width * 2.0;
    let frame_size = frame_half * 2.0;
    
    // Convert UV to local coordinates centered at origin
    let pos = (in.uv - 0.5) * frame_size;
    
    // Calculate SDF based on shape type
    var dist: f32;
    let shape_type = i32(material.shape_type);
    
    if shape_type == 0 {
        // BoxRound
        dist = sd_box(pos, vec2<f32>(half_width, half_height));
    } else if shape_type == 1 {
        // BoxMiter
        dist = sd_box_miter(pos, vec2<f32>(half_width, half_height));
    } else if shape_type == 2 {
        // BoxBevel
        dist = sd_box_bevel(pos, vec2<f32>(half_width, half_height));
    } else {
        // Circle/Ellipse
        if abs(half_width - half_height) < 0.001 {
            dist = sd_circle(pos, half_width);
        } else {
            dist = sd_ellipse(pos, half_width, half_height);
        }
    }
    
    // Anti-aliasing width based on screen-space derivatives
    let aa_width = fwidth(dist);
    let safe_aa_width = clamp(aa_width, 0.5, 10.0);
    
    // Fill: inside the shape (dist < 0)
    let fill_alpha = 1.0 - smoothstep(-safe_aa_width, safe_aa_width, dist);
    let fill_col = vec4<f32>(material.color.rgb, material.color.a * fill_alpha);
    
    // Handle stroke if stroke_width > 0
    if stroke_width > 0.0 {
        let stroke_color = unpack_color(packed_stroke);
        
        // Centered stroke: distance band around the edge
        let half_stroke = stroke_width * 0.5;
        let dist_from_edge = abs(dist);
        let stroke_alpha = 1.0 - smoothstep(half_stroke - safe_aa_width, half_stroke + safe_aa_width, dist_from_edge);
        let stroke_col = vec4<f32>(stroke_color.rgb, stroke_color.a * stroke_alpha);
        
        // Composite: stroke over fill
        let out_a = stroke_col.a + fill_col.a * (1.0 - stroke_col.a);
        
        if out_a <= 0.0 {
            discard;
        }
        
        let out_rgb = (stroke_col.rgb * stroke_col.a + fill_col.rgb * fill_col.a * (1.0 - stroke_col.a)) / out_a;
        return vec4<f32>(out_rgb, out_a);
    } else {
        // No stroke, just fill
        if fill_col.a <= 0.0 {
            discard;
        }
        return fill_col;
    }
}

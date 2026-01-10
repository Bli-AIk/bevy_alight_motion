// Stretch segment shader - UV domain distortion effect
//
// This shader implements the "拉伸片段" (Stretch Segment) effect from Alight Motion.
// It creates a horizontal split through the center and pushes the halves apart.
//
// Current implementation: angle=0 only (vertical split line, horizontal stretch)
// - The image is split vertically at the center
// - Left and right halves are pushed apart horizontally
// - The gap is filled with the center column pixels
//
// Stretch formula: stretch=135 → width doubles (1 + stretch/50 factor applied by CPU)
//
// Uniform 0: color (vec4<f32>) - tint color
// Uniform 1: stretch_params (vec4<f32>) - (angle_radians, stretch_px, offset_uv, smooth_width)
// Uniform 2: original_size (vec4<f32>) - (original_width, original_height, 0, 0)

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var<uniform> color: vec4<f32>;
@group(2) @binding(1) var<uniform> stretch_params: vec4<f32>;
@group(2) @binding(2) var<uniform> original_size: vec4<f32>;
@group(2) @binding(3) var base_texture: texture_2d<f32>;
@group(2) @binding(4) var base_sampler: sampler;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // Extract parameters (angle is currently unused - always horizontal stretch)
    let angle = stretch_params.x;         // Reserved for future angle support
    let stretch_px = stretch_params.y;    // Actual stretch amount in pixels
    let offset = stretch_params.z;        // Reserved for future offset support
    let smooth_width = stretch_params.w;  // Reserved for future smooth support
    
    let orig_width = original_size.x;
    let orig_height = original_size.y;
    
    // Input UV [0, 1] covers the EXPANDED mesh
    let uv = mesh.uv;
    
    // New mesh width after expansion
    let new_width = orig_width + stretch_px;
    
    // Calculate the boundaries in UV space (relative to expanded mesh)
    let gap_ratio = stretch_px / new_width;
    let half_gap = gap_ratio * 0.5;
    
    // UV boundaries:
    // Left region:   [0, 0.5 - half_gap]
    // Center gap:    [0.5 - half_gap, 0.5 + half_gap]
    // Right region:  [0.5 + half_gap, 1.0]
    let gap_left_uv = 0.5 - half_gap;
    let gap_right_uv = 0.5 + half_gap;
    
    // Remap UV to sample from original texture
    var sample_uv_x: f32;
    
    if uv.x <= gap_left_uv {
        // Left region: [0, gap_left_uv] -> [0, 0.5]
        sample_uv_x = (uv.x / gap_left_uv) * 0.5;
    } else if uv.x >= gap_right_uv {
        // Right region: [gap_right_uv, 1.0] -> [0.5, 1.0]
        let right_uv = (uv.x - gap_right_uv) / (1.0 - gap_right_uv);
        sample_uv_x = 0.5 + right_uv * 0.5;
    } else {
        // Center gap region: sample the center line of original texture
        sample_uv_x = 0.5;
    }
    
    // Clamp to valid UV range
    sample_uv_x = clamp(sample_uv_x, 0.0, 1.0);
    
    // Sample the texture
    let sample_uv = vec2<f32>(sample_uv_x, uv.y);
    let tex_color = textureSample(base_texture, base_sampler, sample_uv);
    
    // Apply color tint
    var final_color = tex_color * color;
    
    // Discard fully transparent pixels
    if final_color.a < 0.001 {
        discard;
    }
    
    return final_color;
}

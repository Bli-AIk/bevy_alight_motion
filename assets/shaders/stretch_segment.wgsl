// Stretch segment shader - UV domain distortion effect
//
// This shader implements the "拉伸片段" (Stretch Segment) effect from Alight Motion.
// It creates a split line at a configurable angle and pushes the halves apart.
//
// Implementation approach (pixel space rotation):
// 1. Convert UV to pixel coordinates (using EXPANDED mesh dimensions)
// 2. Rotate coordinates to align split line vertically
// 3. Apply horizontal stretch logic in pixel space
// 4. Rotate back
// 5. Convert back to UV
//
// Key insight: We must use the EXPANDED mesh dimensions for aspect ratio,
// not the original image dimensions, because UV [0,1] maps to the expanded mesh.
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

// Helper: rotate 2D vector by angle
fn rotate_vec(v: vec2<f32>, angle: f32) -> vec2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec2<f32>(
        v.x * c - v.y * s,
        v.x * s + v.y * c
    );
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // Extract parameters
    let angle = stretch_params.x;         // Angle in radians
    let stretch_px = stretch_params.y;    // Stretch amount in pixels
    let offset = stretch_params.z;        // Reserved for future offset support
    let smooth_width = stretch_params.w;  // Reserved for future smooth support
    
    let orig_width = original_size.x;
    let orig_height = original_size.y;
    
    // Calculate EXPANDED mesh dimensions (CPU already expanded the mesh)
    let mesh_width = orig_width + stretch_px;
    let mesh_height = orig_height;
    
    // Input UV [0, 1] covers the EXPANDED mesh
    let uv = mesh.uv;
    
    // 1. Convert UV to pixel coordinates relative to mesh center
    // UV (0,0) -> pixel (-mesh_width/2, -mesh_height/2)
    // UV (1,1) -> pixel (+mesh_width/2, +mesh_height/2)
    let pixel_x = (uv.x - 0.5) * mesh_width;
    let pixel_y = (uv.y - 0.5) * mesh_height;
    let pixel_coord = vec2<f32>(pixel_x, pixel_y);
    
    // 2. Rotate coordinates by -angle to align split line vertically
    // In pixel space, 1px = 1px, so rotation is correct
    let rotated_pixel = rotate_vec(pixel_coord, -angle);
    
    // 3. Apply stretch logic in pixel space
    // The gap width is stretch_px (the amount we pushed the halves apart)
    let half_gap = stretch_px * 0.5;
    
    var sample_rotated_x: f32;
    
    if abs(rotated_pixel.x) < half_gap {
        // Inside the gap (green region): sample the center line
        sample_rotated_x = 0.0;
    } else {
        // Outside the gap (red/blue regions): shift coordinates back
        // Subtract the gap width to find the original texture position
        sample_rotated_x = rotated_pixel.x - sign(rotated_pixel.x) * half_gap;
    }
    
    // Combine with unchanged Y coordinate
    let final_rotated = vec2<f32>(sample_rotated_x, rotated_pixel.y);
    
    // 4. Rotate back to original orientation
    let unrotated_pixel = rotate_vec(final_rotated, angle);
    
    // 5. Convert back to UV coordinates
    // Note: we're sampling from the ORIGINAL texture, so use orig_width/height
    let final_uv = vec2<f32>(
        (unrotated_pixel.x / orig_width) + 0.5,
        (unrotated_pixel.y / orig_height) + 0.5
    );
    
    // 6. Boundary check - discard pixels outside valid UV range
    if final_uv.x < 0.0 || final_uv.x > 1.0 || final_uv.y < 0.0 || final_uv.y > 1.0 {
        discard;
    }
    
    // Sample the texture
    let tex_color = textureSample(base_texture, base_sampler, final_uv);
    
    // Apply color tint
    var final_color = tex_color * color;
    
    // Discard fully transparent pixels
    if final_color.a < 0.001 {
        discard;
    }
    
    return final_color;
}

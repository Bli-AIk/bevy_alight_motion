// Stretch segment shader - UV domain distortion effect
//
// This shader implements the "拉伸片段" (Stretch Segment) effect from Alight Motion.
// It creates a split line at a configurable angle and position, and pushes the halves apart.
//
// Implementation approach (pixel space rotation):
// 1. Convert UV to pixel coordinates (using EXPANDED mesh dimensions from CPU)
// 2. Rotate coordinates to align split line vertically
// 3. Apply horizontal stretch logic in pixel space (with offset for split line position)
// 4. Rotate back
// 5. Convert back to UV (relative to original texture)
//
// Uniform 0: color (vec4<f32>) - tint color
// Uniform 1: stretch_params (vec4<f32>) - (angle_radians, stretch_px, offset_px, smooth_width)
// Uniform 2: original_size (vec4<f32>) - (original_width, original_height, mesh_width, mesh_height)

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
    let offset_px = stretch_params.z;     // Offset of split line in pixels (perpendicular to line)
    let smooth_width = stretch_params.w;  // Reserved for future smooth support
    
    let orig_width = original_size.x;
    let orig_height = original_size.y;
    let mesh_width = original_size.z;   // Expanded mesh width from CPU
    let mesh_height = original_size.w;  // Expanded mesh height from CPU
    
    // Calculate half_gap (must match CPU calculation)
    let angle_factor = mix(1.0, 0.75, abs(sin(angle)));
    let half_gap = stretch_px * 0.5 * angle_factor;
    
    // Input UV [0, 1] covers the EXPANDED mesh
    let uv = mesh.uv;
    
    // 1. Convert UV to pixel coordinates relative to mesh center
    // UV (0,0) -> pixel (-mesh_width/2, -mesh_height/2)
    // UV (1,1) -> pixel (+mesh_width/2, +mesh_height/2)
    let pixel_x = (uv.x - 0.5) * mesh_width;
    let pixel_y = (uv.y - 0.5) * mesh_height;
    let pixel_coord = vec2<f32>(pixel_x, pixel_y);
    
    // 2. Rotate coordinates by +angle to align split line vertically
    // In pixel space, 1px = 1px, so rotation is correct
    let rotated_pixel = rotate_vec(pixel_coord, angle);
    
    // 3. Apply stretch logic in pixel space
    // The split line is at x = -offset_px (after rotation, negative because AM uses opposite direction)
    // We shift our coordinate system so the split line is at x = 0
    let shifted_x = rotated_pixel.x + offset_px;
    
    var sample_rotated_x: f32;
    
    if abs(shifted_x) < half_gap {
        // Inside the gap (green region): sample the split line position
        sample_rotated_x = -offset_px;
    } else {
        // Outside the gap (red/blue regions): shift coordinates back
        // Subtract the gap width to find the original texture position
        // But only if we're on the "gap side" of the original image
        // This ensures that when the split line is completely outside the image,
        // we don't apply any shift (image just appears offset)
        let original_sample_x = rotated_pixel.x - sign(shifted_x) * half_gap;
        
        // Check if we're crossing the split line by applying the shift
        // If original_sample_x and rotated_pixel.x have different signs relative to split line,
        // clamp to the split line position
        let original_shifted = original_sample_x + offset_px;
        if sign(shifted_x) != sign(original_shifted) {
            // The shift would cross the split line, clamp to split line
            sample_rotated_x = -offset_px;
        } else {
            sample_rotated_x = original_sample_x;
        }
    }
    
    // Combine with unchanged Y coordinate
    let final_rotated = vec2<f32>(sample_rotated_x, rotated_pixel.y);
    
    // 4. Rotate back to original orientation
    let unrotated_pixel = rotate_vec(final_rotated, -angle);
    
    // 5. Convert back to UV coordinates (relative to ORIGINAL texture)
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

// Masked sprite shader - clips sprite to a rectangular mask region and applies wipe effects
//
// Uniform 0: color (vec4<f32>) - tint color
// Uniform 1: mask_params (vec4<f32>) - (center_x, center_y, half_width, half_height)
// Uniform 2: wipe_params (vec4<f32>) - (wipe_start, wipe_end, wipe_angle, wipe_feather)
// The mask center is in world coordinates

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var<uniform> color: vec4<f32>;
@group(2) @binding(1) var<uniform> mask_params: vec4<f32>;
@group(2) @binding(2) var<uniform> wipe_params: vec4<f32>;
@group(2) @binding(3) var base_texture: texture_2d<f32>;
@group(2) @binding(4) var base_sampler: sampler;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // Sample the texture
    let tex_color = textureSample(base_texture, base_sampler, mesh.uv);
    
    // Get world position from mesh output
    let world_pos = mesh.world_position.xy;
    
    // Get mask parameters
    let mask_center = mask_params.xy;
    let mask_half_size = mask_params.zw;
    
    // Check if pixel is inside mask (axis-aligned rectangle)
    let rel_pos = world_pos - mask_center;
    let inside_mask = abs(rel_pos.x) <= mask_half_size.x && abs(rel_pos.y) <= mask_half_size.y;
    
    // If outside mask, discard (transparent)
    if !inside_mask {
        discard;
    }
    
    // Apply wipe effect
    // wipe_params: (start, end, angle, feather)
    let wipe_start = wipe_params.x;
    let wipe_end = wipe_params.y;
    let wipe_angle = wipe_params.z;
    let wipe_feather = wipe_params.w;
    
    // UV-based wipe (0 = left, 1 = right for angle=0)
    // For horizontal wipe (angle=0), use UV.x directly
    // For other angles, rotate the UV coordinate
    let cos_angle = cos(wipe_angle);
    let sin_angle = sin(wipe_angle);
    
    // Transform UV to rotated coordinate
    let centered_uv = mesh.uv - vec2<f32>(0.5, 0.5);
    let rotated_x = centered_uv.x * cos_angle + centered_uv.y * sin_angle;
    let wipe_coord = rotated_x + 0.5; // Back to 0-1 range
    
    // Apply wipe: pixels outside [start, end] are discarded
    // With feather, we fade near the edges
    var wipe_alpha = 1.0;
    if wipe_feather > 0.0 {
        // Soft edge with feather
        let start_dist = wipe_coord - wipe_start;
        let end_dist = wipe_end - wipe_coord;
        wipe_alpha = smoothstep(0.0, wipe_feather, start_dist) * smoothstep(0.0, wipe_feather, end_dist);
    } else {
        // Hard edge
        if wipe_coord < wipe_start || wipe_coord > wipe_end {
            discard;
        }
    }
    
    // Apply color tint and wipe alpha
    var final_color = tex_color * color;
    final_color.a *= wipe_alpha;
    
    if final_color.a < 0.001 {
        discard;
    }
    
    return final_color;
}

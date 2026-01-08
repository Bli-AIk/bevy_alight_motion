// Masked sprite shader - clips sprite to a rectangular mask region
//
// Uniform 0: color (vec4<f32>) - tint color
// Uniform 1: mask_params (vec4<f32>) - (center_x, center_y, half_width, half_height)
// The mask center is in world coordinates

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var<uniform> color: vec4<f32>;
@group(2) @binding(1) var<uniform> mask_params: vec4<f32>;
@group(2) @binding(2) var base_texture: texture_2d<f32>;
@group(2) @binding(3) var base_sampler: sampler;

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
    
    // Apply color tint and return
    return tex_color * color;
}

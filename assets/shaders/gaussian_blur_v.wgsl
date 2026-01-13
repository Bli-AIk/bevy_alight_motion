// Gaussian Blur Vertical Pass
// 
// This shader implements the vertical pass of a separable Gaussian blur.
// Uses optimized kernel with bilinear filtering.
//
// The separable approach splits 2D Gaussian blur into two 1D passes:
// - First: Horizontal blur (gaussian_blur_h.wgsl)
// - Second: Vertical blur (this shader)
// 
// This reduces complexity from O(radius²) to O(2 * radius).

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var<uniform> blur_params: vec4<f32>;  // x = radius, y = tex_width, z = tex_height, w = unused
@group(2) @binding(1) var base_texture: texture_2d<f32>;
@group(2) @binding(2) var base_sampler: sampler;

fn gaussian_weight(offset: f32, sigma: f32) -> f32 {
    return exp(-(offset * offset) / (2.0 * sigma * sigma));
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let radius = blur_params.x;
    let tex_width = blur_params.y;
    let tex_height = blur_params.z;
    
    // Early exit for no blur
    if radius < 0.5 {
        return textureSample(base_texture, base_sampler, mesh.uv);
    }
    
    let pixel_size = 1.0 / tex_height;
    let sigma = radius / 2.5;
    
    var total_color = vec4<f32>(0.0);
    var total_weight = 0.0;
    
    // Adaptive kernel size based on radius
    let num_samples = i32(min(radius * 2.0, 32.0));  // Cap at 32 samples per side
    
    // Center sample
    let center_sample = textureSample(base_texture, base_sampler, mesh.uv);
    total_color += center_sample;
    total_weight += 1.0;
    
    // Symmetric sampling (up and down)
    for (var i = 1; i <= num_samples; i = i + 1) {
        let offset = f32(i);
        let weight = gaussian_weight(offset, sigma);
        
        // Skip negligible weights
        if weight < 0.001 {
            break;
        }
        
        let offset_uv = offset * pixel_size;
        
        // Sample up
        let uv_up = mesh.uv + vec2<f32>(0.0, offset_uv);
        if uv_up.y <= 1.0 {
            let sample_up = textureSample(base_texture, base_sampler, uv_up);
            total_color += sample_up * weight;
            total_weight += weight;
        }
        
        // Sample down
        let uv_down = mesh.uv - vec2<f32>(0.0, offset_uv);
        if uv_down.y >= 0.0 {
            let sample_down = textureSample(base_texture, base_sampler, uv_down);
            total_color += sample_down * weight;
            total_weight += weight;
        }
    }
    
    // Normalize
    if total_weight > 0.001 {
        return total_color / total_weight;
    } else {
        return vec4<f32>(0.0);
    }
}

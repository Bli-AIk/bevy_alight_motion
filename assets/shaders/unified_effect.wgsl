// Unified effect shader - combines multiple effects in a single pass
//
// This shader supports four effects that can be enabled/disabled via flags:
// 1. Mask clipping (rectangular region)
// 2. Wipe transition (progressive reveal/hide)
// 3. Stretch segment (UV domain distortion)
// 4. Gaussian blur (optimized cross-shaped sampling)
//
// Each effect can be toggled on/off via the effect_flags uniform.
//
// Uniform layout:
// 0: color (vec4) - tint color
// 1: effect_flags (vec4) - (mask_enabled, wipe_enabled, stretch_enabled, blur_enabled)
// 2: mask_params (vec4) - (center_x, center_y, half_width, half_height)
// 3: wipe_params (vec4) - (wipe_start, wipe_end, wipe_angle, wipe_feather)
// 4: stretch_params (vec4) - (angle_radians, stretch_px, offset_px, smooth_width)
// 5: original_size (vec4) - (orig_width, orig_height, mesh_width, mesh_height)
// 6: mesh_offset (vec4) - (center_offset_x, center_offset_y, 0, 0)
// 9: blur_params (vec4) - (radius_px, orig_width, orig_height, expansion_px)

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var<uniform> color: vec4<f32>;
@group(2) @binding(1) var<uniform> effect_flags: vec4<f32>;
@group(2) @binding(2) var<uniform> mask_params: vec4<f32>;
@group(2) @binding(3) var<uniform> wipe_params: vec4<f32>;
@group(2) @binding(4) var<uniform> stretch_params: vec4<f32>;
@group(2) @binding(5) var<uniform> original_size: vec4<f32>;
@group(2) @binding(6) var<uniform> mesh_offset: vec4<f32>;
@group(2) @binding(7) var base_texture: texture_2d<f32>;
@group(2) @binding(8) var base_sampler: sampler;
@group(2) @binding(9) var<uniform> blur_params: vec4<f32>;

// Helper: rotate 2D vector by angle
fn rotate_vec(v: vec2<f32>, angle: f32) -> vec2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec2<f32>(
        v.x * c - v.y * s,
        v.x * s + v.y * c
    );
}

// Apply stretch segment effect - returns modified UV
fn apply_stretch_segment(uv: vec2<f32>) -> vec2<f32> {
    let angle = stretch_params.x;
    let stretch_px = stretch_params.y;
    let offset_px = stretch_params.z;
    
    let orig_width = original_size.x;
    let orig_height = original_size.y;
    let mesh_width = original_size.z;
    let mesh_height = original_size.w;
    
    let center_off_x = mesh_offset.x;
    let center_off_y = mesh_offset.y;
    
    let angle_factor = mix(1.0, 0.75, abs(sin(angle)));
    let half_gap = stretch_px * 0.5 * angle_factor;
    
    let pixel_x = (uv.x - 0.5) * mesh_width + center_off_x;
    let pixel_y = (uv.y - 0.5) * mesh_height + center_off_y;
    let pixel_coord = vec2<f32>(pixel_x, pixel_y);
    
    let rotated_pixel = rotate_vec(pixel_coord, angle);
    let shifted_x = rotated_pixel.x + offset_px;
    
    var sample_rotated_x: f32;
    if abs(shifted_x) < half_gap {
        sample_rotated_x = -offset_px;
    } else {
        sample_rotated_x = rotated_pixel.x - sign(shifted_x) * half_gap;
    }
    
    let final_rotated = vec2<f32>(sample_rotated_x, rotated_pixel.y);
    let unrotated_pixel = rotate_vec(final_rotated, -angle);
    
    return vec2<f32>(
        (unrotated_pixel.x / orig_width) + 0.5,
        (unrotated_pixel.y / orig_height) + 0.5
    );
}

// Apply wipe effect - returns alpha multiplier
fn apply_wipe(uv: vec2<f32>) -> f32 {
    let wipe_start = wipe_params.x;
    let wipe_end = wipe_params.y;
    let wipe_angle = wipe_params.z;
    let wipe_feather = wipe_params.w;
    
    let cos_angle = cos(wipe_angle);
    let sin_angle = sin(wipe_angle);
    let centered_uv = uv - vec2<f32>(0.5, 0.5);
    let rotated_x = centered_uv.x * cos_angle + centered_uv.y * sin_angle;
    let wipe_coord = rotated_x + 0.5;
    
    if wipe_feather > 0.0 {
        let start_dist = wipe_coord - wipe_start;
        let end_dist = wipe_end - wipe_coord;
        return smoothstep(0.0, wipe_feather, start_dist) * smoothstep(0.0, wipe_feather, end_dist);
    } else {
        if wipe_coord < wipe_start || wipe_coord > wipe_end {
            return 0.0;
        }
        return 1.0;
    }
}

// Apply mask clipping - returns true if inside mask
fn apply_mask(world_pos: vec2<f32>) -> bool {
    let mask_center = mask_params.xy;
    let mask_half_size = mask_params.zw;
    
    if mask_half_size.x > 5000.0 {
        return true;
    }
    
    let rel_pos = world_pos - mask_center;
    return abs(rel_pos.x) <= mask_half_size.x && abs(rel_pos.y) <= mask_half_size.y;
}

// Gaussian weight function
fn gaussian_weight(offset: f32, sigma: f32) -> f32 {
    return exp(-(offset * offset) / (2.0 * sigma * sigma));
}

// 2D Gaussian weight function
fn gaussian_weight_2d(dx: f32, dy: f32, sigma: f32) -> f32 {
    let d2 = dx * dx + dy * dy;
    return exp(-d2 / (2.0 * sigma * sigma));
}

// True 2D Gaussian blur with correct transparent boundary handling
// Boundary pixels outside [0,1] are treated as transparent (rgba(0,0,0,0))
// and participate in the weighted average to create proper edge fade-out
// blur_params: x = radius_px, y = orig_width, z = orig_height, w = expansion_px
fn apply_blur(uv: vec2<f32>) -> vec4<f32> {
    let radius = blur_params.x;
    let orig_width = blur_params.y;
    let orig_height = blur_params.z;
    
    // Pixel size in UV space
    let pixel_size_x = 1.0 / orig_width;
    let pixel_size_y = 1.0 / orig_height;
    
    // Sigma = radius / 2.0 for softer, more natural light diffusion (closer to Alight Motion)
    let sigma = max(radius / 2.0, 0.01);
    
    var total_color = vec4<f32>(0.0);
    var total_weight = 0.0;
    
    // Sample radius covers 3*sigma for good distribution coverage
    // Cap at reasonable value for performance, but no step skipping to avoid artifacts
    let sample_radius = i32(min(ceil(sigma * 3.0), 64.0));
    
    // 2D grid sampling with Gaussian weights - no step skipping for quality
    for (var dy = -sample_radius; dy <= sample_radius; dy = dy + 1) {
        for (var dx = -sample_radius; dx <= sample_radius; dx = dx + 1) {
            let offset_x = f32(dx) * pixel_size_x;
            let offset_y = f32(dy) * pixel_size_y;
            let sample_uv = uv + vec2<f32>(offset_x, offset_y);
            
            // Calculate 2D Gaussian weight
            let weight = gaussian_weight_2d(f32(dx), f32(dy), sigma);
            
            // Skip negligible weights for performance
            if weight < 0.001 {
                continue;
            }
            
            // Sample color - treat out-of-bounds as transparent (rgba(0,0,0,0))
            // This is the key fix: boundary pixels participate in weighted average
            // with zero color contribution, causing proper edge fade-out
            var sample_color: vec4<f32>;
            if sample_uv.x >= 0.0 && sample_uv.x <= 1.0 && sample_uv.y >= 0.0 && sample_uv.y <= 1.0 {
                // Within bounds: normal sampling
                sample_color = textureSample(base_texture, base_sampler, sample_uv);
            } else {
                // Outside bounds: transparent black
                sample_color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
            }
            
            // Always accumulate both color and weight
            total_color += sample_color * weight;
            total_weight += weight;
        }
    }
    
    // Normalize - with the fix above, total_weight should always be non-zero
    // for any UV that the 2D grid covers (which includes all mesh pixels)
    if total_weight > 0.0001 {
        return total_color / total_weight;
    } else {
        // Extreme edge case: return transparent
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // Extract effect flags
    let mask_enabled = effect_flags.x > 0.5;
    let wipe_enabled = effect_flags.y > 0.5;
    let stretch_enabled = effect_flags.z > 0.5;
    let blur_enabled = effect_flags.w > 0.5;
    
    var sample_uv = mesh.uv;
    
    // Apply stretch segment effect if enabled (before blur)
    if stretch_enabled {
        sample_uv = apply_stretch_segment(mesh.uv);
        
        if sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0 {
            discard;
        }
    }
    
    // Sample texture - with or without blur
    var tex_color: vec4<f32>;
    
    if blur_enabled {
        let blur_radius = blur_params.x;
        if blur_radius > 0.5 {
            tex_color = apply_blur(sample_uv);
        } else {
            tex_color = textureSample(base_texture, base_sampler, sample_uv);
        }
    } else {
        tex_color = textureSample(base_texture, base_sampler, sample_uv);
    }
    
    // Apply mask clipping if enabled
    if mask_enabled {
        let world_pos = mesh.world_position.xy;
        if !apply_mask(world_pos) {
            discard;
        }
    }
    
    // Calculate wipe alpha if enabled
    var wipe_alpha = 1.0;
    if wipe_enabled {
        wipe_alpha = apply_wipe(mesh.uv);
        if wipe_alpha < 0.001 {
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

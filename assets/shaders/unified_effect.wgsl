// Unified effect shader - combines multiple effects in a single pass
//
// This shader supports five effects that can be enabled/disabled via flags:
// 1. Mask clipping (rectangular region)
// 2. Wipe transition (progressive reveal/hide)
// 3. Stretch segment (UV domain distortion)
// 4. Gaussian blur (optimized cross-shaped sampling)
// 5. Palette map (color quantization to palette)
//
// Each effect can be toggled on/off via the effect_flags uniform.
//
// All uniform data is packed into a single struct to minimize binding count
// and ensure compatibility with hardware that limits uniform bindings to 15.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

// Packed uniform struct containing all effect parameters
struct UnifiedEffectUniform {
    color: vec4<f32>,              // tint color
    effect_flags: vec4<f32>,       // (mask_enabled, wipe_enabled, stretch_enabled, blur_enabled)
    mask_params: vec4<f32>,        // (center_x, center_y, half_width, half_height)
    wipe_params: vec4<f32>,        // (wipe_start, wipe_end, wipe_angle, wipe_feather)
    stretch_params: vec4<f32>,     // (angle_radians, stretch_px, offset_px, smooth_width)
    original_size: vec4<f32>,      // (orig_width, orig_height, mesh_width, mesh_height)
    mesh_offset: vec4<f32>,        // (center_offset_x, center_offset_y, 0, 0)
    blur_params: vec4<f32>,        // (radius_px, orig_width, orig_height, expansion_px)
    palette_flags: vec4<f32>,      // (enabled, count, shades, alpha)
    palette_color1: vec4<f32>,
    palette_color2: vec4<f32>,
    palette_color3: vec4<f32>,
    palette_color4: vec4<f32>,
    palette_color5: vec4<f32>,
    palette_color6: vec4<f32>,
    palette_color7: vec4<f32>,
    palette_color8: vec4<f32>,
    mask2_params: vec4<f32>,       // (center_x, center_y, half_width, half_height)
    mask2_flags: vec4<f32>,        // (mask2_type, mask1_rotation, mask2_rotation, 0)
    replace_color_flags: vec4<f32>,// (enabled, lock_luminance, 0, 0)
    replace_old_color: vec4<f32>,  // (r, g, b, a)
    replace_new_color: vec4<f32>,  // (r, g, b, a)
    replace_color_params: vec4<f32>,// (threshold, feather, alpha, 0)
    repeat_params1: vec4<f32>,     // (count, offset_x, offset_y, angle_deg)
    repeat_params2: vec4<f32>,     // (scale, alpha, 0, 0)
    // Linear repeat effect
    linear_repeat_params1: vec4<f32>,  // (count, position_x, position_y, angle_deg)
    linear_repeat_params2: vec4<f32>,  // (offset_x, offset_y, scale, alpha)
    linear_repeat_params3: vec4<f32>,  // (start, end, phase, overlap)
    linear_repeat_params4: vec4<f32>,  // (ease_in, ease_out, blend, shape_invert_alt)
    linear_repeat_params5: vec4<f32>,  // (random_order, seed, 0, 0)
    linear_repeat_fill_color: vec4<f32>, // fill color (r, g, b, a)
    // Second linear repeat effect (for stacked/dual effects)
    linear_repeat2_params1: vec4<f32>,
    linear_repeat2_params2: vec4<f32>,
    linear_repeat2_params3: vec4<f32>,
    linear_repeat2_params4: vec4<f32>,
    linear_repeat2_params5: vec4<f32>,
    linear_repeat2_fill_color: vec4<f32>,
    // Radial repeat effect
    radial_repeat_params1: vec4<f32>,  // (count, radius, orientation_deg, startAngle_deg)
    radial_repeat_params2: vec4<f32>,  // (sweep_deg, baseScale, angle_deg, scale)
    radial_repeat_params3: vec4<f32>,  // (alpha, offset_x, offset_y, blend)
    radial_repeat_params4: vec4<f32>,  // (start, end, phase, overlap)
    radial_repeat_params5: vec4<f32>,  // (ease_in, ease_out, shape_invert_alt, seed+random)
    radial_repeat_fill_color: vec4<f32>,
    // Threshold effect
    threshold_params: vec4<f32>,       // (threshold, feather, invert, blendMode)
    // Grid effect
    grid_flags: vec4<f32>,             // (enabled, punchout, screen_space, 0)
    grid_params1: vec4<f32>,           // (pos_x, pos_y, spacing, width)
    grid_params2: vec4<f32>,           // (smoothing, 0, 0, 0)
    grid_color: vec4<f32>,             // (r, g, b, a)
    // Pixelate effect
    pixelate_flags: vec4<f32>,         // (enabled, screen_space, 0, 0)
    pixelate_params1: vec4<f32>,       // (size, stretch_x, stretch_y, angle)
    pixelate_params2: vec4<f32>,       // (vignette, threshold, saturation, 0)
    // Mask blend parameters
    mask_blend: vec4<f32>,             // mask1: (fill_alpha, opacity, stroke_width, 0)
    mask2_blend: vec4<f32>,            // mask2: (fill_alpha, opacity, stroke_width, 0)
    // Stretch2 effect (directional UV-space stretch)
    stretch2_params: vec4<f32>,        // (scale, angle_radians, content_only, 0)
    // Solidcolor effect
    solid_color_params: vec4<f32>,     // (r, g, b, blend_mode)
    solid_color_alpha: vec4<f32>,      // (alpha, 0, 0, 0)
}

@group(2) @binding(0) var<uniform> uniforms: UnifiedEffectUniform;
@group(2) @binding(1) var base_texture: texture_2d<f32>;
@group(2) @binding(2) var base_sampler: sampler;

// Helper: rotate 2D vector by angle
fn rotate_vec(v: vec2<f32>, angle: f32) -> vec2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec2<f32>(
        v.x * c - v.y * s,
        v.x * s + v.y * c
    );
}

// Helper: convert sRGB to linear RGB (single channel)
fn srgb_to_linear_channel(c: f32) -> f32 {
    if c <= 0.04045 {
        return c / 12.92;
    } else {
        return pow((c + 0.055) / 1.055, 2.4);
    }
}

// Helper: convert sRGB color to linear RGB
fn srgb_to_linear(color: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        srgb_to_linear_channel(color.r),
        srgb_to_linear_channel(color.g),
        srgb_to_linear_channel(color.b)
    );
}

fn linear_to_srgb_channel(c: f32) -> f32 {
    if c <= 0.0031308 {
        return c * 12.92;
    } else {
        return 1.055 * pow(c, 1.0 / 2.4) - 0.055;
    }
}

fn linear_to_srgb(color: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        linear_to_srgb_channel(color.r),
        linear_to_srgb_channel(color.g),
        linear_to_srgb_channel(color.b)
    );
}

// Apply stretch2 effect (directional UV-space stretch)
// From AM stretch2.xml shader:
//   sampleCoord = ((((layerNorm - 0.5) * rot) * vec2(1/scale, 1)) * invrot) + 0.5
fn apply_stretch2(uv: vec2<f32>) -> vec2<f32> {
    let scale = uniforms.stretch2_params.x;
    let angle = uniforms.stretch2_params.y;
    let centered = uv - vec2<f32>(0.5);
    let rotated = rotate_vec(centered, angle);
    let stretched = rotated * vec2<f32>(1.0 / scale, 1.0);
    let unrotated = rotate_vec(stretched, -angle);
    return unrotated + vec2<f32>(0.5);
}

// Smooth minimum (cubic polynomial) - matches AM's sminCubic
fn smin_cubic(a: f32, b: f32, k: f32) -> f32 {
    let h = max(k - abs(a - b), 0.0) / k;
    return min(a, b) - h * h * h * k * (1.0 / 6.0);
}

// Apply stretch segment effect - returns modified UV
fn apply_stretch_segment(uv: vec2<f32>) -> vec2<f32> {
    let angle = uniforms.stretch_params.x;
    let stretch_px = uniforms.stretch_params.y;
    let offset_px = uniforms.stretch_params.z;
    let smooth_param = uniforms.stretch_params.w;
    
    let orig_width = uniforms.original_size.x;
    let orig_height = uniforms.original_size.y;
    let mesh_width = uniforms.original_size.z;
    let mesh_height = uniforms.original_size.w;
    
    let center_off_x = uniforms.mesh_offset.x;
    let center_off_y = uniforms.mesh_offset.y;
    
    let angle_factor = mix(1.0, 0.9, abs(sin(angle)));
    let half_gap = stretch_px * 0.5 * angle_factor;
    
    let pixel_x = (uv.x - 0.5) * mesh_width + center_off_x;
    let pixel_y = (uv.y - 0.5) * mesh_height + center_off_y;
    let pixel_coord = vec2<f32>(pixel_x, pixel_y);
    
    let rotated_pixel = rotate_vec(pixel_coord, angle);
    let shifted_x = rotated_pixel.x + offset_px;
    
    // Use sminCubic for smooth blending (matches AM's stretchsegment shader)
    let smooth_k = max(0.00001, smooth_param * half_gap);
    let d = smin_cubic(half_gap, abs(shifted_x), smooth_k);
    let sample_rotated_x = rotated_pixel.x + d * -sign(shifted_x);
    
    let final_rotated = vec2<f32>(sample_rotated_x, rotated_pixel.y);
    let unrotated_pixel = rotate_vec(final_rotated, -angle);
    
    return vec2<f32>(
        (unrotated_pixel.x / orig_width) + 0.5,
        (unrotated_pixel.y / orig_height) + 0.5
    );
}

// Apply wipe effect - returns alpha multiplier
fn apply_wipe(uv: vec2<f32>) -> f32 {
    let wipe_start = uniforms.wipe_params.x;
    let wipe_end = uniforms.wipe_params.y;
    let wipe_angle = uniforms.wipe_params.z;
    let wipe_feather = uniforms.wipe_params.w;
    
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

// Compute mask blend factor for a single mask.
// Returns 1.0 = fully visible, 0.0 = fully hidden.
fn compute_ue_mask_blend_factor(
    world_pos: vec2<f32>,
    mask_params: vec4<f32>,
    mask_rotation: f32,
    mask_type: f32,
    mask_blend: vec4<f32>,
) -> f32 {
    if mask_type < 0.5 || mask_params.z > 5000.0 {
        return 1.0;
    }

    let center = mask_params.xy;
    let half_size = mask_params.zw;
    let fill_alpha = mask_blend.x;
    let opacity = mask_blend.y;
    let sw = mask_blend.z;

    var rel = world_pos - center;
    if abs(mask_rotation) > 0.001 {
        let c = cos(-mask_rotation);
        let s = sin(-mask_rotation);
        rel = vec2<f32>(rel.x * c - rel.y * s, rel.x * s + rel.y * c);
    }

    let is_exclude = mask_type > 2.5;
    let is_ellipse = (mask_type > 1.5 && mask_type < 2.5) || mask_type > 3.5;

    // Shape fill boundary = bounding box minus stroke extension (centered stroke)
    let shape_half = max(half_size - sw * 0.5, vec2<f32>(0.001));

    var mask_sdf: f32;
    if is_ellipse {
        let norm = rel / shape_half;
        let r = length(norm);
        mask_sdf = (r - 1.0) * min(shape_half.x, shape_half.y);
    } else {
        mask_sdf = max(abs(rel.x) - shape_half.x, abs(rel.y) - shape_half.y);
    }

    let fill_factor = select(0.0, fill_alpha, mask_sdf < 0.0);
    // Stroke is solid within its width, with ~1px AA at the outer edge
    let aa = min(1.0, sw * 0.5);
    let stroke_factor = select(0.0, 1.0 - smoothstep(sw * 0.5 - aa, sw * 0.5, abs(mask_sdf)), sw > 0.01);
    let mask_alpha = min(max(fill_factor, stroke_factor), 1.0);

    if is_exclude {
        return 1.0 - opacity * mask_alpha;
    } else {
        return 1.0 - opacity * (1.0 - mask_alpha);
    }
}

// Apply combined masks - returns blend factor (1.0=fully visible, 0.0=fully hidden)
fn apply_masks_blend(world_pos: vec2<f32>) -> f32 {
    let mask1_type = uniforms.effect_flags.x;
    let mask2_type = uniforms.mask2_flags.x;
    let mask1_rotation = uniforms.mask2_flags.y;
    let mask2_rotation = uniforms.mask2_flags.z;

    let mask1_enabled = mask1_type > 0.5;
    let mask2_enabled = mask2_type > 0.5;

    if !mask1_enabled && !mask2_enabled {
        return 1.0;
    }

    var factor = 1.0;
    if mask1_enabled {
        factor *= compute_ue_mask_blend_factor(
            world_pos,
            uniforms.mask_params,
            mask1_rotation,
            mask1_type,
            uniforms.mask_blend,
        );
    }
    if mask2_enabled {
        factor *= compute_ue_mask_blend_factor(
            world_pos,
            uniforms.mask2_params,
            mask2_rotation,
            mask2_type,
            uniforms.mask2_blend,
        );
    }
    return factor;
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
    let radius = uniforms.blur_params.x;
    let orig_width = uniforms.blur_params.y;
    let orig_height = uniforms.blur_params.z;
    
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

// Get palette color by index (0-7), colors are stored in sRGB space
fn get_palette_color(index: i32) -> vec4<f32> {
    var col: vec4<f32>;
    switch(index) {
        case 0: { col = uniforms.palette_color1; }
        case 1: { col = uniforms.palette_color2; }
        case 2: { col = uniforms.palette_color3; }
        case 3: { col = uniforms.palette_color4; }
        case 4: { col = uniforms.palette_color5; }
        case 5: { col = uniforms.palette_color6; }
        case 6: { col = uniforms.palette_color7; }
        case 7: { col = uniforms.palette_color8; }
        default: { col = uniforms.palette_color1; }
    }
    // Return as-is (sRGB); palette matching happens in sRGB space
    return col;
}

// Calculate color distance (Euclidean, matching AM's length())
fn color_distance(c1: vec3<f32>, c2: vec3<f32>) -> f32 {
    let diff = c1 - c2;
    return dot(diff, diff);
}

// Apply palette map effect - quantize color to nearest palette color
// AM processes entirely in sRGB space, so we convert linear→sRGB before matching
fn apply_palette_map(input_color: vec4<f32>) -> vec4<f32> {
    let palette_count = i32(uniforms.palette_flags.y);
    
    // Convert input from linear to sRGB for matching (AM works in sRGB)
    let a = input_color.a;
    let srgb_rgb = linear_to_srgb(input_color.rgb);
    
    // Find nearest palette color in sRGB space
    var min_dist = 1000000.0;
    var nearest_index = 0;
    
    for (var i = 0; i < palette_count; i = i + 1) {
        let palette_rgb = get_palette_color(i).rgb;
        let dist = color_distance(srgb_rgb, palette_rgb);
        if dist < min_dist {
            min_dist = dist;
            nearest_index = i;
        }
    }
    
    let best_color = get_palette_color(nearest_index).rgb;
    
    // AM output: mix(texColor, vec4(bestColor, 1.0) * a, alpha)
    // bestColor is in sRGB, convert to linear for GPU pipeline
    let result_linear = srgb_to_linear(best_color);
    return vec4<f32>(result_linear, a);
}

// Apply replace color effect - replaces old_color with new_color based on threshold and feather
// replace_color_flags: (enabled, lock_luminance, 0, 0)
// replace_color_params: (threshold, feather, alpha, 0)
fn apply_replace_color(input_color: vec4<f32>) -> vec4<f32> {
    let threshold = uniforms.replace_color_params.x;
    let feather = uniforms.replace_color_params.y;
    let effect_alpha = uniforms.replace_color_params.z;
    let lock_luminance = uniforms.replace_color_flags.y > 0.5;
    
    // Uniform colors are passed in sRGB space, convert to linear for blending
    // since input_color from texture is already in linear space
    let old_rgb = srgb_to_linear(uniforms.replace_old_color.rgb);
    var new_rgb = srgb_to_linear(uniforms.replace_new_color.rgb);
    
    // Calculate color distance in linear RGB space (normalized 0-1)
    let input_rgb = input_color.rgb;
    let diff = input_rgb - old_rgb;
    let distance = length(diff) / sqrt(3.0); // Normalize to 0-1 range
    
    // Calculate replacement factor based on threshold and feather
    // If distance < threshold: full replacement
    // If distance > threshold + feather: no replacement
    // In between: smooth transition
    var replace_factor: f32;
    if feather > 0.001 {
        replace_factor = 1.0 - smoothstep(threshold, threshold + feather, distance);
    } else {
        replace_factor = select(0.0, 1.0, distance <= threshold);
    }
    
    // Apply effect alpha
    replace_factor *= effect_alpha;
    
    // If lock_luminance is enabled, preserve original brightness
    if lock_luminance {
        let input_lum = dot(input_rgb, vec3<f32>(0.299, 0.587, 0.114));
        let new_lum = dot(new_rgb, vec3<f32>(0.299, 0.587, 0.114));
        if new_lum > 0.001 {
            new_rgb = new_rgb * (input_lum / new_lum);
        }
    }
    
    // Blend between original and new color (all in linear space)
    let result_rgb = mix(input_rgb, new_rgb, replace_factor);
    
    return vec4<f32>(result_rgb, input_color.a);
}

// AM-compatible 2D cubic bezier easing
// Based on AM's CubicBezierEasing implementation with Newton-Raphson iteration

// Helper: a coefficient for bezier calculation
fn bezier_a(a1: f32, a2: f32) -> f32 {
    return (1.0 - (a2 * 3.0)) + (a1 * 3.0);
}

// Helper: b coefficient for bezier calculation  
fn bezier_b(a1: f32, a2: f32) -> f32 {
    return (a2 * 3.0) - (a1 * 6.0);
}

// Helper: c coefficient for bezier calculation
fn bezier_c(a1: f32) -> f32 {
    return a1 * 3.0;
}

// Calculate bezier value at parameter t
fn calc_bezier(t: f32, a1: f32, a2: f32) -> f32 {
    return ((((bezier_a(a1, a2) * t) + bezier_b(a1, a2)) * t) + bezier_c(a1)) * t;
}

// Calculate bezier slope at parameter t
fn get_slope(t: f32, a1: f32, a2: f32) -> f32 {
    return (bezier_a(a1, a2) * 3.0 * t * t) + (bezier_b(a1, a2) * 2.0 * t) + bezier_c(a1);
}

// Find t parameter for given x value using Newton-Raphson iteration
fn get_t_for_x(x: f32, p1x: f32, p2x: f32) -> f32 {
    // Clamp p1x and p2x like AM does
    let p1x_clamped = min(p1x, 0.95);
    let p2x_clamped = max(p2x, 0.05);
    
    // Determine iteration count based on x position
    var iterations: i32;
    if x < 0.05 || x > 0.95 {
        iterations = 24; // 3 * 8
    } else {
        iterations = 8;  // 1 * 8
    }
    
    var guess = x;
    var prev_slope = 1000.0;
    
    for (var i = 0; i < iterations; i++) {
        let slope = get_slope(guess, p1x_clamped, p2x_clamped);
        if abs(slope) < 0.0001 {
            return guess;
        }
        // Early termination if slope change is small
        if i > 2 && abs(slope - prev_slope) < 0.005 {
            return guess;
        }
        guess = guess - (calc_bezier(guess, p1x_clamped, p2x_clamped) - x) / slope;
        prev_slope = slope;
    }
    
    return guess;
}

// 2D cubic bezier interpolation matching AM's CubicBezierEasing.interpolate()
fn cubic_bezier_2d(t: f32, p1x: f32, p1y: f32, p2x: f32, p2y: f32) -> f32 {
    // Linear case
    if abs(p1x - p1y) < 0.001 && abs(p2x - p2y) < 0.001 {
        return t;
    }
    
    // Handle negative t (extrapolation)
    if t < 0.0 {
        let y_at_001 = calc_bezier(get_t_for_x(0.01, p1x, p2x), p1y, p2y);
        let y_at_0 = calc_bezier(get_t_for_x(0.0, p1x, p2x), p1y, p2y);
        return t * ((y_at_001 - y_at_0) / 0.01);
    }
    
    // Normal case: find t for x, then compute y
    return calc_bezier(get_t_for_x(t, p1x, p2x), p1y, p2y);
}

// AM-compatible easing curve interpolation
// ease_in, ease_out: -1 to 1 range from AM parameters
fn apply_am_easing(progress: f32, ease_in: f32, ease_out: f32) -> f32 {
    if abs(ease_in) < 0.001 && abs(ease_out) < 0.001 {
        return progress;
    }
    // AM's bezier control points calculation from RepeatEasingKt:
    // p1x = max(ease_in/2, 0), p1y = max(-ease_in/2, 0)
    // p2x = 1 - max(ease_out/2, 0), p2y = 1 - max(-ease_out/2, 0)
    let p1x = max(ease_in * 0.5, 0.0);
    let p1y = max(-ease_in * 0.5, 0.0);
    let p2x = 1.0 - max(ease_out * 0.5, 0.0);
    let p2y = 1.0 - max(-ease_out * 0.5, 0.0);
    
    return cubic_bezier_2d(progress, p1x, p1y, p2x, p2y);
}

// 48-bit Java Random implementation (matching java.util.Random exactly)
// State is represented as (hi: u16, lo: u32) where full state = hi << 32 | lo

// Multiply two u32 values, return (hi, lo) of 64-bit result
fn mul_u32_wide(a: u32, b: u32) -> vec2<u32> {
    let a_lo = a & 0xFFFFu;
    let a_hi = a >> 16u;
    let b_lo = b & 0xFFFFu;
    let b_hi = b >> 16u;
    let ll = a_lo * b_lo;
    let lh = a_lo * b_hi;
    let hl = a_hi * b_lo;
    let hh = a_hi * b_hi;
    let mid_sum = (ll >> 16u) + (lh & 0xFFFFu) + (hl & 0xFFFFu);
    let lo = (ll & 0xFFFFu) | ((mid_sum & 0xFFFFu) << 16u);
    let hi = hh + (lh >> 16u) + (hl >> 16u) + (mid_sum >> 16u);
    return vec2<u32>(hi, lo);
}

// Step the 48-bit LCG: state = (state * 0x5DEECE66D + 0xB) & ((1<<48)-1)
fn java_random_step(state_hi: ptr<function, u32>, state_lo: ptr<function, u32>) {
    let mult_hi: u32 = 5u;          // upper 16 bits of 0x5DEECE66D
    let mult_lo: u32 = 0xDEECE66Du; // lower 32 bits
    let prod = mul_u32_wide(*state_lo, mult_lo);
    let cross = (*state_hi) * mult_lo + (*state_lo) * mult_hi;
    let new_lo = prod.y + 0xBu;
    let carry = select(0u, 1u, new_lo < prod.y);
    *state_lo = new_lo;
    *state_hi = (prod.x + cross + carry) & 0xFFFFu;
}

// Java Random.next(31): advance state and return top 31 bits
fn java_random_next31(state_hi: ptr<function, u32>, state_lo: ptr<function, u32>) -> u32 {
    java_random_step(state_hi, state_lo);
    return ((*state_hi) << 15u) | ((*state_lo) >> 17u);
}

// Java Random.nextInt(bound) with rejection sampling
fn java_random_next_int(state_hi: ptr<function, u32>, state_lo: ptr<function, u32>, bound: u32) -> u32 {
    if (bound & (bound - 1u)) == 0u {
        // Power of two: ((long)bound * (long)next(31)) >> 31
        let bits = java_random_next31(state_hi, state_lo);
        let prod = mul_u32_wide(bound, bits);
        return (prod.x << 1u) | (prod.y >> 31u);
    }
    // Rejection sampling for non-power-of-two
    for (var attempt = 0; attempt < 100; attempt = attempt + 1) {
        let bits = java_random_next31(state_hi, state_lo);
        let val = bits % bound;
        if (bits - val + bound - 1u) < 0x80000000u {
            return val;
        }
    }
    return 0u;
}

// Fisher-Yates shuffle using pre-computed Java Random initial state.
// state_lo/state_hi are the initial 48-bit state after seed initialization,
// passed from CPU via bitcast<u32> on uniform floats.
fn get_shuffled_index(original_index: i32, count: i32, init_state_lo: u32, init_state_hi: u32) -> i32 {
    var perm: array<i32, 100>;
    for (var i = 0; i < count && i < 100; i = i + 1) {
        perm[i] = i;
    }
    var s_hi = init_state_hi;
    var s_lo = init_state_lo;
    for (var i = count - 1; i > 0; i = i - 1) {
        let j = i32(java_random_next_int(&s_hi, &s_lo, u32(i + 1)));
        let temp = perm[i];
        perm[i] = perm[j];
        perm[j] = temp;
    }
    if original_index >= 0 && original_index < count && original_index < 100 {
        return perm[original_index];
    }
    return original_index;
}

// Calculate linear repeat progress for a single copy index
// Returns (baseProgress, interpProgress) matching AM's repeatWithEasing algorithm
fn calc_linear_repeat_progress(
    index: i32,
    count: i32,
    start: f32,
    end: f32,
    phase: f32,
    overlap: f32,
    shape: i32,
    invert: bool,
    ease_in: f32,
    ease_out: f32,
    random_order: bool,
    rng_state_lo: u32,
    rng_state_hi: u32
) -> vec2<f32> {
    // Get shuffled index if random_order is enabled
    // The shuffled index is used for position calculation (base_position)
    // while original index is used for baseProgress (rendering order)
    var shuffled_index = index;
    if random_order {
        shuffled_index = get_shuffled_index(index, count, rng_state_lo, rng_state_hi);
    }
    
    let fi_shuffled = f32(shuffled_index);
    let fi_original = f32(index);
    let fcount = f32(count);
    
    // AM algorithm: overlap_value = overlap + 1.0
    let overlap_value = overlap + 1.0;
    // denominator = (2 * overlap_value) + count - 1
    let denominator = (2.0 * overlap_value) + fcount - 1.0;
    // step_width = 1.0 / denominator
    let step_width = 1.0 / denominator;
    // half_width = step_width * overlap_value
    let half_width = step_width * overlap_value;
    
    // base_position uses shuffled index for position calculation
    // AM: intValue2 = ((list.get(i3) + overlap_value) / denominator) + phase
    let base_position = ((fi_shuffled + overlap_value) / denominator) + phase;
    // center_pos = base_position + half_width / 2
    let center_pos = base_position + half_width * 0.5;
    
    // Calculate base progress using original index (rendering order)
    // AM: baseProgress = i / (count - 1)
    var base_progress: f32;
    if count > 1 {
        base_progress = fi_original / (fcount - 1.0);
    } else {
        base_progress = 0.0;
    }
    
    // Calculate interpolation progress based on shape
    var interp_progress: f32;
    
    // Shape constants: 0=RAMP, 1=SQUARE, 2=SMOOTH, 3=TRIANGLE
    if shape == 1 {
        // SQUARE shape
        let in_fade = clamp((base_position - start) / half_width, 0.0, 1.0);
        let out_fade = clamp((end - base_position) / half_width, 0.0, 1.0);
        if start < end {
            interp_progress = min(in_fade, out_fade);
        } else {
            interp_progress = 1.0 - max(in_fade, out_fade);
        }
    } else if shape == 2 {
        // SMOOTH shape (Gaussian)
        if center_pos >= start && center_pos <= end {
            let x = (center_pos - start) / (end - start);
            let centered = (x - 0.5) * 2.0 * 3.14159265;
            interp_progress = exp(-centered * centered * 0.5);
        } else {
            interp_progress = 0.0;
        }
    } else if shape == 3 {
        // TRIANGLE shape
        if center_pos >= start && center_pos <= end {
            let x = (center_pos - start) / (end - start);
            if x < 0.5 {
                interp_progress = x * 2.0;
            } else {
                interp_progress = (1.0 - x) * 2.0;
            }
        } else {
            interp_progress = 0.0;
        }
    } else {
        // RAMP shape (default, shape == 0)
        let range = max(end - start, 0.001);
        interp_progress = (center_pos - start) / range;
    }
    
    // Apply easing
    if abs(ease_in) > 0.001 || abs(ease_out) > 0.001 {
        interp_progress = apply_am_easing(clamp(interp_progress, 0.0, 1.0), ease_in, ease_out);
    }
    
    // Apply invert
    if invert {
        interp_progress = 1.0 - interp_progress;
    }
    
    // Clamp final progress
    interp_progress = clamp(interp_progress, 0.0, 1.0);
    
    return vec2<f32>(base_progress, interp_progress);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // Extract effect flags
    // Mask is enabled if either mask1 or mask2 is enabled
    let mask_enabled = uniforms.effect_flags.x > 0.5 || uniforms.mask2_flags.x > 0.5;
    let wipe_enabled = uniforms.effect_flags.y > 0.5;
    let stretch_enabled = uniforms.effect_flags.z > 0.5;
    let blur_enabled = uniforms.effect_flags.w > 0.5;
    let palette_enabled = uniforms.palette_flags.x > 0.5;
    let replace_color_enabled = uniforms.replace_color_flags.x > 0.5;
    
    // Extract repeat effect params
    let repeat_count = i32(uniforms.repeat_params1.x);
    let repeat_offset = vec2<f32>(uniforms.repeat_params1.y, uniforms.repeat_params1.z);
    let repeat_angle = uniforms.repeat_params1.w * 3.14159265 / 180.0; // degrees to radians
    let repeat_scale = uniforms.repeat_params2.x;
    let repeat_alpha = uniforms.repeat_params2.y;
    let repeat_enabled = repeat_count > 0;
    
    // Extract linear repeat effect params
    // Use round for count to get integer copy counts
    let linear_repeat_count = i32(round(uniforms.linear_repeat_params1.x));
    let linear_repeat_position = vec2<f32>(uniforms.linear_repeat_params1.y, uniforms.linear_repeat_params1.z);
    let linear_repeat_angle_deg = uniforms.linear_repeat_params1.w;
    let linear_repeat_offset = vec2<f32>(uniforms.linear_repeat_params2.x, uniforms.linear_repeat_params2.y);
    let linear_repeat_scale = uniforms.linear_repeat_params2.z;
    let linear_repeat_alpha = uniforms.linear_repeat_params2.w;
    let linear_repeat_start = uniforms.linear_repeat_params3.x;
    let linear_repeat_end = uniforms.linear_repeat_params3.y;
    let linear_repeat_phase = uniforms.linear_repeat_params3.z;
    let linear_repeat_overlap = uniforms.linear_repeat_params3.w;
    let linear_repeat_ease_in = uniforms.linear_repeat_params4.x;
    let linear_repeat_ease_out = uniforms.linear_repeat_params4.y;
    let linear_repeat_blend = uniforms.linear_repeat_params4.z;
    let linear_repeat_shape_invert_alt = i32(uniforms.linear_repeat_params4.w);
    let linear_repeat_shape = linear_repeat_shape_invert_alt / 100;
    let linear_repeat_invert = (linear_repeat_shape_invert_alt / 10) % 10 == 1;
    let linear_repeat_color_alt = linear_repeat_shape_invert_alt % 10 == 1;
    let linear_repeat_random_order = uniforms.linear_repeat_params5.x > 0.5;
    let linear_repeat_rng_lo = bitcast<u32>(uniforms.linear_repeat_params5.y);
    let linear_repeat_rng_hi = bitcast<u32>(uniforms.linear_repeat_params5.z);
    // Linear repeat activation states:
    // - count < 0: effect not activated, render original
    // - count == 0: effect activated but count=0, render nothing (hide)
    // - count > 0: effect activated, render count copies
    let linear_repeat_activated = linear_repeat_count >= 0;
    let linear_repeat_enabled = linear_repeat_count > 0;

    // Second linear repeat effect
    let lr2_count = i32(round(uniforms.linear_repeat2_params1.x));
    let lr2_position = vec2<f32>(uniforms.linear_repeat2_params1.y, uniforms.linear_repeat2_params1.z);
    let lr2_angle_deg = uniforms.linear_repeat2_params1.w;
    let lr2_offset = vec2<f32>(uniforms.linear_repeat2_params2.x, uniforms.linear_repeat2_params2.y);
    let lr2_scale = uniforms.linear_repeat2_params2.z;
    let lr2_alpha = uniforms.linear_repeat2_params2.w;
    let lr2_start = uniforms.linear_repeat2_params3.x;
    let lr2_end = uniforms.linear_repeat2_params3.y;
    let lr2_phase = uniforms.linear_repeat2_params3.z;
    let lr2_overlap = uniforms.linear_repeat2_params3.w;
    let lr2_ease_in = uniforms.linear_repeat2_params4.x;
    let lr2_ease_out = uniforms.linear_repeat2_params4.y;
    let lr2_blend = uniforms.linear_repeat2_params4.z;
    let lr2_sia = i32(uniforms.linear_repeat2_params4.w);
    let lr2_shape = lr2_sia / 100;
    let lr2_invert = (lr2_sia / 10) % 10 == 1;
    let lr2_color_alt = lr2_sia % 10 == 1;
    let lr2_random_order = uniforms.linear_repeat2_params5.x > 0.5;
    let lr2_rng_lo = bitcast<u32>(uniforms.linear_repeat2_params5.y);
    let lr2_rng_hi = bitcast<u32>(uniforms.linear_repeat2_params5.z);
    let lr2_enabled = lr2_count > 0;
    
    // Extract radial repeat effect params
    let rr_raw_count = uniforms.radial_repeat_params1.x;
    let rr_count = max(i32(round(rr_raw_count)), 0);
    let rr_count_f = max(abs(rr_raw_count), 0.001); // raw float for position formula (AM uses unrounded)
    let rr_enabled = rr_raw_count != 0.0; // -1 means "effect present, 0 copies"
    let rr_radius = uniforms.radial_repeat_params1.y;
    let rr_orientation_deg = uniforms.radial_repeat_params1.z;
    let rr_start_angle_deg = uniforms.radial_repeat_params1.w;
    let rr_sweep_deg = uniforms.radial_repeat_params2.x;
    let rr_base_scale = uniforms.radial_repeat_params2.y;
    let rr_angle_deg = uniforms.radial_repeat_params2.z;
    let rr_scale = uniforms.radial_repeat_params2.w;
    let rr_alpha = uniforms.radial_repeat_params3.x;
    let rr_offset = vec2<f32>(uniforms.radial_repeat_params3.y, uniforms.radial_repeat_params3.z);
    let rr_blend = uniforms.radial_repeat_params3.w;
    let rr_start = uniforms.radial_repeat_params4.x;
    let rr_end = uniforms.radial_repeat_params4.y;
    let rr_phase = uniforms.radial_repeat_params4.z;
    let rr_overlap = uniforms.radial_repeat_params4.w;
    let rr_ease_in = uniforms.radial_repeat_params5.x;
    let rr_ease_out = uniforms.radial_repeat_params5.y;
    let rr_sia = i32(uniforms.radial_repeat_params5.z);
    let rr_shape = rr_sia / 100;
    let rr_invert = (rr_sia / 10) % 10 == 1;
    let rr_color_alt = rr_sia % 10 == 1;
    let rr_seed_raw = uniforms.radial_repeat_params5.w;
    let rr_random_order = fract(rr_seed_raw) > 0.3;
    let rr_seed = floor(rr_seed_raw);
    // Compute Java Random state from seed for radial repeat (approximate, uses f32)
    // For typical integer seeds (0, 1, ...) this is exact
    let rr_am_seed = u32(15234322.0 + 35432882176.0 * rr_seed);
    let rr_init = rr_am_seed ^ 0xDEECE66Du; // XOR with lower 32 bits of 0x5DEECE66D
    let rr_init_hi = (((rr_am_seed >> 16u) ^ 5u) & 0xFFFFu); // approximate upper bits XOR
    let rr_rng_lo = rr_init;
    let rr_rng_hi = rr_init_hi;

    // Extract pixelate effect params
    let pixelate_enabled = uniforms.pixelate_flags.x > 0.5;
    let pixelate_screen_space = uniforms.pixelate_flags.y > 0.5;
    let pixelate_size = uniforms.pixelate_params1.x;
    let pixelate_stretch = vec2<f32>(uniforms.pixelate_params1.y, uniforms.pixelate_params1.z);
    let pixelate_angle = uniforms.pixelate_params1.w * 3.14159265 / 180.0; // degrees to radians
    let pixelate_vignette = uniforms.pixelate_params2.x;
    let pixelate_threshold = uniforms.pixelate_params2.y;
    let pixelate_saturation = uniforms.pixelate_params2.z;
    
    var sample_uv = mesh.uv;
    
    // Discard fragments in expansion area when no expansion-capable effect is active
    if !pixelate_enabled && !repeat_enabled && !linear_repeat_enabled && !lr2_enabled && !rr_enabled
        && (mesh.uv.x < 0.0 || mesh.uv.x > 1.0 || mesh.uv.y < 0.0 || mesh.uv.y > 1.0) {
        discard;
    }
    
    // Apply stretch segment effect if enabled (before blur)
    if stretch_enabled {
        sample_uv = apply_stretch_segment(mesh.uv);
        
        // Add small tolerance to prevent edge clipping due to floating point precision
        let eps = 0.002;
        if sample_uv.x < -eps || sample_uv.x > 1.0 + eps || sample_uv.y < -eps || sample_uv.y > 1.0 + eps {
            discard;
        }
        // Clamp to valid range for texture sampling
        sample_uv = clamp(sample_uv, vec2<f32>(0.0), vec2<f32>(1.0));
    }
    
    // Apply stretch2 effect (directional stretch)
    let stretch2_scale = uniforms.stretch2_params.x;
    if stretch2_scale > 0.001 && abs(stretch2_scale - 1.0) > 0.0001 {
        sample_uv = apply_stretch2(sample_uv);
        sample_uv = clamp(sample_uv, vec2<f32>(0.0), vec2<f32>(1.0));
    }
    
    // Apply pixelate effect (AM pixelate2 algorithm)
    // Grid is centered on layer (non-screenSpace) or screen (screenSpace)
    var pixelate_dist_center = 0.0;
    if pixelate_enabled {
        let display_size = vec2<f32>(uniforms.original_size.x, uniforms.original_size.y);

        // AM's grid cell size is in screen pixels
        let size_vec = vec2<f32>(
            pixelate_size * pixelate_stretch.x,
            pixelate_size * pixelate_stretch.y
        );

        // Position in display pixels relative to layer center
        var dp = (sample_uv - vec2<f32>(0.5)) * display_size;

        // Convert dp from local space to screen-aligned space.
        // dp is in the layer's local pixel space (Y-down matching UV).
        // AM computes the grid in FBO pixel space (screen space).
        // The parent hierarchy rotation (from GlobalTransform) maps local→screen.
        // In our convention: screen_offset = (dp.y, -dp.x) for 90° parent rotation.
        // The correct rotation to apply is -parent_rotation (to go from local Y-down to screen Y-down).
        let parent_rotation = uniforms.pixelate_params2.w;
        let total_angle = pixelate_angle - parent_rotation;
        let cos_a = cos(total_angle);
        let sin_a = sin(total_angle);
        var st = vec2<f32>(cos_a * dp.x - sin_a * dp.y, sin_a * dp.x + cos_a * dp.y);

        // Find position within pixel cell (true modulo for negative values)
        var pos_in_pixel = st - floor(st / size_vec) * size_vec;

        // Center, rotate back, un-center (AM's posInPixel adjustment)
        pos_in_pixel -= size_vec * 0.5;
        pos_in_pixel = vec2<f32>(
            cos_a * pos_in_pixel.x + sin_a * pos_in_pixel.y,
            -sin_a * pos_in_pixel.x + cos_a * pos_in_pixel.y
        );
        pos_in_pixel += size_vec * 0.5;

        // Distance from pixel center (for vignette)
        pixelate_dist_center = smoothstep(0.0, 1.0, length((pos_in_pixel / size_vec) - vec2<f32>(0.5)));

        // Snap to pixel center
        let snapped_dp = dp - pos_in_pixel + size_vec * 0.5;
        sample_uv = snapped_dp / display_size + vec2<f32>(0.5);
        // Discard if the grid cell center maps outside the texture
        if sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0 {
            discard;
        }
    }
    
    // Sample texture - with or without blur, with or without repeat
    var tex_color: vec4<f32>;
    var linear_repeat_color_applied = false; // Flag to skip final uniforms.color multiplication
    
    if repeat_enabled {
        // Repeat effect: render multiple copies composited in paint order.
        // AM iterates copies 0..count-1, each painted on canvas (later = on top).
        // pixel_coord is Y-down (UV convention: UV.y=0 at top, UV.y=1 at bottom).
        // AM offset/angle are also Y-down, so no Y-flip needed.
        let orig_width = uniforms.original_size.x;
        let orig_height = uniforms.original_size.y;
        
        let center = vec2<f32>(0.5, 0.5);
        let pixel_coord = (sample_uv - center) * vec2<f32>(orig_width, orig_height);
        
        var accumulated_color = vec4<f32>(0.0);
        
        for (var i = 0; i < repeat_count; i = i + 1) {
            let fi = f32(i);
            
            // AM alpha: linear decay  alpha_i = 1.0 - i * (1.0 - repeat_alpha)
            let cumulative_alpha = 1.0 - fi * (1.0 - repeat_alpha);
            if cumulative_alpha <= 0.0 {
                continue;
            }
            
            let cumulative_offset = repeat_offset * fi;
            let cumulative_angle = repeat_angle * fi;
            let cumulative_scale = pow(repeat_scale, fi);
            
            // Inverse transform: un-offset → un-rotate → un-scale
            var tc = pixel_coord;
            
            // 1. Reverse offset (both pixel_coord and offset are Y-down)
            tc = tc - cumulative_offset;
            
            // 2. Reverse rotation (inverse = rotate by -angle)
            if abs(cumulative_angle) > 0.001 {
                let cos_a = cos(-cumulative_angle);
                let sin_a = sin(-cumulative_angle);
                tc = vec2<f32>(
                    tc.x * cos_a - tc.y * sin_a,
                    tc.x * sin_a + tc.y * cos_a
                );
            }
            
            // 3. Reverse scale
            if abs(cumulative_scale) > 0.001 {
                tc = tc / cumulative_scale;
            }
            
            let half_w = orig_width * 0.5;
            let half_h = orig_height * 0.5;
            
            if tc.x >= -half_w && tc.x <= half_w &&
               tc.y >= -half_h && tc.y <= half_h {
                // Convert back to UV (same Y-down convention)
                let copy_uv = tc / vec2<f32>(orig_width, orig_height) + center;
                var copy_color: vec4<f32>;
                if blur_enabled && uniforms.blur_params.x > 0.5 {
                    copy_color = apply_blur(copy_uv);
                } else {
                    copy_color = textureSample(base_texture, base_sampler, copy_uv);
                }
                
                copy_color.a *= cumulative_alpha;
                accumulated_color = copy_color + accumulated_color * (1.0 - copy_color.a);
            }
        }
        
        tex_color = accumulated_color;
    } else if linear_repeat_enabled {
        // Linear repeat effect: render multiple copies arranged in a line
        // pixel_coord is Y-down (matching AM convention), no Y-flips needed.
        let orig_width = uniforms.original_size.x;
        let orig_height = uniforms.original_size.y;
        
        let center = vec2<f32>(0.5, 0.5);
        let pixel_coord = (sample_uv - center) * vec2<f32>(orig_width, orig_height);
        
        var accumulated_color = vec4<f32>(0.0);
        
        // Effect 2 iteration count (1 if no second effect)
        let total_copies2 = select(1, lr2_count, lr2_enabled);

        // Iterate forward to match AM's paint order (later copies on top)
        for (var j = 0; j < total_copies2; j = j + 1) {
            var d2 = vec2<f32>(0.0, 0.0);
            var scale2 = 1.0;
            var angle2_rad = 0.0;
            var alpha2 = 1.0;
            var interp2 = 0.0;
            if lr2_enabled {
                let progress2 = calc_linear_repeat_progress(
                    j, lr2_count, lr2_start, lr2_end, lr2_phase, lr2_overlap,
                    lr2_shape, lr2_invert, lr2_ease_in, lr2_ease_out,
                    lr2_random_order, lr2_rng_lo, lr2_rng_hi
                );
                let base2 = progress2.x;
                interp2 = progress2.y;
                d2 = lr2_position * base2 + lr2_offset * interp2;
                scale2 = 1.0 + (lr2_scale - 1.0) * interp2;
                angle2_rad = lr2_angle_deg * 3.14159265 / 180.0 * interp2;
                alpha2 = 1.0 + (lr2_alpha - 1.0) * interp2;
            }
            if alpha2 < 0.001 || abs(scale2) < 0.001 {
                continue;
            }

            let total_copies = linear_repeat_count;
            for (var i = 0; i < total_copies; i = i + 1) {
                let progress = calc_linear_repeat_progress(
                    i, total_copies, linear_repeat_start, linear_repeat_end,
                    linear_repeat_phase, linear_repeat_overlap, linear_repeat_shape,
                    linear_repeat_invert, linear_repeat_ease_in, linear_repeat_ease_out,
                    linear_repeat_random_order, linear_repeat_rng_lo, linear_repeat_rng_hi
                );
                let base_progress = progress.x;
                let interp_progress = progress.y;
                
                let d1 = linear_repeat_position * base_progress + linear_repeat_offset * interp_progress;
                let copy_scale1 = 1.0 + (linear_repeat_scale - 1.0) * interp_progress;
                let copy_angle1 = linear_repeat_angle_deg * 3.14159265 / 180.0 * interp_progress;
                let copy_alpha1 = 1.0 + (linear_repeat_alpha - 1.0) * interp_progress;
                
                let combined_alpha = copy_alpha1 * alpha2;
                let combined_scale = copy_scale1 * scale2;
                
                if combined_alpha < 0.001 || abs(combined_scale) < 0.001 {
                    continue;
                }
                
                // Inverse transform: undo effect2, then undo effect1
                var tc = pixel_coord;
                
                // Undo effect 2
                if lr2_enabled {
                    tc = tc - d2;
                    if abs(angle2_rad) > 0.001 {
                        let c2 = cos(-angle2_rad);
                        let s2 = sin(-angle2_rad);
                        tc = vec2<f32>(
                            tc.x * c2 - tc.y * s2,
                            tc.x * s2 + tc.y * c2
                        );
                    }
                    tc = tc / scale2;
                }
                
                // Undo effect 1
                tc = tc - d1;
                if abs(copy_angle1) > 0.001 {
                    let c1 = cos(-copy_angle1);
                    let s1 = sin(-copy_angle1);
                    tc = vec2<f32>(
                        tc.x * c1 - tc.y * s1,
                        tc.x * s1 + tc.y * c1
                    );
                }
                tc = tc / copy_scale1;
                
                let half_w = orig_width * 0.5;
                let half_h = orig_height * 0.5;
                
                if tc.x >= -half_w && tc.x <= half_w &&
                   tc.y >= -half_h && tc.y <= half_h {
                    let copy_uv = tc / vec2<f32>(orig_width, orig_height) + center;
                    var copy_color: vec4<f32>;
                    if blur_enabled && uniforms.blur_params.x > 0.5 {
                        copy_color = apply_blur(copy_uv);
                    } else {
                        copy_color = textureSample(base_texture, base_sampler, copy_uv);
                    }
                    
                    // Color blending from effect 1
                    if linear_repeat_blend > 0.001 {
                        let base_rgb = uniforms.color.rgb;
                        let fill_rgb = uniforms.linear_repeat_fill_color.rgb;
                        var should_blend = true;
                        if linear_repeat_color_alt && (i % 2 == 1) {
                            should_blend = false;
                        }
                        if should_blend {
                            var start_color = base_rgb;
                            var end_color: vec3<f32>;
                            if linear_repeat_blend <= 1.0 {
                                end_color = mix(base_rgb, fill_rgb, linear_repeat_blend);
                            } else {
                                start_color = mix(base_rgb, fill_rgb, linear_repeat_blend - 1.0);
                                end_color = fill_rgb;
                            }
                            let final_rgb = mix(start_color, end_color, interp_progress);
                            copy_color = vec4<f32>(final_rgb, copy_color.a);
                        }
                    }
                    
                    // Color blending from effect 2
                    if lr2_enabled && lr2_blend > 0.001 {
                        let base_rgb2 = copy_color.rgb;
                        let fill_rgb2 = uniforms.linear_repeat2_fill_color.rgb;
                        var should_blend2 = true;
                        if lr2_color_alt && (j % 2 == 1) {
                            should_blend2 = false;
                        }
                        if should_blend2 {
                            var start_color2 = base_rgb2;
                            var end_color2: vec3<f32>;
                            if lr2_blend <= 1.0 {
                                end_color2 = mix(base_rgb2, fill_rgb2, lr2_blend);
                            } else {
                                start_color2 = mix(base_rgb2, fill_rgb2, lr2_blend - 1.0);
                                end_color2 = fill_rgb2;
                            }
                            let final_rgb2 = mix(start_color2, end_color2, interp2);
                            copy_color = vec4<f32>(final_rgb2, copy_color.a);
                        }
                    }
                    
                    copy_color.a *= combined_alpha;
                    accumulated_color = copy_color + accumulated_color * (1.0 - copy_color.a);
                }
            }
        }
        
        tex_color = accumulated_color;
        linear_repeat_color_applied = linear_repeat_blend > 0.001 || (lr2_enabled && lr2_blend > 0.001);
    } else if rr_enabled {
        // Radial repeat: AM's transform chain (TransformKt.transform on Canvas):
        //   translate(L) translate(P) rotate(rotation) scale(S) translate(-P) rotate(orient) scale(size)
        // Copy fields: L=elem.L+offset*interp+(0,r), P=(0,-r), rotation=spread,
        //   S=(mix,mix), orient=orient_param+angle*interp, size=baseScale
        // Forward: pixel = offset*interp + R(spread)*mix*(R(orbit)*baseScale*p + (0,radius))
        // Inverse: p = R(-orbit)*(R(-spread)*(pixel-offset*interp)/mix - (0,radius)) / baseScale
        let orig_width = uniforms.original_size.x;
        let orig_height = uniforms.original_size.y;
        let center = vec2<f32>(0.5, 0.5);
        let pixel_coord = (sample_uv - center) * vec2<f32>(orig_width, orig_height);
        let deg2rad = 3.14159265 / 180.0;
        let gamma = vec3<f32>(2.2);
        let inv_gamma = vec3<f32>(1.0 / 2.2);
        
        // AM composites in sRGB space; accumulate in sRGB premultiplied alpha
        var acc_srgb = vec4<f32>(0.0);
        
        for (var i = 0; i < rr_count; i = i + 1) {
            let progress = calc_linear_repeat_progress(
                i, rr_count, rr_start, rr_end, rr_phase, rr_overlap,
                rr_shape, rr_invert, rr_ease_in, rr_ease_out,
                rr_random_order, rr_rng_lo, rr_rng_hi
            );
            let base_progress = progress.x;
            let interp_progress = progress.y;
            
            // Spread angle (rotation field — rotates around pivot)
            // AM uses the same formula for all counts: startAngle - sweep/2 + (sweep - sweep/count) * base
            // For count=1: (sweep - sweep/1) = 0, so spread = startAngle - sweep/2
            let spread = (rr_start_angle_deg - rr_sweep_deg / 2.0
                + (rr_sweep_deg - rr_sweep_deg / f32(max(rr_count, 1))) * base_progress) * deg2rad;
            // Orbit angle (orientation field — local rotation)
            let orbit = (rr_orientation_deg + rr_angle_deg * interp_progress) * deg2rad;
            
            let mix_scale = 1.0 + (rr_scale - 1.0) * interp_progress;
            let copy_alpha = 1.0 + (rr_alpha - 1.0) * interp_progress;
            
            if copy_alpha < 0.001 || abs(mix_scale) < 0.001 || abs(rr_base_scale) < 0.001 {
                continue;
            }
            
            // Inverse transform (6 steps)
            var tc = pixel_coord - rr_offset * interp_progress;
            let cos_s = cos(-spread);
            let sin_s = sin(-spread);
            tc = vec2<f32>(tc.x * cos_s - tc.y * sin_s, tc.x * sin_s + tc.y * cos_s);
            tc = tc / mix_scale;
            tc = tc - vec2<f32>(0.0, rr_radius);
            let cos_o = cos(-orbit);
            let sin_o = sin(-orbit);
            tc = vec2<f32>(tc.x * cos_o - tc.y * sin_o, tc.x * sin_o + tc.y * cos_o);
            tc = tc / rr_base_scale;
            
            let half_w = orig_width * 0.5;
            let half_h = orig_height * 0.5;
            
            if tc.x >= -half_w && tc.x <= half_w &&
               tc.y >= -half_h && tc.y <= half_h {
                let copy_uv = tc / vec2<f32>(orig_width, orig_height) + center;
                var copy_color: vec4<f32>;
                if blur_enabled && uniforms.blur_params.x > 0.5 {
                    copy_color = apply_blur(copy_uv);
                } else {
                    copy_color = textureSample(base_texture, base_sampler, copy_uv);
                }
                
                // Convert to sRGB for AM-compatible compositing
                var copy_srgb = pow(copy_color.rgb, inv_gamma);
                
                // Color blending in sRGB (AM blends in sRGB space)
                if rr_blend > 0.001 {
                    let base_srgb = pow(uniforms.color.rgb, inv_gamma);
                    let fill_srgb = pow(uniforms.radial_repeat_fill_color.rgb, inv_gamma);
                    var should_blend = true;
                    if rr_color_alt && (i % 2 == 1) {
                        should_blend = false;
                    }
                    if should_blend {
                        var start_color = base_srgb;
                        var end_color: vec3<f32>;
                        if rr_blend <= 1.0 {
                            end_color = mix(base_srgb, fill_srgb, rr_blend);
                        } else {
                            start_color = mix(base_srgb, fill_srgb, rr_blend - 1.0);
                            end_color = fill_srgb;
                        }
                        copy_srgb = mix(start_color, end_color, interp_progress);
                    }
                }
                
                // Composite in sRGB premultiplied alpha (matches AM's Canvas)
                let final_a = copy_color.a * copy_alpha;
                let premult = vec4<f32>(copy_srgb * final_a, final_a);
                acc_srgb = premult + acc_srgb * (1.0 - final_a);
            }
        }
        
        // Convert premultiplied sRGB to linear for output.
        // Output as opaque since AM composites with black bg in sRGB space,
        // and Bevy's linear-space blend would give different results.
        if acc_srgb.a > 0.001 {
            tex_color = vec4<f32>(pow(acc_srgb.rgb, gamma), 1.0);
        } else {
            tex_color = vec4<f32>(0.0);
        }
        linear_repeat_color_applied = rr_blend > 0.001;
    } else if linear_repeat_activated && !linear_repeat_enabled {
        // Linear repeat is activated but count=0: render nothing (hide element)
        tex_color = vec4<f32>(0.0);
    } else if blur_enabled {
        let blur_radius = uniforms.blur_params.x;
        if blur_radius > 0.5 {
            tex_color = apply_blur(sample_uv);
        } else {
            tex_color = textureSample(base_texture, base_sampler, sample_uv);
        }
    } else {
        tex_color = textureSample(base_texture, base_sampler, sample_uv);
    }
    
    // Apply pixelate post-effects (AM algorithm: threshold on alpha, saturation boost, cubic vignette)
    if pixelate_enabled {
        // Threshold: compare against alpha (not luminance like standalone threshold effect)
        let tclamp = step(pixelate_threshold, tex_color.a);

        // Saturation: boost colors by dividing by alpha ratio
        if tex_color.a > 0.0 && pixelate_saturation > 0.0 {
            tex_color /= tex_color.a / max(tex_color.a, pixelate_saturation);
        }

        // Apply threshold clamp
        tex_color = tex_color * tclamp;

        // Vignette: cubic darkening, only when size >= 1.5
        let vignette_gate = step(1.5, pixelate_size);
        tex_color = mix(
            tex_color,
            vec4<f32>(
                min(tex_color.rgb * tex_color.rgb * tex_color.rgb, vec3<f32>(0.9)),
                tex_color.a * tex_color.a * tex_color.a
            ),
            vignette_gate * pixelate_vignette * pixelate_dist_center
        );
    }
    
    // Apply threshold effect if enabled (convert to black & white based on brightness threshold)
    // AM works in sRGB space, so we convert linear→sRGB before processing
    let threshold_enabled = uniforms.replace_color_flags.z > 0.5;
    if threshold_enabled {
        let threshold_value = uniforms.threshold_params.x;
        let threshold_feather = uniforms.threshold_params.y;
        let threshold_invert = uniforms.threshold_params.z > 0.5;
        let threshold_blend_mode = i32(uniforms.threshold_params.w);
        
        // Convert to sRGB space to match AM's processing
        let srgb_rgb = linear_to_srgb(tex_color.rgb);
        let luminance = dot(srgb_rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
        
        // AM threshold formula
        let f = max(min(threshold_feather / 4.0, abs(0.5 - threshold_value)), 0.00196);
        let t = ((threshold_value - 0.5) * (1.003 + f * 2.0)) + 0.5;
        var p = smoothstep(t - f, t + f, luminance);
        
        if threshold_invert {
            p = 1.0 - p;
        }
        
        // Apply blend mode in sRGB space, then convert back to linear
        var result_rgb: vec3<f32>;
        if threshold_blend_mode == 1 {
            result_rgb = srgb_to_linear(srgb_rgb * p);
        } else if threshold_blend_mode == 2 {
            result_rgb = srgb_to_linear(1.0 - ((1.0 - srgb_rgb) * (1.0 - p)));
        } else {
            // Normal: output is just B&W (0 or 1), same in both spaces
            result_rgb = vec3<f32>(p, p, p);
        }
        
        tex_color = vec4<f32>(result_rgb, tex_color.a);
    }
    
    // Apply replace color effect if enabled (AFTER threshold)
    if replace_color_enabled {
        tex_color = apply_replace_color(tex_color);
    }
    
    // Apply palette map effect if enabled
    if palette_enabled {
        let palette_alpha = uniforms.palette_flags.w;
        let quantized_color = apply_palette_map(tex_color);
        // Blend between original and quantized based on palette alpha
        tex_color = mix(tex_color, quantized_color, palette_alpha);
    }
    
    // Apply grid effect if enabled (AM grid2 algorithm)
    let grid_enabled = uniforms.grid_flags.x > 0.5;
    if grid_enabled {
        let grid_punchout = uniforms.grid_flags.y > 0.5;
        let grid_screen_space = uniforms.grid_flags.z > 0.5;
        let grid_pos = uniforms.grid_params1.xy;
        let grid_spacing = uniforms.grid_params1.z;
        let grid_width = uniforms.grid_params1.w;
        let grid_smoothing = uniforms.grid_params2.x;
        let grid_color_val = uniforms.grid_color;

        // AM coordinate setup: normalized + aspect-corrected + centered
        var st: vec2<f32>;
        if grid_screen_space {
            st = mesh.uv;
            // TODO: proper screen-space needs screen size uniform
            st.x = st.x * uniforms.original_size.x / uniforms.original_size.y;
            st.y = 1.0 - st.y;
        } else {
            st = mesh.uv;  // acLayerNorm equivalent [0,1]
            st.x = st.x * uniforms.original_size.x / uniforms.original_size.y;
            st.y = 1.0 - st.y;
        }
        st -= vec2<f32>(0.5);
        st -= vec2<f32>(grid_pos.x, -grid_pos.y) / 1000.0;

        // GLSL mod: x - y * floor(x/y) — always positive result
        let px_cell = (st.x - grid_spacing * floor(st.x / grid_spacing)) / grid_spacing;
        let py_cell = (st.y - grid_spacing * floor(st.y / grid_spacing)) / grid_spacing;

        // Triangle wave: 1 at edges (grid lines), 0 at cell center
        var px = 1.0 - abs(px_cell - 0.5) * 2.0;
        var py = 1.0 - abs(py_cell - 0.5) * 2.0;

        // Width relative to spacing
        let w = clamp(grid_width / grid_spacing, 0.0, 1.0);
        var s = w * grid_smoothing;

        // AM adaptive smoothing for thin lines (inverted smoothstep — WGSL-compatible form)
        s = mix(s, max(s, 0.5 * w), 1.0 - smoothstep(0.01, 0.012, grid_width));
        s = mix(s, max(s, w), 1.0 - smoothstep(0.005, 0.006, grid_width));

        // Grid line intensity
        px = smoothstep(1.0 - w - s, 1.0 - w + s, px);
        py = smoothstep(1.0 - w - s, 1.0 - w + s, py);
        let p = max(px, py);

        // AM-matching composite (blend in sRGB space to match AM's non-linear pipeline)
        // grid_color_val is in sRGB [0,1], convert tex_color to sRGB for blending
        let tex_srgb = linear_to_srgb(tex_color.rgb);
        let c = grid_color_val * p;
        if grid_punchout {
            tex_color = vec4<f32>(tex_color.rgb * (1.0 - p), tex_color.a * (1.0 - p));
        } else {
            let grid_alpha = c.a * tex_color.a;
            let blended_srgb = c.rgb * grid_alpha + tex_srgb * (1.0 - grid_alpha);
            let blended_a = c.a * grid_alpha + tex_color.a * (1.0 - grid_alpha);
            tex_color = vec4<f32>(srgb_to_linear(blended_srgb), blended_a);
        }
    }
    
    // Apply mask blend factor if any mask is enabled
    var mask_factor = 1.0;
    if mask_enabled {
        let world_pos = mesh.world_position.xy;
        mask_factor = apply_masks_blend(world_pos);
        if mask_factor < 0.005 {
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
    // Skip color multiplication for linear-repeat since we already applied the color blend
    var final_color: vec4<f32>;
    if linear_repeat_color_applied {
        // Just apply the alpha from uniforms.color, not the RGB
        final_color = vec4<f32>(tex_color.rgb, tex_color.a * uniforms.color.a);
    } else {
        final_color = tex_color * uniforms.color;
    }

    // Apply solidcolor effect (after color tint, before wipe)
    let sc_alpha = uniforms.solid_color_alpha.x;
    if sc_alpha > 0.001 {
        let sc_color = uniforms.solid_color_params.xyz;
        let blend_mode = i32(uniforms.solid_color_params.w);
        var sc_result: vec3<f32>;
        if blend_mode == 1 {
            // Multiply
            sc_result = final_color.rgb * sc_color;
        } else if blend_mode == 2 {
            // Screen
            sc_result = vec3<f32>(1.0) - (vec3<f32>(1.0) - final_color.rgb) * (vec3<f32>(1.0) - sc_color);
        } else {
            // Normal (blendMode=0): replace RGB with solid color, keep alpha
            sc_result = sc_color * final_color.a;
        }
        final_color = vec4<f32>(
            mix(final_color.rgb, sc_result, sc_alpha),
            final_color.a
        );
    }

    final_color.a *= wipe_alpha;

    // Apply mask in sRGB space to match AM's compositing pipeline.
    // AM blends: output_sRGB = content_sRGB * mask_factor.
    // GPU pipeline is linear, so do sRGB round-trip for mask application.
    if mask_factor < 0.999 {
        let lin = final_color.rgb;
        // linear → sRGB (approximate, matching sRGB standard piecewise curve)
        let srgb = vec3<f32>(
            select(1.055 * pow(lin.r, 1.0 / 2.4) - 0.055, lin.r * 12.92, lin.r <= 0.0031308),
            select(1.055 * pow(lin.g, 1.0 / 2.4) - 0.055, lin.g * 12.92, lin.g <= 0.0031308),
            select(1.055 * pow(lin.b, 1.0 / 2.4) - 0.055, lin.b * 12.92, lin.b <= 0.0031308),
        );
        let masked = srgb * mask_factor;
        // sRGB → linear
        final_color = vec4<f32>(
            select(pow((masked.x + 0.055) / 1.055, 2.4), masked.x / 12.92, masked.x <= 0.04045),
            select(pow((masked.y + 0.055) / 1.055, 2.4), masked.y / 12.92, masked.y <= 0.04045),
            select(pow((masked.z + 0.055) / 1.055, 2.4), masked.z / 12.92, masked.z <= 0.04045),
            final_color.a,
        );
    }

    // AM composites opacity in sRGB space; Bevy's hardware blend is in linear space.
    // Gamma-encode alpha so that the linear-space alpha blend approximates AM's sRGB result.
    // For fully opaque content over black: linear_to_srgb(srgb_to_linear(opacity)) = opacity.
    if final_color.a > 0.001 && final_color.a < 0.999 {
        final_color.a = select(
            pow((final_color.a + 0.055) / 1.055, 2.4),
            final_color.a / 12.92,
            final_color.a <= 0.04045
        );
    }

    if final_color.a < 0.001 {
        discard;
    }
    
    return final_color;
}

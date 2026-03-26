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
    mesh_offset: vec4<f32>,        // (transform_rotation_rad, mirror_bits, scene_width, scene_height)
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
    pixelate_flags: vec4<f32>,         // (enabled, screen_space, scene_scale_x, scene_scale_y)
    pixelate_params1: vec4<f32>,       // (size, stretch_x, stretch_y, angle)
    pixelate_params2: vec4<f32>,       // (vignette, threshold, saturation, 0)
    // Mask blend parameters
    mask_blend: vec4<f32>,             // mask1: (fill_alpha, opacity, stroke_width, mirror_bits)
    mask2_blend: vec4<f32>,            // mask2: (fill_alpha, opacity, stroke_width, mirror_bits)
    // Stretch2 effect (directional UV-space stretch)
    stretch2_params: vec4<f32>,        // (scale, angle_radians, content_only, 0)
    // Solidcolor effect
    solid_color_params: vec4<f32>,     // (r, g, b, blend_mode)
    solid_color_alpha: vec4<f32>,      // (alpha, 0, 0, 0)
    // Second stretch segment effect
    stretch_seg2_params: vec4<f32>,    // (angle_radians, stretch_px, offset_px, smooth_width)
    // Mask1 stretch-segment params (for stretched masks)
    mask1_stretch1_params: vec4<f32>,  // (angle_rad, adj_stretch, offset, smooth)
    mask1_stretch2_params: vec4<f32>,  // (angle_rad, adj_stretch, offset, smooth)
    mask1_stretch_info: vec4<f32>,     // (aspect_w, aspect_h, orig_half_w, orig_half_h)
    // Wavewarp2 effect (波浪歪曲)
    wavewarp2_params1: vec4<f32>,      // (phase, a1_rad, m1_spacing, m2_magnitude)
    wavewarp2_params2: vec4<f32>,      // (a2_rad, damping, damping_space, damping_origin)
    wavewarp2_flags: vec4<f32>,        // (screen_space, enabled, mag_x, mag_y)
    // Mirror effect (镜子)
    mirror_params: vec4<f32>,          // (type_plus_1, blend_mode, alpha, offset)
    // Lift (copy background) effect (复制背景)
    lift_params: vec4<f32>,            // (fill, canvas_width, canvas_height, enabled)
    // Rays (volumetric light rays) effect (射线)
    rays_params1: vec4<f32>,           // (strength, intensity, threshold, quality)
    rays_params2: vec4<f32>,           // (blend, center_x_norm, center_y_norm, enabled)
    rays_threshold_color: vec4<f32>,   // (r, g, b, a) linear
    rays_fill_color: vec4<f32>,        // (r, g, b, a) linear
    // RGB split (chromatic aberration) / RGB 分离
    rgb_split_params: vec4<f32>,       // (offset_x, offset_y, center_channel, mode)
    // Exposure / Gamma effect / 曝光/伽马
    exposure_gamma_params: vec4<f32>,  // (exposure, gamma, offset, enabled)
    // Blend mode / 混合模式
    blend_mode_params: vec4<f32>,      // (mode_id, canvas_w, canvas_h, enabled)
    // ChromaKey (chroma keying) / 色度键
    chromakey_params: vec4<f32>,       // (threshold, feather, defringe, invert)
    chromakey_key_color: vec4<f32>,    // (r, g, b, a) linear
    // Mask 1 linear repeat / 蒙版1线性重复
    mask1_lr_params1: vec4<f32>,       // (count, position_x, position_y, angle_deg)
    mask1_lr_params2: vec4<f32>,       // (offset_x, offset_y, scale, alpha)
    mask1_lr_params3: vec4<f32>,       // (start, end, phase, overlap)
    mask1_lr_params4: vec4<f32>,       // (ease_in, ease_out, 0, shape_invert_alt)
    mask1_lr_params5: vec4<f32>,       // (random_order, seed_lo, seed_hi, 0)
    // Mask 1 second linear repeat (dual) / 蒙版1第二线性重复
    mask1_lr2_params1: vec4<f32>,      // (count, position_x, position_y, angle_deg)
    mask1_lr2_params2: vec4<f32>,      // (offset_x, offset_y, scale, alpha)
    mask1_lr2_params3: vec4<f32>,      // (start, end, phase, overlap)
    mask1_lr2_params4: vec4<f32>,      // (ease_in, ease_out, 0, shape_invert_alt)
    mask1_lr2_params5: vec4<f32>,      // (random_order, seed_lo, seed_hi, 0)
    mask1_repeat_params1: vec4<f32>,   // (count, offset_x_world, offset_y_world, angle_deg)
    mask1_repeat_params2: vec4<f32>,   // (scale, alpha, 0, 0)
    // Mask1 radial repeat params
    mask1_rr_params1: vec4<f32>,       // (count, radius, orientation_deg, start_angle_deg)
    mask1_rr_params2: vec4<f32>,       // (sweep_deg, base_scale, angle_deg, scale)
    mask1_rr_params3: vec4<f32>,       // (alpha, offset_x, offset_y, 0)
    mask1_rr_params4: vec4<f32>,       // (start, end, phase, overlap)
    mask1_rr_params5: vec4<f32>,       // (ease_in, ease_out, shape_invert_alt, seed+random)
    source_flags: vec4<f32>,           // (sampled_from_rtt, 0, 0, 0)
}

@group(2) @binding(0) var<uniform> uniforms: UnifiedEffectUniform;
@group(2) @binding(1) var base_texture: texture_2d<f32>;
@group(2) @binding(2) var base_sampler: sampler;
@group(2) @binding(3) var lift_comp_texture: texture_2d<f32>;
@group(2) @binding(4) var lift_comp_sampler: sampler;
@group(2) @binding(5) var mask_rtt_texture: texture_2d<f32>;
@group(2) @binding(6) var mask_rtt_sampler: sampler;

// Helper: rotate 2D vector by angle
fn rotate_vec(v: vec2<f32>, angle: f32) -> vec2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec2<f32>(
        v.x * c - v.y * s,
        v.x * s + v.y * c
    );
}

// Helper: RGB → HSV conversion (AM compatible)
fn rgb2hsv(c: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    let p = mix(vec4<f32>(c.b, c.g, K.w, K.z), vec4<f32>(c.g, c.b, K.x, K.y), step(c.b, c.g));
    let q = mix(vec4<f32>(p.x, p.y, p.w, c.r), vec4<f32>(c.r, p.y, p.z, p.x), step(p.x, c.r));
    let d = q.x - min(q.w, q.y);
    let e = 1.0e-10;
    return vec3<f32>(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

// Helper: HSV → RGB conversion (AM compatible)
fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(vec3<f32>(c.x, c.x, c.x) + K.xyz) * 6.0 - vec3<f32>(K.w, K.w, K.w));
    return c.z * mix(vec3<f32>(K.x, K.x, K.x), clamp(p - vec3<f32>(K.x, K.x, K.x), vec3<f32>(0.0), vec3<f32>(1.0)), c.y);
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

// Apply wavewarp2 effect (波浪歪曲 / Wave Warp)
// From AM wavewarp2.xml fragment shader.
// Displaces UV coordinates based on a sine wave pattern with damping.
// AM computes offset in acLayerNorm but applies to acScreenNorm, causing
// magnification by (canvas_size / layer_display_size). wavewarp2_flags.zw
// carries per-axis magnification factors from the CPU.
fn apply_wavewarp2(uv: vec2<f32>) -> vec2<f32> {
    let phase = uniforms.wavewarp2_params1.x;
    let a1_rad = uniforms.wavewarp2_params1.y;
    let m1 = uniforms.wavewarp2_params1.z;
    let m2 = uniforms.wavewarp2_params1.w;
    let a2_rad = uniforms.wavewarp2_params2.x;
    let damping_val = uniforms.wavewarp2_params2.y;
    let damping_space_val = uniforms.wavewarp2_params2.z;
    let damping_origin_val = uniforms.wavewarp2_params2.w;
    let screen_space = uniforms.wavewarp2_flags.x > 0.5;
    let mag = vec2<f32>(uniforms.wavewarp2_flags.z, uniforms.wavewarp2_flags.w);

    // AM's acLayerNorm is Y-up (OpenGL FBO convention: y=0 at bottom).
    // Our UV is Y-down (wgpu: v=0 at top). Flip Y to match AM's coordinate space
    // so wave phase computation (dot products with dir1) produces identical results.
    let st = vec2<f32>(uv.x, 1.0 - uv.y);

    // Wave direction vector
    let raw_v = vec2<f32>(cos(a1_rad), -sin(a1_rad));
    let raw_p = dot(st, raw_v);

    // Space damping: modifies wave frequency based on position
    var space_damp = 1.0;
    if damping_space_val < 0.0 {
        space_damp = 1.0 - (clamp(abs(raw_p - damping_origin_val), 0.0, 1.0) * (0.0 - damping_space_val));
    } else if damping_space_val > 0.0 {
        space_damp = 1.0 - ((1.0 - clamp(abs(raw_p - damping_origin_val), 0.0, 1.0)) * damping_space_val);
    }

    let space = m1 * space_damp;

    // Main wave: project position onto direction, scaled by spacing
    let v = vec2<f32>(cos(a1_rad), -sin(a1_rad)) * space;
    let p = dot(st, v);

    // Distance for magnitude damping (guard against division by zero)
    var ddist = 0.0;
    if abs(space) > 0.0001 {
        ddist = abs(p / space);
    }

    // Magnitude damping
    var damp = 1.0;
    if damping_val < 0.0 {
        damp = 1.0 - (clamp(abs(ddist - damping_origin_val), 0.0, 1.0) * (0.0 - damping_val));
    } else if damping_val > 0.0 {
        damp = 1.0 - ((1.0 - clamp(abs(ddist - damping_origin_val), 0.0, 1.0)) * damping_val);
    }

    // Displacement: direction from combined angle, amplitude from m2 * damp / 100
    // AM uses texture2DCv which Y-flips sampling (OpenGL FBO convention),
    // so the Y component of the offset is effectively negated. We use +sin(a2)
    // instead of AM's -sin(a2) to match the effective displacement direction.
    let offs_dir = vec2<f32>(cos(a2_rad), sin(a2_rad)) * (m2 * damp) / 100.0;
    let offs = offs_dir * sin(p + phase * 6.28318) * mag;

    return uv + offs;
}

// Smooth minimum (cubic polynomial) - matches AM's sminCubic
fn smin_cubic(a: f32, b: f32, k: f32) -> f32 {
    let h = max(k - abs(a - b), 0.0) / k;
    return min(a, b) - h * h * h * k * (1.0 / 6.0);
}

// Decode packed mask mirror flags from mask_blend.w / mask2_blend.w.
// bit0 = X mirrored, bit1 = Y mirrored.
fn decode_axis_sign(sign_code_f32: f32) -> vec2<f32> {
    let sign_code = i32(round(sign_code_f32));
    let sign_x = select(1.0, -1.0, (sign_code % 2) == 1);
    let sign_y = select(1.0, -1.0, sign_code >= 2);
    return vec2<f32>(sign_x, sign_y);
}

fn decode_mask_axis_sign(mask_blend: vec4<f32>) -> vec2<f32> {
    return decode_axis_sign(mask_blend.w);
}

// Generic stretch segment: matches AM's stretchsegment.xml exactly.
// Converts UV to scene-normalized space, applies AM's stretch formula, converts back.
// Parameters:
//   params.x = angle (radians)
//   params.y = adjStretch = stretch_raw / 500.0 (scene-norm units)
//   params.z = offset_norm = offset_raw / 1000.0 (scene-norm units)
//   params.w = smooth (raw AM value, 0..1)
// in_width/in_height + cx/cy define the input UV-to-pixel mapping.
// Output always uses orig_width/orig_height for pixel-to-UV conversion.
fn apply_stretch_segment_gen(
    uv: vec2<f32>,
    params: vec4<f32>,
    in_width: f32, in_height: f32,
) -> vec2<f32> {
    let angle = params.x;       // Original AM angle (NOT rotation-compensated)
    let adj_stretch = params.y;
    let offset_norm = params.z;
    let smooth_raw = params.w;

    let orig_width = uniforms.original_size.x;
    let orig_height = uniforms.original_size.y;
    let scene_width = uniforms.mesh_offset.z;
    let scene_height = uniforms.mesh_offset.w;
    let transform_rot = uniforms.mesh_offset.x;
    let axis_sign = decode_axis_sign(uniforms.mesh_offset.y);

    // Convert mesh UV to pixel coords (layer-local, relative to center).
    // Y is flipped: UV.y=0 is top (Bevy), but AM's scene-norm has +Y = up (OpenGL).
    let local_px_x = (uv.x - 0.5) * in_width * axis_sign.x;
    let local_px_y = (0.5 - uv.y) * in_height * axis_sign.y;

    // Rotate local coords to screen space using Bevy's transform rotation.
    // AM's stretch formula operates in screen-normalized space, which is anisotropic
    // (scene_width != scene_height). Rotating the angle alone doesn't account for this.
    let cos_r = cos(transform_rot);
    let sin_r = sin(transform_rot);
    let screen_px_x = local_px_x * cos_r - local_px_y * sin_r;
    let screen_px_y = local_px_x * sin_r + local_px_y * cos_r;

    // Convert to scene-normalized coords (matching AM's st = acScreenNorm - acLayerCenterNorm)
    let st = vec2<f32>(screen_px_x / scene_width, screen_px_y / scene_height);

    // Direction vector (exactly as AM: vec2(1,0) * rot(angle))
    let v = vec2<f32>(cos(angle), sin(angle));

    // Distance along direction + offset (AM formula)
    let dist = dot(st, v) + offset_norm;

    // Smooth cubic min (AM formula)
    let smooth_k = max(0.00001, smooth_raw * adj_stretch);
    let d = smin_cubic(adj_stretch, abs(dist), smooth_k);

    // Displace in scene-norm space along direction
    let displaced_norm = st + v * d * -sign(dist);

    // Convert back to screen pixel coords
    let disp_screen_px_x = displaced_norm.x * scene_width;
    let disp_screen_px_y = displaced_norm.y * scene_height;

    // Rotate back to local space (inverse rotation: rotate by -transform_rot)
    let disp_local_px_x = disp_screen_px_x * cos_r + disp_screen_px_y * sin_r;
    let disp_local_px_y = -disp_screen_px_x * sin_r + disp_screen_px_y * cos_r;

    // Convert to original-image UV (Y flipped back: positive scene-norm → UV < 0.5)
    let disp_uv_px_x = disp_local_px_x * axis_sign.x;
    let disp_uv_px_y = disp_local_px_y * axis_sign.y;
    return vec2<f32>(
        (disp_uv_px_x / orig_width) + 0.5,
        0.5 - (disp_uv_px_y / orig_height)
    );
}

// Single stretch segment (mesh UV → original-image UV)
fn apply_stretch_segment(uv: vec2<f32>) -> vec2<f32> {
    return apply_stretch_segment_gen(
        uv,
        uniforms.stretch_params,
        uniforms.original_size.z, uniforms.original_size.w,
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
    let axis_sign = decode_mask_axis_sign(mask_blend);

    var rel = world_pos - center;
    if abs(mask_rotation) > 0.001 {
        let c = cos(-mask_rotation);
        let s = sin(-mask_rotation);
        rel = vec2<f32>(rel.x * c - rel.y * s, rel.x * s + rel.y * c);
    }
    rel = rel * axis_sign;

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

// Compute mask blend factor for a mask with stretch-segment effects.
// Operates in world/screen space directly: computes the AM stretch-segment
// displacement on the world-relative position to find the "source" position,
// then checks if the source is within the mask's original bounds.
fn compute_ue_mask_blend_factor_stretched(
    world_pos: vec2<f32>,
    mask_params: vec4<f32>,
    mask_rotation: f32,
    mask_type: f32,
    mask_blend: vec4<f32>,
    stretch1: vec4<f32>,
    stretch2: vec4<f32>,
    stretch_info: vec4<f32>,
) -> f32 {
    if mask_type < 0.5 || mask_params.z > 5000.0 {
        return 1.0;
    }

    let center = mask_params.xy;
    let fill_alpha = mask_blend.x;
    let opacity = mask_blend.y;
    let is_exclude = mask_type > 2.5;
    let axis_sign = decode_mask_axis_sign(mask_blend);

    // World-relative coords = screen-relative coords in 2D
    let rel = world_pos - center;

    // Scene dimensions in world units (scaled by fit_scale)
    let scene_w = stretch_info.x;
    let scene_h = stretch_info.y;

    // Convert to scene-normalized coords (same space as AM's stretch formula)
    var st = vec2<f32>(rel.x / scene_w, rel.y / scene_h);

    // Apply stretch-segment displacement(s) in REVERSE order — AM applies effects as
    // sequential render passes, so the LAST effect (seg2) maps the output position first,
    // then the FIRST effect (seg1) maps at the displaced position.
    // This matches the layer shader's apply order in the dual-stretch path.
    if stretch2.y > 0.0001 {
        let angle = stretch2.x;
        let adj_stretch = stretch2.y;
        let offset_norm = stretch2.z;
        let smooth_raw = stretch2.w;
        let v = vec2<f32>(cos(angle), sin(angle));
        let dist = dot(st, v) + offset_norm;
        let smooth_k = max(0.00001, smooth_raw * adj_stretch);
        let d = smin_cubic(adj_stretch, abs(dist), smooth_k);
        st = st + v * d * -sign(dist);
    }
    if stretch1.y > 0.0001 {
        let angle = stretch1.x;
        let adj_stretch = stretch1.y;
        let offset_norm = stretch1.z;
        let smooth_raw = stretch1.w;
        let v = vec2<f32>(cos(angle), sin(angle));
        let dist = dot(st, v) + offset_norm;
        let smooth_k = max(0.00001, smooth_raw * adj_stretch);
        let d = smin_cubic(adj_stretch, abs(dist), smooth_k);
        st = st + v * d * -sign(dist);
    }

    // Convert displaced scene-norm back to world coords
    let disp_world = vec2<f32>(st.x * scene_w, st.y * scene_h);

    // Rotate to mask-local coords to check against original shape bounds
    var disp_local = disp_world;
    if abs(mask_rotation) > 0.001 {
        let c = cos(-mask_rotation);
        let s = sin(-mask_rotation);
        disp_local = vec2<f32>(disp_world.x * c - disp_world.y * s, disp_world.x * s + disp_world.y * c);
    }
    disp_local = disp_local * axis_sign;

    // Check if displaced position falls within the ORIGINAL (un-expanded) mask shape
    // Use SDF with smoothstep for anti-aliasing at the boundary (matches AM's rendered mask AA)
    let orig_half = stretch_info.zw;
    let mask_sdf = max(abs(disp_local.x) - orig_half.x, abs(disp_local.y) - orig_half.y);
    let mask_alpha = fill_alpha * (1.0 - smoothstep(-1.0, 1.0, mask_sdf));

    if is_exclude {
        return 1.0 - opacity * mask_alpha;
    } else {
        return 1.0 - opacity * (1.0 - mask_alpha);
    }
}

// Compute mask blend factor with basic repeat effect.
// Basic repeat: each copy i has offset*i, angle*i, scale^i, alpha decay.
// Offset is pre-converted to mask-local world units on the CPU side.
fn compute_mask_with_basic_repeat(
    world_pos: vec2<f32>,
    mask_params: vec4<f32>,
    mask_rotation: f32,
    mask_type: f32,
    mask_blend: vec4<f32>,
    rp1: vec4<f32>,       // (count, offset_x_world, offset_y_world, angle_deg)
    rp2: vec4<f32>,       // (scale, alpha, 0, 0)
) -> f32 {
    if mask_type < 0.5 || mask_params.z > 5000.0 {
        return 1.0;
    }

    let center = mask_params.xy;
    let half_size = mask_params.zw;
    let fill_alpha = mask_blend.x;
    let opacity = mask_blend.y;
    let sw = mask_blend.z;
    let is_exclude = mask_type > 2.5;
    let is_ellipse = (mask_type > 1.5 && mask_type < 2.5) || mask_type > 3.5;
    let shape_half = max(half_size - sw * 0.5, vec2<f32>(0.001));
    let axis_sign = decode_mask_axis_sign(mask_blend);

    // rel in mask-local frame
    var rel_base = world_pos - center;
    if abs(mask_rotation) > 0.001 {
        let c = cos(-mask_rotation);
        let s = sin(-mask_rotation);
        rel_base = vec2<f32>(rel_base.x * c - rel_base.y * s, rel_base.x * s + rel_base.y * c);
    }
    rel_base = rel_base * axis_sign;

    let rp_count = i32(rp1.x);
    let rp_offset = rp1.yz * axis_sign;           // mask-local world units
    let rp_angle_rad = rp1.w * 3.14159265 / 180.0;
    let rp_scale = rp2.x;
    let rp_alpha = rp2.y;

    var max_mask_alpha = 0.0;

    for (var i = 0; i < rp_count; i = i + 1) {
        let fi = f32(i);

        // AM: cumulative_alpha = 1.0 - i * (1.0 - alpha)
        let cum_alpha = 1.0 - fi * (1.0 - rp_alpha);
        if cum_alpha <= 0.0 {
            continue;
        }

        let cum_offset = rp_offset * fi;
        let cum_angle = rp_angle_rad * fi;
        let cum_scale = pow(rp_scale, fi);

        if abs(cum_scale) < 0.001 {
            continue;
        }

        // Shift and transform in mask-local frame
        var rel_copy = rel_base - cum_offset;

        if abs(cum_angle) > 0.001 {
            let ca = cos(-cum_angle);
            let sa = sin(-cum_angle);
            rel_copy = vec2<f32>(rel_copy.x * ca - rel_copy.y * sa,
                                 rel_copy.x * sa + rel_copy.y * ca);
        }

        let copy_half = shape_half * cum_scale;

        var mask_sdf: f32;
        if is_ellipse {
            let norm = rel_copy / copy_half;
            let r = length(norm);
            mask_sdf = (r - 1.0) * min(copy_half.x, copy_half.y);
        } else {
            mask_sdf = max(abs(rel_copy.x) - copy_half.x, abs(rel_copy.y) - copy_half.y);
        }

        let copy_fill = select(0.0, fill_alpha, mask_sdf < 0.0);
        let aa = min(1.0, sw * 0.5);
        let copy_stroke = select(0.0, 1.0 - smoothstep(sw * 0.5 - aa, sw * 0.5, abs(mask_sdf)), sw > 0.01);
        let copy_mask_alpha = min(max(copy_fill, copy_stroke), 1.0) * cum_alpha;

        max_mask_alpha = max(max_mask_alpha, copy_mask_alpha);
    }

    if is_exclude {
        return 1.0 - opacity * max_mask_alpha;
    } else {
        return 1.0 - opacity * (1.0 - max_mask_alpha);
    }
}

// Compute mask blend factor with linear repeat effect(s).
// Loops over repeat copies, shifting the mask center for each copy,
// and unions (max) their mask contributions.
// Position/offset are pre-converted to mask-local world units on the CPU side.
fn compute_mask_with_linear_repeat(
    world_pos: vec2<f32>,
    mask_params: vec4<f32>,
    mask_rotation: f32,
    mask_type: f32,
    mask_blend: vec4<f32>,
    lr1: vec4<f32>,       // (count, position_x, position_y, angle_deg)
    lr2: vec4<f32>,       // (offset_x, offset_y, scale, alpha)
    lr3: vec4<f32>,       // (start, end, phase, overlap)
    lr4: vec4<f32>,       // (ease_in, ease_out, 0, shape_invert_alt)
    lr5: vec4<f32>,       // (random_order, seed_lo, seed_hi, 0)
    lr2_1: vec4<f32>,     // second repeat params1
    lr2_2: vec4<f32>,     // second repeat params2
    lr2_3: vec4<f32>,     // second repeat params3
    lr2_4: vec4<f32>,     // second repeat params4
    lr2_5: vec4<f32>,     // second repeat params5
) -> f32 {
    if mask_type < 0.5 || mask_params.z > 5000.0 {
        return 1.0;
    }

    let center = mask_params.xy;
    let half_size = mask_params.zw;
    let fill_alpha = mask_blend.x;
    let opacity = mask_blend.y;
    let sw = mask_blend.z;
    let is_exclude = mask_type > 2.5;
    let is_ellipse = (mask_type > 1.5 && mask_type < 2.5) || mask_type > 3.5;
    let shape_half = max(half_size - sw * 0.5, vec2<f32>(0.001));
    let axis_sign = decode_mask_axis_sign(mask_blend);

    // Compute rel in mask-local frame (without repeat displacement)
    var rel_base = world_pos - center;
    if abs(mask_rotation) > 0.001 {
        let c = cos(-mask_rotation);
        let s = sin(-mask_rotation);
        rel_base = vec2<f32>(rel_base.x * c - rel_base.y * s, rel_base.x * s + rel_base.y * c);
    }
    rel_base = rel_base * axis_sign;

    // Parse first repeat params
    let lr1_count = i32(lr1.x);
    let lr1_position = lr1.yz * axis_sign;      // already in mask-local world units
    let lr1_angle_deg = lr1.w;
    let lr1_offset = lr2.xy * axis_sign;
    let lr1_scale = lr2.z;
    let lr1_alpha = lr2.w;
    let lr1_start = lr3.x;
    let lr1_end = lr3.y;
    let lr1_phase = lr3.z;
    let lr1_overlap = lr3.w;
    let lr1_ease_in = lr4.x;
    let lr1_ease_out = lr4.y;
    let lr1_sia = i32(lr4.w);
    let lr1_shape = lr1_sia / 100;
    let lr1_invert = ((lr1_sia % 100) / 10) == 1;
    let lr1_random = lr5.x > 0.5;
    let lr1_rng_lo = bitcast<u32>(lr5.y);
    let lr1_rng_hi = bitcast<u32>(lr5.z);

    // Parse second repeat params
    let lr2_count = i32(lr2_1.x);
    let lr2_enabled = lr2_count > 0;
    let lr2_position = lr2_1.yz * axis_sign;
    let lr2_angle_deg = lr2_1.w;
    let lr2_offset_val = lr2_2.xy * axis_sign;
    let lr2_scale_val = lr2_2.z;
    let lr2_alpha_val = lr2_2.w;
    let lr2_start = lr2_3.x;
    let lr2_end = lr2_3.y;
    let lr2_phase = lr2_3.z;
    let lr2_overlap = lr2_3.w;
    let lr2_ease_in = lr2_4.x;
    let lr2_ease_out = lr2_4.y;
    let lr2_sia = i32(lr2_4.w);
    let lr2_shape = lr2_sia / 100;
    let lr2_invert = ((lr2_sia % 100) / 10) == 1;
    let lr2_random = lr2_5.x > 0.5;
    let lr2_rng_lo_val = bitcast<u32>(lr2_5.y);
    let lr2_rng_hi_val = bitcast<u32>(lr2_5.z);

    var max_mask_alpha = 0.0;

    let n2 = select(1, lr2_count, lr2_enabled);
    for (var j = 0; j < n2; j = j + 1) {
        var d2 = vec2<f32>(0.0, 0.0);
        var copy_scale2 = 1.0;
        var copy_angle2_rad = 0.0;
        var copy_alpha2 = 1.0;

        if lr2_enabled {
            let progress2 = calc_linear_repeat_progress(
                j, lr2_count, lr2_start, lr2_end, lr2_phase, lr2_overlap,
                lr2_shape, lr2_invert, lr2_ease_in, lr2_ease_out,
                lr2_random, lr2_rng_lo_val, lr2_rng_hi_val
            );
            let base2 = progress2.x;
            let interp2 = progress2.y;
            d2 = lr2_position * base2 + lr2_offset_val * interp2;
            copy_scale2 = 1.0 + (lr2_scale_val - 1.0) * interp2;
            copy_angle2_rad = lr2_angle_deg * 3.14159265 / 180.0 * interp2;
            copy_alpha2 = 1.0 + (lr2_alpha_val - 1.0) * interp2;
        }
        if copy_alpha2 < 0.001 || abs(copy_scale2) < 0.001 {
            continue;
        }

        for (var i = 0; i < lr1_count; i = i + 1) {
            let progress1 = calc_linear_repeat_progress(
                i, lr1_count, lr1_start, lr1_end, lr1_phase, lr1_overlap,
                lr1_shape, lr1_invert, lr1_ease_in, lr1_ease_out,
                lr1_random, lr1_rng_lo, lr1_rng_hi
            );
            let base1 = progress1.x;
            let interp1 = progress1.y;
            let d1 = lr1_position * base1 + lr1_offset * interp1;
            let copy_scale1 = 1.0 + (lr1_scale - 1.0) * interp1;
            let copy_angle1_rad = lr1_angle_deg * 3.14159265 / 180.0 * interp1;
            let copy_alpha1 = 1.0 + (lr1_alpha - 1.0) * interp1;

            let combined_alpha = copy_alpha1 * copy_alpha2;
            let combined_scale = copy_scale1 * copy_scale2;

            if combined_alpha < 0.001 || abs(combined_scale) < 0.001 {
                continue;
            }

            // Displacement in mask-local frame (position/offset pre-converted to world units)
            let displacement = d1 + d2;
            let combined_angle = copy_angle1_rad + copy_angle2_rad;

            // Shift rel_base by displacement and apply per-copy rotation/scale
            var rel_copy = rel_base - displacement;

            if abs(combined_angle) > 0.001 {
                let ca = cos(-combined_angle);
                let sa = sin(-combined_angle);
                rel_copy = vec2<f32>(rel_copy.x * ca - rel_copy.y * sa,
                                     rel_copy.x * sa + rel_copy.y * ca);
            }

            let copy_half = shape_half * combined_scale;

            var mask_sdf: f32;
            if is_ellipse {
                let norm = rel_copy / copy_half;
                let r = length(norm);
                mask_sdf = (r - 1.0) * min(copy_half.x, copy_half.y);
            } else {
                mask_sdf = max(abs(rel_copy.x) - copy_half.x, abs(rel_copy.y) - copy_half.y);
            }

            let copy_fill = select(0.0, fill_alpha, mask_sdf < 0.0);
            let aa = min(1.0, sw * 0.5);
            let copy_stroke = select(0.0, 1.0 - smoothstep(sw * 0.5 - aa, sw * 0.5, abs(mask_sdf)), sw > 0.01);
            let copy_mask_alpha = min(max(copy_fill, copy_stroke), 1.0) * combined_alpha;

            max_mask_alpha = max(max_mask_alpha, copy_mask_alpha);
        }
    }

    if is_exclude {
        return 1.0 - opacity * max_mask_alpha;
    } else {
        return 1.0 - opacity * (1.0 - max_mask_alpha);
    }
}

// Compute mask blend factor for SDF mask with radial repeat effect.
// Uses the same radial distribution as the main radial repeat rendering,
// but evaluates an SDF mask shape at each radially-placed copy position.
fn compute_mask_with_radial_repeat(
    world_pos: vec2<f32>,
    mask_params: vec4<f32>,
    mask_rotation: f32,
    mask_type: f32,
    mask_blend: vec4<f32>,
    rr1: vec4<f32>,       // (count, radius, orientation_deg, start_angle_deg)
    rr2: vec4<f32>,       // (sweep_deg, base_scale, angle_deg, scale)
    rr3: vec4<f32>,       // (alpha, offset_x, offset_y, 0)
    rr4: vec4<f32>,       // (start, end, phase, overlap)
    rr5: vec4<f32>,       // (ease_in, ease_out, shape_invert_alt, seed+random)
) -> f32 {
    if mask_type < 0.5 || mask_params.z > 5000.0 {
        return 1.0;
    }

    let center = mask_params.xy;
    let half_size = mask_params.zw;
    let fill_alpha = mask_blend.x;
    let opacity = mask_blend.y;
    let sw = mask_blend.z;
    let is_exclude = mask_type > 2.5;
    let is_ellipse = (mask_type > 1.5 && mask_type < 2.5) || mask_type > 3.5;
    let shape_half = max(half_size - sw * 0.5, vec2<f32>(0.001));
    let axis_sign = decode_mask_axis_sign(mask_blend);

    // Compute rel in mask-local frame
    var rel_base = world_pos - center;
    if abs(mask_rotation) > 0.001 {
        let c = cos(-mask_rotation);
        let s = sin(-mask_rotation);
        rel_base = vec2<f32>(rel_base.x * c - rel_base.y * s, rel_base.x * s + rel_base.y * c);
    }
    rel_base = rel_base * axis_sign;

    // Parse radial repeat params
    let rr_count = max(i32(round(rr1.x)), 0);
    let rr_radius = rr1.y;
    let rr_orientation_deg = rr1.z;
    let rr_start_angle_deg = rr1.w;
    let rr_sweep_deg = rr2.x;
    let rr_base_scale = rr2.y;
    let rr_angle_deg = rr2.z;
    let rr_scale = rr2.w;
    let rr_alpha = rr3.x;
    let rr_offset = vec2<f32>(rr3.y, rr3.z) * axis_sign;
    let rr_start = rr4.x;
    let rr_end = rr4.y;
    let rr_phase = rr4.z;
    let rr_overlap = rr4.w;
    let rr_ease_in = rr5.x;
    let rr_ease_out = rr5.y;
    let rr_sia = i32(rr5.z);
    let rr_shape = rr_sia / 100;
    let rr_invert = (rr_sia / 10) % 10 == 1;
    let rr_seed_raw = rr5.w;
    let rr_random_order = fract(rr_seed_raw) > 0.3;
    let rr_seed = floor(rr_seed_raw);
    let rr_am_seed = u32(15234322.0 + 35432882176.0 * rr_seed);
    let rr_init = rr_am_seed ^ 0xDEECE66Du;
    let rr_init_hi = (((rr_am_seed >> 16u) ^ 5u) & 0xFFFFu);
    let rr_rng_lo = rr_init;
    let rr_rng_hi = rr_init_hi;

    let deg2rad = 3.14159265 / 180.0;
    var max_mask_alpha = 0.0;

    for (var i = 0; i < rr_count; i = i + 1) {
        let progress = calc_linear_repeat_progress(
            i, rr_count, rr_start, rr_end, rr_phase, rr_overlap,
            rr_shape, rr_invert, rr_ease_in, rr_ease_out,
            rr_random_order, rr_rng_lo, rr_rng_hi
        );
        let base_progress = progress.x;
        let interp_progress = progress.y;

        let spread = (rr_start_angle_deg - rr_sweep_deg / 2.0
            + (rr_sweep_deg - rr_sweep_deg / f32(max(rr_count, 1))) * base_progress) * deg2rad;
        let orbit = (rr_orientation_deg + rr_angle_deg * interp_progress) * deg2rad;

        let mix_scale = 1.0 + (rr_scale - 1.0) * interp_progress;
        let copy_alpha = 1.0 + (rr_alpha - 1.0) * interp_progress;

        if copy_alpha < 0.001 || abs(mix_scale) < 0.001 || abs(rr_base_scale) < 0.001 {
            continue;
        }

        // Inverse transform: undo radial placement to get mask-local coords for this copy
        // World coords are Y-up (vs AM pixel coords Y-down), so:
        // - rotation angles are NOT negated (forward uses R(-θ) in world, inverse uses R(θ))
        // - radius vector is (0, +r) not (0, -r) since forward displaced by (0, -r) in world
        var tc = rel_base - rr_offset * interp_progress;
        let cos_s = cos(spread);
        let sin_s = sin(spread);
        tc = vec2<f32>(tc.x * cos_s - tc.y * sin_s, tc.x * sin_s + tc.y * cos_s);
        tc = tc / mix_scale;
        tc = tc + vec2<f32>(0.0, rr_radius);
        let cos_o = cos(orbit);
        let sin_o = sin(orbit);
        tc = vec2<f32>(tc.x * cos_o - tc.y * sin_o, tc.x * sin_o + tc.y * cos_o);
        tc = tc / rr_base_scale;

        let copy_half = shape_half;

        var mask_sdf: f32;
        if is_ellipse {
            let norm = tc / copy_half;
            let r = length(norm);
            mask_sdf = (r - 1.0) * min(copy_half.x, copy_half.y);
        } else {
            mask_sdf = max(abs(tc.x) - copy_half.x, abs(tc.y) - copy_half.y);
        }

        let copy_fill = select(0.0, fill_alpha, mask_sdf < 0.0);
        let aa = min(1.0, sw * 0.5);
        let copy_stroke = select(0.0, 1.0 - smoothstep(sw * 0.5 - aa, sw * 0.5, abs(mask_sdf)), sw > 0.01);
        let copy_mask_alpha = min(max(copy_fill, copy_stroke), 1.0) * copy_alpha;

        max_mask_alpha = max(max_mask_alpha, copy_mask_alpha);
    }

    if is_exclude {
        return 1.0 - opacity * max_mask_alpha;
    } else {
        return 1.0 - opacity * (1.0 - max_mask_alpha);
    }
}

// Compute mask blend factor by sampling an RTT (render-to-texture) mask.
// The mask layer's content was rendered to a texture; we sample its alpha
// to determine inside/outside.
// mask_rtt_bounds: vec4(center_x, center_y, half_w, half_h) in world coords
// mask_rotation: rotation angle in radians
// mask_type: 5.0=include, 6.0=exclude
fn compute_texture_mask_blend(
    world_pos: vec2<f32>,
    mask_rtt_bounds: vec4<f32>,
    mask_rotation: f32,
    mask_type: f32,
) -> f32 {
    let center = mask_rtt_bounds.xy;
    let half_size = mask_rtt_bounds.zw;

    // Transform to mask-local coordinates (undo rotation)
    let rel = world_pos - center;
    let cos_r = cos(-mask_rotation);
    let sin_r = sin(-mask_rotation);
    let local = vec2<f32>(
        rel.x * cos_r - rel.y * sin_r,
        rel.x * sin_r + rel.y * cos_r,
    );

    // Map to UV space [0,1]
    let uv = local / (half_size * 2.0) + 0.5;

    // Out-of-bounds → transparent (no mask)
    if uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 {
        if mask_type > 5.5 {
            return 1.0; // exclude: outside mask bounds = visible
        }
        return 0.0; // include: outside mask bounds = hidden
    }

    // Flip Y for RTT (render target has flipped Y)
    let sample_uv = vec2<f32>(uv.x, 1.0 - uv.y);
    let mask_sample = textureSample(mask_rtt_texture, mask_rtt_sampler, sample_uv);
    let mask_alpha = mask_sample.a;

    if mask_type > 5.5 {
        return 1.0 - mask_alpha; // exclude: invert mask
    }
    return mask_alpha;
}

// Helper: sample RTT mask texture at a given world position with bounds/rotation.
// Returns the mask alpha at that position (0.0 = hidden, 1.0 = visible).
fn sample_texture_mask_at(
    world_pos: vec2<f32>,
    center: vec2<f32>,
    half_size: vec2<f32>,
    mask_rotation: f32,
) -> f32 {
    let rel = world_pos - center;
    let cos_r = cos(-mask_rotation);
    let sin_r = sin(-mask_rotation);
    let local = vec2<f32>(
        rel.x * cos_r - rel.y * sin_r,
        rel.x * sin_r + rel.y * cos_r,
    );
    let uv = local / (half_size * 2.0) + 0.5;
    if uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 {
        return 0.0;
    }
    let sample_uv = vec2<f32>(uv.x, 1.0 - uv.y);
    let mask_sample = textureSample(mask_rtt_texture, mask_rtt_sampler, sample_uv);
    return mask_sample.a;
}

// Texture mask with basic repeat: sample RTT at multiple offset positions.
fn compute_texture_mask_with_basic_repeat(
    world_pos: vec2<f32>,
    mask_rtt_bounds: vec4<f32>,
    mask_rotation: f32,
    mask_type: f32,
    rp1: vec4<f32>,       // (count, offset_x_world, offset_y_world, angle_deg)
    rp2: vec4<f32>,       // (scale, alpha, 0, 0)
) -> f32 {
    let center = mask_rtt_bounds.xy;
    let half_size = mask_rtt_bounds.zw;
    let is_exclude = mask_type > 5.5;

    let rp_count = i32(rp1.x);
    let rp_offset = rp1.yz;
    let rp_angle_rad = rp1.w * 3.14159265 / 180.0;
    let rp_scale = rp2.x;
    let rp_alpha = rp2.y;

    var max_mask_alpha = 0.0;

    for (var i = 0; i < rp_count; i = i + 1) {
        let fi = f32(i);
        let cum_alpha = 1.0 - fi * (1.0 - rp_alpha);
        if cum_alpha <= 0.0 {
            continue;
        }
        let cum_offset = rp_offset * fi;
        let cum_angle = rp_angle_rad * fi;
        let cum_scale = pow(rp_scale, fi);
        if abs(cum_scale) < 0.001 {
            continue;
        }

        // Apply repeat transform: offset the world position, then sample RTT
        let copy_center = center + cum_offset;
        let copy_half = half_size * cum_scale;
        let copy_rotation = mask_rotation + cum_angle;

        let alpha = sample_texture_mask_at(world_pos, copy_center, copy_half, copy_rotation);
        max_mask_alpha = max(max_mask_alpha, alpha * cum_alpha);
    }

    if is_exclude {
        return 1.0 - max_mask_alpha;
    }
    return max_mask_alpha;
}

// Texture mask with linear repeat: sample RTT at linearly displaced positions.
fn compute_texture_mask_with_linear_repeat(
    world_pos: vec2<f32>,
    mask_rtt_bounds: vec4<f32>,
    mask_rotation: f32,
    mask_type: f32,
    lr1: vec4<f32>,  // (count, position.x, position.y, offset.x)
    lr2: vec4<f32>,  // (offset.y, angle_deg, scale, alpha)
    lr3: vec4<f32>,  // (start, end, phase, ease_in)
    lr4: vec4<f32>,  // (ease_out, overlap, invert, shape)
    lr5: vec4<f32>,  // (fill_alpha, blend, 0, 0)
) -> f32 {
    let center = mask_rtt_bounds.xy;
    let half_size = mask_rtt_bounds.zw;
    let is_exclude = mask_type > 5.5;

    let lr_count = i32(lr1.x);
    let lr_position = lr1.yz;
    let lr_offset = vec2<f32>(lr1.w, lr2.x);
    let lr_angle_rad = lr2.y * 3.14159265 / 180.0;
    let lr_scale = lr2.z;
    let lr_alpha = lr2.w;

    var max_mask_alpha = 0.0;

    for (var i = 0; i < lr_count; i = i + 1) {
        let fi = f32(i);
        let cum_alpha = 1.0 - fi * (1.0 - lr_alpha);
        if cum_alpha <= 0.0 {
            continue;
        }

        let cum_offset = (lr_position + lr_offset * fi);
        let cum_angle = lr_angle_rad * fi;
        let cum_scale = pow(lr_scale, fi);
        if abs(cum_scale) < 0.001 {
            continue;
        }

        let copy_center = center + cum_offset;
        let copy_half = half_size * cum_scale;
        let copy_rotation = mask_rotation + cum_angle;

        let alpha = sample_texture_mask_at(world_pos, copy_center, copy_half, copy_rotation);
        max_mask_alpha = max(max_mask_alpha, alpha * cum_alpha);
    }

    if is_exclude {
        return 1.0 - max_mask_alpha;
    }
    return max_mask_alpha;
}

// Compute texture mask blend factor with radial repeat effect.
// Like compute_texture_mask_with_linear_repeat but places copies radially.
fn compute_texture_mask_with_radial_repeat(
    world_pos: vec2<f32>,
    mask_rtt_bounds: vec4<f32>,
    mask_rotation: f32,
    mask_type: f32,
    rr1: vec4<f32>,       // (count, radius, orientation_deg, start_angle_deg)
    rr2: vec4<f32>,       // (sweep_deg, base_scale, angle_deg, scale)
    rr3: vec4<f32>,       // (alpha, offset_x, offset_y, 0)
    rr4: vec4<f32>,       // (start, end, phase, overlap)
    rr5: vec4<f32>,       // (ease_in, ease_out, shape_invert_alt, seed+random)
) -> f32 {
    let center = mask_rtt_bounds.xy;
    let half_size = mask_rtt_bounds.zw;
    let is_exclude = mask_type > 5.5;

    let rr_count = max(i32(round(rr1.x)), 0);
    let rr_radius = rr1.y;
    let rr_orientation_deg = rr1.z;
    let rr_start_angle_deg = rr1.w;
    let rr_sweep_deg = rr2.x;
    let rr_base_scale = rr2.y;
    let rr_angle_deg = rr2.z;
    let rr_scale = rr2.w;
    let rr_alpha = rr3.x;
    let rr_offset = vec2<f32>(rr3.y, rr3.z);
    let rr_start = rr4.x;
    let rr_end = rr4.y;
    let rr_phase = rr4.z;
    let rr_overlap = rr4.w;
    let rr_ease_in = rr5.x;
    let rr_ease_out = rr5.y;
    let rr_sia = i32(rr5.z);
    let rr_shape = rr_sia / 100;
    let rr_invert = (rr_sia / 10) % 10 == 1;
    let rr_seed_raw = rr5.w;
    let rr_random_order = fract(rr_seed_raw) > 0.3;
    let rr_seed = floor(rr_seed_raw);
    let rr_am_seed = u32(15234322.0 + 35432882176.0 * rr_seed);
    let rr_init = rr_am_seed ^ 0xDEECE66Du;
    let rr_init_hi = (((rr_am_seed >> 16u) ^ 5u) & 0xFFFFu);
    let rr_rng_lo = rr_init;
    let rr_rng_hi = rr_init_hi;

    let deg2rad = 3.14159265 / 180.0;
    var max_mask_alpha = 0.0;

    for (var i = 0; i < rr_count; i = i + 1) {
        let progress = calc_linear_repeat_progress(
            i, rr_count, rr_start, rr_end, rr_phase, rr_overlap,
            rr_shape, rr_invert, rr_ease_in, rr_ease_out,
            rr_random_order, rr_rng_lo, rr_rng_hi
        );
        let base_progress = progress.x;
        let interp_progress = progress.y;

        let spread = (rr_start_angle_deg - rr_sweep_deg / 2.0
            + (rr_sweep_deg - rr_sweep_deg / f32(max(rr_count, 1))) * base_progress) * deg2rad;
        let orbit = (rr_orientation_deg + rr_angle_deg * interp_progress) * deg2rad;

        let mix_scale = 1.0 + (rr_scale - 1.0) * interp_progress;
        let copy_alpha = 1.0 + (rr_alpha - 1.0) * interp_progress;

        if copy_alpha < 0.001 || abs(mix_scale) < 0.001 || abs(rr_base_scale) < 0.001 {
            continue;
        }

        // Forward transform: compute copy center and half_size in world coords (Y-up)
        // AM's forward rotation is CW, which in world coords (Y-up) is R(-θ)
        let copy_scale = mix_scale * rr_base_scale;
        let copy_half = half_size * copy_scale;
        let copy_rotation = mask_rotation - spread - orbit;
        // Copy center: translate by (0, -radius) in world then rotate by -spread, then offset
        let cos_s = cos(-spread);
        let sin_s = sin(-spread);
        let r_vec = vec2<f32>(0.0, -rr_radius);
        let rotated_r = vec2<f32>(r_vec.x * cos_s - r_vec.y * sin_s,
                                   r_vec.x * sin_s + r_vec.y * cos_s);
        let copy_center = center + rotated_r * mix_scale + rr_offset * interp_progress;

        let alpha = sample_texture_mask_at(world_pos, copy_center, copy_half, copy_rotation);
        max_mask_alpha = max(max_mask_alpha, alpha * copy_alpha);
    }

    if is_exclude {
        return 1.0 - max_mask_alpha;
    }
    return max_mask_alpha;
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
        // Texture-based mask (embedScene/group mask with RTT)
        let is_texture_mask = mask1_type > 4.5;
        if is_texture_mask {
            // Check if mask has repeat effects
            let has_mask_basic_repeat = uniforms.mask1_repeat_params1.x > 0.5;
            let has_mask_linear_repeat = uniforms.mask1_lr_params1.x > 0.5;
            let has_mask_radial_repeat = uniforms.mask1_rr_params1.x > 0.5;
            if has_mask_basic_repeat {
                factor *= compute_texture_mask_with_basic_repeat(
                    world_pos,
                    uniforms.mask_params,
                    mask1_rotation,
                    mask1_type,
                    uniforms.mask1_repeat_params1,
                    uniforms.mask1_repeat_params2,
                );
            } else if has_mask_radial_repeat {
                factor *= compute_texture_mask_with_radial_repeat(
                    world_pos,
                    uniforms.mask_params,
                    mask1_rotation,
                    mask1_type,
                    uniforms.mask1_rr_params1,
                    uniforms.mask1_rr_params2,
                    uniforms.mask1_rr_params3,
                    uniforms.mask1_rr_params4,
                    uniforms.mask1_rr_params5,
                );
            } else if has_mask_linear_repeat {
                factor *= compute_texture_mask_with_linear_repeat(
                    world_pos,
                    uniforms.mask_params,
                    mask1_rotation,
                    mask1_type,
                    uniforms.mask1_lr_params1,
                    uniforms.mask1_lr_params2,
                    uniforms.mask1_lr_params3,
                    uniforms.mask1_lr_params4,
                    uniforms.mask1_lr_params5,
                );
            } else {
                factor *= compute_texture_mask_blend(
                    world_pos,
                    uniforms.mask_params,
                    mask1_rotation,
                    mask1_type,
                );
            }
        } else {
        // Check if mask has repeat effects
        let has_mask_basic_repeat = uniforms.mask1_repeat_params1.x > 0.5;
        let has_mask_linear_repeat = uniforms.mask1_lr_params1.x > 0.5;
        let has_mask_radial_repeat = uniforms.mask1_rr_params1.x > 0.5;
        // Use stretch-aware evaluation if mask has stretch-segment effects
        let has_mask_stretch = uniforms.mask1_stretch1_params.y > 0.0001
                            || uniforms.mask1_stretch2_params.y > 0.0001;
        if has_mask_basic_repeat {
            factor *= compute_mask_with_basic_repeat(
                world_pos,
                uniforms.mask_params,
                mask1_rotation,
                mask1_type,
                uniforms.mask_blend,
                uniforms.mask1_repeat_params1,
                uniforms.mask1_repeat_params2,
            );
        } else if has_mask_radial_repeat {
            factor *= compute_mask_with_radial_repeat(
                world_pos,
                uniforms.mask_params,
                mask1_rotation,
                mask1_type,
                uniforms.mask_blend,
                uniforms.mask1_rr_params1,
                uniforms.mask1_rr_params2,
                uniforms.mask1_rr_params3,
                uniforms.mask1_rr_params4,
                uniforms.mask1_rr_params5,
            );
        } else if has_mask_linear_repeat {
            factor *= compute_mask_with_linear_repeat(
                world_pos,
                uniforms.mask_params,
                mask1_rotation,
                mask1_type,
                uniforms.mask_blend,
                uniforms.mask1_lr_params1,
                uniforms.mask1_lr_params2,
                uniforms.mask1_lr_params3,
                uniforms.mask1_lr_params4,
                uniforms.mask1_lr_params5,
                uniforms.mask1_lr2_params1,
                uniforms.mask1_lr2_params2,
                uniforms.mask1_lr2_params3,
                uniforms.mask1_lr2_params4,
                uniforms.mask1_lr2_params5,
            );
        } else if has_mask_stretch {
            factor *= compute_ue_mask_blend_factor_stretched(
                world_pos,
                uniforms.mask_params,
                mask1_rotation,
                mask1_type,
                uniforms.mask_blend,
                uniforms.mask1_stretch1_params,
                uniforms.mask1_stretch2_params,
                uniforms.mask1_stretch_info,
            );
        } else {
            factor *= compute_ue_mask_blend_factor(
                world_pos,
                uniforms.mask_params,
                mask1_rotation,
                mask1_type,
                uniforms.mask_blend,
            );
        }
        } // close is_texture_mask else
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
    let input_alpha = input_color.a;
    if input_alpha <= 0.0001 {
        return input_color;
    }

    // Replace-color should operate on straight RGB. Intrinsic/group RTT inputs can arrive with
    // RGB already attenuated by alpha, so detect that case and un-premultiply before matching.
    let max_input = max(max(input_color.r, input_color.g), input_color.b);
    let looks_premultiplied = input_alpha > 0.001 && max_input <= input_alpha + 0.001;
    let input_rgb_linear = clamp(
        select(input_color.rgb, input_color.rgb / input_alpha, looks_premultiplied),
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );
    let src_color = linear_to_srgb(input_rgb_linear);
    let tex_color = vec4<f32>(src_color * input_alpha, input_alpha);
    let old_color = uniforms.replace_old_color;
    let new_color = uniforms.replace_new_color;

    let rgb2yuv = mat3x3<f32>(
        vec3<f32>(0.299, -0.14713, 0.615),
        vec3<f32>(0.587, -0.28886, -0.51499),
        vec3<f32>(0.114, 0.436, -0.10001),
    );
    let yuv2rgb = mat3x3<f32>(
        vec3<f32>(1.0, 1.0, 1.0),
        vec3<f32>(0.0, -0.39465, 2.03211),
        vec3<f32>(1.13983, -0.58060, 0.0),
    );

    let old_color_yuv = old_color.rgb * rgb2yuv;
    let src_color_yuv = src_color * rgb2yuv;
    var diff_yuv = abs(old_color_yuv - src_color_yuv);
    diff_yuv.x *= 0.5;
    diff_yuv.y *= 4.0;
    diff_yuv.z *= 4.0;
    let distance = length(diff_yuv);

    let eff_feather = max(feather, 0.0005);
    let low = max(threshold - eff_feather, 0.0);
    let high = min(threshold + eff_feather, 4.0);
    let low_edge = min(low, high - 0.0005);
    var replace_factor = 1.0 - smoothstep(low_edge, high, distance);
    if feather <= 0.001 {
        replace_factor = select(0.0, 1.0, distance <= threshold);
    }
    replace_factor *= new_color.a * effect_alpha;

    var new_color_adj = new_color.rgb / max(new_color.a, 0.001);
    if lock_luminance {
        var new_color_yuv = new_color_adj * rgb2yuv;
        new_color_yuv.x = src_color_yuv.x;
        new_color_adj = new_color_yuv * yuv2rgb;
    } else {
        new_color_adj = src_color - (old_color.rgb / max(old_color.a, 0.001)) + new_color_adj;
    }

    let replaced = mix(src_color, new_color_adj, replace_factor);
    let final_srgb = mix(tex_color, vec4<f32>(replaced, 1.0) * input_alpha, replace_factor);

    if final_srgb.a > 0.001 {
        let straight_linear = srgb_to_linear(final_srgb.rgb / final_srgb.a);
        return vec4<f32>(straight_linear * final_srgb.a, final_srgb.a);
    }

    return vec4<f32>(0.0);
}

// ChromaKey (chroma keying) effect / 色度键效果
// chromakey_params: (threshold, feather, defringe, invert)
// chromakey_key_color: linear RGBA key color
fn apply_chromakey(input_color: vec4<f32>) -> vec4<f32> {
    let threshold = uniforms.chromakey_params.x;
    let feather = uniforms.chromakey_params.y;
    let do_defringe = uniforms.chromakey_params.z > 0.5;
    let do_invert = uniforms.chromakey_params.w > 0.5;

    let key_color = uniforms.chromakey_key_color;

    // Convert texture from linear to sRGB (AM works in sRGB/gamma space)
    let tex_srgb = vec4<f32>(linear_to_srgb(input_color.rgb), input_color.a);

    // Un-premultiply alpha
    var src_color: vec3<f32>;
    if tex_srgb.a > 0.0001 {
        src_color = tex_srgb.rgb / tex_srgb.a;
    } else {
        if do_invert {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
        return input_color;
    }

    // Key color is in linear; convert to sRGB for comparison
    let key_srgb = vec3<f32>(
        linear_to_srgb_ch(key_color.r),
        linear_to_srgb_ch(key_color.g),
        linear_to_srgb_ch(key_color.b),
    );

    // Chroma distance in YCbCr space (CbCr only, luminance-independent)
    // AM's chromakey matches by hue/saturation, ignoring brightness differences
    let key_cb = dot(key_srgb, vec3<f32>(-0.14713, -0.28886, 0.436));
    let key_cr = dot(key_srgb, vec3<f32>(0.615, -0.51499, -0.10001));
    let src_cb = dot(src_color, vec3<f32>(-0.14713, -0.28886, 0.436));
    let src_cr = dot(src_color, vec3<f32>(0.615, -0.51499, -0.10001));
    let diff = distance(vec2<f32>(key_cb, key_cr), vec2<f32>(src_cb, src_cr));

    // Smoothstep: p=1 when close match, p=0 when far
    let eff_feather = max(feather, 0.0005);
    let b = max(threshold - eff_feather, 0.0);
    let a = min(threshold + eff_feather, 4.0);
    let low_edge = min(b, a - 0.0005);
    var p = 1.0 - smoothstep(low_edge, a, diff);

    if do_invert {
        p = 1.0 - p;
    }

    // p = mask value: 1.0 = fully keyed (transparent), 0.0 = fully opaque
    let new_alpha = tex_srgb.a * (1.0 - p);

    var result_rgb = src_color;
    // Defringe: suppress key color spill at semi-transparent edges
    if do_defringe && p > 0.01 && new_alpha > 0.001 {
        let key_lum = dot(key_srgb, vec3<f32>(0.299, 0.587, 0.114));
        let desat = vec3<f32>(key_lum, key_lum, key_lum);
        // At edges (partial p), desaturate the key color contribution
        result_rgb = mix(src_color, mix(src_color, desat, p), min(p * 2.0, 1.0));
    }

    // Re-premultiply and convert back to linear
    let result_premul = result_rgb * new_alpha;
    return vec4<f32>(srgb_to_linear(result_premul), new_alpha);
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
    } else if count == 1 {
        // Keep count=1 aligned with the CPU repeat implementation:
        // there is a single base copy, not an extra displaced clone.
        base_progress = 0.0;
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

// Apply volumetric light rays (god rays) effect.
// Samples along radial directions from center, accumulating brightness above threshold.
// 射线效果：从中心沿径向采样，累积亮度超过阈值的像素。
// AM operates in sRGB/gamma space (GLES 2.0 without sRGB framebuffers).
// Our textures are Rgba8UnormSrgb, so textureSample returns linear values.
// Convert linear↔sRGB to match AM's color-space behavior.
fn linear_to_srgb_ch(c: f32) -> f32 {
    return pow(clamp(c, 0.0, 1.0), 1.0 / 2.2);
}
fn srgb_to_linear_ch(c: f32) -> f32 {
    return pow(clamp(c, 0.0, 1.0), 2.2);
}
fn linear_to_srgb3(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(linear_to_srgb_ch(c.x), linear_to_srgb_ch(c.y), linear_to_srgb_ch(c.z));
}
fn srgb_to_linear3(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(srgb_to_linear_ch(c.x), srgb_to_linear_ch(c.y), srgb_to_linear_ch(c.z));
}

fn apply_rays(base_color: vec4<f32>, uv: vec2<f32>) -> vec4<f32> {
    let strength = uniforms.rays_params1.x;
    let intensity = uniforms.rays_params1.y;
    let threshold = uniforms.rays_params1.z;
    let quality = uniforms.rays_params1.w;
    let blend = uniforms.rays_params2.x;
    let center = vec2<f32>(uniforms.rays_params2.y, uniforms.rays_params2.z);

    // threshold_color and fill_color are passed in sRGB space (matching AM)
    let threshold_color = uniforms.rays_threshold_color.rgb;
    let fill_color_srgb = vec4<f32>(uniforms.rays_fill_color.rgb, uniforms.rays_fill_color.a);

    let luminance_weight = vec3<f32>(0.2126, 0.7152, 0.0722);

    let orig_w = uniforms.original_size.x;
    let orig_h = uniforms.original_size.y;
    let texel_size = vec2<f32>(1.0 / orig_w, 1.0 / orig_h);

    let v = uv - center;
    let speed = length(vec2<f32>(strength / 2.0) / texel_size) * length(uv - center);
    let n_samples = i32(clamp(quality, 2.0, 800.0));

    let vnorm = normalize(v) * texel_size * speed;

    // Aspect ratio correction (matches AM's acScreenSize-based offsetScale)
    var offset_scale = vec2<f32>(1.0);
    if orig_h > orig_w {
        offset_scale.x *= orig_w / orig_h;
    } else {
        offset_scale.y *= orig_h / orig_w;
    }

    // Convert base color to sRGB for calculations (AM works in gamma space)
    let base_srgb = vec4<f32>(linear_to_srgb3(base_color.rgb), base_color.a);

    var out_color = vec4<f32>(0.0);
    for (var i = 1; i < n_samples; i++) {
        let p = f32(i) / f32(n_samples - 1);
        var offs = vnorm * p;
        offs *= offset_scale;

        let sample_pos = uv - offs;
        // Sample with clamp-to-edge (matches AM's texture2DCv behavior)
        let tex_linear = textureSample(base_texture, base_sampler, sample_pos);
        let tex_srgb = vec4<f32>(linear_to_srgb3(tex_linear.rgb), tex_linear.a);

        let luminance = dot(tex_srgb.rgb - threshold_color, luminance_weight);
        if luminance > threshold {
            out_color += mix(tex_srgb, fill_color_srgb, blend) * (1.0 - p);
        }
    }

    // Final composition in sRGB space, then convert back to linear
    let result_srgb = base_srgb + out_color * intensity / f32(n_samples);
    return vec4<f32>(srgb_to_linear3(result_srgb.rgb), result_srgb.a);
}

/// Apply RGB split (chromatic aberration) effect.
/// 应用 RGB 分离（色差）效果
/// Samples texture at offset UVs to separate R/G/B channels.
// Sample texture with transparent fallback for out-of-bounds UVs
// Used by RGB split to avoid edge clamping artifacts
fn sample_transparent(uv: vec2<f32>) -> vec4<f32> {
    if uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    return textureSample(base_texture, base_sampler, uv);
}

fn apply_rgb_split(uv: vec2<f32>) -> vec4<f32> {
    let params = uniforms.rgb_split_params;
    let offset = params.xy;
    let center_channel = i32(params.z);
    let mode = i32(params.w);

    let color_mid = sample_transparent(uv);
    let color_low = sample_transparent(uv - offset);
    let color_high = sample_transparent(uv + offset);

    // Recombine channels: the center channel stays from color_mid
    var out_color: vec4<f32>;
    if center_channel == 0 {
        out_color = vec4<f32>(color_mid.r, color_low.g, color_high.b, 1.0);
    } else if center_channel == 1 {
        out_color = vec4<f32>(color_low.r, color_mid.g, color_high.b, 1.0);
    } else {
        out_color = vec4<f32>(color_low.r, color_high.g, color_mid.b, 1.0);
    }

    let luminance_weighting = vec3<f32>(0.2126, 0.7152, 0.0722);

    if mode == 0 {
        // Mask: multiply by center alpha
        return out_color * color_mid.a;
    } else if mode == 1 {
        // Luma: luminance-based compositing
        let r = vec4<f32>(out_color.r, 0.0, 0.0, out_color.r);
        let g = vec4<f32>(0.0, out_color.g, 0.0, out_color.g);
        let b = vec4<f32>(0.0, 0.0, out_color.b, out_color.b);
        let m = (r + g + b) / 3.0;
        var c = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        let lum = dot(luminance_weighting * color_mid.rgb, vec3<f32>(1.0));
        let l = vec4<f32>(lum, lum, lum, lum);
        c = m + (c * (1.0 - m.a));
        c = l + (c * (1.0 - l.a));
        return c;
    } else if mode == 2 {
        // Light: keep center alpha
        return vec4<f32>(out_color.rgb, color_mid.a);
    } else {
        // Dark: average alpha
        return out_color * ((color_low.a + color_mid.a + color_high.a) / 3.0);
    }
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
    var stretch_edge_alpha: f32 = 1.0;
    
    // Discard fragments in expansion area when no expansion-capable effect is active
    let wavewarp2_enabled = uniforms.wavewarp2_flags.y > 0.5;
    let mirror_enabled = uniforms.mirror_params.x > 0.5;
    let rgb_split_active = uniforms.rgb_split_params.w >= -0.5;
    if !pixelate_enabled && !stretch_enabled && !repeat_enabled && !linear_repeat_enabled && !lr2_enabled && !rr_enabled && !wavewarp2_enabled && !mirror_enabled && !rgb_split_active
        && (mesh.uv.x < 0.0 || mesh.uv.x > 1.0 || mesh.uv.y < 0.0 || mesh.uv.y > 1.0) {
        discard;
    }
    
    
    // Apply pixelate effect FIRST (before stretch) to match AM's sequential render pipeline.
    // In AM, pixelate runs AFTER stretch as a separate render pass, sampling the stretch
    // output at grid-snapped screen positions.  In our single-pass shader, the equivalent
    // UV lookup is:  original[ stretch_inv( pixelate_snap(P) ) ]
    // i.e. snap the SCREEN position first, then apply stretch displacement.
    // Using original_size.zw (= mesh dimensions, which include stretch expansion when active)
    // ensures correct UV↔pixel mapping for the current mesh geometry.
    var pixelate_dist_center = 0.0;
    if pixelate_enabled {
        let display_size = vec2<f32>(uniforms.original_size.z, uniforms.original_size.w);

        // Cell size in layer pixels (= inner-scene pixels for 1:1 layers)
        let size_vec = vec2<f32>(
            pixelate_size * pixelate_stretch.x,
            pixelate_size * pixelate_stretch.y
        );

        // Position in pixels relative to center (mesh.uv = screen position)
        let dp = (mesh.uv - vec2<f32>(0.5)) * display_size;

        // Convert to AM's coordinate convention (Y negated: GL Y-up → WebGPU Y-down)
        var st_am = vec2<f32>(dp.x, -dp.y);

        // Apply rotation: pixelate angle adjusted for parent rotation
        let parent_rotation = uniforms.pixelate_params2.w;
        let total_angle = pixelate_angle - parent_rotation;
        let cos_a = cos(total_angle);
        let sin_a = sin(total_angle);
        var st_rot = vec2<f32>(
            cos_a * st_am.x - sin_a * st_am.y,
            sin_a * st_am.x + cos_a * st_am.y
        );

        // Find position within pixel cell (true modulo for negative values)
        var pos_in_pixel = st_rot - floor(st_rot / size_vec) * size_vec;

        // Center, rotate back, un-center (AM's posInPixel adjustment)
        pos_in_pixel -= size_vec * 0.5;
        pos_in_pixel = vec2<f32>(
            cos_a * pos_in_pixel.x + sin_a * pos_in_pixel.y,
            -sin_a * pos_in_pixel.x + cos_a * pos_in_pixel.y
        );
        pos_in_pixel += size_vec * 0.5;

        // Distance from pixel center (for vignette)
        pixelate_dist_center = smoothstep(0.0, 1.0, length((pos_in_pixel / size_vec) - vec2<f32>(0.5)));

        // Snap to cell center in AM coords, then convert back to UV
        let snapped_am = st_am - pos_in_pixel + size_vec * 0.5;
        let snapped_dp = vec2<f32>(snapped_am.x, -snapped_am.y);
        sample_uv = snapped_dp / display_size + vec2<f32>(0.5);

        // Discard out-of-bounds only when no stretch follows (stretch has its own bounds check)
        if !stretch_enabled {
            if sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0 {
                discard;
            }
        }
    }
    
    // Apply stretch segment effect (after pixelate snap).
    // sample_uv is the pixelate-snapped screen position (or mesh.uv if no pixelate).
    // Stretch computes displacement at this snapped position, matching AM's pipeline:
    //   AM: original[ stretch1( stretch2( pixelate_snap(P) ) ) ]
    if stretch_enabled {
        let seg2_stretch = uniforms.stretch_seg2_params.y;
        let has_seg2 = abs(seg2_stretch) > 0.001;

        if has_seg2 {
            // Dual stretch: apply seg2 at (snapped) screen position, then seg1 at result.
            sample_uv = apply_stretch_segment_gen(
                sample_uv,
                uniforms.stretch_seg2_params,
                uniforms.original_size.z, uniforms.original_size.w,
            );
            sample_uv = apply_stretch_segment_gen(
                sample_uv,
                uniforms.stretch_params,
                uniforms.original_size.x, uniforms.original_size.y,
            );
        } else {
            // Single stretch
            sample_uv = apply_stretch_segment(sample_uv);
        }
        
        // Soft edge AA at stretch boundaries using screen-space derivatives.
        // AM renders shapes via NanoVG SDF with feather-based smoothstep centered on the edge.
        // We approximate by fading alpha symmetrically around UV boundary [0,1]:
        // smoothstep(-aa, +aa, uv) = 0.5 at uv=0 (shape edge), fading both inward and outward.
        let fw = fwidth(sample_uv);
        let aa_half = fw * 2.0;
        stretch_edge_alpha = smoothstep(-aa_half.x, aa_half.x, sample_uv.x)
                           * smoothstep(-aa_half.x, aa_half.x, 1.0 - sample_uv.x)
                           * smoothstep(-aa_half.y, aa_half.y, sample_uv.y)
                           * smoothstep(-aa_half.y, aa_half.y, 1.0 - sample_uv.y);
        if stretch_edge_alpha < 0.001 {
            discard;
        }
        if sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0 {
            discard;
        }
        // Clamp to valid range for texture sampling
        sample_uv = clamp(sample_uv, vec2<f32>(0.0), vec2<f32>(1.0));
    }
    
    // Apply stretch2 effect (directional stretch)
    let stretch2_scale = uniforms.stretch2_params.x;
    if stretch2_scale > 0.001 && abs(stretch2_scale - 1.0) > 0.0001 {
        sample_uv = apply_stretch2(sample_uv);
        if sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0 {
            discard;
        }
        sample_uv = clamp(sample_uv, vec2<f32>(0.0), vec2<f32>(1.0));
    }
    
    // Apply wavewarp2 effect (波浪歪曲)
    if wavewarp2_enabled {
        let warped = apply_wavewarp2(sample_uv);
        // Discard fragments where displaced UV falls outside texture bounds
        // (mesh is expanded beyond content; these pixels should be transparent)
        if warped.x < 0.0 || warped.x > 1.0 || warped.y < 0.0 || warped.y > 1.0 {
            discard;
        }
        sample_uv = warped;
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
        
        // AM composites in sRGB space; accumulate in sRGB premultiplied alpha
        let rp_gamma = vec3<f32>(2.2);
        let rp_inv_gamma = vec3<f32>(1.0 / 2.2);
        var acc_srgb = vec4<f32>(0.0);
        
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
                
                // Composite in sRGB premultiplied alpha (matches AM's Canvas)
                let final_a = copy_color.a * cumulative_alpha;
                let copy_srgb = pow(copy_color.rgb, rp_inv_gamma);
                let premult = vec4<f32>(copy_srgb * final_a, final_a);
                acc_srgb = premult + acc_srgb * (1.0 - final_a);
            }
        }
        
        // Convert back to premultiplied linear color for downstream repeat compositing.
        if acc_srgb.a > 0.001 {
            tex_color = vec4<f32>(pow(acc_srgb.rgb / acc_srgb.a, rp_gamma) * acc_srgb.a, acc_srgb.a);
        } else {
            tex_color = vec4<f32>(0.0);
        }
    } else if linear_repeat_enabled {
        // Linear repeat effect: render multiple copies arranged in a line
        // pixel_coord is Y-down (matching AM convention), no Y-flips needed.
        let orig_width = uniforms.original_size.x;
        let orig_height = uniforms.original_size.y;
        
        let center = vec2<f32>(0.5, 0.5);
        let pixel_coord = (sample_uv - center) * vec2<f32>(orig_width, orig_height);
        
        // AM composites in sRGB space; accumulate in sRGB premultiplied alpha
        let lr_gamma = vec3<f32>(2.2);
        let lr_inv_gamma = vec3<f32>(1.0 / 2.2);
        var acc_lr_srgb = vec4<f32>(0.0);
        
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
                    
                    // Convert to sRGB for AM-compatible compositing
                    var copy_srgb = pow(copy_color.rgb, lr_inv_gamma);
                    
                    // Color blending from effect 1 (in sRGB space)
                    if linear_repeat_blend > 0.001 {
                        let base_srgb = pow(uniforms.color.rgb, lr_inv_gamma);
                        let fill_srgb = pow(uniforms.linear_repeat_fill_color.rgb, lr_inv_gamma);
                        var should_blend = true;
                        if linear_repeat_color_alt && (i % 2 == 1) {
                            should_blend = false;
                        }
                        if should_blend {
                            var start_color = base_srgb;
                            var end_color: vec3<f32>;
                            if linear_repeat_blend <= 1.0 {
                                end_color = mix(base_srgb, fill_srgb, linear_repeat_blend);
                            } else {
                                start_color = mix(base_srgb, fill_srgb, linear_repeat_blend - 1.0);
                                end_color = fill_srgb;
                            }
                            copy_srgb = mix(start_color, end_color, interp_progress);
                        }
                    }
                    
                    // Color blending from effect 2 (in sRGB space)
                    if lr2_enabled && lr2_blend > 0.001 {
                        let fill_srgb2 = pow(uniforms.linear_repeat2_fill_color.rgb, lr_inv_gamma);
                        var should_blend2 = true;
                        if lr2_color_alt && (j % 2 == 1) {
                            should_blend2 = false;
                        }
                        if should_blend2 {
                            var start_color2 = copy_srgb;
                            var end_color2: vec3<f32>;
                            if lr2_blend <= 1.0 {
                                end_color2 = mix(copy_srgb, fill_srgb2, lr2_blend);
                            } else {
                                start_color2 = mix(copy_srgb, fill_srgb2, lr2_blend - 1.0);
                                end_color2 = fill_srgb2;
                            }
                            copy_srgb = mix(start_color2, end_color2, interp2);
                        }
                    }
                    
                    // Composite in sRGB premultiplied alpha (matches AM's Canvas)
                    let final_a = copy_color.a * combined_alpha;
                    let premult = vec4<f32>(copy_srgb * final_a, final_a);
                    acc_lr_srgb = premult + acc_lr_srgb * (1.0 - final_a);
                }
            }
        }
        
        // Convert back to premultiplied linear color for downstream repeat compositing.
        if acc_lr_srgb.a > 0.001 {
            tex_color = vec4<f32>(pow(acc_lr_srgb.rgb / acc_lr_srgb.a, lr_gamma) * acc_lr_srgb.a, acc_lr_srgb.a);
        } else {
            tex_color = vec4<f32>(0.0);
        }
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

    // Apply RGB split (chromatic aberration) effect / RGB 分离效果
    // Uses mode >= 0 as enabled flag (-1.0 in .w = disabled)
    let rgb_split_mode_raw = uniforms.rgb_split_params.w;
    let rgb_split_enabled = rgb_split_mode_raw >= -0.5;

    // Apply lift (copy background) effect: blend layer content with background composite.
    // lift_params = (fill, canvas_width, canvas_height, enabled)
    let lift_enabled = uniforms.lift_params.w > 0.5;
    var lift_skip_color_tint = false;

    if lift_enabled {
        let lift_fill = uniforms.lift_params.x;
        let lift_canvas_w = uniforms.lift_params.y;
        let lift_canvas_h = uniforms.lift_params.z;
        let screen_uv = vec2<f32>(
            (mesh.world_position.x + lift_canvas_w / 2.0) / lift_canvas_w,
            (lift_canvas_h / 2.0 - mesh.world_position.y) / lift_canvas_h
        );

        if rgb_split_enabled {
            // Lift + RGB-split: AM applies lift first, then rgb-split on the composite.
            // Sample composite texture at 3 offset screen positions for RGB channel split.
            let offset = uniforms.rgb_split_params.xy;
            let center_channel = i32(uniforms.rgb_split_params.z);
            let mode = i32(uniforms.rgb_split_params.w);
            // Use original texture size (not expanded mesh size) for UV-to-screen conversion.
            // RGB-split offset is in texture UV space (0-1), and orig_size maps texture to world.
            let orig_w = uniforms.original_size.x;
            let orig_h = uniforms.original_size.y;
            let screen_offset = vec2<f32>(
                offset.x * orig_w / lift_canvas_w,
                offset.y * orig_h / lift_canvas_h
            );
            let color_mid = textureSample(lift_comp_texture, lift_comp_sampler, screen_uv);
            let color_low = textureSample(lift_comp_texture, lift_comp_sampler, screen_uv - screen_offset);
            let color_high = textureSample(lift_comp_texture, lift_comp_sampler, screen_uv + screen_offset);

            var out_color: vec4<f32>;
            if center_channel == 0 {
                out_color = vec4<f32>(color_mid.r, color_low.g, color_high.b, 1.0);
            } else if center_channel == 1 {
                out_color = vec4<f32>(color_low.r, color_mid.g, color_high.b, 1.0);
            } else {
                out_color = vec4<f32>(color_low.r, color_high.g, color_mid.b, 1.0);
            }

            let luminance_weighting = vec3<f32>(0.2126, 0.7152, 0.0722);
            if mode == 0 {
                tex_color = vec4<f32>(out_color.rgb, 1.0) * color_mid.a;
            } else if mode == 1 {
                let r = vec4<f32>(out_color.r, 0.0, 0.0, out_color.r);
                let g = vec4<f32>(0.0, out_color.g, 0.0, out_color.g);
                let b = vec4<f32>(0.0, 0.0, out_color.b, out_color.b);
                let m = (r + g + b) / 3.0;
                var c = vec4<f32>(0.0, 0.0, 0.0, 0.0);
                let lum = dot(luminance_weighting * color_mid.rgb, vec3<f32>(1.0));
                let l = vec4<f32>(lum, lum, lum, lum);
                c = m + (c * (1.0 - m.a));
                c = l + (c * (1.0 - l.a));
                tex_color = vec4<f32>(c.rgb, c.a);
            } else if mode == 2 {
                tex_color = vec4<f32>(out_color.rgb, color_mid.a);
            } else {
                let avg_a = (color_low.a + color_mid.a + color_high.a) / 3.0;
                tex_color = vec4<f32>(out_color.rgb, 1.0) * avg_a;
            }

            // Apply fill blending: for fill > 0, mix with original layer content
            if lift_fill > 0.001 {
                let orig_tinted = textureSample(base_texture, base_sampler, sample_uv) * uniforms.color;
                tex_color = mix(tex_color, orig_tinted, lift_fill);
            }
        } else {
            // Lift without rgb-split: standard background composite blending
            let comp_color = textureSample(lift_comp_texture, lift_comp_sampler, screen_uv);
            let tinted_tex = tex_color * uniforms.color;
            tex_color = mix(comp_color * tinted_tex.a, tinted_tex, lift_fill);
        }
        lift_skip_color_tint = true;
    } else if rgb_split_enabled {
        // No lift: apply rgb-split normally on base texture
        tex_color = apply_rgb_split(sample_uv);
    }

    // Apply rays (volumetric light rays) effect (射线效果)
    let rays_enabled = uniforms.rays_params2.w > 0.5;
    if rays_enabled {
        tex_color = apply_rays(tex_color, sample_uv);
    }

    // Apply mirror effect (镜子): sample at mirrored UV and blend with original.
    // mirror_params.x encodes type+1 (0=disabled, 1=horizontal, 2=vertical).
    let mirror_type = uniforms.mirror_params.x;
    if mirror_type > 0.5 {
        let mirror_blend_mode = i32(uniforms.mirror_params.y);
        let mirror_alpha = uniforms.mirror_params.z;
        let mirror_offset = uniforms.mirror_params.w;

        // In the mesh expansion area, the original content is transparent.
        // AM treats pixels outside the layer FBO as rgba(0,0,0,0).
        // Texture clamping would return edge pixels instead, so we override.
        if sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0 {
            tex_color = vec4<f32>(0.0);
        }

        var mirror_uv = sample_uv;
        if mirror_type > 1.5 {
            // Vertical: flip Y. AM uses Y-up acLayerNorm, our UV is Y-down.
            // AM: st.y = 1 - st.y + offset → in Y-up space
            // Ours: v_new = 1 - v - offset → negate offset for Y-down
            mirror_uv.y = 1.0 - mirror_uv.y - mirror_offset;
        } else {
            // Horizontal: flip X (same convention in both coordinate systems)
            mirror_uv.x = 1.0 - mirror_uv.x + mirror_offset;
        }

        // Sample mirrored UV (treat out-of-bounds as transparent like AM's FBO)
        var mirror_color: vec4<f32>;
        if mirror_uv.x < 0.0 || mirror_uv.x > 1.0 || mirror_uv.y < 0.0 || mirror_uv.y > 1.0 {
            mirror_color = vec4<f32>(0.0);
        } else {
            mirror_color = textureSample(base_texture, base_sampler, mirror_uv);
        }

        // Blend modes matching AM's mirror shader
        var mirror_result: vec4<f32>;
        if mirror_blend_mode == 1 {
            // Multiply
            if tex_color.a > 0.001 {
                mirror_result = vec4((tex_color.rgb / tex_color.a) * mirror_color.rgb, 1.0) * tex_color.a;
            } else {
                mirror_result = vec4(0.0);
            }
        } else if mirror_blend_mode == 2 {
            // Screen
            if tex_color.a > 0.001 {
                mirror_result = vec4(1.0 - ((1.0 - tex_color.rgb / tex_color.a) * (1.0 - mirror_color.rgb)), 1.0) * tex_color.a;
            } else {
                mirror_result = vec4(0.0);
            }
        } else if mirror_blend_mode == 3 {
            // Over
            mirror_result = tex_color * (1.0 - mirror_color.a) + mirror_color;
        } else if mirror_blend_mode == 4 {
            // Under
            mirror_result = mirror_color * (1.0 - tex_color.a) + tex_color;
        } else {
            // Normal (blendMode == 0)
            mirror_result = mirror_color;
        }

        tex_color = mix(tex_color, mirror_result, mirror_alpha);
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
    
    // Apply exposure/gamma effect if enabled
    // AM shader: rgb += offset*a; rgb = pow(rgb, 1/gamma); rgb *= pow(2, exposure)
    // AM processes in sRGB space (not linear)
    let exposure_enabled = uniforms.exposure_gamma_params.w > 0.5;
    if exposure_enabled {
        let exposure_val = uniforms.exposure_gamma_params.x;
        let gamma_val = uniforms.exposure_gamma_params.y;
        let offset_val = uniforms.exposure_gamma_params.z;
        
        // Convert to sRGB space for processing (AM works in sRGB)
        var eg_rgb = linear_to_srgb(tex_color.rgb);
        let eg_a = tex_color.a;
        
        // Apply AM formula: offset → gamma → exposure
        eg_rgb = eg_rgb + vec3<f32>(offset_val) * eg_a;
        eg_rgb = pow(max(eg_rgb, vec3<f32>(0.0)), vec3<f32>(1.0 / gamma_val));
        eg_rgb = eg_rgb * pow(2.0, exposure_val);
        
        // Convert back to linear
        tex_color = vec4<f32>(srgb_to_linear(eg_rgb), eg_a);
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

    // Apply chromakey effect if enabled / 色度键效果
    let chromakey_enabled = uniforms.chromakey_key_color.a > 0.5;
    if chromakey_enabled {
        tex_color = apply_chromakey(tex_color);
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
    // Skip for lift since we pre-applied color tint before the lift blend
    var final_color: vec4<f32>;
    if linear_repeat_color_applied || lift_skip_color_tint {
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

    // Apply stretch edge AA alpha (soft boundary fade)
    if stretch_edge_alpha < 0.999 {
        final_color = vec4<f32>(final_color.rgb, final_color.a * stretch_edge_alpha);
    }

    // AM composites opacity in sRGB space; Bevy's hardware blend is in linear space.
    // Gamma-encode alpha so that the linear-space alpha blend approximates AM's sRGB result.
    // For fully opaque content over black: linear_to_srgb(srgb_to_linear(opacity)) = opacity.
    let source_is_rtt = uniforms.source_flags.x > 0.5;
    if !source_is_rtt && final_color.a > 0.001 && final_color.a < 0.999 {
        final_color.a = select(
            pow((final_color.a + 0.055) / 1.055, 2.4),
            final_color.a / 12.92,
            final_color.a <= 0.04045
        );
    }

    // Convert to premultiplied alpha for AlphaMode2d::Premultiplied blending.
    // AM uses premultiplied compositing (ONE, ONE_MINUS_SRC_ALPHA):
    //   screen = src.rgb + dst.rgb * (1 - src.a)
    // For RGB split, the effect outputs non-premultiplied RGB (especially mode 2/Light)
    // which creates additive color fringes at transparent regions. We keep that
    // output as-is but scale by layer opacity. For all other cases, we premultiply
    // normally: rgb *= alpha.
    if rgb_split_mode_raw >= -0.5 {
        // RGB split: scale RGB by layer opacity (preserves additive fringe behavior)
        final_color = vec4<f32>(final_color.rgb * uniforms.color.a, final_color.a);
    } else if source_is_rtt {
        // RTT sources already arrive as premultiplied linear color from the previous pass.
        // Re-premultiplying or re-gamma-correcting alpha would darken nested composites.
        final_color = final_color;
    } else {
        // Standard: premultiply rgb by alpha
        final_color = vec4<f32>(final_color.rgb * final_color.a, final_color.a);
    }

    // Discard fully invisible pixels (both alpha and RGB are near zero)
    let max_channel = max(max(final_color.r, final_color.g), final_color.b);
    if final_color.a < 0.001 && max_channel < 0.001 {
        discard;
    }

    // ─── Layer Blend Modes ───────────────────────────────────────────────
    // Apply blend mode if enabled. Uses the composite RTT (lift_comp_texture)
    // as the background, same as lift effect. AM blend formulas operate on
    // premultiplied colors: top = fg_premul, bot = bg_premul * fg_alpha.
    let blend_mode_enabled = uniforms.blend_mode_params.w > 0.5;
    if blend_mode_enabled && final_color.a > 0.001 {
        let blend_canvas_w = uniforms.blend_mode_params.y;
        let blend_canvas_h = uniforms.blend_mode_params.z;
        let blend_screen_uv = vec2<f32>(
            (mesh.world_position.x + blend_canvas_w / 2.0) / blend_canvas_w,
            (blend_canvas_h / 2.0 - mesh.world_position.y) / blend_canvas_h
        );
        let bg_linear = textureSample(lift_comp_texture, lift_comp_sampler, blend_screen_uv);

        // Convert from linear to sRGB premultiplied space (AM operates in sRGB).
        // Unpremultiply → gamma encode → re-premultiply.
        let fg_a = final_color.a;
        var fg_srgb: vec3<f32>;
        if fg_a > 0.001 {
            fg_srgb = linear_to_srgb(final_color.rgb / fg_a) * fg_a;
        } else {
            fg_srgb = vec3<f32>(0.0);
        }
        var bg_srgb: vec3<f32>;
        var bg_a = bg_linear.a;
        if bg_a > 0.001 {
            bg_srgb = linear_to_srgb(bg_linear.rgb / bg_a) * bg_a;
        } else {
            bg_srgb = vec3<f32>(0.0);
        }

        let top = fg_srgb;
        let bot = bg_srgb * fg_a;
        let blend_id = i32(uniforms.blend_mode_params.x + 0.5);
        var blended = top; // fallback = normal

        // Darken family
        if blend_id == 1 {
            // Multiply: fg * (bg on white)
            let bg4 = vec4<f32>(bg_srgb, bg_a);
            let backgrnd = vec4<f32>(1.0, 1.0, 1.0, 1.0) * (1.0 - bg4.a) + bg4 * bg4.a;
            blended = vec4<f32>(fg_srgb, fg_a).rgb * backgrnd.rgb;
        } else if blend_id == 2 {
            // Darken: min(top, bot)
            blended = min(top, bot);
        } else if blend_id == 3 {
            // Darker Color: pick whichever has lower luminance
            let lum_w = vec3<f32>(0.2126, 0.7152, 0.0722);
            blended = select(top, bot, length(lum_w * top) > length(lum_w * bot));
        } else if blend_id == 4 {
            // Color Burn: 1 - (1-bot) / top
            blended = vec3<f32>(1.0) - (vec3<f32>(1.0) - bot) / max(top, vec3<f32>(0.001));
        } else if blend_id == 5 {
            // Linear Burn: top + bot - 1
            blended = top + bot - vec3<f32>(1.0);
        }
        // Lighten family
        else if blend_id == 6 {
            // Screen: top + bot - top*bot (premul equivalent)
            blended = top + bot - top * bot;
        } else if blend_id == 7 {
            // Lighten: max(top, bot)
            blended = max(top, bot);
        } else if blend_id == 8 {
            // Lighter Color: pick whichever has higher luminance
            let lum_w = vec3<f32>(0.2126, 0.7152, 0.0722);
            blended = select(bot, top, length(lum_w * top) > length(lum_w * bot));
        } else if blend_id == 9 {
            // Color Dodge: bot / (1 - top)
            blended = bot / max(vec3<f32>(1.0) - top, vec3<f32>(0.001));
        } else if blend_id == 10 {
            // Linear Dodge (Add): bot + top
            blended = bot + top;
        }
        // Contrast family
        else if blend_id == 11 {
            // Overlay: conditional multiply/screen based on bot
            let t = step(vec3<f32>(0.5), bot);
            blended = t * (vec3<f32>(1.0) - (vec3<f32>(1.0) - 2.0 * (bot - 0.5)) * (vec3<f32>(1.0) - top))
                    + (vec3<f32>(1.0) - t) * (2.0 * bot * top);
        } else if blend_id == 12 {
            // Soft Light
            let t = step(vec3<f32>(0.5), top);
            blended = t * (vec3<f32>(1.0) - (vec3<f32>(1.0) - bot) * (vec3<f32>(1.0) - (top - 0.5)))
                    + (vec3<f32>(1.0) - t) * (bot * (top + 0.5));
        } else if blend_id == 13 {
            // Hard Light
            let t = step(vec3<f32>(0.5), top);
            blended = t * (vec3<f32>(1.0) - (vec3<f32>(1.0) - bot) * (vec3<f32>(1.0) - 2.0 * (top - 0.5)))
                    + (vec3<f32>(1.0) - t) * (bot * 2.0 * top);
        } else if blend_id == 14 {
            // Soft Overlay (same as soft light but based on bot)
            let t = step(vec3<f32>(0.5), bot);
            blended = t * (vec3<f32>(1.0) - (vec3<f32>(1.0) - bot) * (vec3<f32>(1.0) - (top - 0.5)))
                    + (vec3<f32>(1.0) - t) * (bot * (top + 0.5));
        } else if blend_id == 15 {
            // Vivid Light
            let t = step(vec3<f32>(0.5), top);
            blended = t * (vec3<f32>(1.0) - (vec3<f32>(1.0) - bot) * 2.0 * (top - 0.5))
                    + (vec3<f32>(1.0) - t) * (bot * (vec3<f32>(1.0) - 2.0 * top));
        }
        // Difference family
        else if blend_id == 16 {
            // Pin Light
            let t = step(vec3<f32>(0.5), top);
            blended = t * max(bot, 2.0 * (top - 0.5))
                    + (vec3<f32>(1.0) - t) * min(bot, 2.0 * top);
        } else if blend_id == 17 {
            // Difference: |bot - top| (AM formula)
            blended = abs(bg_srgb * fg_a - fg_srgb);
        } else if blend_id == 18 {
            // Exclusion: 0.5 - 2*(bot-0.5)*(top-0.5)
            blended = vec3<f32>(0.5) - 2.0 * (bot - 0.5) * (top - 0.5);
        } else if blend_id == 19 {
            // Subtract: bot - top
            blended = bot - top;
        } else if blend_id == 20 {
            // Divide: bot / top
            blended = bot / max(top, vec3<f32>(0.001));
        }
        // HSL / Component family
        else if blend_id == 21 {
            // Hue: take hue from top, saturation+value from bot
            let top_hsv = rgb2hsv(top);
            let bot_hsv = rgb2hsv(bot);
            blended = hsv2rgb(vec3<f32>(top_hsv.x, bot_hsv.y, bot_hsv.z));
        } else if blend_id == 22 {
            // Saturation: take saturation from top, hue+value from bot
            let top_hsv = rgb2hsv(top);
            let bot_hsv = rgb2hsv(bot);
            blended = hsv2rgb(vec3<f32>(bot_hsv.x, top_hsv.y, bot_hsv.z));
        } else if blend_id == 23 {
            // Color: take Y from bot, UV from top (YUV color space)
            let rgb2yuv = mat3x3<f32>(
                vec3<f32>(0.299, -0.14713, 0.615),
                vec3<f32>(0.587, -0.28886, -0.51499),
                vec3<f32>(0.114, 0.436, -0.10001)
            );
            let yuv2rgb = mat3x3<f32>(
                vec3<f32>(1.0, 1.0, 1.0),
                vec3<f32>(0.0, -0.39465, 2.03211),
                vec3<f32>(1.13983, -0.58060, 0.0)
            );
            let bot_yuv = rgb2yuv * bot;
            let top_yuv = rgb2yuv * top;
            blended = yuv2rgb * vec3<f32>(bot_yuv.x, top_yuv.y, top_yuv.z);
        } else if blend_id == 24 {
            // Luminance: take Y from top, UV from bot (YUV color space)
            let rgb2yuv = mat3x3<f32>(
                vec3<f32>(0.299, -0.14713, 0.615),
                vec3<f32>(0.587, -0.28886, -0.51499),
                vec3<f32>(0.114, 0.436, -0.10001)
            );
            let yuv2rgb = mat3x3<f32>(
                vec3<f32>(1.0, 1.0, 1.0),
                vec3<f32>(0.0, -0.39465, 2.03211),
                vec3<f32>(1.13983, -0.58060, 0.0)
            );
            let bot_yuv = rgb2yuv * bot;
            let top_yuv = rgb2yuv * top;
            blended = yuv2rgb * vec3<f32>(top_yuv.x, bot_yuv.y, bot_yuv.z);
        }

        // Convert blended result from sRGB back to linear premultiplied
        let clamped = clamp(blended, vec3<f32>(0.0), vec3<f32>(1.0));
        if fg_a > 0.001 {
            final_color = vec4<f32>(srgb_to_linear(clamped / fg_a) * fg_a, fg_a);
        } else {
            final_color = vec4<f32>(vec3<f32>(0.0), fg_a);
        }
    }
    
    return final_color;
}

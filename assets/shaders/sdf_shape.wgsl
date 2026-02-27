// SDF Shape Shader - Custom implementation for Bevy
//
// Renders various shapes using signed distance field techniques.
//
// Shape types:
// 0=BoxRound, 1=BoxMiter, 2=BoxBevel, 3=Circle/Ellipse,
// 4=RoundRect, 5=Polygon, 6=Star, 7=Pie, 8=Plus, 9=Multifoil,
// 10=Line, 11=Arc, 12=Triangle, 13=Quad, 14=Penta, 15=Path

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct SdfMaterialUniform {
    color: vec4<f32>,
    params: vec4<f32>,
    mask_params: vec4<f32>,
    mask2_params: vec4<f32>,
    shape_type: f32,
    mask_type: f32,
    mask2_type: f32,
    frame_half: f32,
    mask_rotation: f32,
    mask2_rotation: f32,
    border_mode: f32,
    border2_width: f32,
    border2_packed_color: f32,
    border2_mode: f32,
    border_aa_width: f32,
    base_half_width: f32,
    shape_extra: vec4<f32>,
    shape_extra2: vec4<f32>,
    shape_extra3: vec4<f32>,
    shape_extra4: vec4<f32>,
    shape_extra5: vec4<f32>,
    shape_extra6: vec4<f32>,
    shape_extra7: vec4<f32>,
    gradient_start_color: vec4<f32>,
    gradient_end_color: vec4<f32>,
    gradient_points: vec4<f32>,
    gradient_config: vec4<f32>,
    mask_blend: vec4<f32>,
    mask2_blend: vec4<f32>,
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
    let q = p / vec2<f32>(a, b);
    let d = length(q) - 1.0;
    let scale = min(a, b);
    return d * scale;
}

// SDF for rounded rectangle with explicit corner radius
fn sd_roundrect(p: vec2<f32>, half_size: vec2<f32>, r: f32) -> f32 {
    let cr = min(r, min(half_size.x, half_size.y));
    let d = abs(p) - half_size + vec2<f32>(cr);
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0) - cr;
}

// SDF for regular polygon (N sides)
fn sd_polygon(p: vec2<f32>, n: f32, radius: f32, offset_deg: f32) -> f32 {
    // Use polygon winding-number SDF with n vertices
    let ni = max(i32(round(n)), 3);
    let nf = f32(ni);
    let offset_rad = radians(offset_deg) - 1.5707963;
    let angle_step = 2.0 * 3.141593 / nf;
    
    let first_angle = offset_rad;
    let first_v = vec2<f32>(cos(first_angle), sin(first_angle)) * radius;
    let w0 = p - first_v;
    var ds = vec2<f32>(dot(w0, w0), 1.0);
    var prev = first_v;
    
    for (var i = 1; i < 64; i++) {
        if i >= ni { break; }
        let a = offset_rad + f32(i) * angle_step;
        let curr = vec2<f32>(cos(a), sin(a)) * radius;
        ds = poly_edge(p, prev, curr, ds.x, ds.y);
        prev = curr;
    }
    ds = poly_edge(p, prev, first_v, ds.x, ds.y);
    return ds.y * sqrt(ds.x);
}

// SDF for star shape
fn sd_star(p: vec2<f32>, n: f32, outer_r: f32, inner_r: f32, offset_deg: f32) -> f32 {
    let ni = max(i32(round(n)), 3);
    let nf = f32(ni);
    let offset_rad = radians(offset_deg) - 1.5707963;
    let total = ni * 2;
    let half_step = 3.141593 / nf;

    let first_angle = offset_rad;
    let first_v = vec2<f32>(cos(first_angle), sin(first_angle)) * outer_r;
    let w0 = p - first_v;
    var ds = vec2<f32>(dot(w0, w0), 1.0);
    var prev = first_v;

    for (var i = 1; i < 64; i++) {
        if i >= total { break; }
        let a = offset_rad + f32(i) * half_step;
        let r = select(inner_r, outer_r, i % 2 == 0);
        let curr = vec2<f32>(cos(a), sin(a)) * r;
        ds = poly_edge(p, prev, curr, ds.x, ds.y);
        prev = curr;
    }
    ds = poly_edge(p, prev, first_v, ds.x, ds.y);
    return ds.y * sqrt(ds.x);
}

// Positive modulo (always returns value in [0, m))
fn fmod_pos(x: f32, m: f32) -> f32 {
    let r = x % m;
    return select(r, r + m, r < 0.0);
}

// SDF for pie/arc sector - angle convention matches AM (negated angles)
fn sd_pie(p: vec2<f32>, start_deg: f32, end_deg: f32, radius: f32) -> f32 {
    let d_circle = length(p) - radius;
    let PI = 3.141593;
    let TWO_PI = 6.283185;
    // AM negates angles and adds large offset to ensure positive
    let start_rad = radians(-start_deg) + 2000.0 * PI;
    let end_rad = radians(-end_deg) + 2000.0 * PI;
    let low = min(start_rad, end_rad);
    let high = max(start_rad, end_rad);
    let diff = ((high - low) / TWO_PI) % 2.0;
    let low_n = fmod_pos(low, TWO_PI);
    // Compute half_span and mid_rad directly from diff to avoid precision issues
    var half_span: f32;
    var mid_rad: f32;
    if diff < 1.0 {
        half_span = (1.0 - diff) * PI;
        mid_rad = low_n - half_span;
    } else {
        half_span = (diff - 1.0) * PI;
        mid_rad = low_n + half_span;
    }
    // Rotate point so sector center aligns with +x axis (clockwise by mid_rad)
    let cs = cos(mid_rad);
    let sn = sin(mid_rad);
    let q = vec2<f32>(cs * p.x + sn * p.y, -sn * p.x + cs * p.y);
    let angle = atan2(abs(q.y), q.x);
    if angle < half_span {
        return d_circle;
    }
    // Distance to edge ray
    let edge_cs = cos(half_span);
    let edge_sn = sin(half_span);
    let edge_dir = vec2<f32>(edge_cs, edge_sn);
    let q_abs = vec2<f32>(q.x, abs(q.y));
    let t = clamp(dot(q_abs, edge_dir), 0.0, radius);
    return length(q_abs - edge_dir * t);
}

// SDF for plus/cross shape
fn sd_plus(p: vec2<f32>, half_w: f32, half_h: f32, stem: f32) -> f32 {
    // stem is absolute pixel value (stemSize from AM), not a percentage
    let stem_half = stem / 2.0;
    // Union of horizontal and vertical bars
    let d_horiz = sd_box(p, vec2<f32>(half_w, stem_half));
    let d_vert = sd_box(p, vec2<f32>(stem_half, half_h));
    return min(d_horiz, d_vert);
}

// SDF for multifoil (flower/clover shape) - polygon with bezier-sampled vertices matching AM
// Internal multifoil SDF with integer lobe count
fn sd_multifoil_int(p: vec2<f32>, ni: i32, outer_r: f32, inner_r: f32, offset_deg: f32) -> f32 {
    let nf = f32(ni);
    let offset_rad = radians(offset_deg) - 1.5707963;
    let lobe_angle = 2.0 * 3.141593 / nf;
    let half_lobe = lobe_angle * 0.5;
    let pw = inner_r * 3.141593 / nf;

    // First vertex: Tip(0)
    let first_a = offset_rad;
    let first_v = vec2<f32>(cos(first_a), sin(first_a)) * outer_r;
    let w0 = p - first_v;
    var ds = vec2<f32>(dot(w0, w0), 1.0);
    var prev = first_v;

    for (var i = 0; i < 32; i++) {
        if i >= ni { break; }
        let ta = offset_rad + f32(i) * lobe_angle;
        let va = ta + half_lobe;
        let na = ta + lobe_angle;

        let tip = vec2<f32>(cos(ta), sin(ta)) * outer_r;
        let valley = vec2<f32>(cos(va), sin(va)) * inner_r;
        let ntip = vec2<f32>(cos(na), sin(na)) * outer_r;

        // Bezier control points (perpendicular tangent at tip)
        let tip_out = tip + pw * vec2<f32>(-sin(ta), cos(ta));
        let ntip_in = ntip + pw * vec2<f32>(sin(na), -cos(na));

        // Falling bezier (Tip→Valley): B(t) = (1-t)³·T + 3(1-t)²t·TO + t²(3-2t)·V
        let f1 = 0.2963 * tip + 0.4444 * tip_out + 0.2593 * valley;  // t=1/3
        let f2 = 0.0370 * tip + 0.2222 * tip_out + 0.7407 * valley;  // t=2/3

        // Rising bezier (Valley→NextTip): B(t) = (1-t)²(1+2t)·V + 3(1-t)t²·NI + t³·NT
        let r1 = 0.7407 * valley + 0.2222 * ntip_in + 0.0370 * ntip;  // t=1/3
        let r2 = 0.2593 * valley + 0.4444 * ntip_in + 0.2963 * ntip;  // t=2/3

        if i > 0 {
            ds = poly_edge(p, prev, tip, ds.x, ds.y);
        }
        ds = poly_edge(p, tip, f1, ds.x, ds.y);
        ds = poly_edge(p, f1, f2, ds.x, ds.y);
        ds = poly_edge(p, f2, valley, ds.x, ds.y);
        ds = poly_edge(p, valley, r1, ds.x, ds.y);
        ds = poly_edge(p, r1, r2, ds.x, ds.y);
        prev = r2;
    }
    ds = poly_edge(p, prev, first_v, ds.x, ds.y);
    return ds.y * sqrt(ds.x);
}

fn sd_multifoil(p: vec2<f32>, n: f32, outer_r: f32, inner_r: f32, offset_deg: f32) -> f32 {
    let ni = max(i32(round(n)), 3);
    return sd_multifoil_int(p, ni, outer_r, inner_r, offset_deg);
}

// SDF for line segment
fn sd_line(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let ba_len2 = dot(ba, ba);
    let t = clamp(select(0.0, dot(pa, ba) / ba_len2, ba_len2 > 0.0), 0.0, 1.0);
    return length(pa - ba * t);
}

// SDF for arc - angle convention matches AM (negated angles)
fn sd_arc(p: vec2<f32>, start_deg: f32, end_deg: f32, radius: f32) -> f32 {
    let PI = 3.141593;
    let TWO_PI = 6.283185;
    // AM negates angles and adds large offset
    let start_rad = radians(-start_deg) + 2000.0 * PI;
    let end_rad = radians(-end_deg) + 2000.0 * PI;
    let low = min(start_rad, end_rad);
    let high = max(start_rad, end_rad);
    let diff = ((high - low) / TWO_PI) % 2.0;
    let low_n = fmod_pos(low, TWO_PI);
    // Compute half_span and mid_rad directly from diff to avoid precision issues
    var half_span: f32;
    var mid_rad: f32;
    if diff < 1.0 {
        half_span = (1.0 - diff) * PI;
        mid_rad = low_n - half_span;
    } else {
        half_span = (diff - 1.0) * PI;
        mid_rad = low_n + half_span;
    }
    let cs = cos(mid_rad);
    let sn = sin(mid_rad);
    let q = vec2<f32>(cs * p.x + sn * p.y, -sn * p.x + cs * p.y);
    let angle = atan2(abs(q.y), q.x);
    if angle < half_span {
        return abs(length(p) - radius);
    }
    // Distance to endpoint
    let end_cs = cos(half_span);
    let end_sn = sin(half_span);
    let end_pt = vec2<f32>(end_cs, end_sn) * radius;
    let q_abs = vec2<f32>(q.x, abs(q.y));
    return length(q_abs - end_pt);
}

// SDF for triangle (3 arbitrary points)
fn sd_triangle(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, c: vec2<f32>) -> f32 {
    let e0 = b - a; let v0 = p - a;
    let e1 = c - b; let v1 = p - b;
    let e2 = a - c; let v2 = p - c;
    // Protect against zero-length edges (dot(e,e)==0 causes NaN)
    let d0e = dot(e0, e0); let d1e = dot(e1, e1); let d2e = dot(e2, e2);
    let pq0 = v0 - e0 * clamp(select(0.0, dot(v0, e0) / d0e, d0e > 0.0), 0.0, 1.0);
    let pq1 = v1 - e1 * clamp(select(0.0, dot(v1, e1) / d1e, d1e > 0.0), 0.0, 1.0);
    let pq2 = v2 - e2 * clamp(select(0.0, dot(v2, e2) / d2e, d2e > 0.0), 0.0, 1.0);
    let d0 = dot(pq0, pq0);
    let d1 = dot(pq1, pq1);
    let d2 = dot(pq2, pq2);
    let s = sign(e0.x * e2.y - e0.y * e2.x);
    let d_min = min(min(
        vec2<f32>(d0, s * (v0.x * e0.y - v0.y * e0.x)),
        vec2<f32>(d1, s * (v1.x * e1.y - v1.y * e1.x))),
        vec2<f32>(d2, s * (v2.x * e2.y - v2.y * e2.x)));
    return -sqrt(d_min.x) * sign(d_min.y);
}

// Helper: compute edge contribution for polygon SDF (distance + winding)
fn poly_edge(p: vec2<f32>, vi: vec2<f32>, vj: vec2<f32>, d_sq: f32, s: f32) -> vec2<f32> {
    let e = vj - vi;
    let w = p - vi;
    let el2 = dot(e, e);
    let b = w - e * clamp(select(0.0, dot(w, e) / el2, el2 > 0.0), 0.0, 1.0);
    let new_d = min(d_sq, dot(b, b));
    // Winding number contribution
    var new_s = s;
    let c1 = p.y >= vi.y;
    let c2 = p.y < vj.y;
    let c3 = e.x * w.y > e.y * w.x;
    if (c1 && c2 && c3) || (!c1 && !c2 && !c3) {
        new_s = -new_s;
    }
    return vec2<f32>(new_d, new_s);
}

// SDF for quadrilateral (4 points, convex or concave) - proper winding number
fn sd_quad(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, c: vec2<f32>, d: vec2<f32>) -> f32 {
    let w0 = p - a;
    var ds = vec2<f32>(dot(w0, w0), 1.0);
    ds = poly_edge(p, a, b, ds.x, ds.y);
    ds = poly_edge(p, b, c, ds.x, ds.y);
    ds = poly_edge(p, c, d, ds.x, ds.y);
    ds = poly_edge(p, d, a, ds.x, ds.y);
    return ds.y * sqrt(ds.x);
}

// SDF for pentagon (5 points) - proper winding number
fn sd_penta(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, c: vec2<f32>, d: vec2<f32>, e: vec2<f32>) -> f32 {
    let w0 = p - a;
    var ds = vec2<f32>(dot(w0, w0), 1.0);
    ds = poly_edge(p, a, b, ds.x, ds.y);
    ds = poly_edge(p, b, c, ds.x, ds.y);
    ds = poly_edge(p, c, d, ds.x, ds.y);
    ds = poly_edge(p, d, e, ds.x, ds.y);
    ds = poly_edge(p, e, a, ds.x, ds.y);
    return ds.y * sqrt(ds.x);
}

// Compute gradient color based on UV position in shape-local space
// shape_uv: (0,0)=top-left, (1,1)=bottom-right of shape (AM convention)
fn compute_gradient_color(shape_uv: vec2<f32>) -> vec4<f32> {
    let grad_type = i32(material.gradient_config.x);
    let start = material.gradient_points.xy;
    let end = material.gradient_points.zw;
    var t: f32;
    if grad_type == 1 {
        // Linear gradient: project onto start→end line
        let dir = end - start;
        let len_sq = dot(dir, dir);
        if len_sq < 0.0001 {
            t = 0.5;
        } else {
            t = clamp(dot(shape_uv - start, dir) / len_sq, 0.0, 1.0);
        }
    } else if grad_type == 2 {
        // Radial gradient: distance from start, normalized to reach end
        let radius = length(end - start);
        if radius < 0.0001 {
            t = 0.0;
        } else {
            t = clamp(length(shape_uv - start) / radius, 0.0, 1.0);
        }
    } else {
        // Sweep gradient: angle from center (start), reference direction to end
        // AM convention: endColor at the reference angle, sweeping clockwise to startColor
        let to_end = end - start;
        let base_angle = atan2(to_end.y, to_end.x);
        let to_uv = shape_uv - start;
        let angle = atan2(to_uv.y, to_uv.x);
        let diff = base_angle - angle;
        let pi2 = 6.28318530718;
        t = fmod_pos(diff, pi2) / pi2;
    }
    // Mix in sRGB space (matching AM's NanoVG behavior), then convert to linear
    let srgb = mix(material.gradient_start_color, material.gradient_end_color, t);
    let lin = vec3<f32>(
        select(pow((srgb.r + 0.055) / 1.055, 2.4), srgb.r / 12.92, srgb.r <= 0.04045),
        select(pow((srgb.g + 0.055) / 1.055, 2.4), srgb.g / 12.92, srgb.g <= 0.04045),
        select(pow((srgb.b + 0.055) / 1.055, 2.4), srgb.b / 12.92, srgb.b <= 0.04045),
    );
    return vec4<f32>(lin, srgb.a);
}

// Unpack RGBA from u32 bits stored in f32
fn unpack_color(packed: f32) -> vec4<f32> {
    let bits = bitcast<u32>(packed);
    let r_srgb = f32((bits >> 24u) & 0xFFu) / 255.0;
    let g_srgb = f32((bits >> 16u) & 0xFFu) / 255.0;
    let b_srgb = f32((bits >> 8u) & 0xFFu) / 255.0;
    let a = f32(bits & 0xFFu) / 255.0;
    
    // Convert sRGB to linear using exact sRGB transfer function
    // (matches Bevy's hardware sRGB surface for lossless round-trip)
    let r = select(pow((r_srgb + 0.055) / 1.055, 2.4), r_srgb / 12.92, r_srgb <= 0.04045);
    let g = select(pow((g_srgb + 0.055) / 1.055, 2.4), g_srgb / 12.92, g_srgb <= 0.04045);
    let b = select(pow((b_srgb + 0.055) / 1.055, 2.4), b_srgb / 12.92, b_srgb <= 0.04045);
    
    return vec4<f32>(r, g, b, a);
}

// Compute border alpha matching AM's directional border model.
// Inside (mode=1): border from shape edge inward, fade at inner extent
// Outside (mode=-1): border from shape edge outward, fade at outer extent
// Centered (mode=0): border centered on shape edge, fade at both extents
fn compute_border_alpha(dist: f32, width: f32, mode: f32, aa: f32) -> f32 {
    if mode > 0.5 {
        // INSIDE border: extends from dist=0 (edge) to dist=-width (inward)
        let inward = -dist;  // positive = deeper inside
        // Sharp clip at shape edge (no AA bleed outside shape)
        let edge_clip = step(0.0, inward);
        // Border ring: 1.0 when inward < width (within the border), 0.0 beyond
        let inner_fade = 1.0 - step(width, inward);
        return edge_clip * inner_fade;
    } else if mode < -0.5 {
        // OUTSIDE border: extends from dist=0 (edge) to dist=+width (outward)
        let outward = dist;  // positive = further outside
        let edge_clip = step(0.0, outward);
        // Border ring: 1.0 when outward < width (within the border), 0.0 beyond
        let outer_fade = 1.0 - step(width, outward);
        return edge_clip * outer_fade;
    } else {
        // CENTERED border: rendered via NanoVG path stroke, crisp edges
        let half = width * 0.5;
        let d = abs(dist);
        return step(d, half);
    }
}

// Compute mask blend factor for a single mask.
// Returns 1.0 = fully visible, 0.0 = fully hidden.
fn compute_mask_blend_factor(
    world_pos: vec2<f32>,
    mask_params: vec4<f32>,
    mask_rotation: f32,
    mask_type: f32,
    mask_blend: vec4<f32>,
) -> f32 {
    let center = mask_params.xy;
    let half_size = mask_params.zw;
    let fill_alpha = mask_blend.x;
    let opacity = mask_blend.y;
    let sw = mask_blend.z;

    var rel = world_pos - center;
    let rot = -mask_rotation;
    let c = cos(rot);
    let s = sin(rot);
    rel = vec2<f32>(rel.x * c - rel.y * s, rel.x * s + rel.y * c);

    let is_exclude = mask_type > 2.5;
    let is_ellipse = (mask_type > 1.5 && mask_type < 2.5) || mask_type > 3.5;

    // Shape fill boundary = bounding box minus stroke extension (centered stroke assumed)
    let shape_half = max(half_size - sw * 0.5, vec2<f32>(0.001));

    var mask_sdf: f32;
    if is_ellipse {
        let norm = rel / shape_half;
        let r = length(norm);
        mask_sdf = (r - 1.0) * min(shape_half.x, shape_half.y);
    } else {
        mask_sdf = max(abs(rel.x) - shape_half.x, abs(rel.y) - shape_half.y);
    }

    // Compute mask rendered alpha: fill contribution + stroke contribution
    let fill_factor = select(0.0, fill_alpha, mask_sdf < 0.0);
    // Stroke is solid within its width, hard edge for pixel-perfect rendering
    let stroke_factor = select(0.0, step(abs(mask_sdf), sw * 0.5), sw > 0.01);
    let mask_alpha = min(max(fill_factor, stroke_factor), 1.0);

    // Apply mask formula
    if is_exclude {
        return 1.0 - opacity * mask_alpha;
    } else {
        return 1.0 - opacity * (1.0 - mask_alpha);
    }
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let _debug_st = i32(material.shape_type);

    // Compute mask blend factor (1.0 = fully visible, 0.0 = fully hidden)
    let mask1_type = material.mask_type;
    let mask2_type = material.mask2_type;
    let mask1_enabled = mask1_type > 0.5 && material.mask_params.z < 5000.0;
    let mask2_enabled = mask2_type > 0.5 && material.mask2_params.z < 5000.0;

    var mask_factor = 1.0;
    if mask1_enabled {
        mask_factor *= compute_mask_blend_factor(
            in.world_position.xy,
            material.mask_params,
            material.mask_rotation,
            mask1_type,
            material.mask_blend,
        );
    }
    if mask2_enabled {
        mask_factor *= compute_mask_blend_factor(
            in.world_position.xy,
            material.mask2_params,
            material.mask2_rotation,
            mask2_type,
            material.mask2_blend,
        );
    }
    if mask_factor < 0.005 {
        discard;
    }

    let half_width = material.params.x;
    let half_height = material.params.y;
    let stroke_width = material.params.z;
    let packed_stroke = material.params.w;
    
    // The mesh is created with size = frame_half * 2 x frame_half * 2
    // UV (0,0) is bottom-left, (1,1) is top-right
    // We need position relative to center
    
    // Use the frame_half stored in the material (computed at spawn time)
    let frame_size = material.frame_half * 2.0;
    
    // Convert UV to local coordinates centered at origin
    let pos = (in.uv - 0.5) * frame_size;
    
    // Calculate SDF based on shape type
    var dist: f32;
    let shape_type = i32(material.shape_type);
    
    // Scale factor for vertex-based shapes (triangle, quad, penta, path, line)
    // base_half_width stores the initial half_width at spawn time
    // Current half_width (params.x) changes with animation scale
    var vertex_scale = 1.0;
    if material.base_half_width > 0.01 {
        vertex_scale = half_width / material.base_half_width;
    }

    if shape_type == 0 {
        // BoxRound
        dist = sd_box(pos, vec2<f32>(half_width, half_height));
    } else if shape_type == 1 {
        // BoxMiter
        dist = sd_box_miter(pos, vec2<f32>(half_width, half_height));
    } else if shape_type == 2 {
        // BoxBevel
        dist = sd_box_bevel(pos, vec2<f32>(half_width, half_height));
    } else if shape_type == 3 {
        // Circle/Ellipse
        if abs(half_width - half_height) < 0.001 {
            dist = sd_circle(pos, half_width);
        } else {
            dist = sd_ellipse(pos, half_width, half_height);
        }
    } else if shape_type == 4 {
        // RoundRect
        let corner_r = material.shape_extra.x;
        dist = sd_roundrect(pos, vec2<f32>(half_width, half_height), corner_r);
    } else if shape_type == 5 {
        // Polygon
        let side_count = material.shape_extra.x;
        let radius = material.shape_extra.y * vertex_scale;
        let offset_angle = material.shape_extra.z;
        dist = sd_polygon(pos, side_count, radius, offset_angle);
    } else if shape_type == 6 {
        // Star
        let point_count = material.shape_extra.x;
        let outer_r = material.shape_extra.y * vertex_scale;
        let inner_r = material.shape_extra.z * vertex_scale;
        let offset_angle = material.shape_extra.w;
        dist = sd_star(pos, point_count, outer_r, inner_r, offset_angle);
    } else if shape_type == 7 {
        // Pie
        let start_angle = material.shape_extra.x;
        let end_angle = material.shape_extra.y;
        let radius = material.shape_extra.z * vertex_scale;
        dist = sd_pie(pos, start_angle, end_angle, radius);
    } else if shape_type == 8 {
        // Plus
        let stem_size = material.shape_extra.x;
        dist = sd_plus(pos, half_width, half_height, stem_size);
    } else if shape_type == 9 {
        // Multifoil
        let point_count = material.shape_extra.x;
        let outer_r = material.shape_extra.y * vertex_scale;
        let inner_r = material.shape_extra.z * vertex_scale;
        let offset_angle = material.shape_extra.w;
        dist = sd_multifoil(pos, point_count, outer_r, inner_r, offset_angle);
    } else if shape_type == 10 {
        // Line
        let p1 = vec2<f32>(material.shape_extra.x, material.shape_extra.y) * vertex_scale;
        let p2 = vec2<f32>(material.shape_extra.z, material.shape_extra.w) * vertex_scale;
        dist = sd_line(pos, p1, p2);
    } else if shape_type == 11 {
        // Arc - radius-based shapes scale via their radius parameter
        let start_angle = material.shape_extra.x;
        let end_angle = material.shape_extra.y;
        let radius = material.shape_extra.z * vertex_scale;
        dist = sd_arc(pos, start_angle, end_angle, radius);
    } else if shape_type == 12 {
        // Triangle
        let p1 = vec2<f32>(material.shape_extra.x, material.shape_extra.y) * vertex_scale;
        let p2 = vec2<f32>(material.shape_extra.z, material.shape_extra.w) * vertex_scale;
        let p3 = vec2<f32>(material.shape_extra2.x, material.shape_extra2.y) * vertex_scale;
        dist = sd_triangle(pos, p1, p2, p3);
    } else if shape_type == 13 {
        // Quad
        let p1 = vec2<f32>(material.shape_extra.x, material.shape_extra.y) * vertex_scale;
        let p2 = vec2<f32>(material.shape_extra.z, material.shape_extra.w) * vertex_scale;
        let p3 = vec2<f32>(material.shape_extra2.x, material.shape_extra2.y) * vertex_scale;
        let p4 = vec2<f32>(material.shape_extra2.z, material.shape_extra2.w) * vertex_scale;
        dist = sd_quad(pos, p1, p2, p3, p4);
    } else if shape_type == 14 {
        // Penta
        let p1 = vec2<f32>(material.shape_extra.x, material.shape_extra.y) * vertex_scale;
        let p2 = vec2<f32>(material.shape_extra.z, material.shape_extra.w) * vertex_scale;
        let p3 = vec2<f32>(material.shape_extra2.x, material.shape_extra2.y) * vertex_scale;
        let p4 = vec2<f32>(material.shape_extra2.z, material.shape_extra2.w) * vertex_scale;
        let p5 = vec2<f32>(material.shape_extra3.x, material.shape_extra3.y) * vertex_scale;
        dist = sd_penta(pos, p1, p2, p3, p4, p5);
    } else if shape_type == 15 {
        // Path - polygon SDF with up to 14 vertices from shape_extra fields
        // Vertex count stored in shape_extra7.z (0 = auto-detect via zero-check)
        let vertex_count = i32(material.shape_extra7.z);
        var pts: array<vec2<f32>, 14>;
        pts[0] = vec2<f32>(material.shape_extra.x, material.shape_extra.y) * vertex_scale;
        pts[1] = vec2<f32>(material.shape_extra.z, material.shape_extra.w) * vertex_scale;
        pts[2] = vec2<f32>(material.shape_extra2.x, material.shape_extra2.y) * vertex_scale;
        pts[3] = vec2<f32>(material.shape_extra2.z, material.shape_extra2.w) * vertex_scale;
        pts[4] = vec2<f32>(material.shape_extra3.x, material.shape_extra3.y) * vertex_scale;
        pts[5] = vec2<f32>(material.shape_extra3.z, material.shape_extra3.w) * vertex_scale;
        pts[6] = vec2<f32>(material.shape_extra4.x, material.shape_extra4.y) * vertex_scale;
        pts[7] = vec2<f32>(material.shape_extra4.z, material.shape_extra4.w) * vertex_scale;
        pts[8] = vec2<f32>(material.shape_extra5.x, material.shape_extra5.y) * vertex_scale;
        pts[9] = vec2<f32>(material.shape_extra5.z, material.shape_extra5.w) * vertex_scale;
        pts[10] = vec2<f32>(material.shape_extra6.x, material.shape_extra6.y) * vertex_scale;
        pts[11] = vec2<f32>(material.shape_extra6.z, material.shape_extra6.w) * vertex_scale;
        pts[12] = vec2<f32>(material.shape_extra7.x, material.shape_extra7.y) * vertex_scale;
        pts[13] = vec2<f32>(0.0, 0.0); // sentinel
        // Determine actual vertex count
        var nv = vertex_count;
        if nv <= 0 {
            // Auto-detect: find first vertex that matches vertex 0
            nv = 3;
            for (var i = 3; i < 14; i++) {
                if length(pts[i] - pts[0]) < 0.01 {
                    break;
                }
                nv = i + 1;
            }
        }
        nv = min(nv, 14);
        let w0 = pos - pts[0];
        var ds = vec2<f32>(dot(w0, w0), 1.0);
        for (var i = 0; i < 14; i++) {
            if i >= nv { break; }
            let next = select(i + 1, 0, i + 1 >= nv);
            ds = poly_edge(pos, pts[i], pts[next], ds.x, ds.y);
        }
        dist = ds.y * sqrt(ds.x);
    } else {
        // Fallback: circle
        dist = sd_circle(pos, half_width);
    }
    
    // NaN protection: if SDF function produced NaN (e.g., from degenerate geometry),
    // discard the pixel to prevent opaque rendering over the entire quad
    if dist != dist {
        discard;
    }
    
    // Infinity protection: -Inf would make fill_alpha=1.0 everywhere
    if dist < -1e10 || dist > 1e10 {
        discard;
    }
    
    // Rendering matching AM's dual-path border model:
    // - NanoVG renders fills and centered strokes with crisp edges
    // - Inside/outside borders use pixel-scan effect with smoothstep at inner edge
    let aa = max(fwidth(dist), 0.5);
    
    // Fill: inside the shape (dist <= 0), hard edge for pixel-perfect rendering
    let fill_alpha = step(0.0, -dist);
    // Compute fill color: use gradient if enabled, otherwise solid color
    var fill_base_color = material.color;
    if material.gradient_config.x > 0.5 {
        // Convert local pos to shape UV: (0,0)=top-left, (1,1)=bottom-right
        // Note: In Bevy's Rectangle mesh, UV.y=0 is top, UV.y=1 is bottom
        // pos = (uv - 0.5) * frame_size, so pos.y increases downward on screen
        let shape_uv = vec2<f32>(
            (pos.x + half_width) / (2.0 * half_width),
            (pos.y + half_height) / (2.0 * half_height)
        );
        fill_base_color = compute_gradient_color(shape_uv);
    }
    let fill_col = vec4<f32>(fill_base_color.rgb, fill_base_color.a * fill_alpha);
    
    // Handle stroke if stroke_width > 0
    var final_color: vec4<f32>;
    var stroke_alpha_contrib: f32 = 0.0;
    if stroke_width > 0.0 {
        let stroke_color = unpack_color(packed_stroke);
        let stroke_alpha = compute_border_alpha(dist, stroke_width, material.border_mode, aa);
        let stroke_col = vec4<f32>(stroke_color.rgb, stroke_color.a * stroke_alpha);
        stroke_alpha_contrib = stroke_col.a;
        
        // Composite: stroke over fill
        var out_a = stroke_col.a + fill_col.a * (1.0 - stroke_col.a);
        
        if material.border2_width > 0.0 {
            // Second border
            let b2_color = unpack_color(material.border2_packed_color);
            let b2_alpha = compute_border_alpha(dist, material.border2_width, material.border2_mode, aa);
            let b2_col = vec4<f32>(b2_color.rgb, b2_color.a * b2_alpha);
            
            // Composite: border2 over (border1 over fill)
            let c1_a = out_a;
            let c1_rgb = select(
                vec3<f32>(0.0),
                (stroke_col.rgb * stroke_col.a + fill_col.rgb * fill_col.a * (1.0 - stroke_col.a)) / out_a,
                out_a > 0.01
            );
            let c1 = vec4<f32>(c1_rgb, c1_a);
            
            out_a = b2_col.a + c1.a * (1.0 - b2_col.a);
            if out_a < 0.01 {
                discard;
            }
            let out_rgb = (b2_col.rgb * b2_col.a + c1.rgb * c1.a * (1.0 - b2_col.a)) / out_a;
            final_color = vec4<f32>(out_rgb, out_a);
        } else {
            if out_a < 0.01 {
                discard;
            }
            let out_rgb = (stroke_col.rgb * stroke_col.a + fill_col.rgb * fill_col.a * (1.0 - stroke_col.a)) / out_a;
            final_color = vec4<f32>(out_rgb, out_a);
        }
    } else if material.border2_width > 0.0 {
        // Only border2, no border1
        let b2_color = unpack_color(material.border2_packed_color);
        let b2_alpha = compute_border_alpha(dist, material.border2_width, material.border2_mode, aa);
        let b2_col = vec4<f32>(b2_color.rgb, b2_color.a * b2_alpha);
        
        let out_a = b2_col.a + fill_col.a * (1.0 - b2_col.a);
        if out_a < 0.01 {
            discard;
        }
        let out_rgb = (b2_col.rgb * b2_col.a + fill_col.rgb * fill_col.a * (1.0 - b2_col.a)) / out_a;
        final_color = vec4<f32>(out_rgb, out_a);
    } else {
        // No stroke, just fill
        if fill_col.a <= 0.0 {
            discard;
        }
        final_color = fill_col;
    }

    // Apply mask in sRGB space to match AM's compositing pipeline.
    if mask_factor < 0.999 {
        let lin = final_color.rgb;
        let srgb = vec3<f32>(
            select(1.055 * pow(lin.r, 1.0 / 2.4) - 0.055, lin.r * 12.92, lin.r <= 0.0031308),
            select(1.055 * pow(lin.g, 1.0 / 2.4) - 0.055, lin.g * 12.92, lin.g <= 0.0031308),
            select(1.055 * pow(lin.b, 1.0 / 2.4) - 0.055, lin.b * 12.92, lin.b <= 0.0031308),
        );
        let masked = srgb * mask_factor;
        final_color = vec4<f32>(
            select(pow((masked.x + 0.055) / 1.055, 2.4), masked.x / 12.92, masked.x <= 0.04045),
            select(pow((masked.y + 0.055) / 1.055, 2.4), masked.y / 12.92, masked.y <= 0.04045),
            select(pow((masked.z + 0.055) / 1.055, 2.4), masked.z / 12.92, masked.z <= 0.04045),
            final_color.a,
        );
    }

    // Pixelate2 threshold: make sub-threshold pixels transparent.
    // AM pixelates first (mixing thin stroke into surrounding fill), then thresholds.
    // For dark-filled shapes, pixelation makes even the stroke pixels fall below threshold,
    // so the entire shape becomes transparent. No-fill shapes (color.a ≈ 0) keep their stroke.
    let pix_threshold = material.gradient_config.y;
    if pix_threshold > 0.001 && material.color.a > 0.5 {
        let fc = material.color.rgb;
        let srgb_fill = vec3<f32>(
            select(1.055 * pow(fc.r, 1.0 / 2.4) - 0.055, fc.r * 12.92, fc.r <= 0.0031308),
            select(1.055 * pow(fc.g, 1.0 / 2.4) - 0.055, fc.g * 12.92, fc.g <= 0.0031308),
            select(1.055 * pow(fc.b, 1.0 / 2.4) - 0.055, fc.b * 12.92, fc.b <= 0.0031308),
        );
        let fill_lum = dot(srgb_fill, vec3<f32>(0.2126, 0.7152, 0.0722));
        if fill_lum < pix_threshold {
            discard;
        }
    }

    // AM composites opacity in sRGB space; Bevy's hardware blend is in linear space.
    // NOTE: sRGB alpha correction disabled — narrows AA fringes too aggressively.

    if final_color.a < 0.005 {
        discard;
    }
    return final_color;
}

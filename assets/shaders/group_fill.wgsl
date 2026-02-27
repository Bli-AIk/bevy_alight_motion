// Group fill shader - renders solid color or gradient using RTT texture as alpha mask.
//
// Matches Alight Motion's OpenGL shaders (tex2d_grad_linear/radial/sweep.frag).
// AM uses OpenGL convention (UV y=0 at bottom), Bevy uses y=0 at top,
// so we flip UV Y for gradient calculations.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct GroupFillUniform {
    fill_color: vec4<f32>,
    // gradient_config.x: 0=solid, 1=linear, 2=radial, 3=sweep
    gradient_config: vec4<f32>,
    gradient_start_color: vec4<f32>,
    gradient_end_color: vec4<f32>,
    // (start_x, start_y, end_x, end_y) in AM UV space
    gradient_points: vec4<f32>,
};

@group(2) @binding(0) var<uniform> fill: GroupFillUniform;
@group(2) @binding(1) var base_texture: texture_2d<f32>;
@group(2) @binding(2) var base_sampler: sampler;

const PI: f32 = 3.14159265358979323846;

// Approximate sRGB → linear conversion
fn srgb_to_linear(c: f32) -> f32 {
    return pow(c, 2.2);
}

fn srgb_to_linear3(c: vec3<f32>) -> vec3<f32> {
    return vec3(srgb_to_linear(c.x), srgb_to_linear(c.y), srgb_to_linear(c.z));
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(base_texture, base_sampler, mesh.uv);
    let dst_a = tex.a;

    if dst_a < 0.001 {
        discard;
    }

    let grad_type = i32(fill.gradient_config.x);
    var color: vec4<f32>;

    // Flip UV Y to match AM's OpenGL convention (y=0 at bottom).
    let st = vec2(mesh.uv.x, 1.0 - mesh.uv.y);

    if grad_type == 0 {
        // Solid color fill
        color = fill.fill_color;
    } else if grad_type == 1 {
        // Linear gradient (matches AM tex2d_grad_linear.frag)
        let start = fill.gradient_points.xy;
        let end_pt = fill.gradient_points.zw;
        let dir = end_pt - start;
        let len_sq = dot(dir, dir);
        var t: f32 = 0.0;
        if len_sq > 0.0001 {
            t = clamp(dot(st - start, dir) / len_sq, 0.0, 1.0);
        }
        t = smoothstep(0.0, 1.0, t);
        color = mix(fill.gradient_start_color, fill.gradient_end_color, t);
    } else if grad_type == 2 {
        // Radial gradient (matches AM tex2d_grad_radial.frag)
        let center = fill.gradient_points.xy;
        let edge = fill.gradient_points.zw;
        let radius = distance(center, edge);
        var t: f32 = 0.0;
        if radius > 0.0001 {
            t = clamp(distance(st, center) / radius, 0.0, 1.0);
        }
        t = smoothstep(0.0, 1.0, t);
        color = mix(fill.gradient_start_color, fill.gradient_end_color, t);
    } else {
        // Sweep gradient (matches AM tex2d_grad_sweep.frag)
        // AM uses atan(x, y) not atan(y, x)
        let center = fill.gradient_points.xy;
        let edge = fill.gradient_points.zw;
        let xy = st - center;
        let zw = edge - center;
        let a = atan2(xy.x, xy.y);
        let b = atan2(zw.x, zw.y);
        let t = ((a - b) + PI) / (2.0 * PI);
        color = mix(fill.gradient_start_color, fill.gradient_end_color, t);
    }

    // AM blending: result = (src * dst.a + dst * (1-src.a) * vec4(1,1,1,dst.a))
    // For opaque fill (color.a=1): result = color.rgb * dst.a (pure fill silhouette)
    // For semi-transparent fill: children colors show through proportionally.
    //
    // Gradient colors are stored in sRGB space (AM interpolates in sRGB).
    // Convert to linear for Bevy's linear pipeline output.
    var linear_rgb: vec3<f32>;
    if grad_type > 0 {
        linear_rgb = srgb_to_linear3(color.rgb);
    } else {
        linear_rgb = color.rgb; // Solid fills already in linear
    }
    let children_rgb = tex.rgb / max(dst_a, 0.001);
    let blended_rgb = mix(children_rgb, linear_rgb, color.a);
    return vec4(blended_rgb * dst_a, dst_a);
}

// Numinous Post-Stack Shader

struct PostParams {
    width: f32,
    height: f32,
    era: u32,       // 0=Phosphor, 1=8-bit, 2=Vector, 3=Modern
    time: f32,
}

@group(0) @binding(0) var<uniform> params: PostParams;
@group(0) @binding(1) var t_sampler: sampler;
@group(0) @binding(2) var t_color: texture_2d<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Fullscreen triangle
    var out: VertexOutput;
    let u = f32((vertex_index << 1u) & 2u);
    let v = f32(vertex_index & 2u);
    out.uv = vec2<f32>(u, v);
    out.position = vec4<f32>(u * 2.0 - 1.0, 1.0 - v * 2.0, 0.0, 1.0);
    return out;
}

fn luminance(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.299, 0.587, 0.114));
}

// Simple ACES-like tonemap
fn tonemap(color: vec3<f32>) -> vec3<f32> {
    let a = 2.51f;
    let b = 0.03f;
    let c = 2.43f;
    let d = 0.59f;
    let e = 0.14f;
    return saturate((color * (a * color + b)) / (color * (c * color + d) + e));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(t_color, t_sampler, in.uv).rgb;

    // TODO: implement real bloom via mipmaps in a separate pass.
    // For now, simulate a soft bloom in a single pass if needed, or rely on HDR.

    if (params.era == 0u) { // Phosphor
        let lum = luminance(color);
        color = vec3<f32>(min(lum / 6.0, 1.0), min(lum, 1.0), min(lum / 4.0, 1.0));
        
        // Scanlines
        let y = in.uv.y * params.height;
        if (y % 3.0 < 1.0) {
            color *= 0.6;
        }
    } else if (params.era == 1u) { // 8-bit
        // Simple chunky pixel simulation and palette snap could go here.
        // Doing full ordered dither requires bayer matrix.
    } else if (params.era == 2u) { // Vector
        let lum = luminance(color);
        if (lum < 0.15) { // ~40/255
            color = vec3<f32>(0.0);
        } else {
            color *= 1.5;
        }
    }

    // Tonemap back to SDR
    color = tonemap(color / 255.0); // Assuming input is 0-255 scale

    return vec4<f32>(color, 1.0);
}

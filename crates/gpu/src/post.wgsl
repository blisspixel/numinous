struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct PostParams {
    inverse_source_size: vec2<f32>,
    threshold: f32,
    bloom_strength: f32,
    exposure: f32,
    padding_0: f32,
    padding_1: f32,
    padding_2: f32,
};

@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var linear_sampler: sampler;
@group(0) @binding(2) var<uniform> params: PostParams;
@group(0) @binding(3) var bloom_texture: texture_2d<f32>;

@vertex
fn vertex_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    let positions = array(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let position = positions[index];
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.uv = position * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5);
    return output;
}

@fragment
fn linearize(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(source_texture, linear_sampler, input.uv).rgb;
    return vec4<f32>(color * params.exposure * 1.25, 1.0);
}

@fragment
fn bright_pass(input: VertexOutput) -> @location(0) vec4<f32> {
    let offset = params.inverse_source_size * 0.5;
    let color = (
        textureSample(source_texture, linear_sampler, input.uv + vec2<f32>(-offset.x, -offset.y)).rgb
        + textureSample(source_texture, linear_sampler, input.uv + vec2<f32>(offset.x, -offset.y)).rgb
        + textureSample(source_texture, linear_sampler, input.uv + vec2<f32>(-offset.x, offset.y)).rgb
        + textureSample(source_texture, linear_sampler, input.uv + vec2<f32>(offset.x, offset.y)).rgb
    ) * 0.25;
    let luminance = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    let contribution = max(luminance - params.threshold, 0.0) / max(luminance, 0.0001);
    return vec4<f32>(color * contribution, 1.0);
}

fn blur(input: VertexOutput, direction: vec2<f32>) -> vec4<f32> {
    let step = params.inverse_source_size * direction;
    var color = textureSample(source_texture, linear_sampler, input.uv).rgb * 0.227027;
    color += textureSample(source_texture, linear_sampler, input.uv + step * 1.384615).rgb * 0.316216;
    color += textureSample(source_texture, linear_sampler, input.uv - step * 1.384615).rgb * 0.316216;
    color += textureSample(source_texture, linear_sampler, input.uv + step * 3.230769).rgb * 0.070270;
    color += textureSample(source_texture, linear_sampler, input.uv - step * 3.230769).rgb * 0.070270;
    return vec4<f32>(color, 1.0);
}

@fragment
fn blur_horizontal(input: VertexOutput) -> @location(0) vec4<f32> {
    return blur(input, vec2<f32>(1.0, 0.0));
}

@fragment
fn blur_vertical(input: VertexOutput) -> @location(0) vec4<f32> {
    return blur(input, vec2<f32>(0.0, 1.0));
}

@fragment
fn composite(input: VertexOutput) -> @location(0) vec4<f32> {
    let scene = textureSample(source_texture, linear_sampler, input.uv).rgb;
    let bloom = textureSample(bloom_texture, linear_sampler, input.uv).rgb;
    let hdr = (scene + bloom * params.bloom_strength) * params.exposure;
    let mapped = vec3<f32>(1.0) - exp(-hdr);
    return vec4<f32>(mapped, 1.0);
}

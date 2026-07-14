// Escape-time fractals, computed per pixel into a packed RGBA storage buffer.
// mode 0: Mandelbrot (z starts at 0, c is the pixel).
// mode 1: Julia (z starts at the pixel, c is a constant).
// The portable GPU-compute path (see lib.rs).

struct Params {
    width: u32,
    height: u32,
    max_iter: u32,
    mode: u32,
    center_x: f32,
    center_y: f32,
    scale: f32,
    _pad1: f32,
    c_x: f32,
    c_y: f32,
    _pad2: f32,
    _pad3: f32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.width || gid.y >= params.height) {
        return;
    }
    let aspect = f32(params.width) / f32(params.height);
    let u = f32(gid.x) / f32(params.width) - 0.5;
    let v = f32(gid.y) / f32(params.height) - 0.5;
    let px = params.center_x + u * params.scale * aspect;
    let py = params.center_y + v * params.scale;

    var zx = 0.0;
    var zy = 0.0;
    var cx = px;
    var cy = py;
    if (params.mode == 1u) {
        zx = px;
        zy = py;
        cx = params.c_x;
        cy = params.c_y;
    }

    var i = 0u;
    loop {
        if (i >= params.max_iter) { break; }
        let nx = zx * zx - zy * zy + cx;
        let ny = 2.0 * zx * zy + cy;
        zx = nx;
        zy = ny;
        i = i + 1u;
        if (zx * zx + zy * zy > 4.0) { break; }
    }

    // Julia retains its established warm bands. Mandelbrot maps smooth escape
    // time to a vivid cosine palette: hue identifies iteration depth while
    // the dark interior keeps the set itself legible as negative space.
    var color = vec3<f32>(10.0, 11.0, 15.0) / 255.0;
    if (params.mode == 0u) {
        if (i < params.max_iter) {
            let magnitude_squared = max(zx * zx + zy * zy, 4.000001);
            let smooth_i = f32(i) + 1.0 - log2(0.5 * log2(magnitude_squared));
            let band = smooth_i * 0.071;
            let phase = vec3<f32>(0.00, 0.34, 0.68);
            let acid = vec3<f32>(0.55) + 0.45 * cos(6.2831853 * (vec3<f32>(band) + phase));
            let pulse = 0.62 + 0.38 * (0.5 + 0.5 * cos(6.2831853 * band * 0.37));
            color = clamp(acid * pulse + vec3<f32>(0.02, 0.04, 0.08), vec3<f32>(0.0), vec3<f32>(1.0));
        }
    } else {
        if (i == params.max_iter) {
            color = vec3<f32>(255.0, 204.0, 102.0) / 255.0;
        } else if (i > 20u) {
            color = vec3<f32>(255.0, 120.0, 60.0) / 255.0;
        } else if (i > 5u) {
            color = vec3<f32>(16.0, 20.0, 34.0) / 255.0;
        }
    }

    let r = u32(clamp(color.x, 0.0, 1.0) * 255.0);
    let g = u32(clamp(color.y, 0.0, 1.0) * 255.0);
    let b = u32(clamp(color.z, 0.0, 1.0) * 255.0);
    output[gid.y * params.width + gid.x] = r | (g << 8u) | (b << 16u) | (255u << 24u);
}

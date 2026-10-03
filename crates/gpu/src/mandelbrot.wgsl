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
@group(0) @binding(2) var<uniform> palette: array<vec4<f32>, 8>;

fn escape_color(escape: f32) -> vec3<f32> {
    if (escape <= palette[0].w) { return palette[0].xyz / 255.0; }
    for (var j = 1u; j < 8u; j = j + 1u) {
        let lo = palette[j - 1u];
        let hi = palette[j];
        if (escape <= hi.w) {
            let t = (escape - lo.w) / (hi.w - lo.w);
            return mix(lo.xyz, hi.xyz, t * t * (3.0 - 2.0 * t)) / 255.0;
        }
    }
    return palette[7].xyz / 255.0;
}

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
    if (params.mode == 0u) {
        let x = cx - 0.25;
        let y2 = cy * cy;
        let q = x * x + y2;
        if (q * (q + x) <= 0.25 * y2 || (cx + 1.0) * (cx + 1.0) + y2 <= 0.0625) {
            i = params.max_iter;
        }
    }
    loop {
        if (i >= params.max_iter) { break; }
        let nx = zx * zx - zy * zy + cx;
        let ny = 2.0 * zx * zy + cy;
        zx = nx;
        zy = ny;
        i = i + 1u;
        if (zx * zx + zy * zy > 4.0) { break; }
    }

    // The color table comes from core, shared with PNG and CPU App fallback.
    // Julia retains its established warm bands.
    var color = palette[0].xyz / 255.0;
    if (params.mode == 0u) {
        if (i < params.max_iter) {
            for (var extra = 0u; extra < 2u; extra = extra + 1u) {
                let nx = zx * zx - zy * zy + cx;
                zy = 2.0 * zx * zy + cy;
                zx = nx;
            }
            let smooth_i = f32(i) + 3.0 - log2(0.5 * log(zx * zx + zy * zy));
            color = escape_color(smooth_i);
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

    let r = u32(round(clamp(color.x, 0.0, 1.0) * 255.0));
    let g = u32(round(clamp(color.y, 0.0, 1.0) * 255.0));
    let b = u32(round(clamp(color.z, 0.0, 1.0) * 255.0));
    output[gid.y * params.width + gid.x] = r | (g << 8u) | (b << 16u) | (255u << 24u);
}

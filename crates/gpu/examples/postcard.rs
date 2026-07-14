//! Render the Mandelbrot set on this machine's GPU and save it as a PNG.
//! Run: cargo run -p numinous-gpu --example postcard

use std::io::BufWriter;

fn main() {
    let ctx = numinous_gpu::GpuContext::new().expect("acquire a GPU or CPU adapter");
    println!("Rendering on: {} ({})", ctx.adapter_name(), ctx.backend());

    let (width, height) = (1200u32, 900u32);
    let rgba = ctx
        .render_mandelbrot(width, height, -0.75, 0.0, 3.0, 300)
        .expect("render Mandelbrot frame");

    let file = std::fs::File::create("mandelbrot.png").expect("create mandelbrot.png");
    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("write png header");
    writer.write_image_data(&rgba).expect("write png data");

    println!("wrote mandelbrot.png ({width}x{height})");
}

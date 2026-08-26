//! Reusable CPU reference for the GPU post-processing spike.

const BLOOM_THRESHOLD: f32 = 0.72;
const BLOOM_STRENGTH: f32 = 0.78;
const EXPOSURE: f32 = 1.08;
const LINEAR_GAIN: f32 = 1.25 * EXPOSURE;
const TONE_MAP_MAX: f32 = 4.0;
const TONE_MAP_STEPS: usize = 4096;

/// A reusable CPU implementation of the same post-processing stages.
pub(crate) struct CpuPostRenderer {
    width: u32,
    height: u32,
    half_width: usize,
    half_height: usize,
    linear: Vec<[f32; 3]>,
    bright: Vec<[f32; 3]>,
    horizontal: Vec<[f32; 3]>,
    bloom: Vec<[f32; 3]>,
    output: Vec<u8>,
    decode: [f32; 256],
    tone_map: Vec<u8>,
    x_samples: Vec<SamplePair>,
    y_samples: Vec<SamplePair>,
}

#[derive(Clone, Copy)]
struct SamplePair {
    low: usize,
    high: usize,
    fraction: f32,
}

impl CpuPostRenderer {
    pub(crate) fn new() -> Self {
        let decode = std::array::from_fn(|index| srgb_to_linear(index as f32 / 255.0));
        let tone_map = (0..=TONE_MAP_STEPS)
            .map(|index| {
                let hdr = index as f32 * TONE_MAP_MAX / TONE_MAP_STEPS as f32;
                let mapped = 1.0 - (-hdr).exp();
                (linear_to_srgb(mapped) * 255.0).round() as u8
            })
            .collect();
        Self {
            width: 0,
            height: 0,
            half_width: 0,
            half_height: 0,
            linear: Vec::new(),
            bright: Vec::new(),
            horizontal: Vec::new(),
            bloom: Vec::new(),
            output: Vec::new(),
            decode,
            tone_map,
            x_samples: Vec::new(),
            y_samples: Vec::new(),
        }
    }

    pub(crate) fn process_rgba(
        &mut self,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) -> Result<&[u8], String> {
        let pixel_count = width
            .checked_mul(height)
            .and_then(|pixels| usize::try_from(pixels).ok())
            .ok_or_else(|| "CPU post dimensions overflow".to_string())?;
        let expected = pixel_count
            .checked_mul(4)
            .ok_or_else(|| "CPU post byte length overflows".to_string())?;
        if width == 0 || height == 0 || rgba.len() != expected {
            return Err(format!(
                "CPU post requires a nonempty {width}x{height} RGBA frame with {expected} bytes"
            ));
        }
        self.resize(width, height, pixel_count);
        self.decode_source(rgba);
        self.extract_bright_pass();
        blur_horizontal(
            &self.bright,
            &mut self.horizontal,
            self.half_width,
            self.half_height,
        );
        blur_vertical(
            &self.horizontal,
            &mut self.bloom,
            self.half_width,
            self.half_height,
        );
        composite(
            &self.linear,
            &self.bloom,
            &mut self.output,
            self.width as usize,
            self.half_width,
            &self.x_samples,
            &self.y_samples,
            &self.tone_map,
        );
        Ok(&self.output)
    }

    fn resize(&mut self, width: u32, height: u32, pixel_count: usize) {
        if self.width == width && self.height == height {
            return;
        }
        self.width = width;
        self.height = height;
        self.half_width = width.div_ceil(2) as usize;
        self.half_height = height.div_ceil(2) as usize;
        let half_pixels = self.half_width * self.half_height;
        self.linear.resize(pixel_count, [0.0; 3]);
        self.bright.resize(half_pixels, [0.0; 3]);
        self.horizontal.resize(half_pixels, [0.0; 3]);
        self.bloom.resize(half_pixels, [0.0; 3]);
        self.output.resize(pixel_count * 4, 0);
        self.x_samples = sample_pairs(width as usize, self.half_width);
        self.y_samples = sample_pairs(height as usize, self.half_height);
    }

    fn decode_source(&mut self, rgba: &[u8]) {
        for (destination, source) in self.linear.iter_mut().zip(rgba.chunks_exact(4)) {
            *destination = [
                self.decode[source[0] as usize] * LINEAR_GAIN,
                self.decode[source[1] as usize] * LINEAR_GAIN,
                self.decode[source[2] as usize] * LINEAR_GAIN,
            ];
        }
    }

    fn extract_bright_pass(&mut self) {
        let width = self.width as usize;
        let height = self.height as usize;
        for half_y in 0..self.half_height {
            for half_x in 0..self.half_width {
                let mut color = [0.0; 3];
                let mut count = 0.0;
                for y in half_y * 2..(half_y * 2 + 2).min(height) {
                    for x in half_x * 2..(half_x * 2 + 2).min(width) {
                        let pixel = self.linear[y * width + x];
                        for channel in 0..3 {
                            color[channel] += pixel[channel];
                        }
                        count += 1.0;
                    }
                }
                for channel in &mut color {
                    *channel /= count;
                }
                let luminance = color[0] * 0.2126 + color[1] * 0.7152 + color[2] * 0.0722;
                let contribution = (luminance - BLOOM_THRESHOLD).max(0.0) / luminance.max(0.0001);
                for channel in &mut color {
                    *channel *= contribution;
                }
                self.bright[half_y * self.half_width + half_x] = color;
            }
        }
    }
}

fn blur_horizontal(source: &[[f32; 3]], destination: &mut [[f32; 3]], width: usize, height: usize) {
    for y in 0..height {
        for x in 0..width {
            destination[y * width + x] = weighted_blur(source, width, height, x, y, true);
        }
    }
}

fn blur_vertical(source: &[[f32; 3]], destination: &mut [[f32; 3]], width: usize, height: usize) {
    for y in 0..height {
        for x in 0..width {
            destination[y * width + x] = weighted_blur(source, width, height, x, y, false);
        }
    }
}

fn weighted_blur(
    source: &[[f32; 3]],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    horizontal: bool,
) -> [f32; 3] {
    const OFFSETS: [isize; 5] = [-3, -1, 0, 1, 3];
    const WEIGHTS: [f32; 5] = [0.070270, 0.316216, 0.227027, 0.316216, 0.070270];
    let mut output = [0.0; 3];
    for (offset, weight) in OFFSETS.into_iter().zip(WEIGHTS) {
        let sample_x = if horizontal {
            x.saturating_add_signed(offset).min(width - 1)
        } else {
            x
        };
        let sample_y = if horizontal {
            y
        } else {
            y.saturating_add_signed(offset).min(height - 1)
        };
        let sample = source[sample_y * width + sample_x];
        for channel in 0..3 {
            output[channel] += sample[channel] * weight;
        }
    }
    output
}

#[allow(clippy::too_many_arguments)]
fn composite(
    linear: &[[f32; 3]],
    bloom: &[[f32; 3]],
    output: &mut [u8],
    width: usize,
    half_width: usize,
    x_samples: &[SamplePair],
    y_samples: &[SamplePair],
    tone_map: &[u8],
) {
    for (y, y_sample) in y_samples.iter().copied().enumerate() {
        for (x, x_sample) in x_samples.iter().copied().enumerate() {
            let top = lerp_color(
                bloom[y_sample.low * half_width + x_sample.low],
                bloom[y_sample.low * half_width + x_sample.high],
                x_sample.fraction,
            );
            let bottom = lerp_color(
                bloom[y_sample.high * half_width + x_sample.low],
                bloom[y_sample.high * half_width + x_sample.high],
                x_sample.fraction,
            );
            let glow = lerp_color(top, bottom, y_sample.fraction);
            let source = linear[y * width + x];
            let offset = (y * width + x) * 4;
            for channel in 0..3 {
                let hdr = (source[channel] + glow[channel] * BLOOM_STRENGTH) * EXPOSURE;
                let index = ((hdr.clamp(0.0, TONE_MAP_MAX) / TONE_MAP_MAX) * TONE_MAP_STEPS as f32)
                    .round() as usize;
                output[offset + channel] = tone_map[index];
            }
            output[offset + 3] = 255;
        }
    }
}

fn sample_pairs(full_size: usize, half_size: usize) -> Vec<SamplePair> {
    (0..full_size)
        .map(|position| {
            let coordinate = (position as f32 + 0.5) * half_size as f32 / full_size as f32 - 0.5;
            let low = coordinate.floor().max(0.0) as usize;
            let high = (low + 1).min(half_size - 1);
            SamplePair {
                low: low.min(half_size - 1),
                high,
                fraction: coordinate.fract().max(0.0),
            }
        })
        .collect()
}

fn lerp_color(left: [f32; 3], right: [f32; 3], amount: f32) -> [f32; 3] {
    std::array::from_fn(|channel| left[channel] + (right[channel] - left[channel]) * amount)
}

fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f32) -> f32 {
    if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
mod tests {
    use super::CpuPostRenderer;

    #[test]
    fn output_is_opaque_deterministic_and_reuses_odd_dimensions() {
        let source = [
            0, 0, 0, 255, 255, 255, 255, 255, 32, 64, 128, 255, 9, 8, 7, 255, 240, 10, 20, 255, 80,
            90, 100, 255,
        ];
        let mut renderer = CpuPostRenderer::new();
        let first = renderer
            .process_rgba(3, 2, &source)
            .expect("first frame")
            .to_vec();
        let second = renderer.process_rgba(3, 2, &source).expect("second frame");
        assert_eq!(first, second);
        assert!(second.chunks_exact(4).all(|pixel| pixel[3] == 255));
        assert!(second.iter().any(|byte| *byte != 0));
    }

    #[test]
    fn malformed_frames_are_rejected() {
        let mut renderer = CpuPostRenderer::new();
        assert!(renderer.process_rgba(0, 1, &[]).is_err());
        assert!(renderer.process_rgba(2, 2, &[0; 15]).is_err());
    }

    #[test]
    fn one_emissive_pixel_lights_neighbors() {
        let mut source = vec![0; 9 * 9 * 4];
        for alpha in source.iter_mut().skip(3).step_by(4) {
            *alpha = 255;
        }
        for y in 4..=5 {
            for x in 4..=5 {
                source[(y * 9 + x) * 4..(y * 9 + x) * 4 + 3].fill(255);
            }
        }
        let mut renderer = CpuPostRenderer::new();
        let output = renderer.process_rgba(9, 9, &source).expect("bloom frame");
        let neighbor = &output[(4 * 9 + 3) * 4..(4 * 9 + 3) * 4 + 3];
        assert!(neighbor.iter().any(|channel| *channel > 0));
    }
}

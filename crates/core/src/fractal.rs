//! Shared escape-time color field for Mandelbrot pixels and GPU uniforms.

/// Near-black outer field and interior, identical to the room stage.
pub const MANDELBROT_STAGE: [u8; 3] = crate::palette::STAGE;

/// RGB knots and their smooth escape counts, in ascending order.
///
/// Lightness rises toward the boundary. Hue adds depth without being the only
/// way to read it. The GPU uploads this table directly from core.
pub const MANDELBROT_PALETTE: [[f32; 4]; 8] = [
    [10.0, 11.0, 15.0, 4.0],
    [46.0, 27.0, 74.0, 6.0],
    [57.0, 64.0, 137.0, 8.0],
    [31.0, 124.0, 150.0, 12.0],
    [65.0, 174.0, 147.0, 20.0],
    [195.0, 178.0, 100.0, 32.0],
    [232.0, 181.0, 194.0, 64.0],
    [236.0, 224.0, 206.0, 160.0],
];

/// Color a finite smooth escape count with continuous, eased color bands.
///
/// Counts at or below four and non-finite inputs retain the uniform dark stage.
#[must_use]
pub fn mandelbrot_color(escape: f32) -> [u8; 3] {
    if !escape.is_finite() || escape <= MANDELBROT_PALETTE[0][3] {
        return MANDELBROT_STAGE;
    }
    for pair in MANDELBROT_PALETTE.windows(2) {
        let [lo, hi] = [pair[0], pair[1]];
        if escape <= hi[3] {
            let t = (escape - lo[3]) / (hi[3] - lo[3]);
            let blend = t * t * (3.0 - 2.0 * t);
            return std::array::from_fn(|channel| {
                (lo[channel] + (hi[channel] - lo[channel]) * blend).round() as u8
            });
        }
    }
    let last = MANDELBROT_PALETTE[MANDELBROT_PALETTE.len() - 1];
    [last[0] as u8, last[1] as u8, last[2] as u8]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outer_field_is_uniform_and_the_gradient_has_no_color_steps() {
        for escape in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0, 0.0, 4.0] {
            assert_eq!(mandelbrot_color(escape), MANDELBROT_STAGE);
        }
        let mut previous = MANDELBROT_STAGE;
        for sample in 401..=16_100 {
            let color = mandelbrot_color(sample as f32 / 100.0);
            assert!(color.iter().zip(previous).all(|(&a, b)| a.abs_diff(b) <= 1));
            previous = color;
        }
        assert_eq!(mandelbrot_color(1000.0), [236, 224, 206]);
    }

    #[test]
    fn escape_depth_remains_ordered_in_grayscale() {
        let mut previous = 0.0;
        for knot in MANDELBROT_PALETTE {
            let color = mandelbrot_color(knot[3]);
            let light = crate::photosensitivity::relative_luminance(color[0], color[1], color[2]);
            assert!(light > previous);
            previous = light;
        }
    }
}

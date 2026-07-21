//! The Concentration Bell: random points in high D sit near one radius.
//!
//! Sample unit-cube (or Gaussian) vectors; their norms concentrate. Extremes
//! die. CLICK: DRAW A SAMPLE. See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const SEED: u64 = 0xC0C5_E17A_7100_0001;
const BINS: usize = 32;

fn phase_unit(t: f64) -> f64 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn finite_pokes(pokes: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .copied()
        .filter(|&(x, y)| x.is_finite() && y.is_finite())
        .map(|(x, y)| (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)))
        .collect()
}

fn dim(t: f64, hand: Option<(f64, f64)>) -> u32 {
    if let Some((x, _)) = hand {
        (2 + (x * 48.0) as u32).clamp(2, 50)
    } else {
        (2 + (phase_unit(t) * 40.0) as u32).clamp(2, 40)
    }
}

fn sample_norms(d: u32, n: usize, seed: u64) -> Vec<f64> {
    let mut rng = SplitMix64::new(SEED ^ seed ^ (d as u64).wrapping_mul(0x9E37));
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let mut s = 0.0;
        for _ in 0..d {
            // Standard normal via Box-Muller pair, one component.
            let u1 = rng.next_f64().clamp(1e-12, 1.0);
            let u2 = rng.next_f64();
            let g = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
            s += g * g;
        }
        out.push(s.sqrt());
    }
    out
}

fn histogram(norms: &[f64], bins: usize) -> (Vec<u32>, f64, f64) {
    let mut min = f64::MAX;
    let mut max = f64::MIN;
    for &v in norms {
        min = min.min(v);
        max = max.max(v);
    }
    if !min.is_finite() || max - min < 1e-9 {
        min = 0.0;
        max = 1.0;
    }
    let mut hist = vec![0u32; bins];
    for &v in norms {
        let t = ((v - min) / (max - min)).clamp(0.0, 1.0 - 1e-9);
        let i = (t * bins as f64) as usize;
        hist[i.min(bins - 1)] += 1;
    }
    (hist, min, max)
}

fn draw(canvas: &mut dyn Surface, hist: &[u32], mean_mark: f64, min: f64, max: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || hist.is_empty() {
        return;
    }
    let peak = hist.iter().copied().max().unwrap_or(1).max(1);
    let bins = hist.len();
    for (i, &c) in hist.iter().enumerate() {
        let x = ((i as f64 + 0.5) / bins as f64 * width as f64).round() as i32;
        let h = (c as f64 / peak as f64) * height as f64 * 0.85;
        let y1 = height.saturating_sub(1) as i32;
        let y0 = (y1 as f64 - h).round() as i32;
        canvas.line(x, y1, x, y0, if c * 2 > peak { '#' } else { '*' });
    }
    // Mean radius tick.
    if max > min {
        let u = ((mean_mark - min) / (max - min)).clamp(0.0, 1.0);
        let mx = (u * width.saturating_sub(1) as f64).round() as i32;
        canvas.line(mx, 0, mx, height.saturating_sub(1) as i32, '+');
    }
}

/// Concentration Bell room.
#[derive(Debug, Default)]
pub struct Concentration {
    seed: u64,
}

impl Concentration {
    /// Create the room with default seed (0).
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }
    /// Create with variation seed.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }
}

impl Room for Concentration {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "concentration",
            title: "The Concentration Bell",
            wing: "Number & Pattern",
            blurb: "Random points in high dimension all sit near the same radius; extremes die. t \
                    raises d; CLICK: DRAW A SAMPLE.",
            accent: [100, 180, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let d = dim(t, None);
        let norms = sample_norms(d, 400, self.seed);
        let mean = norms.iter().sum::<f64>() / norms.len().max(1) as f64;
        let (hist, min, max) = histogram(&norms, BINS);
        draw(canvas, &hist, mean, min, max);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "norm bell",
            root: 196.0,
            tempo: 108,
            line: &[0, 3, 7, 12, 7, 3, 0, 7],
            encodes: "high-D norms crowding one radius",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: DRAW A SAMPLE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = dim(t, None);
        let norms = sample_norms(d, 200, self.seed);
        let mean = norms.iter().sum::<f64>() / norms.len() as f64;
        let var = norms.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / norms.len() as f64;
        Some(format!(
            "d={d}  mean={mean:.1}  sd={:.2}  CLICK",
            var.sqrt()
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let d = dim(t, hands.last().copied());
        let extra = hands.len() * 80;
        let norms = sample_norms(d, 300 + extra, self.seed ^ hands.len() as u64);
        let mean = norms.iter().sum::<f64>() / norms.len().max(1) as f64;
        let (hist, min, max) = histogram(&norms, BINS);
        draw(canvas, &hist, mean, min, max);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let d = dim(t, hands.last().copied());
        let norms = sample_norms(d, 300 + hands.len() * 50, self.seed ^ hands.len() as u64);
        let mean = norms.iter().sum::<f64>() / norms.len() as f64;
        let var = norms.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / norms.len() as f64;
        Some(format!(
            "SAMPLE d={d}  mean={mean:.1}  sd={:.2}",
            var.sqrt()
        ))
    }

    fn reveal(&self) -> &'static str {
        "In high dimension, independent coordinates make Euclidean norms \
         concentrate tightly around one typical radius. Extreme outliers become \
         rare: the same geometry that haunts modern machine learning."
    }
}

#[cfg(test)]
mod tests {
    use super::{Concentration, sample_norms};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Concentration::new().status(0.4).unwrap();
        assert!(s.contains("CLICK") || s.contains("d="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn sample_changes() {
        let r = Concentration::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.8,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn high_d_tighter_relative() {
        let lo = sample_norms(3, 200, 1);
        let hi = sample_norms(40, 200, 1);
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let sd = |v: &[f64]| {
            let m = mean(v);
            (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / v.len() as f64).sqrt()
        };
        // Coefficient of variation shrinks with d.
        assert!(sd(&hi) / mean(&hi) < sd(&lo) / mean(&lo));
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Concentration::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(Concentration::new().motif().unwrap().line.len() >= 6);
    }
}

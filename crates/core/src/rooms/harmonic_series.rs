//! Harmonic series: H_n = sum 1/k grows like ln n + gamma.
//!
//! DRAG: TUNE N. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

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

fn n_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 { 0.0 } else { (seed % 20) as f64 };
    if let Some((x, _)) = hand {
        5.0 + x * 95.0 + s
    } else {
        10.0 + phase_unit(t) * 80.0 + s
    }
}

fn harmonic(n: u32) -> f64 {
    let mut h = 0.0;
    for k in 1..=n {
        h += 1.0 / k as f64;
    }
    h
}

fn draw(canvas: &mut dyn Surface, n_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n_mark = n_f.round().clamp(3.0, 120.0) as u32;
    let max_n = 80u32;
    let gamma = 0.5772156649;
    let h_max = harmonic(max_n);
    let mut prev_h: Option<(i32, i32)> = None;
    let mut prev_l: Option<(i32, i32)> = None;
    for n in 1..=max_n {
        let h = harmonic(n);
        let ln = (n as f64).ln() + gamma;
        let x = ((n as f64 / max_n as f64) * width.saturating_sub(1) as f64).round() as i32;
        let yh = ((1.0 - h / h_max) * height.saturating_sub(1) as f64 * 0.9 + height as f64 * 0.05)
            .round() as i32;
        let yl = ((1.0 - ln / h_max) * height.saturating_sub(1) as f64 * 0.9 + height as f64 * 0.05)
            .round()
            .clamp(0.0, height.saturating_sub(1) as f64) as i32;
        if let Some((ox, oy)) = prev_h {
            canvas.line(ox, oy, x, yh, '#');
        }
        if let Some((ox, oy)) = prev_l {
            canvas.line(ox, oy, x, yl, '.');
        }
        prev_h = Some((x, yh));
        prev_l = Some((x, yl));
    }
    let xm = ((n_mark as f64 / max_n as f64) * width.saturating_sub(1) as f64).round() as i32;
    canvas.line(xm, 0, xm, height.saturating_sub(1) as i32, '|');
    let _ = seed;
}

/// Harmonic series room.
#[derive(Debug, Default)]
pub struct HarmonicSeries {
    seed: u64,
}

impl HarmonicSeries {
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

impl Room for HarmonicSeries {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "harmonic-series",
            title: "Harmonic Series",
            wing: "Number & Pattern",
            blurb: "H_n grows like ln n + gamma. t and DRAG: TUNE N.",
            accent: [50, 100, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, n_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "harmonic-series",
            root: 18.35,
            tempo: 78,
            line: &[0, 2, 4, 7, 9, 12, 7, 2],
            encodes: "harmonic H_n diverges like natural log plus Euler gamma",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE N")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = n_param(t, None, self.seed).round() as u32;
        let h = harmonic(n.max(1));
        Some(format!("n={n}  H={h:.2}  DRAG:N"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = n_param(t, hands.last().copied(), self.seed);
        draw(canvas, n, self.seed ^ hands.len() as u64);
        if let Some(&(x, y)) = hands.last() {
            let (bw, bh) = canvas.draw_bounds();
            if bw > 0 && bh > 0 {
                let px = (x * bw.saturating_sub(1) as f64).round() as i32;
                let py = (y * bh.saturating_sub(1) as f64).round() as i32;
                canvas.line(px - 2, py, px + 2, py, 'o');
                canvas.line(px, py - 2, px, py + 2, 'o');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = n_param(t, hands.last().copied(), self.seed).round() as u32;
        let h = harmonic(n.max(1));
        Some(format!("N={n}  H={h:.3}"))
    }

    fn reveal(&self) -> &'static str {
        "The harmonic series H_n = 1 + 1/2 + ... + 1/n diverges, but slowly: it \
         tracks ln n + gamma, Euler's constant. Integral tests and integral bounds \
         make the log growth inevitable."
    }
}

#[cfg(test)]
mod tests {
    use super::{HarmonicSeries, harmonic};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn h4() {
        assert!((harmonic(4) - 2.083333333).abs() < 1e-6);
    }

    #[test]
    fn status_invites() {
        let s = HarmonicSeries::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("H="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = HarmonicSeries::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        HarmonicSeries::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

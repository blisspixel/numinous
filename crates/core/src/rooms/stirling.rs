//! Stirling approximation: n! ~ sqrt(2 pi n) (n/e)^n.
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
    let s = if seed == 0 { 0.0 } else { (seed % 5) as f64 };
    if let Some((x, _)) = hand {
        2.0 + x * 28.0 + s
    } else {
        3.0 + phase_unit(t) * 22.0 + s
    }
}

fn log_fact(n: u32) -> f64 {
    let mut s = 0.0;
    for k in 2..=n {
        s += (k as f64).ln();
    }
    s
}

fn log_stirling(n: f64) -> f64 {
    if n < 1.0 {
        return 0.0;
    }
    0.5 * (2.0 * std::f64::consts::PI * n).ln() + n * n.ln() - n
}

fn draw(canvas: &mut dyn Surface, n_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n_mark = n_f.round().clamp(2.0, 35.0) as u32;
    let max_n = 30u32;
    let max_log = log_fact(max_n);
    let mut prev_f: Option<(i32, i32)> = None;
    let mut prev_s: Option<(i32, i32)> = None;
    for n in 1..=max_n {
        let lf = log_fact(n);
        let ls = log_stirling(n as f64);
        let x = ((n as f64 / max_n as f64) * width.saturating_sub(1) as f64).round() as i32;
        let yf = ((1.0 - lf / max_log) * height.saturating_sub(1) as f64 * 0.9
            + height as f64 * 0.05)
            .round() as i32;
        let ys = ((1.0 - ls / max_log) * height.saturating_sub(1) as f64 * 0.9
            + height as f64 * 0.05)
            .round()
            .clamp(0.0, height.saturating_sub(1) as f64) as i32;
        if let Some((ox, oy)) = prev_f {
            canvas.line(ox, oy, x, yf, '#');
        }
        if let Some((ox, oy)) = prev_s {
            canvas.line(ox, oy, x, ys, '.');
        }
        prev_f = Some((x, yf));
        prev_s = Some((x, ys));
    }
    let xm = ((n_mark as f64 / max_n as f64) * width.saturating_sub(1) as f64).round() as i32;
    canvas.line(xm, 0, xm, height.saturating_sub(1) as i32, '|');
    let _ = seed;
}

/// Stirling approximation room.
#[derive(Debug, Default)]
pub struct Stirling {
    seed: u64,
}

impl Stirling {
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

impl Room for Stirling {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "stirling",
            title: "Stirling Approx",
            wing: "Number & Pattern",
            blurb: "n! vs sqrt(2 pi n)(n/e)^n on a log scale. t and DRAG: TUNE N.",
            accent: [80, 60, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, n_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "stirling",
            root: 16.35,
            tempo: 86,
            line: &[0, 3, 7, 10, 12, 10, 7, 3],
            encodes: "Stirling: factorial asymptotics from Gamma and Laplace method",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE N")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = n_param(t, None, self.seed).round() as u32;
        let n = n.max(1);
        let ratio = (log_fact(n) - log_stirling(n as f64)).exp();
        Some(format!("n={n}  n!/S={ratio:.3}  DRAG:N"))
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
        let n = n.max(1);
        let ratio = (log_fact(n) - log_stirling(n as f64)).exp();
        Some(format!("N={n}  ratio={ratio:.3}"))
    }

    fn reveal(&self) -> &'static str {
        "Stirling's approximation n! ~ sqrt(2 pi n) (n/e)^n turns factorial growth \
         into elementary functions. Relative error vanishes as n grows; it is the \
         workhorse of statistical mechanics and combinatorial asymptotics."
    }
}

#[cfg(test)]
mod tests {
    use super::Stirling;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Stirling::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("n!"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = Stirling::new();
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
        Stirling::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}

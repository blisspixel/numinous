//! Basel problem: sum 1/n^2 converges to pi^2/6.
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
    let s = if seed == 0 { 0.0 } else { (seed % 15) as f64 };
    if let Some((x, _)) = hand {
        3.0 + x * 80.0 + s
    } else {
        5.0 + phase_unit(t) * 60.0 + s
    }
}

fn partial_basel(n: u32) -> f64 {
    let mut s = 0.0;
    for k in 1..=n {
        s += 1.0 / (k as f64 * k as f64);
    }
    s
}

fn draw(canvas: &mut dyn Surface, n_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n_mark = n_f.round().clamp(2.0, 100.0) as u32;
    let max_n = 60u32;
    let target = std::f64::consts::PI * std::f64::consts::PI / 6.0;
    let mut prev: Option<(i32, i32)> = None;
    for n in 1..=max_n {
        let s = partial_basel(n);
        let x = ((n as f64 / max_n as f64) * width.saturating_sub(1) as f64).round() as i32;
        let y = ((1.0 - s / target) * height.saturating_sub(1) as f64 * 0.85 + height as f64 * 0.1)
            .round()
            .clamp(0.0, height.saturating_sub(1) as f64) as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, x, y, '#');
        }
        prev = Some((x, y));
    }
    // asymptote pi^2/6 at top of plot (y small when s~target)
    let y_t = (height as f64 * 0.1).round() as i32;
    canvas.line(0, y_t, width.saturating_sub(1) as i32, y_t, '.');
    let xm = ((n_mark as f64 / max_n as f64) * width.saturating_sub(1) as f64).round() as i32;
    canvas.line(xm, 0, xm, height.saturating_sub(1) as i32, '|');
    let _ = seed;
}

/// Basel problem room.
#[derive(Debug, Default)]
pub struct Basel {
    seed: u64,
}

impl Basel {
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

impl Room for Basel {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "basel",
            title: "Basel Problem",
            wing: "Number & Pattern",
            blurb: "sum 1/n^2 climbs to pi^2/6. t and DRAG: TUNE N.",
            accent: [120, 40, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, n_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "basel",
            root: 17.32,
            tempo: 82,
            line: &[0, 5, 7, 12, 7, 5, 0, 12],
            encodes: "Basel: Euler proved sum 1/n^2 equals pi squared over six",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE N")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = n_param(t, None, self.seed).round() as u32;
        let s = partial_basel(n.max(1));
        let target = std::f64::consts::PI * std::f64::consts::PI / 6.0;
        let err = (target - s).max(0.0);
        Some(format!("n={n}  s={s:.3}  err={err:.3}  DRAG:N"))
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
        let s = partial_basel(n.max(1));
        let target = std::f64::consts::PI * std::f64::consts::PI / 6.0;
        let pct = ((s / target).clamp(0.0, 1.0) * 100.0).round() as i32;
        let err = (target - s).max(0.0);
        Some(format!("s={s:.4}  {pct}% of pi2/6  err={err:.3}"))
    }

    fn reveal(&self) -> &'static str {
        "The Basel problem asked for the exact sum of 1/n^2. Euler found pi^2/6, \
         linking primes-free squares to the circle constant through Fourier series \
         of x^2 on [-pi, pi]."
    }
}

#[cfg(test)]
mod tests {
    use super::{Basel, partial_basel};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn approaches_target() {
        let s = partial_basel(1000);
        let t = std::f64::consts::PI * std::f64::consts::PI / 6.0;
        assert!((s - t).abs() < 0.001);
    }

    #[test]
    fn status_invites() {
        let s = Basel::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("s=") || s.contains("pi2"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn action_reports_fraction_of_target() {
        let s = Basel::new()
            .status_input(
                0.5,
                &[RoomInput::PointerDown {
                    x: 0.7,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert!(s.contains("pi2/6") || s.contains("err"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = Basel::new();
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
        Basel::new().render(&mut c, 0.45);
        assert!(c.ink_count() > 0);
    }
}

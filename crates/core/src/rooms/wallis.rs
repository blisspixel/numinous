//! Wallis product: pi/2 as an infinite product of rational pairs.
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
    let s = if seed == 0 { 0.0 } else { (seed % 10) as f64 };
    if let Some((x, _)) = hand {
        2.0 + x * 60.0 + s
    } else {
        4.0 + phase_unit(t) * 50.0 + s
    }
}

/// Partial Wallis: product_{k=1}^n (4k^2)/(4k^2-1) -> pi/2.
fn wallis_partial(n: u32) -> f64 {
    let mut p = 1.0;
    for k in 1..=n {
        let kk = k as f64;
        p *= (4.0 * kk * kk) / (4.0 * kk * kk - 1.0);
    }
    p
}

fn draw(canvas: &mut dyn Surface, n_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n_mark = n_f.round().clamp(2.0, 80.0) as u32;
    let max_n = 50u32;
    let target = std::f64::consts::FRAC_PI_2;
    let mut prev: Option<(i32, i32)> = None;
    for n in 1..=max_n {
        let w = wallis_partial(n);
        let x = ((n as f64 / max_n as f64) * width.saturating_sub(1) as f64).round() as i32;
        // plot w/target so it approaches 1
        let ratio = (w / target).clamp(0.0, 1.2);
        let y = ((1.0 - ratio / 1.2) * height.saturating_sub(1) as f64 * 0.9 + height as f64 * 0.05)
            .round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, x, y, '#');
        }
        prev = Some((x, y));
    }
    let y1 = ((1.0 - 1.0 / 1.2) * height.saturating_sub(1) as f64 * 0.9 + height as f64 * 0.05)
        .round() as i32;
    canvas.line(0, y1, width.saturating_sub(1) as i32, y1, '.');
    let xm = ((n_mark as f64 / max_n as f64) * width.saturating_sub(1) as f64).round() as i32;
    canvas.line(xm, 0, xm, height.saturating_sub(1) as i32, '|');
    let _ = seed;
}

/// Wallis product room.
#[derive(Debug, Default)]
pub struct Wallis {
    seed: u64,
}

impl Wallis {
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

impl Room for Wallis {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "wallis",
            title: "Wallis Product",
            wing: "Number & Pattern",
            blurb: "Product (4k^2)/(4k^2-1) -> pi/2. t and DRAG: TUNE N.",
            accent: [100, 50, 80],
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
            key: "wallis",
            root: 13.75,
            tempo: 88,
            line: &[0, 5, 7, 9, 12, 9, 7, 5],
            encodes: "Wallis product builds pi from rational even-odd pairs",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE N")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = n_param(t, None, self.seed).round() as u32;
        let w = wallis_partial(n.max(1));
        let pi_est = 2.0 * w;
        Some(format!("n={n}  2W={pi_est:.4}  DRAG:N"))
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
        let w = wallis_partial(n.max(1));
        let pi_est = 2.0 * w;
        Some(format!("N={n}  pi~{pi_est:.4}"))
    }

    fn reveal(&self) -> &'static str {
        "Wallis wrote pi/2 as the infinite product (2/1)(2/3)(4/3)(4/5)(6/5)(6/7)... \
         Modern form multiplies 4k^2/(4k^2-1). It is an early analytic expression \
         for pi from ratios alone."
    }
}

#[cfg(test)]
mod tests {
    use super::{Wallis, wallis_partial};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn approaches_pi_over_two() {
        let w = wallis_partial(200);
        assert!((w - std::f64::consts::FRAC_PI_2).abs() < 0.01);
    }

    #[test]
    fn status_invites() {
        let s = Wallis::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("2W"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = Wallis::new();
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
        Wallis::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

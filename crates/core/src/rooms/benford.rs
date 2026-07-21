//! Benford's law: leading-digit frequencies log10(1+1/d).
//!
//! DRAG: TUNE BASE. See `docs/ROOMS.md`.

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

fn base_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 { 0.0 } else { (seed % 3) as f64 };
    // base from 2 to 16-ish, default 10
    if let Some((x, _)) = hand {
        2.0 + x * 14.0 + s * 0.1
    } else {
        4.0 + phase_unit(t) * 10.0 + s * 0.1
    }
}

fn benford_p(d: u32, base: f64) -> f64 {
    if d == 0 || base <= 1.0 {
        return 0.0;
    }
    ((1.0 + 1.0 / d as f64).ln()) / base.ln()
}

fn draw(canvas: &mut dyn Surface, base: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let base = base.clamp(2.0, 16.0);
    let max_d = (base.floor() as u32 - 1).clamp(1, 15);
    let max_p = benford_p(1, base).max(1e-9);
    let bar_w = (width / (max_d as usize + 1)).max(1);
    for d in 1..=max_d {
        let p = benford_p(d, base);
        let h = ((p / max_p) * height.saturating_sub(2) as f64 * 0.9).round() as i32;
        let x0 = ((d as usize - 1) * bar_w) as i32 + 1;
        let x1 = (d as usize * bar_w) as i32 - 1;
        let y0 = height.saturating_sub(2) as i32;
        let y1 = y0 - h;
        for x in x0..=x1.max(x0) {
            canvas.line(x, y0, x, y1, if d == 1 { '#' } else { '*' });
        }
    }
    let _ = seed;
}

/// Benford law room.
#[derive(Debug, Default)]
pub struct Benford {
    seed: u64,
}

impl Benford {
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

impl Room for Benford {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "benford",
            title: "Benford Law",
            wing: "Chance & Order",
            blurb: "Leading digits: log law P(d)=log(1+1/d). t and DRAG: TUNE BASE.",
            accent: [40, 90, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, base_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "benford",
            root: 15.43,
            tempo: 90,
            line: &[0, 5, 3, 8, 12, 8, 3, 5],
            encodes: "Benford: mantissa scale invariance makes leading 1 most common",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE BASE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let b = base_param(t, None, self.seed);
        let p1 = benford_p(1, b);
        Some(format!("b={b:.1}  P1={p1:.2}  DRAG:BASE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let b = base_param(t, hands.last().copied(), self.seed);
        draw(canvas, b, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let b = base_param(t, hands.last().copied(), self.seed);
        let p1 = benford_p(1, b);
        let p9 = benford_p(9, b);
        let ratio = if p9 > 1e-9 { p1 / p9 } else { 0.0 };
        Some(format!("P(1)={p1:.3}  P(9)={p9:.3}  1/9={ratio:.1}x"))
    }

    fn reveal(&self) -> &'static str {
        "Benford's law says the leading digit d of many real-world numbers occurs \
         with probability log_b(1+1/d). Scale invariance of mantissas forces more \
         1s than 9s; fraud audits use the same skew."
    }
}

#[cfg(test)]
mod tests {
    use super::{Benford, benford_p};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn digit_one_about_30pct() {
        let p = benford_p(1, 10.0);
        let expected = (2.0_f64).log10();
        assert!((p - expected).abs() < 1e-9);
    }

    #[test]
    fn status_invites() {
        let s = Benford::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("P1"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn base_changes() {
        let r = Benford::new();
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
        Benford::new().render(&mut c, 0.6);
        assert!(c.ink_count() > 0);
    }
}

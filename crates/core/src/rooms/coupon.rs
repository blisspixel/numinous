//! Coupon collector: expected trials to complete a set of n types.
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
    let s = if seed == 0 { 0.0 } else { (seed % 8) as f64 };
    if let Some((x, _)) = hand {
        2.0 + x * 40.0 + s
    } else {
        4.0 + phase_unit(t) * 32.0 + s
    }
}

fn expected_t(n: u32) -> f64 {
    if n == 0 {
        return 0.0;
    }
    // E[T] = n * H_n
    let mut h = 0.0;
    for k in 1..=n {
        h += 1.0 / k as f64;
    }
    n as f64 * h
}

fn draw(canvas: &mut dyn Surface, n_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n_mark = n_f.round().clamp(2.0, 48.0) as u32;
    let max_n = 40u32;
    let max_e = expected_t(max_n);
    let mut prev: Option<(i32, i32)> = None;
    for n in 1..=max_n {
        let e = expected_t(n);
        let x = ((n as f64 / max_n as f64) * width.saturating_sub(1) as f64).round() as i32;
        let y = ((1.0 - e / max_e) * height.saturating_sub(1) as f64 * 0.9 + height as f64 * 0.05)
            .round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, x, y, '#');
        }
        prev = Some((x, y));
    }
    // n log n guide
    let mut prev_g: Option<(i32, i32)> = None;
    for n in 2..=max_n {
        let g = n as f64 * (n as f64).ln();
        let x = ((n as f64 / max_n as f64) * width.saturating_sub(1) as f64).round() as i32;
        let y = ((1.0 - g / max_e) * height.saturating_sub(1) as f64 * 0.9 + height as f64 * 0.05)
            .round()
            .clamp(0.0, height.saturating_sub(1) as f64) as i32;
        if let Some((ox, oy)) = prev_g {
            canvas.line(ox, oy, x, y, '.');
        }
        prev_g = Some((x, y));
    }
    let xm = ((n_mark as f64 / max_n as f64) * width.saturating_sub(1) as f64).round() as i32;
    canvas.line(xm, 0, xm, height.saturating_sub(1) as i32, '|');
    let _ = seed;
}

/// Coupon collector room.
#[derive(Debug, Default)]
pub struct Coupon {
    seed: u64,
}

impl Coupon {
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

impl Room for Coupon {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "coupon",
            title: "Coupon Collector",
            wing: "Chance & Order",
            blurb: "Expected waits n H_n to finish a set. t and DRAG: TUNE N.",
            accent: [100, 80, 30],
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
            key: "coupon",
            root: 21.83,
            tempo: 94,
            line: &[0, 5, 7, 10, 12, 10, 7, 5],
            encodes: "coupon collector: expected time n H_n to complete all types",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE N")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = n_param(t, None, self.seed).round() as u32;
        let e = expected_t(n.max(1));
        let last = n as f64; // expected waits for last coupon is n
        Some(format!("n={n}  E={e:.1}  last~{last:.0}  DRAG:N"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = n_param(t, hands.last().copied(), self.seed);
        draw(canvas, n, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = n_param(t, hands.last().copied(), self.seed)
            .round()
            .max(1.0) as u32;
        let e = expected_t(n);
        let hn = e / n as f64;
        let last = n as f64;
        Some(format!("E={e:.1}  ~n Hn  Hn={hn:.2}  last~{last:.0}"))
    }

    fn reveal(&self) -> &'static str {
        "To collect all n coupon types with equal odds, the expected number of \
         draws is n H_n, about n log n. The last few types dominate the wait: \
         each new one takes longer as the unseen set shrinks."
    }
}

#[cfg(test)]
mod tests {
    use super::{Coupon, expected_t};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn expected_two() {
        // n=2: 2(1+1/2)=3
        assert!((expected_t(2) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn status_invites() {
        let s = Coupon::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("E="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = Coupon::new();
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
        Coupon::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

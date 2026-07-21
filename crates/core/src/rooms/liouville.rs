//! Liouville function: lambda(n) = (-1)^{Omega(n)} by prime factors with multiplicity.
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
        16.0 + x * 140.0 + s
    } else {
        30.0 + phase_unit(t) * 110.0 + s
    }
}

fn lambda(n: u32) -> i8 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut omega = 0u32;
    let mut p = 2u32;
    while p * p <= x {
        while x.is_multiple_of(p) {
            x /= p;
            omega += 1;
        }
        p += if p == 2 { 1 } else { 2 };
    }
    if x > 1 {
        omega += 1;
    }
    if omega.is_multiple_of(2) { 1 } else { -1 }
}

fn draw(canvas: &mut dyn Surface, n_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = n_f.round().clamp(8.0, 180.0) as u32;
    let y_pos = (height as f64 * 0.3).round() as i32;
    let y_neg = (height as f64 * 0.7).round() as i32;
    let cy = height as f64 * 0.5;
    // Cumulative L(n) = sum lambda
    let mut sum = 0i32;
    let mut prev: Option<(i32, i32)> = None;
    let mut max_abs = 1u32;
    let mut sums = Vec::with_capacity(n as usize);
    for k in 1..=n {
        sum += i32::from(lambda(k));
        max_abs = max_abs.max(sum.unsigned_abs());
        sums.push(sum);
    }
    let max_abs = max_abs.max(1) as f64;
    let y_scale = height as f64 * 0.4 / max_abs;
    canvas.line(0, cy as i32, width.saturating_sub(1) as i32, cy as i32, '.');
    for (i, &s) in sums.iter().enumerate() {
        let x = ((i as f64 / n.saturating_sub(1).max(1) as f64) * width.saturating_sub(1) as f64)
            .round() as i32;
        let y = (cy - s as f64 * y_scale).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, x, y, '#');
        }
        // mark sign of lambda as tick
        let lv = lambda(i as u32 + 1);
        let ty = if lv > 0 { y_pos } else { y_neg };
        canvas.line(x, ty, x, ty, if lv > 0 { '+' } else { '*' });
        prev = Some((x, y));
    }
    let _ = seed;
}

/// Liouville function room.
#[derive(Debug, Default)]
pub struct Liouville {
    seed: u64,
}

impl Liouville {
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

impl Room for Liouville {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "liouville",
            title: "Liouville Function",
            wing: "Number & Pattern",
            blurb: "lambda(n) by total prime factors; L(n) sum. t and DRAG: TUNE N.",
            accent: [100, 40, 100],
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
            key: "liouville",
            root: 34.65,
            tempo: 84,
            line: &[0, 2, 7, 5, 12, 5, 7, 2],
            encodes: "Liouville lambda: parity of Omega, complete additive character",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE N")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = n_param(t, None, self.seed).round() as u32;
        let mut s = 0i32;
        for k in 1..=n {
            s += i32::from(lambda(k));
        }
        Some(format!("n={n}  L={s}  DRAG:N"))
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
        let n = n_param(t, hands.last().copied(), self.seed).round() as u32;
        let mut s = 0i32;
        for k in 1..=n {
            s += i32::from(lambda(k));
        }
        Some(format!("N={n}  L={s}"))
    }

    fn reveal(&self) -> &'static str {
        "Liouville's lambda(n) is +1 or -1 by the parity of the number of prime \
         factors counting multiplicity. Its partial sums L(n) are another \
         arithmetic path whose average behavior is equivalent to the prime number \
         theorem."
    }
}

#[cfg(test)]
mod tests {
    use super::{Liouville, lambda};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn lambda_basics() {
        assert_eq!(lambda(1), 1);
        assert_eq!(lambda(2), -1);
        assert_eq!(lambda(4), 1); // 2*2
        assert_eq!(lambda(6), 1); // 2*3
    }

    #[test]
    fn status_invites() {
        let s = Liouville::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("L="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = Liouville::new();
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
        Liouville::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

//! Euler's totient: phi(n) counts units mod n.
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
        12.0 + x * 100.0 + s
    } else {
        20.0 + phase_unit(t) * 80.0 + s
    }
}

fn phi(n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    let mut result = n;
    let mut x = n;
    let mut p = 2u32;
    while p * p <= x {
        if x.is_multiple_of(p) {
            while x.is_multiple_of(p) {
                x /= p;
            }
            result -= result / p;
        }
        p += if p == 2 { 1 } else { 2 };
    }
    if x > 1 {
        result -= result / x;
    }
    result
}

fn draw(canvas: &mut dyn Surface, n_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = n_f.round().clamp(8.0, 140.0) as u32;
    // y = n line (diagonal reference) and phi(n) below it.
    let mut prev_n: Option<(i32, i32)> = None;
    let mut prev_p: Option<(i32, i32)> = None;
    for k in 1..=n {
        let x = ((k as f64 / n as f64) * width.saturating_sub(1) as f64).round() as i32;
        let yn = ((1.0 - k as f64 / n as f64) * height.saturating_sub(1) as f64 * 0.9
            + height as f64 * 0.05)
            .round() as i32;
        let pk = phi(k) as f64;
        let yp = ((1.0 - pk / n as f64) * height.saturating_sub(1) as f64 * 0.9
            + height as f64 * 0.05)
            .round() as i32;
        if let Some((ox, oy)) = prev_n {
            canvas.line(ox, oy, x, yn, '.');
        }
        if let Some((ox, oy)) = prev_p {
            canvas.line(ox, oy, x, yp, '#');
        }
        prev_n = Some((x, yn));
        prev_p = Some((x, yp));
    }
    let _ = seed;
}

/// Euler totient room.
#[derive(Debug, Default)]
pub struct EulerTotient {
    seed: u64,
}

impl EulerTotient {
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

impl Room for EulerTotient {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "euler-totient",
            title: "Euler Totient",
            wing: "Number & Pattern",
            blurb: "phi(n): count of units mod n. t and DRAG: TUNE N.",
            accent: [40, 110, 80],
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
            key: "euler-totient",
            root: 32.7,
            tempo: 88,
            line: &[0, 4, 7, 11, 12, 11, 7, 4],
            encodes: "Euler phi: multiplicative count of residues coprime to n",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE N")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = n_param(t, None, self.seed).round() as u32;
        let p = phi(n.max(1));
        Some(format!("n={n}  phi={p}  DRAG:N"))
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
        let n = n.max(1);
        let p = phi(n);
        // Density phi(n)/n of units mod n.
        let dens = p as f64 / n as f64;
        Some(format!("n={n}  phi={p}  dens={dens:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Euler's totient phi(n) counts integers in 1..n coprime to n. It is \
         multiplicative, appears in Euler's theorem a^{phi(n)} = 1 mod n when \
         gcd(a,n)=1, and underpins RSA key size as (p-1)(q-1)."
    }
}

#[cfg(test)]
mod tests {
    use super::{EulerTotient, phi};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn phi_basics() {
        assert_eq!(phi(1), 1);
        assert_eq!(phi(2), 1);
        assert_eq!(phi(7), 6);
        assert_eq!(phi(10), 4);
    }

    #[test]
    fn status_invites() {
        let s = EulerTotient::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("phi"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = EulerTotient::new();
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
        EulerTotient::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

//! Ulam Spiral: primes on a square spiral of naturals.
//!
//! Write 1 at the center, spiral out; mark primes. Diagonals light with primes.
//! DRAG: ZOOM THE SPIRAL. See `docs/ROOMS.md`.

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

fn is_prime(n: u32) -> bool {
    if n < 2 {
        return false;
    }
    if n.is_multiple_of(2) {
        return n == 2;
    }
    let mut d = 3u32;
    while d * d <= n {
        if n.is_multiple_of(d) {
            return false;
        }
        d += 2;
    }
    true
}

fn max_n(t: f64, hand: Option<(f64, f64)>) -> u32 {
    // Larger ceilings so primes fill a large window.
    if let Some((x, _)) = hand {
        (400 + (x * 2_600.0) as u32).clamp(400, 3_200)
    } else {
        (500 + (phase_unit(t) * 2_000.0) as u32).clamp(400, 2_800)
    }
}

/// Ulam coordinates: n -> (x,y) integer lattice, 1 at origin.
fn ulam_xy(n: u32) -> (i32, i32) {
    if n <= 1 {
        return (0, 0);
    }
    // Walk the spiral from 1.
    let mut x = 0i32;
    let mut y = 0i32;
    let mut dx = 0i32;
    let mut dy = -1i32;
    for _ in 1..n {
        if x == y || (x < 0 && x == -y) || (x > 0 && x == 1 - y) {
            let t = dx;
            dx = -dy;
            dy = t;
        }
        x += dx;
        y += dy;
    }
    (x, y)
}

fn draw(canvas: &mut dyn Surface, max_n: u32, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let mut min_x = 0i32;
    let mut max_x = 0i32;
    let mut min_y = 0i32;
    let mut max_y = 0i32;
    let mut pts = Vec::new();
    for n in 1..=max_n {
        let (x, y) = ulam_xy(n);
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
        if is_prime(n) {
            pts.push((x, y, n));
        }
    }
    let _ = seed;
    let dx = (max_x - min_x).max(1) as f64;
    let dy = (max_y - min_y).max(1) as f64;
    let cell = ((width.min(height) as f64 / dx.max(dy)).floor() as i32).clamp(1, 3);
    for &(x, y, n) in &pts {
        let u = (x - min_x) as f64 / dx;
        let v = (y - min_y) as f64 / dy;
        let px = (u * width.saturating_sub(1) as f64).round() as i32;
        let py = ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32;
        let ch = if n < 20 { '#' } else { '*' };
        for dy in 0..cell {
            for dx in 0..cell {
                canvas.plot(px + dx, py + dy, ch);
            }
        }
    }
}

/// Ulam Spiral room.
#[derive(Debug, Default)]
pub struct UlamSpiral {
    seed: u64,
}

impl UlamSpiral {
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

impl Room for UlamSpiral {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "ulam-spiral",
            title: "The Ulam Spiral",
            wing: "Number & Pattern",
            blurb: "Naturals on a square spiral; primes light diagonals. t and DRAG: ZOOM THE \
                    SPIRAL.",
            accent: [100, 140, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let n = max_n(t, None)
            + if self.seed == 0 {
                0
            } else {
                (self.seed % 20) as u32
            };
        draw(canvas, n, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "ulam",
            root: 220.0,
            tempo: 108,
            line: &[0, 5, 7, 10, 12, 10, 7, 5],
            encodes: "primes aligning on diagonals of a number spiral",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: ZOOM THE SPIRAL")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = max_n(t, None);
        let primes = (1..=n).filter(|&k| is_prime(k)).count();
        Some(format!("N={n}  primes={primes}  DRAG:ZOOM"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = max_n(t, hands.last().copied());
        draw(canvas, n, self.seed ^ hands.len() as u64);
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.line(px - 2, py, px + 2, py, '+');
                canvas.line(px, py - 2, px, py + 2, '+');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = max_n(t, hands.last().copied());
        let primes = (1..=n).filter(|&k| is_prime(k)).count();
        Some(format!("ZOOM N={n}  primes={primes}"))
    }

    fn reveal(&self) -> &'static str {
        "Ulam's spiral writes the naturals in a square coil and marks primes. \
         Unexpected diagonal alignments appear: quadratic polynomials dense \
         with primes, visible as weather on a number lattice."
    }
}

#[cfg(test)]
mod tests {
    use super::{UlamSpiral, is_prime, ulam_xy};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = UlamSpiral::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("ZOOM"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn zoom_changes() {
        let r = UlamSpiral::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
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
    fn primes_and_origin() {
        assert_eq!(ulam_xy(1), (0, 0));
        assert!(is_prime(17));
        assert!(!is_prime(1));
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        UlamSpiral::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(UlamSpiral::new().motif().unwrap().line.len() >= 6);
    }
}

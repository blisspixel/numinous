//! Arnold cat map: linear toral automorphism, classic chaos toy.
//!
//! (x,y) -> (x+y, x+2y) mod 1. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
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

fn iters(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (1 + (x * 12.0) as usize).clamp(1, 14)
    } else {
        (2 + (phase_unit(t) * 8.0) as usize).clamp(1, 12)
    }
}

/// One step of the inverse cat map: the inverse of `(x + y, x + 2y)` mod 1 is
/// `(2x - y, y - x)` mod 1.
///
/// Iterated at most 14 times per pixel. Both this map and its inverse stretch
/// by at most `(3 + sqrt 5) / 2`, about 2.618, so rounding of about `1e-16`
/// grows to under `1e-10` by then, far below a pixel.
fn inverse_step(u: f64, v: f64) -> (f64, f64) {
    ((2.0 * u - v).rem_euclid(1.0), (v - u).rem_euclid(1.0))
}

fn draw(canvas: &mut dyn Surface, n: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Seed a few image patches: disks and a stripe (the "cat")
    let shift = if seed == 0 {
        0.0
    } else {
        (seed % 9) as f64 * 0.01
    };
    for y in 0..height {
        for x in 0..width {
            let mut u = (x as f64 + 0.5) / width as f64;
            let mut v = (y as f64 + 0.5) / height as f64;
            // inverse map n times so we sample the preimage of the pattern
            for _ in 0..n {
                (u, v) = inverse_step(u, v);
            }
            u = (u + shift).rem_euclid(1.0);
            let face = (u - 0.35).hypot(v - 0.55) < 0.12
                || (u - 0.55).hypot(v - 0.55) < 0.12
                || (u - 0.45).hypot(v - 0.35) < 0.08
                || ((u - 0.45).abs() < 0.18 && (v - 0.72).abs() < 0.04);
            if face {
                canvas.plot(x as i32, y as i32, if n >= 6 { '#' } else { '*' });
            } else if (u * 16.0).floor() as i32 % 4 == 0 {
                canvas.plot(x as i32, y as i32, '.');
            }
        }
    }
}

/// Arnold cat map room.
#[derive(Debug, Default)]
pub struct CatMap {
    seed: u64,
}

impl CatMap {
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

impl Room for CatMap {

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, iters(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "cat map",
            root: 174.61,
            tempo: 112,
            line: &[0, 5, 7, 12, 7, 5, 0, 12],
            encodes: "integer shear on the torus mixes then returns",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE ITERS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = iters(t, None);
        Some(format!("n={n}  cat  DRAG:ITERS"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = iters(t, hands.last().copied());
        draw(canvas, n, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = iters(t, hands.last().copied());
        // Determinant 1: area-preserving; Lyapunov ~ ln((3+sqrt(5))/2)
        let lyap = ((3.0_f64 + 5.0_f64.sqrt()) / 2.0).ln();
        Some(format!("ITERS n={n}  lyap~{lyap:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Arnold's cat map is a linear map of the torus: stretch, shear, fold \
         back mod 1. Area is preserved and nearby points diverge \
         exponentially, yet on any finite pixel grid the map is a bijection, \
         so it is exactly periodic: scramble the cat long enough and the \
         unharmed cat must walk back, whole, as if nothing happened. Chaos \
         and perfect recurrence in the same innocent-looking shear."
    }
}

#[cfg(test)]
mod tests {
    use super::{CatMap, inverse_step, iters};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn the_deepest_preimage_stays_on_the_exact_rational_orbit() {
        // Pixel centers are rationals, and the inverse map sends a/D to
        // integers mod D exactly, so integer arithmetic is an oracle with no
        // rounding at all. The deepest the room ever iterates is the hand's
        // maximum.
        let deepest = iters(0.0, Some((1.0, 0.5)));
        assert!(deepest <= 14);
        let (width, height) = (40i64, 28i64);
        let d = 2 * width * height;
        for y in 0..height {
            for x in 0..width {
                let (mut a, mut b) = ((2 * x + 1) * height, (2 * y + 1) * width);
                let mut u = (x as f64 + 0.5) / width as f64;
                let mut v = (y as f64 + 0.5) / height as f64;
                for _ in 0..deepest {
                    (a, b) = ((2 * a - b).rem_euclid(d), (b - a).rem_euclid(d));
                    (u, v) = inverse_step(u, v);
                }
                let circle = |f: f64, n: i64| {
                    let gap = (f - n as f64 / d as f64).abs();
                    gap.min(1.0 - gap)
                };
                assert!(circle(u, a) < 1e-9 && circle(v, b) < 1e-9, "pixel ({x}, {y})");
            }
        }
    }

    #[test]
    fn status_invites() {
        let s = CatMap::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("ITERS"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn iters_change() {
        let r = CatMap::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.95,
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
        CatMap::new().render(&mut c, 0.35);
        assert!(c.ink_count() > 0);
    }
}

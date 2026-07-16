//! Quadratic residues: Legendre symbol matrix mod p.
//!
//! DRAG: TUNE P. See `docs/ROOMS.md`.

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

fn is_prime_u(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n % 2 == 0 {
        return n == 2;
    }
    let mut d = 3u64;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d += 2;
    }
    true
}

fn next_odd_prime(mut n: u64) -> u64 {
    if n < 3 {
        return 3;
    }
    if n % 2 == 0 {
        n += 1;
    }
    while !is_prime_u(n) {
        n += 2;
    }
    n
}

fn modulus(t: f64, hand: Option<(f64, f64)>, seed: u64) -> u64 {
    let s = if seed == 0 { 0 } else { (seed % 5) * 2 };
    let base = if let Some((x, _)) = hand {
        5.0 + x * 40.0
    } else {
        7.0 + phase_unit(t) * 35.0
    };
    next_odd_prime((base as u64 + s).clamp(5, 53))
}

/// Legendre(a/p) via Euler criterion: a^{(p-1)/2} mod p in {0,1,p-1}.
fn legendre(a: i64, p: u64) -> i8 {
    let p = p as i64;
    let a = a.rem_euclid(p);
    if a == 0 {
        return 0;
    }
    let exp = (p - 1) / 2;
    let mut base = a;
    let mut e = exp;
    let mut r: i64 = 1;
    while e > 0 {
        if e & 1 == 1 {
            r = (r * base).rem_euclid(p);
        }
        base = (base * base).rem_euclid(p);
        e >>= 1;
    }
    if r == 0 {
        0
    } else if r == 1 {
        1
    } else {
        -1
    }
}

fn draw(canvas: &mut dyn Surface, p: u64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || p < 3 {
        return;
    }
    let n = (p as usize).min(width).min(height);
    let cell_w = (width / n).max(1);
    let cell_h = (height / n).max(1);
    let off = if seed == 0 {
        0usize
    } else {
        (seed as usize) % 3
    };
    for i in 0..n {
        for j in 0..n {
            // row i = residue a, col j = shifted probe
            let a = ((i + off) as i64) % (p as i64);
            let b = ((j + off / 2) as i64) % (p as i64);
            let sym = legendre(a * b + 1, p);
            let ch = match sym {
                1 => '#',
                -1 => '.',
                _ => ' ',
            };
            let x0 = (j * cell_w) as i32;
            let y0 = (i * cell_h) as i32;
            if ch != ' ' {
                canvas.line(x0, y0, x0 + cell_w as i32 - 1, y0, ch);
            }
        }
    }
}

/// Quadratic residues room.
#[derive(Debug, Default)]
pub struct QuadraticResidues {
    seed: u64,
}

impl QuadraticResidues {
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

impl Room for QuadraticResidues {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "quadratic-residues",
            title: "Quadratic Residues",
            wing: "Number & Pattern",
            blurb: "Legendre symbol checkerboard mod p. t and DRAG: TUNE P.",
            accent: [120, 80, 60],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, modulus(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "quadratic-residues",
            root: 246.94,
            tempo: 70,
            line: &[0, 5, 7, 12, 7, 5, 0, 5],
            encodes: "Legendre matrix: quadratic residue patterns mod prime p",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE P")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = modulus(t, None, self.seed);
        Some(format!("p={p}  legendre  DRAG:P"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let p = modulus(t, hands.last().copied(), self.seed);
        draw(canvas, p, self.seed ^ hands.len() as u64);
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
        let p = modulus(t, hands.last().copied(), self.seed);
        let mut res = 0u32;
        for a in 1..p {
            if legendre(a as i64, p) == 1 {
                res += 1;
            }
        }
        // Exactly (p-1)/2 nonzero quadratic residues mod p.
        let half = (p - 1) / 2;
        Some(format!("p={p}  residues={res}  (p-1)/2={half}"))
    }

    fn reveal(&self) -> &'static str {
        "An integer a is a quadratic residue mod prime p if some x satisfies \
         x^2 = a mod p. The Legendre symbol (a/p) is +1 for residues, -1 for \
         non-residues, and 0 when p divides a. Half the nonzero residues are \
         squares; the matrix here is that dichotomy as texture."
    }
}

#[cfg(test)]
mod tests {
    use super::QuadraticResidues;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = QuadraticResidues::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("legendre"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn p_changes() {
        let r = QuadraticResidues::new();
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
        QuadraticResidues::new().render(&mut c, 0.45);
        assert!(c.ink_count() > 0);
    }
}

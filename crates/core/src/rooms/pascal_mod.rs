//! Pascal mod n: the divisor fractal (Sierpinski when n=2).
//!
//! Color binom(row, k) by residue mod m. For m=2 the pattern is exact
//! Sierpinski; Kummer's theorem ties the fractal to carry counting. DRAG:
//! TURN THE MODULUS. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const MAX_ROWS: usize = 48;

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

fn modulus(t: f64, hand: Option<(f64, f64)>) -> u32 {
    if let Some((x, _)) = hand {
        (2 + (x * 10.0) as u32).clamp(2, 12)
    } else {
        (2 + (phase_unit(t) * 6.0) as u32).clamp(2, 8)
    }
}

/// Row residues mod m for rows 0..n-1 (inclusive width row+1).
fn triangle(rows: usize, m: u32) -> Vec<Vec<u32>> {
    let rows = rows.clamp(2, MAX_ROWS);
    let m = m.max(2);
    let mut tri = Vec::with_capacity(rows);
    tri.push(vec![1u32]);
    for r in 1..rows {
        let prev = &tri[r - 1];
        let mut row = Vec::with_capacity(r + 1);
        for k in 0..=r {
            let left = if k == 0 { 0 } else { prev[k - 1] };
            let right = if k == r { 0 } else { prev[k] };
            row.push((left + right) % m);
        }
        tri.push(row);
    }
    tri
}

fn ink(res: u32, m: u32) -> char {
    if res == 0 {
        ' '
    } else if res == 1 {
        '*'
    } else if res * 2 < m {
        '+'
    } else {
        '#'
    }
}

fn draw(canvas: &mut dyn Surface, tri: &[Vec<u32>], m: u32) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || tri.is_empty() {
        return;
    }
    let rows = tri.len();
    for (r, row) in tri.iter().enumerate() {
        let y = ((r as f64 / rows as f64) * height.saturating_sub(1) as f64).round() as i32;
        let row_w = row.len();
        for (k, &v) in row.iter().enumerate() {
            let ch = ink(v, m);
            if ch == ' ' {
                continue;
            }
            // Center each row.
            let frac = (k as f64 + 0.5) / row_w as f64;
            let x = ((0.08 + 0.84 * frac) * width.saturating_sub(1) as f64).round() as i32;
            canvas.plot(x, y, ch);
        }
    }
}

/// Pascal mod n room.
#[derive(Debug, Default)]
pub struct PascalMod {
    seed: u64,
}

impl PascalMod {
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

impl Room for PascalMod {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "pascal-mod",
            title: "The Divisor Fractal",
            wing: "Number & Pattern",
            blurb: "Pascal's triangle mod m: residue paints a fractal. mod 2 is Sierpinski; \
                    Kummer ties carries to the pattern. t grows rows; DRAG: TURN THE MODULUS.",
            accent: [160, 100, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let m = modulus(t, None);
        let extra = if self.seed == 0 {
            0
        } else {
            (self.seed % 5) as usize
        };
        let rows = 16 + (phase_unit(t) * 24.0) as usize + extra;
        let tri = triangle(rows, m);
        draw(canvas, &tri, m);
    }

    fn postcard_t(&self) -> f64 {
        0.2
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "pascal sieve",
            root: 164.81,
            tempo: 96,
            line: &[0, 3, 7, 12, 7, 3, 0, 12],
            encodes: "binomial residues carving Sierpinski from addition",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TURN THE MODULUS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let m = modulus(t, None);
        let rows = 16 + (phase_unit(t) * 24.0) as usize;
        let label = if m == 2 { "SIERP" } else { "MOD" };
        Some(format!("m={m}  rows={rows}  {label}  DRAG:MOD"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let m = modulus(t, hands.last().copied());
        let rows = 18 + (phase_unit(t) * 20.0) as usize;
        let tri = triangle(rows, m);
        draw(canvas, &tri, m);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let m = modulus(t, hands.last().copied());
        let rows = 18 + (phase_unit(t) * 20.0) as usize;
        let label = if m == 2 { "SIERPINSKI" } else { "FRACTAL" };
        Some(format!("MOD m={m}  rows={rows}  {label}"))
    }

    fn reveal(&self) -> &'static str {
        "Binomial coefficients mod m paint self-similar triangles. For m=2 the \
         picture is Sierpinski's gasket; Kummer's theorem says the fractal counts \
         carries when adding in base m. Arithmetic is the geometry."
    }
}

#[cfg(test)]
mod tests {
    use super::{PascalMod, triangle};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = PascalMod::new().status(0.1).unwrap();
        assert!(s.contains("DRAG") || s.contains("MOD"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn mod_changes() {
        let r = PascalMod::new();
        let o = r.status(0.1).unwrap();
        let a = r
            .status_input(
                0.1,
                &[RoomInput::PointerDown {
                    x: 0.85,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn mod2_has_zeros() {
        let t = triangle(8, 2);
        assert_eq!(t[0], vec![1]);
        assert_eq!(t[1], vec![1, 1]);
        assert_eq!(t[2], vec![1, 0, 1]);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        PascalMod::new().render(&mut c, 0.15);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(PascalMod::new().motif().unwrap().line.len() >= 6);
    }
}

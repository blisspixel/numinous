//! Pell equation path: continued fraction convergents of sqrt(d).
//!
//! DRAG: TUNE D. See `docs/ROOMS.md`.

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

fn is_square(n: u64) -> bool {
    let r = (n as f64).sqrt().round() as u64;
    r * r == n
}

fn disc(t: f64, hand: Option<(f64, f64)>, seed: u64) -> u64 {
    let s = if seed == 0 { 0 } else { seed % 7 };
    let base = if let Some((x, _)) = hand {
        2.0 + x * 60.0
    } else {
        2.0 + phase_unit(t) * 55.0
    };
    let mut d = (base as u64 + s).clamp(2, 80);
    while is_square(d) {
        d += 1;
        if d > 90 {
            d = 2;
            break;
        }
    }
    d
}

/// Continued fraction partial quotients for sqrt(d).
fn cf_sqrt(d: u64, max_terms: usize) -> Vec<u64> {
    let a0 = (d as f64).sqrt().floor() as u64;
    let mut out = vec![a0];
    let mut m = 0u64;
    let mut den = 1u64;
    let mut a = a0;
    for _ in 0..max_terms.saturating_sub(1) {
        m = den * a - m;
        if m == 0 {
            break;
        }
        den = (d - m * m) / den;
        if den == 0 {
            break;
        }
        a = (a0 + m) / den;
        out.push(a);
        // period often returns when a == 2*a0
        if a == 2 * a0 && out.len() > 1 {
            break;
        }
    }
    out
}

/// Convergents (p_k, q_k) from partial quotients.
fn convergents(a: &[u64]) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    let mut p0 = 0u64;
    let mut p1 = 1u64;
    let mut q0 = 1u64;
    let mut q1 = 0u64;
    for &ai in a {
        let p = ai.saturating_mul(p1).saturating_add(p0);
        let q = ai.saturating_mul(q1).saturating_add(q0);
        out.push((p, q));
        p0 = p1;
        p1 = p;
        q0 = q1;
        q1 = q;
    }
    out
}

fn draw(canvas: &mut dyn Surface, d: u64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let a = cf_sqrt(d, 24);
    let conv = convergents(&a);
    if conv.is_empty() {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let scale = (width.min(height) as f64) * 0.08;
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.15
    };
    // hyperbola sketch x^2 - d y^2 ~ 1 as guide
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..80 {
        let y = -1.5 + 3.0 * (i as f64 / 79.0);
        let inside = 1.0 + d as f64 * y * y;
        if inside < 0.0 {
            prev = None;
            continue;
        }
        let x = inside.sqrt();
        let px = (cx + (x * scale * rot.cos() - y * scale * 3.0 * rot.sin())).round() as i32;
        let py = (cy - (x * scale * rot.sin() + y * scale * 3.0 * rot.cos())).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // plot convergents as points (p/q scaled)
    let mut last: Option<(i32, i32)> = None;
    for (i, &(p, q)) in conv.iter().enumerate() {
        if q == 0 {
            continue;
        }
        let x = p as f64 / q as f64;
        // map ratio near sqrt(d) to horizontal, term index vertical-ish
        let u = (x - (d as f64).sqrt()) * 8.0;
        let v = (i as f64 - conv.len() as f64 * 0.5) * 0.6;
        let px = (cx + (u * scale * 2.0 + v * 2.0)).round() as i32;
        let py = (cy - (v * scale * 2.0 - u)).round() as i32;
        if let Some((ox, oy)) = last {
            canvas.line(ox, oy, px, py, '#');
        }
        canvas.line(px - 1, py, px + 1, py, '#');
        last = Some((px, py));
        // mark Pell solutions where p^2 - d q^2 = +/-1
        let check = p.saturating_mul(p) as i128 - (d as i128) * (q as i128) * (q as i128);
        if check == 1 || check == -1 {
            canvas.line(px, py - 2, px, py + 2, 'o');
        }
    }
}

/// Pell path room.
#[derive(Debug, Default)]
pub struct PellPath {
    seed: u64,
}

impl PellPath {
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

impl Room for PellPath {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "pell-path",
            title: "Pell Path",
            wing: "Number & Pattern",
            blurb: "Convergents of sqrt(d) chase the Pell hyperbola. t and DRAG: TUNE D.",
            accent: [60, 120, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, disc(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "pell-path",
            root: 349.23,
            tempo: 78,
            line: &[0, 7, 5, 12, 9, 5, 0, 7],
            encodes: "Pell x^2-d y^2=1: CF convergents of sqrt(d) hit solutions",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE D")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = disc(t, None, self.seed);
        let n = cf_sqrt(d, 24).len();
        Some(format!("d={d}  cf={n}  DRAG:D"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let d = disc(t, hands.last().copied(), self.seed);
        draw(canvas, d, self.seed ^ hands.len() as u64);
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
        let d = disc(t, hands.last().copied(), self.seed);
        Some(format!("D={d}  pell"))
    }

    fn reveal(&self) -> &'static str {
        "The Pell equation x^2 - d y^2 = 1 (d not square) has infinitely many \
         solutions generated from a fundamental one. Continued-fraction convergents \
         of sqrt(d) find that seed: when a convergent (p/q) satisfies the equation, \
         you have climbed onto the hyperbola."
    }
}

#[cfg(test)]
mod tests {
    use super::PellPath;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = PellPath::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("cf"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn d_changes() {
        let r = PellPath::new();
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
        PellPath::new().render(&mut c, 0.35);
        assert!(c.ink_count() > 0);
    }
}

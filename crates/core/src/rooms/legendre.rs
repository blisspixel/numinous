//! Legendre polynomials: orthogonal basis on [-1,1], multipole shapes.
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

fn level(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 { 0.0 } else { (seed % 3) as f64 };
    if let Some((x, _)) = hand {
        x * 8.0 + s
    } else {
        phase_unit(t) * 7.0 + s
    }
}

fn legendre(n: usize, x: f64) -> f64 {
    // Bonnet: (n+1) P_{n+1} = (2n+1) x P_n - n P_{n-1}
    if n == 0 {
        return 1.0;
    }
    if n == 1 {
        return x;
    }
    let mut p_nm2 = 1.0;
    let mut p_nm1 = x;
    for k in 1..n {
        let p = ((2.0 * k as f64 + 1.0) * x * p_nm1 - k as f64 * p_nm2) / (k as f64 + 1.0);
        p_nm2 = p_nm1;
        p_nm1 = p;
    }
    p_nm1
}

fn draw(canvas: &mut dyn Surface, n_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = n_f.round().clamp(0.0, 10.0) as usize;
    let cy = height as f64 * 0.5;
    let y_scale = height as f64
        * 0.4
        * (1.0
            + if seed == 0 {
                0.0
            } else {
                (seed % 3) as f64 * 0.03
            });
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let x = -1.0 + 2.0 * (col as f64) / width.saturating_sub(1).max(1) as f64;
        let y = legendre(n, x).clamp(-1.2, 1.2);
        let py = (cy - y * y_scale).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
        }
        prev = Some((col as i32, py));
    }
    canvas.line(0, cy as i32, width.saturating_sub(1) as i32, cy as i32, '.');
    // Endpoints P( +/-1) = (+/-1)^n
    let pe = if n.is_multiple_of(2) { 1.0 } else { -1.0 };
    let py0 = (cy - 1.0 * y_scale).round() as i32;
    let py1 = (cy - pe * y_scale).round() as i32;
    canvas.line(0, py0 - 1, 0, py0 + 1, 'o');
    canvas.line(
        width.saturating_sub(1) as i32,
        py1 - 1,
        width.saturating_sub(1) as i32,
        py1 + 1,
        'o',
    );
}

/// Legendre room.
#[derive(Debug, Default)]
pub struct Legendre {
    seed: u64,
}

impl Legendre {
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

impl Room for Legendre {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "legendre",
            title: "Legendre P_n",
            wing: "Number & Pattern",
            blurb: "Orthogonal polynomials on [-1,1]. t and DRAG: TUNE N.",
            accent: [40, 120, 60],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, level(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "legendre",
            root: 130.81,
            tempo: 92,
            line: &[0, 2, 5, 9, 12, 9, 5, 2],
            encodes: "Legendre P_n: orthogonal multipoles on the unit interval",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE N")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = level(t, None, self.seed).round();
        Some(format!("n={n:.0}  P_n  DRAG:N"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = level(t, hands.last().copied(), self.seed);
        draw(canvas, n, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = level(t, hands.last().copied(), self.seed)
            .round()
            .clamp(0.0, 12.0) as usize;
        // P_n has n zeros on (-1,1).
        Some(format!("n={n}  zeros={n}  P_n"))
    }

    fn reveal(&self) -> &'static str {
        "Legendre polynomials P_n are orthogonal on [-1,1] with weight 1. They \
         expand multipole fields, Laplace solutions in spherical coordinates, and \
         any square-integrable function on the interval."
    }
}

#[cfg(test)]
mod tests {
    use super::Legendre;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Legendre::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("P_n"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = Legendre::new();
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
        Legendre::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

//! Hermite polynomials: quantum harmonic oscillator wave envelopes.
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

fn hermite(n: usize, x: f64) -> f64 {
    // Physicists' Hermite via recurrence: H0=1, H1=2x, H_{n+1}=2x H_n - 2n H_{n-1}
    if n == 0 {
        return 1.0;
    }
    if n == 1 {
        return 2.0 * x;
    }
    let mut h_nm2 = 1.0;
    let mut h_nm1 = 2.0 * x;
    for k in 1..n {
        let h = 2.0 * x * h_nm1 - 2.0 * k as f64 * h_nm2;
        h_nm2 = h_nm1;
        h_nm1 = h;
    }
    h_nm1
}

fn psi(n: usize, x: f64) -> f64 {
    // unnormalized: H_n(x) e^{-x^2/2}
    hermite(n, x) * (-0.5 * x * x).exp()
}

fn draw(canvas: &mut dyn Surface, n_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = n_f.round().clamp(0.0, 10.0) as usize;
    let cy = height as f64 * 0.5;
    let y_scale = height as f64 * 0.35
        / (1.0
            + if seed == 0 {
                0.0
            } else {
                (seed % 3) as f64 * 0.05
            });
    // Envelope |psi| and signed wave.
    let mut prev: Option<(i32, i32)> = None;
    let mut max_abs: f64 = 1e-9;
    let mut samples = Vec::with_capacity(width);
    for col in 0..width {
        let x = -4.0 + 8.0 * (col as f64) / width.saturating_sub(1).max(1) as f64;
        let y = psi(n, x);
        max_abs = max_abs.max(y.abs());
        samples.push(y);
    }
    for (col, y) in samples.iter().enumerate() {
        let yn = y / max_abs;
        let py = (cy - yn * y_scale).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
        }
        prev = Some((col as i32, py));
    }
    // Zero line.
    canvas.line(0, cy as i32, width.saturating_sub(1) as i32, cy as i32, '.');
}

/// Hermite room.
#[derive(Debug, Default)]
pub struct Hermite {
    seed: u64,
}

impl Hermite {
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

impl Room for Hermite {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "hermite",
            title: "Hermite Wave",
            wing: "Waves & Sound",
            blurb: "Harmonic oscillator Hermite modes. t and DRAG: TUNE N.",
            accent: [90, 40, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, level(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "hermite",
            root: 138.59,
            tempo: 90,
            line: &[0, 3, 7, 12, 10, 7, 3, 0],
            encodes: "Hermite H_n times Gaussian: quantum oscillator levels",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE N")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = level(t, None, self.seed).round();
        Some(format!("n={n:.0}  HO  DRAG:N"))
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
            .clamp(0.0, 10.0) as usize;
        // Quantum HO: E_n = n + 1/2 (hbar omega units).
        let e = n as f64 + 0.5;
        let nodes = n;
        Some(format!("n={n}  E={e:.1}  nodes={nodes}"))
    }

    fn reveal(&self) -> &'static str {
        "The quantum harmonic oscillator eigenfunctions are Hermite polynomials \
         times a Gaussian. Energy steps are equal; each higher n adds a node and \
         spreads probability farther from the well center."
    }
}

#[cfg(test)]
mod tests {
    use super::Hermite;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Hermite::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("HO"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = Hermite::new();
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
        Hermite::new().render(&mut c, 0.45);
        assert!(c.ink_count() > 0);
    }
}

//! Dirichlet eta: alternating zeta, converges where zeta needs analytic continuation.
//!
//! DRAG: TUNE S. See `docs/ROOMS.md`.

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

fn s_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.08
    };
    if let Some((x, _)) = hand {
        0.2 + x * 2.5 + s
    } else {
        0.4 + phase_unit(t) * 2.0 + s
    }
    .clamp(0.15, 3.0)
}

fn eta_partial(s: f64, terms: usize) -> f64 {
    let mut sum = 0.0;
    for n in 1..=terms {
        let term = (n as f64).powf(-s);
        if n % 2 == 0 {
            sum -= term;
        } else {
            sum += term;
        }
    }
    sum
}

fn draw(canvas: &mut dyn Surface, s0: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // partial sum staircase for fixed s
    let terms = 40
        + if seed == 0 {
            0
        } else {
            (seed % 5) as usize * 4
        };
    let mut sum = 0.0;
    let mut prev: Option<(i32, i32)> = None;
    for n in 1..=terms {
        let term = (n as f64).powf(-s0);
        if n % 2 == 0 {
            sum -= term;
        } else {
            sum += term;
        }
        let px = ((n as f64 / terms as f64) * width.saturating_sub(1) as f64).round() as i32;
        let py = ((0.5 - sum * 0.35) * height.saturating_sub(1) as f64).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
    // eta(s) curve across s
    prev = None;
    for col in 0..width {
        let s = 0.2 + 2.8 * (col as f64) / width.saturating_sub(1).max(1) as f64;
        let e = eta_partial(s, 48);
        let py = ((0.75 - e * 0.25) * height.saturating_sub(1) as f64).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '.');
        }
        prev = Some((col as i32, py));
    }
    let mx = (((s0 - 0.2) / 2.8) * width.saturating_sub(1) as f64)
        .round()
        .clamp(0.0, width.saturating_sub(1) as f64) as i32;
    canvas.line(mx, 0, mx, height as i32 - 1, '|');
}

/// Dirichlet eta room.
#[derive(Debug, Default)]
pub struct DirichletEta {
    seed: u64,
}

impl DirichletEta {
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

impl Room for DirichletEta {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "dirichlet-eta",
            title: "Dirichlet Eta",
            wing: "Analysis",
            blurb: "Alternating zeta: eta(s)=sum (-1)^{n-1}/n^s. t and DRAG: TUNE S.",
            accent: [70, 70, 110],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, s_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "dirichlet-eta",
            root: 233.08,
            tempo: 68,
            line: &[0, 5, 8, 12, 8, 5, 0, 7],
            encodes: "eta(s)=(1-2^{1-s}) zeta(s): alternating series for zeta",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE S")
    }

    fn status(&self, t: f64) -> Option<String> {
        let s = s_param(t, None, self.seed);
        let e = eta_partial(s, 40);
        Some(format!("s={s:.2}  eta={e:.2}  DRAG:S"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let s = s_param(t, hands.last().copied(), self.seed);
        draw(canvas, s, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let s = s_param(t, hands.last().copied(), self.seed);
        let e40 = eta_partial(s, 40);
        let e80 = eta_partial(s, 80);
        let drift = (e80 - e40).abs();
        Some(format!("eta={e80:.3}  drift40={drift:.1e}"))
    }

    fn reveal(&self) -> &'static str {
        "The Dirichlet eta function is the alternating zeta series. It converges \
         for Re(s)>0 and relates to zeta by eta(s)=(1-2^{1-s}) zeta(s). That \
         identity is one door into analytic continuation of the Riemann zeta function."
    }
}

#[cfg(test)]
mod tests {
    use super::DirichletEta;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = DirichletEta::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("eta"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn s_changes() {
        let r = DirichletEta::new();
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
        DirichletEta::new().render(&mut c, 0.45);
        assert!(c.ink_count() > 0);
    }
}

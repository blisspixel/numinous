//! Logistic bifurcation diagram: r sweeps, long-term x densifies.
//!
//! Distinct from logistic-map cobweb/orbit rooms: this is the full r-x weather.
//! DRAG: SET R WINDOW. See `docs/ROOMS.md`.

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

fn r_window(t: f64, hand: Option<(f64, f64)>) -> (f64, f64) {
    // Window of r values in [2.5, 4.0]
    if let Some((x, y)) = hand {
        let mid = 2.5 + x * 1.5;
        let half = 0.05 + y * 0.4;
        ((mid - half).max(2.5), (mid + half).min(4.0))
    } else {
        let u = phase_unit(t);
        let lo = 2.8 + u * 0.6;
        (lo, (lo + 0.8).min(4.0))
    }
}

fn draw(canvas: &mut dyn Surface, r0: f64, r1: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let r0 = r0.min(r1 - 1e-6);
    let r1 = r1.max(r0 + 1e-6);
    let burn = 80usize;
    let keep = 40usize;
    let start_x = if seed == 0 {
        0.5
    } else {
        0.2 + (seed % 60) as f64 * 0.01
    };
    for col in 0..width {
        let r = r0 + (r1 - r0) * (col as f64 / width.saturating_sub(1).max(1) as f64);
        let mut x = start_x;
        for _ in 0..burn {
            x = r * x * (1.0 - x);
        }
        for k in 0..keep {
            x = r * x * (1.0 - x);
            if !x.is_finite() {
                break;
            }
            let py = ((1.0 - x.clamp(0.0, 1.0)) * height.saturating_sub(1) as f64).round() as i32;
            let ch = if k + 5 > keep { '#' } else { '*' };
            canvas.plot(col as i32, py, ch);
        }
    }
}

/// Logistic bifurcation diagram room.
#[derive(Debug, Default)]
pub struct Bifurcation {
    seed: u64,
}

impl Bifurcation {
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

impl Room for Bifurcation {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "bifurcation",
            title: "Bifurcation Weather",
            wing: "Motion & Dynamics",
            blurb: "Logistic map long-term x as r sweeps: period doubling into chaos. t and DRAG: \
                    SET R WINDOW.",
            accent: [200, 40, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (r0, r1) = r_window(t, None);
        draw(canvas, r0, r1, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.7
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "bifurcation",
            root: 233.08,
            tempo: 106,
            line: &[0, 0, 5, 5, 7, 7, 12, 12],
            encodes: "period doubling cascade into dense chaos bands",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET R WINDOW")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (r0, r1) = r_window(t, None);
        Some(format!("r=[{r0:.2},{r1:.2}]  DRAG:WINDOW"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (r0, r1) = r_window(t, hands.last().copied());
        draw(canvas, r0, r1, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (r0, r1) = r_window(t, hands.last().copied());
        let mid = 0.5 * (r0 + r1);
        let span = (r1 - r0).max(0.0);
        let band = if mid < 3.0 {
            "fixed"
        } else if mid < 3.45 {
            "period"
        } else if mid < 3.57 {
            "double"
        } else {
            "chaos"
        };
        Some(format!("mid={mid:.2}  span={span:.2}  {band}"))
    }

    fn reveal(&self) -> &'static str {
        "Feigenbaum's cascade: as r rises, the logistic map doubles its period \
         repeatedly, then bursts into chaos with windows of order. The diagram \
         is a map of long-term weather, not a single orbit."
    }
}

#[cfg(test)]
mod tests {
    use super::Bifurcation;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Bifurcation::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("WINDOW"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn window_changes() {
        let r = Bifurcation::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.2,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Bifurcation::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Bifurcation::new().motif().unwrap().line.len() >= 6);
    }
}

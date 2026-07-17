//! Brusselator: oscillatory reaction-diffusion toy (space-time stripes).
//!
//! DRAG: TUNE A. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const CELLS: usize = 64;
const STEPS: usize = 120;
const DT: f64 = 0.02;

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

fn a_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.03
    };
    if let Some((x, _)) = hand {
        0.5 + x * 2.0 + s
    } else {
        0.8 + phase_unit(t) * 1.2 + s
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let b = 1.8
        + if seed == 0 {
            0.0
        } else {
            (seed % 4) as f64 * 0.05
        };
    let mut u = vec![a; CELLS];
    let mut v = vec![b / a.max(0.1); CELLS];
    // Seed a bump
    let mid = CELLS / 2;
    u[mid] += 0.3;
    v[mid] -= 0.1;
    let du = 0.025;
    let dv = 0.015;
    let mut history = Vec::with_capacity(STEPS);
    for _ in 0..STEPS {
        let mut nu = u.clone();
        let mut nv = v.clone();
        for i in 0..CELLS {
            let im = if i == 0 { CELLS - 1 } else { i - 1 };
            let ip = if i + 1 == CELLS { 0 } else { i + 1 };
            let lap_u = u[im] + u[ip] - 2.0 * u[i];
            let lap_v = v[im] + v[ip] - 2.0 * v[i];
            let uu = u[i];
            let vv = v[i];
            // Brusselator kinetics
            nu[i] = uu + DT * (a - (b + 1.0) * uu + uu * uu * vv + du * lap_u);
            nv[i] = vv + DT * (b * uu - uu * uu * vv + dv * lap_v);
            if !nu[i].is_finite() {
                nu[i] = a;
            }
            if !nv[i].is_finite() {
                nv[i] = b / a.max(0.1);
            }
        }
        u = nu;
        v = nv;
        history.push(u.clone());
    }
    // Space-time plot: x = cell, y = time (recent at bottom)
    for row in 0..height {
        let hi = (row * STEPS / height.max(1)).min(STEPS - 1);
        let row_u = &history[hi];
        for col in 0..width {
            let ci = (col * CELLS / width.max(1)).min(CELLS - 1);
            let val = row_u[ci];
            let ch = if val > a + 0.4 {
                '#'
            } else if val > a + 0.15 {
                '*'
            } else if val > a - 0.1 {
                '+'
            } else {
                '.'
            };
            canvas.plot(col as i32, row as i32, ch);
        }
    }
}

/// Brusselator room.
#[derive(Debug, Default)]
pub struct Brusselator {
    seed: u64,
}

impl Brusselator {
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

impl Room for Brusselator {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "brusselator",
            title: "Brusselator",
            wing: "Motion & Dynamics",
            blurb: "Chemical oscillator waves in space-time. t and DRAG: TUNE A.",
            accent: [160, 80, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, a_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "brusselator",
            root: 329.63,
            tempo: 108,
            line: &[0, 3, 6, 9, 12, 9, 6, 3],
            encodes: "autonomous chemical clock with spatial waves",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = a_param(t, None, self.seed);
        Some(format!("A={a:.2}  brus  DRAG:A"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = a_param(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
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
        let a = a_param(t, hands.last().copied(), self.seed);
        // Well-mixed Hopf: b > 1 + a^2. Room holds b near classic 3.
        let hopf = 1.0 + a * a;
        let b = 3.0;
        let margin = b - hopf;
        let phase = if margin > 0.2 {
            "osc"
        } else if margin > 0.0 {
            "hopf+"
        } else {
            "steady"
        };
        Some(format!("a={a:.2}  b-H={margin:.2}  {phase}"))
    }

    fn reveal(&self) -> &'static str {
        "The Brusselator is a two-species chemical oscillator from the Brussels \
         school. With diffusion it forms traveling waves and Turing patterns: \
         a minimal model of rhythmic chemistry."
    }
}

#[cfg(test)]
mod tests {
    use super::Brusselator;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Brusselator::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("A"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn a_changes() {
        let r = Brusselator::new();
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
        Brusselator::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

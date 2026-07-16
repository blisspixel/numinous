//! Lotka-Volterra predator-prey: phase spiral of rabbits and foxes.
//!
//! DRAG: TUNE PREY RATE. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const STEPS: usize = 2_400;
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

fn prey_rate(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.4 + x * 1.6 + s
    } else {
        0.6 + phase_unit(t) * 1.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, alpha: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let beta = 0.5;
    let delta = 0.4;
    let gamma = 0.3
        + if seed == 0 {
            0.0
        } else {
            (seed % 4) as f64 * 0.02
        };
    // Multiple orbits in prey-predator plane
    for start_i in 0..6 {
        let mut x = 0.5 + start_i as f64 * 0.35;
        let mut y = 0.4 + (start_i as f64 * 0.15);
        let mut min_x = f64::MAX;
        let mut max_x = f64::MIN;
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;
        let mut pts = Vec::with_capacity(STEPS);
        for _ in 0..STEPS {
            let dx = alpha * x - beta * x * y;
            let dy = delta * x * y - gamma * y;
            x = (x + dx * DT).max(1e-6);
            y = (y + dy * DT).max(1e-6);
            if !x.is_finite() || !y.is_finite() {
                break;
            }
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
            pts.push((x, y));
        }
        let dx = (max_x - min_x).max(1e-6);
        let dy = (max_y - min_y).max(1e-6);
        // Global frame using first orbit bounds expanded
        let gx0 = 0.0;
        let gx1 = (max_x * 1.1).max(2.0);
        let gy0 = 0.0;
        let gy1 = (max_y * 1.1).max(2.0);
        let _ = (dx, dy);
        for (i, &(px, py)) in pts.iter().enumerate() {
            let u = ((px - gx0) / (gx1 - gx0)).clamp(0.0, 1.0);
            let v = ((py - gy0) / (gy1 - gy0)).clamp(0.0, 1.0);
            let ix = (u * width.saturating_sub(1) as f64).round() as i32;
            let iy = ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32;
            canvas.plot(ix, iy, if i % 17 == 0 { '#' } else { '*' });
        }
    }
}

/// Lotka-Volterra room.
#[derive(Debug, Default)]
pub struct LotkaVolterra {
    seed: u64,
}

impl LotkaVolterra {
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

impl Room for LotkaVolterra {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "lotka-volterra",
            title: "Lotka-Volterra",
            wing: "Motion & Dynamics",
            blurb: "Predator and prey chase each other in closed orbits. t and DRAG: TUNE PREY \
                    RATE.",
            accent: [80, 160, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, prey_rate(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "lotka volterra",
            root: 220.0,
            tempo: 96,
            line: &[0, 5, 9, 12, 9, 5, 0, 7],
            encodes: "prey boom then predator boom in a closed loop",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE PREY RATE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = prey_rate(t, None, self.seed);
        Some(format!("a={a:.2}  LV  DRAG:PREY"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = prey_rate(t, hands.last().copied(), self.seed);
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
        let a = prey_rate(t, hands.last().copied(), self.seed);
        // Equilibrium (gamma/delta, alpha/beta) with beta=0.5 delta=0.4
        let x_eq = 0.3 / 0.4;
        let y_eq = a / 0.5;
        Some(format!("a={a:.3}  eq~({x_eq:.2},{y_eq:.2})"))
    }

    fn reveal(&self) -> &'static str {
        "Lotka-Volterra is the classical predator-prey ODE: prey grow, predators \
         eat, predators die. Orbits close around a neutral equilibrium; real \
         ecology later added density limits and chaos."
    }
}

#[cfg(test)]
mod tests {
    use super::LotkaVolterra;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = LotkaVolterra::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("PREY"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn rate_changes() {
        let r = LotkaVolterra::new();
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
        LotkaVolterra::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

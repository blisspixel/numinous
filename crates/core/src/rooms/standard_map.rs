//! Chirikov standard map: kicked rotor chaos on a torus.
//!
//! p' = p + K sin(theta); theta' = theta + p'. DRAG: TUNE K. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const ORBITS: usize = 24;
const STEPS: usize = 200;

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

fn k_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 6) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        x * 3.0 + s
    } else {
        0.5 + phase_unit(t) * 2.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, k: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let two_pi = std::f64::consts::TAU;
    let base = if seed == 0 {
        0.0
    } else {
        (seed % 20) as f64 * 0.05
    };
    for o in 0..ORBITS {
        let mut theta = (o as f64 + 0.5) / ORBITS as f64 + base * 0.1;
        let mut p = -0.5 + o as f64 / ORBITS as f64 + base * 0.05;
        for step in 0..STEPS {
            p = (p + (k / two_pi) * (two_pi * theta).sin()).rem_euclid(1.0);
            // Keep p in [-0.5,0.5] for display via shift.
            let p_disp = if p > 0.5 { p - 1.0 } else { p };
            theta = (theta + p).rem_euclid(1.0);
            if !theta.is_finite() || !p.is_finite() {
                break;
            }
            let px = (theta * width.saturating_sub(1) as f64).round() as i32;
            let py = ((0.5 - p_disp) * height.saturating_sub(1) as f64).round() as i32;
            let ch = if step + 40 > STEPS {
                '#'
            } else if o % 3 == 0 {
                '*'
            } else {
                '+'
            };
            canvas.plot(px, py, ch);
        }
    }
}

/// Chirikov standard map room.
#[derive(Debug, Default)]
pub struct StandardMap {
    seed: u64,
}

impl StandardMap {
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

impl Room for StandardMap {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "standard-map",
            title: "Chirikov Map",
            wing: "Motion & Dynamics",
            blurb: "Kicked rotor on a torus: KAM curves break into chaos. t and DRAG: TUNE K.",
            accent: [160, 40, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, k_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "chirikov",
            root: 123.47,
            tempo: 100,
            line: &[0, 3, 7, 10, 14, 10, 7, 3],
            encodes: "kick strength breaking invariant curves",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE K")
    }

    fn status(&self, t: f64) -> Option<String> {
        let k = k_param(t, None, self.seed);
        Some(format!("K={k:.2}  rotor  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let k = k_param(t, hands.last().copied(), self.seed);
        draw(canvas, k, self.seed ^ hands.len() as u64);
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
        let k = k_param(t, hands.last().copied(), self.seed);
        let two_pi = std::f64::consts::TAU;
        // Sample one orbit: fraction of steps that cross |p| band (chaos proxy).
        let mut theta = 0.3_f64;
        let mut p = 0.1_f64;
        let mut crossings = 0u32;
        let mut steps = 0u32;
        let mut prev_p = p;
        for _ in 0..400 {
            p = (p + (k / two_pi) * (two_pi * theta).sin()).rem_euclid(1.0);
            theta = (theta + p).rem_euclid(1.0);
            if !theta.is_finite() || !p.is_finite() {
                break;
            }
            let p_disp = if p > 0.5 { p - 1.0 } else { p };
            let prev_disp = if prev_p > 0.5 { prev_p - 1.0 } else { prev_p };
            if prev_disp.signum() != p_disp.signum() && prev_disp.abs() > 1e-6 {
                crossings += 1;
            }
            prev_p = p;
            steps += 1;
        }
        let rate = if steps > 0 {
            crossings as f64 / steps as f64
        } else {
            0.0
        };
        let regime = if k < 0.97 {
            "KAM"
        } else if k < 1.5 {
            "mixed"
        } else {
            "chaos"
        };
        Some(format!("K={k:.2}  flip={rate:.2}  {regime}"))
    }

    fn reveal(&self) -> &'static str {
        "Chirikov's standard map is a kicked rotor on the cylinder. For small K, \
         KAM curves survive; past a critical kick they break and chaos floods \
         the phase plane. It is the textbook of Hamiltonian chaos."
    }
}

#[cfg(test)]
mod tests {
    use super::StandardMap;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = StandardMap::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = StandardMap::new();
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
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        StandardMap::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(StandardMap::new().motif().unwrap().line.len() >= 6);
    }
}

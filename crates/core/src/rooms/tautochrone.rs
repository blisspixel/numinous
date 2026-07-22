//! Tautochrone: beads on a cycloid reach the bottom together.
//!
//! DRAG: TUNE STARTS. See `docs/ROOMS.md`.

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

/// Shared phase along the tautochrone (0 = starts, 1 = all at bottom).
fn race(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        (x + s).clamp(0.0, 1.0)
    } else {
        (phase_unit(t) + s).clamp(0.0, 1.0)
    }
}

fn draw(canvas: &mut dyn Surface, phase: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let r = (height as f64) * 0.36;
    let scale = (width as f64 * 0.94) / (std::f64::consts::PI * r).max(1.0);
    let ox = width as f64 * 0.04;
    let oy = height as f64 * 0.08;
    // Half-cycloid trough: x = r(theta - sin), y = r(1 - cos), theta 0..pi.
    let steps = 320;
    let mut prev: Option<(i32, i32)> = None;
    let mut path: Vec<(f64, f64, f64)> = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let th = std::f64::consts::PI * (i as f64 / steps as f64);
        let x = r * (th - th.sin());
        let y = r * (1.0 - th.cos());
        path.push((th, x, y));
        let px = (ox + x * scale).round() as i32;
        let py = (oy + y * scale).round() as i32;
        if let Some((qx, qy)) = prev {
            canvas.line(qx, qy, px, py, '#');
            canvas.line(qx, qy + 1, px, py + 1, '*');
        }
        prev = Some((px, py));
    }
    // Five beads start at different arc fractions; tautochrone shares arrival phase.
    let starts = [0.1, 0.28, 0.46, 0.64, 0.82];
    let n = path.len().saturating_sub(1).max(1);
    for (bi, &s0) in starts.iter().enumerate() {
        let u = s0 + (1.0 - s0) * phase;
        let idx = ((u.clamp(0.0, 1.0) * n as f64).round() as usize).min(n);
        let (_, x, y) = path[idx];
        let px = (ox + x * scale).round() as i32;
        let py = (oy + y * scale).round() as i32;
        let ch = match bi % 3 {
            0 => 'o',
            1 => '*',
            _ => '+',
        };
        for dy in -2..=2 {
            for dx in -2..=2 {
                if dx * dx + dy * dy <= 5 {
                    canvas.plot(px + dx, py + dy, ch);
                }
            }
        }
    }
    let _ = seed;
}

/// Tautochrone room.
#[derive(Debug, Default)]
pub struct Tautochrone {
    seed: u64,
}

impl Tautochrone {
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

impl Room for Tautochrone {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "tautochrone",
            title: "Tautochrone",
            wing: "Motion & Dynamics",
            blurb: "Beads on a cycloid finish together. t and DRAG: TUNE STARTS.",
            accent: [70, 110, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, race(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "tautochrone",
            root: 277.18,
            tempo: 80,
            line: &[0, 4, 7, 4, 0, 7, 12, 7],
            encodes: "cycloid isochrone: start height does not change fall time",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE STARTS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = race(t, None, self.seed);
        Some(format!("p={p:.2}  sameT  DRAG:START"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let p = race(t, hands.last().copied(), self.seed);
        draw(canvas, p, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let p = race(t, hands.last().copied(), self.seed);
        // Three beads at starts 0.15, 0.4, 0.7; each at s0+(1-s0)*p; all meet at p=1.
        let starts = [0.15_f64, 0.4, 0.7];
        let mut gaps = 0.0_f64;
        for i in 0..2 {
            let u0 = starts[i] + (1.0 - starts[i]) * p;
            let u1 = starts[i + 1] + (1.0 - starts[i + 1]) * p;
            gaps += (u1 - u0).abs();
        }
        let meet = ((1.0 - p) * 100.0).round() as i32;
        Some(format!("p={p:.2}  gap={gaps:.2}  left={meet}%"))
    }

    fn reveal(&self) -> &'static str {
        "Huygens proved the cycloid is a tautochrone: the time for a bead to slide \
         to the bottom is independent of its start height. The same curve is also \
         the brachistochrone, the fastest path between two points."
    }
}

#[cfg(test)]
mod tests {
    use super::Tautochrone;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Tautochrone::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("sameT"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn phase_changes() {
        let r = Tautochrone::new();
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
        Tautochrone::new().render(&mut c, 0.45);
        assert!(c.ink_count() > 0);
    }
}

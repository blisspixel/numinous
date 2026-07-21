//! Standing wave on a string: two counter-propagating sinusoids.
//!
//! DRAG: SET MODE. See `docs/ROOMS.md`.

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

fn mode(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (1 + (x * 7.0) as usize).clamp(1, 8)
    } else {
        (1 + (phase_unit(t) * 5.0) as usize).clamp(1, 6)
    }
}

fn draw(canvas: &mut dyn Surface, n: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cy = (height.saturating_sub(1) / 2) as f64;
    let amp = height as f64 * 0.4;
    let phase = if seed == 0 {
        0.0
    } else {
        (seed % 8) as f64 * 0.2
    };
    // y = A sin(n pi x) cos(omega t) with frozen phase as envelope
    let mut prev: Option<(i32, i32)> = None;
    let cos_p = phase.cos();
    for col in 0..width {
        let x = col as f64 / width.saturating_sub(1).max(1) as f64;
        let y = amp * (n as f64 * std::f64::consts::PI * x).sin() * cos_p;
        let py = (cy - y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
        }
        prev = Some((col as i32, py));
    }
    // nodes
    for k in 0..=n {
        let x = k as f64 / n as f64;
        let px = (x * width.saturating_sub(1) as f64).round() as i32;
        canvas.line(px, (cy - 2.0) as i32, px, (cy + 2.0) as i32, '|');
    }
    // string ends
    canvas.plot(0, cy.round() as i32, 'o');
    canvas.plot(width.saturating_sub(1) as i32, cy.round() as i32, 'o');
}

/// Standing wave room.
#[derive(Debug, Default)]
pub struct StandingWave {
    seed: u64,
}

impl StandingWave {
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

impl Room for StandingWave {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "standing-wave",
            title: "Standing Wave",
            wing: "Waves & Sound",
            blurb: "Fixed-end string modes with nodes and antinodes. t and DRAG: SET MODE.",
            accent: [40, 100, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, mode(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "standing wave",
            root: 523.25,
            tempo: 80,
            line: &[0, 0, 5, 5, 7, 7, 12, 12],
            encodes: "left and right waves freeze into nodes",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET MODE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = mode(t, None);
        Some(format!("n={n}  string  DRAG:MODE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = mode(t, hands.last().copied());
        draw(canvas, n, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = mode(t, hands.last().copied());
        // Fixed ends: n+1 nodes (incl ends), n antinodes, node spacing L/n.
        let nodes = n + 1;
        let dx = 1.0 / n as f64;
        Some(format!("n={n}  nodes={nodes}  anti={n}  dx={dx:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "A standing wave is two traveling waves of equal frequency and opposite \
         direction. On a fixed string, mode n has n+1 nodes including the ends: \
         the harmonics of music."
    }
}

#[cfg(test)]
mod tests {
    use super::StandingWave;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = StandingWave::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("MODE") || s.contains("string"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn mode_changes() {
        let r = StandingWave::new();
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
        StandingWave::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 0);
    }
}

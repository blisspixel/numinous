//! Ricker map: discrete population model with boom and bust.
//!
//! x' = x exp(r (1 - x)). DRAG: TUNE R. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const ORBIT: usize = 200;

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

fn r_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.5 + x * 3.5 + s
    } else {
        1.5 + phase_unit(t) * 2.0 + s
    }
}

fn ricker(x: f64, r: f64) -> f64 {
    x * (r * (1.0 - x)).exp()
}

fn draw(canvas: &mut dyn Surface, r: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Graph y = x exp(r(1-x))
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=width {
        let x = i as f64 / width.saturating_sub(1).max(1) as f64 * 2.5;
        let y = ricker(x, r).clamp(0.0, 3.0);
        let px = i as i32;
        let py = ((1.0 - y / 3.0) * height.saturating_sub(1) as f64).round() as i32;
        if let Some(o) = prev {
            canvas.line(o.0, o.1, px, py, '#');
        }
        prev = Some((px, py));
    }
    canvas.line(
        0,
        height.saturating_sub(1) as i32,
        width.saturating_sub(1) as i32,
        0,
        '.',
    );
    let mut x = if seed == 0 {
        0.3
    } else {
        0.1 + (seed % 20) as f64 * 0.02
    };
    let mut px = ((x / 2.5) * width.saturating_sub(1) as f64).round() as i32;
    let mut py = height.saturating_sub(1) as i32;
    for i in 0..ORBIT {
        let y = ricker(x, r);
        if !y.is_finite() || y > 10.0 {
            break;
        }
        let qx = ((x / 2.5).clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32;
        let qy =
            ((1.0 - (y / 3.0).clamp(0.0, 1.0)) * height.saturating_sub(1) as f64).round() as i32;
        canvas.line(px, py, qx, py, if i % 2 == 0 { '*' } else { '+' });
        canvas.line(qx, py, qx, qy, if i % 2 == 0 { '*' } else { '+' });
        let dx = ((y / 2.5).clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32;
        let dy =
            ((1.0 - (y / 3.0).clamp(0.0, 1.0)) * height.saturating_sub(1) as f64).round() as i32;
        canvas.line(qx, qy, dx, dy, '.');
        px = dx;
        py = dy;
        x = y;
    }
}

/// Ricker map room.
#[derive(Debug, Default)]
pub struct Ricker {
    seed: u64,
}

impl Ricker {
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

impl Room for Ricker {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "ricker",
            title: "Ricker Map",
            wing: "Motion & Dynamics",
            blurb: "Population boom-bust: x exp(r(1-x)). t and DRAG: TUNE R.",
            accent: [40, 160, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, r_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "ricker",
            root: 174.61,
            tempo: 96,
            line: &[0, 5, 12, 5, 0, 7, 12, 0],
            encodes: "growth rate driving boom and crash cycles",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE R")
    }

    fn status(&self, t: f64) -> Option<String> {
        let r = r_param(t, None, self.seed);
        Some(format!("r={r:.2}  pop  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let r = r_param(t, hands.last().copied(), self.seed);
        draw(canvas, r, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let r = r_param(t, hands.last().copied(), self.seed);
        // Fixed point at x=1; period-doubling cascade for larger r.
        let mut x = if self.seed == 0 {
            0.3
        } else {
            0.1 + (self.seed % 20) as f64 * 0.02
        };
        for _ in 0..40 {
            x = ricker(x, r);
            if !x.is_finite() {
                break;
            }
        }
        let mut mn = x;
        let mut mx = x;
        for _ in 0..80 {
            x = ricker(x, r);
            if !x.is_finite() {
                break;
            }
            mn = mn.min(x);
            mx = mx.max(x);
        }
        let band = if r < 2.0 {
            "stable"
        } else if r < 2.7 {
            "period"
        } else {
            "chaos"
        };
        Some(format!("r={r:.2}  x=[{mn:.2},{mx:.2}]  {band}"))
    }

    fn reveal(&self) -> &'static str {
        "The Ricker map is a classic discrete population model. As r rises, the \
         fixed point loses stability through period doubling into chaos: boom \
         and bust written as a one-dimensional map."
    }
}

#[cfg(test)]
mod tests {
    use super::Ricker;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Ricker::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Ricker::new();
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
        Ricker::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Ricker::new().motif().unwrap().line.len() >= 6);
    }
}

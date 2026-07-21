//! Cissoid of Diocles: classical cubic used to double the cube.
//!
//! DRAG: TUNE SCALE. See `docs/ROOMS.md`.

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

fn scale(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.03
    };
    if let Some((x, _)) = hand {
        0.3 + x * 0.7 + s
    } else {
        0.4 + phase_unit(t) * 0.5 + s
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = width as f64 * 0.25;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let rad = (width.min(height) as f64) * 0.35 * a.clamp(0.25, 1.0);
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 7) as f64 * 0.02
    };
    // guiding circle and line
    for i in 0..64 {
        let th = std::f64::consts::TAU * (i as f64 / 64.0);
        let px = (cx + rad * 0.5 * th.cos()).round() as i32;
        let py = (cy - rad * 0.5 * th.sin()).round() as i32;
        canvas.plot(px, py, '.');
    }
    let line_x = (cx + rad + j).round() as i32;
    canvas.line(line_x, 0, line_x, height.saturating_sub(1) as i32, '|');
    // cissoid: y^2 (2a - x) = x^3
    let mut prev: Option<(i32, i32)> = None;
    let steps = 200;
    for i in 1..steps {
        let x = rad * (i as f64 / steps as f64) * 1.8;
        let denom = (2.0 * rad - x).max(1e-6);
        let y2 = x * x * x / denom;
        if y2 < 0.0 {
            prev = None;
            continue;
        }
        let y = y2.sqrt();
        let px = (cx + x * 0.5).round() as i32;
        let py1 = (cy - y * 0.5).round() as i32;
        let py2 = (cy + y * 0.5).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py1, '#');
        }
        canvas.plot(px, py2, '#');
        prev = Some((px, py1));
    }
}

/// Cissoid room.
#[derive(Debug, Default)]
pub struct Cissoid {
    seed: u64,
}

impl Cissoid {
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

impl Room for Cissoid {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "cissoid",
            title: "Cissoid",
            wing: "Shape & Space",
            blurb: "Diocles' ivy curve for doubling the cube. t and DRAG: TUNE SCALE.",
            accent: [80, 140, 60],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, scale(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "cissoid",
            root: 103.8,
            tempo: 82,
            line: &[0, 4, 7, 11, 7, 4, 0, 12],
            encodes: "ivy curve that solves cube doubling by intersections",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE SCALE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = scale(t, None, self.seed);
        Some(format!("a={a:.2}  cissoid  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = scale(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let a = scale(t, hands.last().copied(), self.seed);
        // Cissoid of Diocles: vertical asymptote x = 2a.
        let asym = 2.0 * a;
        Some(format!("a={a:.2}  asym x={asym:.2}  ivy"))
    }

    fn reveal(&self) -> &'static str {
        "The cissoid of Diocles is the classical ivy curve y^2 (2a - x) = x^3. \
         Greeks used its intersections to double the cube: a cubic problem \
         reduced to a geometric construction."
    }
}

#[cfg(test)]
mod tests {
    use super::Cissoid;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Cissoid::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("cissoid"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn scale_changes() {
        let r = Cissoid::new();
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
        Cissoid::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

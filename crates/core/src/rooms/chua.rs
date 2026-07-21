//! Chua's circuit: double-scroll chaotic attractor from a piecewise-linear diode.
//!
//! DRAG: TUNE ALPHA. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const STEPS: usize = 6_000;
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

fn alpha(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        8.0 + x * 4.0 + s
    } else {
        9.0 + phase_unit(t) * 2.5 + s
    }
}

fn f_diode(x: f64) -> f64 {
    // Piecewise-linear Chua diode
    let m0 = -1.143;
    let m1 = -0.714;
    m1 * x + 0.5 * (m0 - m1) * ((x + 1.0).abs() - (x - 1.0).abs())
}

fn draw(canvas: &mut dyn Surface, a: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let beta = 14.286;
    let mut x = 0.1
        + if seed == 0 {
            0.0
        } else {
            (seed % 7) as f64 * 0.01
        };
    let mut y = 0.0;
    let mut z = 0.0;
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    let mut pts = Vec::with_capacity(STEPS);
    for _ in 0..200 {
        let dx = a * (y - x - f_diode(x));
        let dy = x - y + z;
        let dz = -beta * y;
        x += dx * DT;
        y += dy * DT;
        z += dz * DT;
    }
    for _ in 0..STEPS {
        let dx = a * (y - x - f_diode(x));
        let dy = x - y + z;
        let dz = -beta * y;
        x += dx * DT;
        y += dy * DT;
        z += dz * DT;
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
    for (i, &(px, py)) in pts.iter().enumerate() {
        let u = ((px - min_x) / dx).clamp(0.0, 1.0);
        let v = ((py - min_y) / dy).clamp(0.0, 1.0);
        let ix = (u * width.saturating_sub(1) as f64).round() as i32;
        let iy = ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32;
        canvas.plot(ix, iy, if i % 13 == 0 { '#' } else { '*' });
    }
}

/// Chua circuit room.
#[derive(Debug, Default)]
pub struct Chua {
    seed: u64,
}

impl Chua {
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

impl Room for Chua {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "chua",
            title: "Chua Circuit",
            wing: "Motion & Dynamics",
            blurb: "Double-scroll chaos from a nonlinear diode circuit. t and DRAG: TUNE ALPHA.",
            accent: [200, 60, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, alpha(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "chua",
            root: 207.65,
            tempo: 94,
            line: &[0, 3, 7, 10, 14, 10, 7, 3],
            encodes: "two scrolls joined by a piecewise diode",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE ALPHA")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = alpha(t, None, self.seed);
        Some(format!("a={a:.2}  chua  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = alpha(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let a = alpha(t, hands.last().copied(), self.seed);
        let beta = 14.286;
        let mut x = 0.1_f64;
        let mut y = 0.0_f64;
        let mut z = 0.0_f64;
        for _ in 0..200 {
            let dx = a * (y - x - f_diode(x));
            let dy = x - y + z;
            let dz = -beta * y;
            x += dx * DT;
            y += dy * DT;
            z += dz * DT;
        }
        // Measure the settled trajectory only.
        let mut min_x = x;
        let mut max_x = x;
        let mut flips = 0u32;
        let mut sign = if x >= 0.0 { 1.0 } else { -1.0 };
        for _ in 0..800 {
            let dx = a * (y - x - f_diode(x));
            let dy = x - y + z;
            let dz = -beta * y;
            x += dx * DT;
            y += dy * DT;
            z += dz * DT;
            if !x.is_finite() {
                break;
            }
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            let s = if x >= 0.0 { 1.0 } else { -1.0 };
            if s != sign {
                flips += 1;
                sign = s;
            }
        }
        let span = max_x - min_x;
        Some(format!("a={a:.1}  span={span:.2}  flips={flips}"))
    }

    fn reveal(&self) -> &'static str {
        "Chua's circuit is a simple electronic loop with a piecewise-linear \
         diode. It was the first physical system designed to exhibit chaos, \
         painting the famous double-scroll attractor."
    }
}

#[cfg(test)]
mod tests {
    use super::Chua;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Chua::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Chua::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.1,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Chua::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}

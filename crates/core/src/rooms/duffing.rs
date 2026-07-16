//! Duffing oscillator: cubic spring chaos and double-well portraits.
//!
//! x'' + delta x' + alpha x + beta x^3 = gamma cos(omega t).
//! DRAG: TUNE DRIVE. See `docs/ROOMS.md`.

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

fn gamma(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 7) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.1 + x * 0.8 + s
    } else {
        0.2 + phase_unit(t) * 0.5 + s
    }
}

fn integrate(g: f64) -> Vec<(f64, f64)> {
    // Softening double-well-ish: alpha=-1, beta=1, delta=0.3, omega=1.2
    let delta = 0.3;
    let alpha = -1.0;
    let beta = 1.0;
    let omega = 1.2;
    let mut x: f64 = 0.1;
    let mut v: f64 = 0.0;
    let mut t: f64 = 0.0;
    let mut out = Vec::with_capacity(STEPS);
    for _ in 0..STEPS {
        let acc = -delta * v - alpha * x - beta * x * x * x + g * (omega * t).cos();
        v += DT * acc;
        x += DT * v;
        t += DT;
        if !x.is_finite() || !v.is_finite() {
            break;
        }
        out.push((x, v));
    }
    out
}

fn draw(canvas: &mut dyn Surface, pts: &[(f64, f64)]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || pts.len() < 2 {
        return;
    }
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for &(x, y) in pts {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    let dx = (max_x - min_x).max(1e-6);
    let dy = (max_y - min_y).max(1e-6);
    let mut prev: Option<(i32, i32)> = None;
    for (i, &(x, y)) in pts.iter().enumerate() {
        let u = 0.08 + 0.84 * (x - min_x) / dx;
        let v = 0.08 + 0.84 * (y - min_y) / dy;
        let px = (u * width.saturating_sub(1) as f64).round() as i32;
        let py = ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32;
        if let Some(o) = prev {
            let ch = if i + 400 > pts.len() {
                '#'
            } else if i % 5 == 0 {
                '+'
            } else {
                '*'
            };
            canvas.line(o.0, o.1, px, py, ch);
        }
        prev = Some((px, py));
    }
}

/// Duffing oscillator room.
#[derive(Debug, Default)]
pub struct Duffing {
    seed: u64,
}

impl Duffing {
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

impl Room for Duffing {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "duffing",
            title: "The Duffing Well",
            wing: "Motion & Dynamics",
            blurb: "Driven cubic oscillator: double-well chaos under strong drive. t and DRAG: \
                    TUNE DRIVE.",
            accent: [180, 80, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let g = gamma(t, None, self.seed);
        draw(canvas, &integrate(g));
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "duffing",
            root: 123.47,
            tempo: 86,
            line: &[0, 5, 3, 8, 12, 8, 3, 5],
            encodes: "drive strength folding a double well",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE DRIVE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let g = gamma(t, None, self.seed);
        Some(format!("gamma={g:.2}  DRAG:DRIVE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let g = gamma(t, hands.last().copied(), self.seed);
        draw(canvas, &integrate(g));
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
        let g = gamma(t, hands.last().copied(), self.seed);
        // Lightweight extrema sample (not the full render integration).
        let delta = 0.3;
        let alpha = -1.0;
        let beta = 1.0;
        let omega = 1.2;
        let mut x = 0.1_f64;
        let mut v = 0.0_f64;
        let mut time = 0.0_f64;
        let mut max_x = 0.0_f64;
        let mut max_v = 0.0_f64;
        for _ in 0..900 {
            let acc = -delta * v - alpha * x - beta * x * x * x + g * (omega * time).cos();
            v += DT * acc;
            x += DT * v;
            time += DT;
            if !x.is_finite() || !v.is_finite() {
                break;
            }
            max_x = max_x.max(x.abs());
            max_v = max_v.max(v.abs());
        }
        let band = if max_x > 1.5 {
            "chaos?"
        } else if max_x > 0.8 {
            "large"
        } else {
            "mild"
        };
        Some(format!("g={g:.2}  |x|={max_x:.2}  |v|={max_v:.2}  {band}"))
    }

    fn reveal(&self) -> &'static str {
        "Duffing's equation adds a cubic spring and a periodic drive. For weak \
         drive the phase portrait is a tame loop; for stronger gamma the orbit \
         folds into a strange attractor straddling two wells."
    }
}

#[cfg(test)]
mod tests {
    use super::Duffing;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Duffing::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("DRIVE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn drive_changes() {
        let r = Duffing::new();
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
        Duffing::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Duffing::new().motif().unwrap().line.len() >= 6);
    }
}

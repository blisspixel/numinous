//! SIR epidemic model: susceptible, infected, recovered compartments.
//!
//! DRAG: TUNE R0. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const STEPS: usize = 200;
const DT: f64 = 0.05;

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

fn r0(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.5 + x * 4.0 + s
    } else {
        0.8 + phase_unit(t) * 2.5 + s
    }
}

fn draw(canvas: &mut dyn Surface, r0_val: f64, seed: u64) -> f64 {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return 0.0;
    }
    let gamma = 0.2;
    let beta = r0_val * gamma;
    let mut s = 0.99;
    let mut i = 0.01
        + if seed == 0 {
            0.0
        } else {
            (seed % 4) as f64 * 0.002
        };
    let mut r = 0.0;
    let mut hist = Vec::with_capacity(STEPS);
    for _ in 0..STEPS {
        let ds = -beta * s * i;
        let di = beta * s * i - gamma * i;
        let dr = gamma * i;
        s = (s + ds * DT).clamp(0.0, 1.0);
        i = (i + di * DT).clamp(0.0, 1.0);
        r = (r + dr * DT).clamp(0.0, 1.0);
        // renormalize mild drift
        let tot = (s + i + r).max(1e-9);
        s /= tot;
        i /= tot;
        r /= tot;
        hist.push((s, i, r));
    }
    let peak_i = hist.iter().map(|h| h.1).fold(0.0_f64, f64::max);
    for col in 0..width {
        let hi = (col * STEPS / width.max(1)).min(STEPS - 1);
        let (ss, ii, rr) = hist[hi];
        let ys = ((1.0 - ss) * height.saturating_sub(1) as f64).round() as i32;
        let yi = ((1.0 - ii) * height.saturating_sub(1) as f64).round() as i32;
        let yr = ((1.0 - rr) * height.saturating_sub(1) as f64).round() as i32;
        canvas.plot(col as i32, ys, '.');
        canvas.plot(col as i32, yi, '#');
        canvas.plot(col as i32, yr, '+');
    }
    peak_i
}

/// SIR epidemic room.
#[derive(Debug, Default)]
pub struct Sir {
    seed: u64,
}

impl Sir {
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

impl Room for Sir {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "sir",
            title: "SIR Epidemic",
            wing: "Chance & Order",
            blurb: "Susceptible, infected, recovered curves. t and DRAG: TUNE R0.",
            accent: [180, 40, 60],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let _ = draw(canvas, r0(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "sir",
            root: 130.0,
            tempo: 74,
            line: &[0, 5, 9, 12, 9, 5, 2, 0],
            encodes: "infection wave crest set by basic reproduction number",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE R0")
    }

    fn status(&self, t: f64) -> Option<String> {
        let r = r0(t, None, self.seed);
        Some(format!("R0={r:.2}  sir  DRAG:R0"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let r = r0(t, hands.last().copied(), self.seed);
        let _ = draw(canvas, r, self.seed ^ hands.len() as u64);
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
        let r = r0(t, hands.last().copied(), self.seed);
        let thr = if r > 1.0 { "epidemic" } else { "fade" };
        Some(format!("R0={r:.3}  {thr}"))
    }

    fn reveal(&self) -> &'static str {
        "The SIR model splits a population into susceptible, infected, and \
         recovered. R0 = beta/gamma decides the fate: above one an epidemic \
         crest rises; below one infection fades. Simple, sharp, classical."
    }
}

#[cfg(test)]
mod tests {
    use super::Sir;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Sir::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("R0"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn r0_changes() {
        let r = Sir::new();
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
        Sir::new().render(&mut c, 0.6);
        assert!(c.ink_count() > 0);
    }
}

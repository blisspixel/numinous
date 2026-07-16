//! Arithmetic-geometric mean: Gauss AGM converges to elliptic integrals.
//!
//! DRAG: TUNE R. See `docs/ROOMS.md`.

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

fn ratio(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 4) as f64 * 0.03
    };
    if let Some((x, _)) = hand {
        0.05 + x * 0.9 + s
    } else {
        0.1 + phase_unit(t) * 0.8 + s
    }
    .clamp(0.02, 0.98)
}

fn agm_steps(a0: f64, g0: f64, max_steps: usize) -> Vec<(f64, f64)> {
    let mut out = vec![(a0, g0)];
    let mut a = a0;
    let mut g = g0;
    for _ in 0..max_steps {
        let an = 0.5 * (a + g);
        let gn = (a * g).sqrt();
        out.push((an, gn));
        if (an - gn).abs() < 1e-12 {
            break;
        }
        a = an;
        g = gn;
    }
    out
}

fn draw(canvas: &mut dyn Surface, r: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let a0 = 1.0;
    let g0 = r.clamp(0.02, 0.98);
    let steps = agm_steps(a0, g0, 16);
    let pad = if seed == 0 { 0i32 } else { (seed % 2) as i32 };
    // two sequences as bars
    for (i, &(a, g)) in steps.iter().enumerate() {
        let x = ((i as f64 / steps.len().max(1) as f64) * width.saturating_sub(1) as f64).round()
            as i32
            + pad;
        let ya = ((1.0 - a) * height.saturating_sub(1) as f64).round() as i32;
        let yg = ((1.0 - g) * height.saturating_sub(1) as f64).round() as i32;
        canvas.line(x, height as i32 - 1, x, ya, '#');
        canvas.line(x + 1, height as i32 - 1, x + 1, yg, '=');
    }
    // limit line
    if let Some(&(a, g)) = steps.last() {
        let lim = 0.5 * (a + g);
        let y = ((1.0 - lim) * height.saturating_sub(1) as f64).round() as i32;
        canvas.line(0, y, width as i32 - 1, y, '-');
    }
}

/// AGM room.
#[derive(Debug, Default)]
pub struct AgmMean {
    seed: u64,
}

impl AgmMean {
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

impl Room for AgmMean {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "agm-mean",
            title: "Arithmetic-Geometric Mean",
            wing: "Analysis",
            blurb: "a,g converge by average and geometric mean. t and DRAG: TUNE R.",
            accent: [80, 100, 70],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, ratio(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "agm-mean",
            root: 554.37,
            tempo: 80,
            line: &[0, 4, 7, 11, 14, 11, 7, 4],
            encodes: "AGM: arithmetic and geometric means lock to elliptic K",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE R")
    }

    fn status(&self, t: f64) -> Option<String> {
        let r = ratio(t, None, self.seed);
        let steps = agm_steps(1.0, r, 16);
        let (a, g) = *steps.last().unwrap_or(&(1.0, r));
        Some(format!("r={r:.2}  agm={:.3}  DRAG:R", 0.5 * (a + g)))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let r = ratio(t, hands.last().copied(), self.seed);
        draw(canvas, r, self.seed ^ hands.len() as u64);
        if let Some(&(x, y)) = hands.last() {
            let (bw, bh) = canvas.draw_bounds();
            if bw > 0 && bh > 0 {
                let px = (x * bw.saturating_sub(1) as f64).round() as i32;
                let py = (y * bh.saturating_sub(1) as f64).round() as i32;
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
        let r = ratio(t, hands.last().copied(), self.seed);
        let steps = agm_steps(1.0, r, 16);
        let (a, g) = *steps.last().unwrap_or(&(1.0, r));
        Some(format!("R={r:.3}  agm={:.3}", 0.5 * (a + g)))
    }

    fn reveal(&self) -> &'static str {
        "The arithmetic-geometric mean iterates a <- (a+g)/2 and g <- sqrt(a g). \
         The common limit AGM(a,g) is related to the complete elliptic integral \
         of the first kind. Gauss used it to compute pi and elliptic arcs."
    }
}

#[cfg(test)]
mod tests {
    use super::AgmMean;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = AgmMean::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("agm"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn r_changes() {
        let r = AgmMean::new();
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
        AgmMean::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 0);
    }
}

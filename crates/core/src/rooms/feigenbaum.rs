//! Feigenbaum cascade readout: period-doubling r values as a ladder.
//!
//! Marks successive bifurcation parameters of the logistic map.
//! DRAG: SET THE GENERATION. See `docs/ROOMS.md`.

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

fn generation(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (1 + (x * 8.0) as usize).clamp(1, 9)
    } else {
        (2 + (phase_unit(t) * 6.0) as usize).clamp(1, 8)
    }
}

/// Approximate period-doubling bifurcation values (known sequence).
fn bif_r(n: usize) -> f64 {
    // r_infty ~ 3.5699456; early values tabulated
    const RS: [f64; 10] = [
        3.0,
        3.449_489_7,
        3.544_090_3,
        3.564_407_3,
        3.568_759_4,
        3.569_691_6,
        3.569_891_3,
        3.569_934_0,
        3.569_943_2,
        3.569_945_2,
    ];
    RS[n.min(RS.len() - 1)]
}

fn draw(canvas: &mut dyn Surface, g: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Bifurcation diagram fragment near cascade
    let r0 = 2.9;
    let r1 = 3.7;
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.001
    };
    for col in 0..width {
        let r = r0 + (r1 - r0) * (col as f64 / width.saturating_sub(1).max(1) as f64) + j;
        let mut x = 0.5;
        for _ in 0..80 {
            x = r * x * (1.0 - x);
        }
        for _ in 0..40 {
            x = r * x * (1.0 - x);
            let py = ((1.0 - x.clamp(0.0, 1.0)) * height.saturating_sub(1) as f64).round() as i32;
            canvas.plot(col as i32, py, '.');
        }
    }
    // Vertical marks at first g bifurcations
    for n in 0..g {
        let r = bif_r(n);
        let u = ((r - r0) / (r1 - r0)).clamp(0.0, 1.0);
        let px = (u * width.saturating_sub(1) as f64).round() as i32;
        canvas.line(px, 0, px, height.saturating_sub(1) as i32, '|');
        canvas.plot(px, 1, '#');
    }
}

/// Feigenbaum cascade room.
#[derive(Debug, Default)]
pub struct Feigenbaum {
    seed: u64,
}

impl Feigenbaum {
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

impl Room for Feigenbaum {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "feigenbaum",
            title: "Feigenbaum Ladder",
            wing: "Motion & Dynamics",
            blurb: "Period-doubling cascade of the logistic map, marked. t and DRAG: SET THE \
                    GENERATION.",
            accent: [220, 100, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, generation(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.7
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "feigenbaum",
            root: 185.0,
            tempo: 76,
            line: &[0, 0, 5, 5, 7, 7, 12, 12],
            encodes: "period doubling toward the onset of chaos",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE GENERATION")
    }

    fn status(&self, t: f64) -> Option<String> {
        let g = generation(t, None);
        let r = bif_r(g.saturating_sub(1));
        Some(format!("gen={g}  r~{r:.4}  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let g = generation(t, hands.last().copied());
        draw(canvas, g, self.seed ^ hands.len() as u64);
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
        let g = generation(t, hands.last().copied());
        let r = bif_r(g.saturating_sub(1));
        // Period at generation g is 2^g (period-doubling cascade).
        let period = 1u64 << g.min(12);
        Some(format!("gen={g}  r={r:.4}  per={period}"))
    }

    fn reveal(&self) -> &'static str {
        "Feigenbaum found that period-doubling cascades share a universal ratio \
         delta ~ 4.669. The logistic map is the flagship: each vertical mark is \
         a bifurcation r_n approaching the chaos threshold."
    }
}

#[cfg(test)]
mod tests {
    use super::Feigenbaum;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Feigenbaum::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("gen"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn gen_changes() {
        let r = Feigenbaum::new();
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
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Feigenbaum::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Feigenbaum::new().motif().unwrap().line.len() >= 6);
    }
}

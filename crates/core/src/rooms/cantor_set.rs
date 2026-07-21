//! Cantor set and devil's staircase: remove middle thirds forever.
//!
//! Top: ternary Cantor dust construction. Bottom: Cantor function (devil's
//! staircase). DRAG: SET THE DEPTH. See `docs/ROOMS.md`.

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

fn depth(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (1 + (x * 8.0) as usize).clamp(1, 9)
    } else {
        (2 + (phase_unit(t) * 6.0) as usize).clamp(1, 8)
    }
}

/// Intervals remaining after `d` middle-third removals on [0,1].
fn cantor_intervals(d: usize) -> Vec<(f64, f64)> {
    let mut segs = vec![(0.0, 1.0)];
    for _ in 0..d {
        let mut next = Vec::with_capacity(segs.len() * 2);
        for &(a, b) in &segs {
            let w = b - a;
            next.push((a, a + w / 3.0));
            next.push((a + 2.0 * w / 3.0, b));
        }
        segs = next;
        if segs.len() > 512 {
            break;
        }
    }
    segs
}

/// Cantor (devil's staircase) function via ternary digit rewrite.
fn cantor_fn(x: f64) -> f64 {
    let mut x = x.clamp(0.0, 1.0);
    let mut y: f64 = 0.0;
    let mut w: f64 = 0.5;
    for _ in 0..28 {
        x *= 3.0;
        if x < 1.0 {
            // ternary digit 0 -> binary 0
        } else if x < 2.0 {
            // middle third: stay constant for the remaining climb
            y += w;
            break;
        } else {
            // ternary digit 2 -> binary 1
            y += w;
            x -= 2.0;
        }
        w *= 0.5;
    }
    y.clamp(0.0, 1.0)
}

fn draw(canvas: &mut dyn Surface, d: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Upper half: Cantor dust bars by generation strip.
    let top_h = (height as f64 * 0.42).round() as i32;
    for (gi, g) in (0..=d).enumerate() {
        let row = ((gi as f64 / (d + 1) as f64) * top_h as f64).round() as i32;
        let row2 = ((((gi + 1) as f64) / (d + 1) as f64) * top_h as f64).round() as i32;
        for &(a, b) in &cantor_intervals(g) {
            let x0 = (a * width.saturating_sub(1) as f64).round() as i32;
            let x1 = (b * width.saturating_sub(1) as f64).round() as i32;
            for yy in row..row2.max(row + 1) {
                canvas.line(x0, yy, x1.max(x0), yy, if g == d { '#' } else { '=' });
            }
        }
    }
    // Lower half: devil's staircase.
    let y_base = (height as f64 * 0.52).round() as i32;
    let y_span = height.saturating_sub(y_base as usize + 1) as f64;
    let mut prev: Option<(i32, i32)> = None;
    let samples = width.saturating_mul(2).max(40);
    let shift = if seed == 0 {
        0.0
    } else {
        (seed % 11) as f64 * 0.001
    };
    for i in 0..=samples {
        let u = i as f64 / samples as f64;
        let x = (u + shift).fract();
        let y = cantor_fn(x);
        let px = (u * width.saturating_sub(1) as f64).round() as i32;
        let py = y_base + ((1.0 - y) * y_span).round() as i32;
        if let Some(o) = prev {
            canvas.line(o.0, o.1, px, py, '*');
        }
        prev = Some((px, py));
    }
}

/// Cantor set room.
#[derive(Debug, Default)]
pub struct CantorSet {
    seed: u64,
}

impl CantorSet {
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

impl Room for CantorSet {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "cantor-set",
            title: "The Devil's Staircase",
            wing: "Fractals",
            blurb: "Middle-third Cantor dust above; Cantor function (devil's staircase) below. t \
                    and DRAG: SET THE DEPTH.",
            accent: [160, 40, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, depth(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.7
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "cantor",
            root: 207.65,
            tempo: 80,
            line: &[0, 0, 0, 7, 7, 7, 12, 12],
            encodes: "remove the middle third forever; climb in plateaus",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE DEPTH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = depth(t, None);
        let segs = cantor_intervals(d).len();
        Some(format!("depth={d}  segs={segs}  DRAG:DEPTH"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let d = depth(t, hands.last().copied());
        draw(canvas, d, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let d = depth(t, hands.last().copied());
        let segs = cantor_intervals(d).len();
        let measure = (2.0f64 / 3.0).powi(d as i32);
        // Hausdorff dim of middle-thirds Cantor is log2/log3.
        let dim = 2.0_f64.ln() / 3.0_f64.ln();
        Some(format!("d={d}  segs={segs}  len={measure:.3}  H={dim:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The middle-thirds Cantor set has measure zero yet uncountably many \
         points. The Cantor function maps it onto [0,1] while staying constant \
         on every removed interval: a continuous staircase that is almost \
         everywhere flat, yet climbs from 0 to 1."
    }
}

#[cfg(test)]
mod tests {
    use super::{CantorSet, cantor_intervals};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = CantorSet::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("DEPTH"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn depth_changes() {
        let r = CantorSet::new();
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
    fn intervals_double() {
        assert_eq!(cantor_intervals(0).len(), 1);
        assert_eq!(cantor_intervals(1).len(), 2);
        assert_eq!(cantor_intervals(2).len(), 4);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        CantorSet::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(CantorSet::new().motif().unwrap().line.len() >= 6);
    }
}

//! Roulette gallery: a circle rolling on a fixed circle, both epi and hypo paths.
//!
//! Distinct from pure hypotrochoid/epitrochoid rooms: dual overlay. DRAG: TUNE K.
//! See `docs/ROOMS.md`.

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

fn k_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.1
    };
    if let Some((x, _)) = hand {
        2.5 + x * 4.5 + s
    } else {
        3.0 + phase_unit(t) * 3.5 + s
    }
}

fn draw(canvas: &mut dyn Surface, k: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let r = (width.min(height) as f64) * 0.09;
    let big = k * r;
    let d = r
        * (0.85
            + if seed == 0 {
                0.0
            } else {
                (seed % 4) as f64 * 0.05
            });
    // epi
    let r_sum = big + r;
    let scale = (width.min(height) as f64) * 0.35 / (r_sum + d).max(1.0);
    let steps = 500;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = 3.0 * std::f64::consts::TAU * (i as f64 / steps as f64);
        let x = r_sum * th.cos() - d * ((r_sum / r) * th).cos();
        let y = r_sum * th.sin() - d * ((r_sum / r) * th).sin();
        let px = (cx + scale * x).round() as i32;
        let py = (cy - scale * y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
    // hypo overlay smaller
    let r_diff = (big - r).max(r * 0.5);
    let d2 = r * 0.6;
    let scale2 = scale * 0.7;
    prev = None;
    for i in 0..=steps {
        let th = 3.0 * std::f64::consts::TAU * (i as f64 / steps as f64);
        let x = r_diff * th.cos() + d2 * ((r_diff / r) * th).cos();
        let y = r_diff * th.sin() - d2 * ((r_diff / r) * th).sin();
        let px = (cx + scale2 * x).round() as i32;
        let py = (cy - scale2 * y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '*');
        }
        prev = Some((px, py));
    }
}

/// Roulette gallery room.
#[derive(Debug, Default)]
pub struct Roulette {
    seed: u64,
}

impl Roulette {
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

impl Room for Roulette {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "roulette",
            title: "Roulette Gallery",
            wing: "Shape & Space",
            blurb: "Epi and hypo rolling paths overlaid. t and DRAG: TUNE K.",
            accent: [180, 40, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, k_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "roulette",
            root: 329.63,
            tempo: 104,
            line: &[0, 3, 7, 12, 15, 12, 7, 3],
            encodes: "outer and inner rolls drawn as one twin path",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE K")
    }

    fn status(&self, t: f64) -> Option<String> {
        let k = k_param(t, None, self.seed);
        Some(format!("k={k:.1}  roulette  DRAG:K"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let k = k_param(t, hands.last().copied(), self.seed);
        draw(canvas, k, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let k = k_param(t, hands.last().copied(), self.seed);
        // k = R/r; petals ~ |k-1| or k+1 depending epi/hypo family in room.
        let petals = (k - 1.0).abs().round().max(1.0) as i32;
        Some(format!("k={k:.2}  petals~{petals}  roulettes"))
    }

    fn reveal(&self) -> &'static str {
        "Roulettes are paths of points carried by one curve rolling on another. \
         Circles on circles give epitrochoids and hypotrochoids: the twin \
         Spirograph families overlaid here."
    }
}

#[cfg(test)]
mod tests {
    use super::Roulette;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Roulette::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("roulette"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn k_changes() {
        let r = Roulette::new();
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
        Roulette::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

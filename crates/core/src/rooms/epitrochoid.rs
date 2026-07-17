//! Epitrochoid: roulette of a circle rolling outside a fixed circle.
//!
//! DRAG: TUNE RATIO. See `docs/ROOMS.md`.

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
        (seed % 5) as f64 * 0.1
    };
    if let Some((x, _)) = hand {
        1.5 + x * 5.0 + s
    } else {
        2.0 + phase_unit(t) * 3.5 + s
    }
}

fn draw(canvas: &mut dyn Surface, k: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let r = (width.min(height) as f64) * 0.1;
    let big = k * r;
    let rr = r;
    let d = rr
        * (0.7
            + if seed == 0 {
                0.0
            } else {
                (seed % 4) as f64 * 0.08
            });
    // x = (R+r) cos t - d cos((R+r)/r t)
    let r_sum = big + rr;
    let scale = (width.min(height) as f64) * 0.38 / (r_sum + d).max(1.0);
    let steps = 800;
    let mut prev: Option<(i32, i32)> = None;
    let turns = 4.0;
    for i in 0..=steps {
        let th = turns * std::f64::consts::TAU * (i as f64 / steps as f64);
        let x = r_sum * th.cos() - d * ((r_sum / rr) * th).cos();
        let y = r_sum * th.sin() - d * ((r_sum / rr) * th).sin();
        let px = (cx + scale * x).round() as i32;
        let py = (cy - scale * y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
}

/// Epitrochoid room.
#[derive(Debug, Default)]
pub struct Epitrochoid {
    seed: u64,
}

impl Epitrochoid {
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

impl Room for Epitrochoid {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "epitrochoid",
            title: "Epitrochoid",
            wing: "Shape & Space",
            blurb: "Outer rolling roulette: epicycloid family. t and DRAG: TUNE RATIO.",
            accent: [80, 60, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, ratio(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "epitrochoid",
            root: 246.94,
            tempo: 96,
            line: &[0, 4, 7, 11, 14, 11, 7, 4],
            encodes: "outer rolling ratio blooms petals outward",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE RATIO")
    }

    fn status(&self, t: f64) -> Option<String> {
        let k = ratio(t, None, self.seed);
        Some(format!("R/r={k:.1}  epi  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let k = ratio(t, hands.last().copied(), self.seed);
        draw(canvas, k, self.seed ^ hands.len() as u64);
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
        let k = ratio(t, hands.last().copied(), self.seed);
        // Outer rolling: fixed R = k r; petal count tracks round(R/r) = round(k).
        let petals = k.round() as i32;
        let r_sum = k + 1.0;
        Some(format!("R/r={k:.1}  petals~{petals}  R+r={r_sum:.1}"))
    }

    fn reveal(&self) -> &'static str {
        "An epitrochoid is traced by a point on a circle rolling outside a \
         fixed circle. Special cases include the cardioid and nephroid. The \
         ratio of radii sets how many petals bloom."
    }
}

#[cfg(test)]
mod tests {
    use super::Epitrochoid;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Epitrochoid::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("epi"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn ratio_changes() {
        let r = Epitrochoid::new();
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
        Epitrochoid::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

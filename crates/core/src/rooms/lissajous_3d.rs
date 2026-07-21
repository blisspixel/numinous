//! 3D Lissajous: three orthogonal harmonics as a space curve.
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
        (seed % 4) as f64 * 0.15
    };
    if let Some((x, _)) = hand {
        1.0 + x * 4.0 + s
    } else {
        1.5 + phase_unit(t) * 3.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, b: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let a = 1.0;
    let b = b.clamp(1.0, 5.5);
    let c = 2.0
        + if seed == 0 {
            0.0
        } else {
            (seed % 3) as f64 * 0.5
        };
    let sc = (width.min(height) as f64) * 0.38;
    let steps = 400;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let t = 2.0 * std::f64::consts::PI * (i as f64 / steps as f64);
        let x = (a * t).sin();
        let y = (b * t + 0.3).sin();
        let z = (c * t + 0.7).sin();
        let d = 1.0 / (2.4 + z * 0.5);
        let px = (cx + x * sc * d).round() as i32;
        let py = (cy - y * sc * d * 0.55).round() as i32;
        if let Some((ox, oy)) = prev {
            let ch = if z > 0.2 {
                '#'
            } else if z > -0.2 {
                '*'
            } else {
                '.'
            };
            canvas.line(ox, oy, px, py, ch);
        }
        prev = Some((px, py));
    }
}

/// 3D Lissajous room.
#[derive(Debug, Default)]
pub struct Lissajous3d {
    seed: u64,
}

impl Lissajous3d {
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

impl Room for Lissajous3d {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "lissajous-3d",
            title: "Lissajous 3D",
            wing: "Waves & Sound",
            blurb: "Three orthogonal sines draw a space knot. t and DRAG: TUNE RATIO.",
            accent: [30, 140, 120],
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
            key: "lissajous-3d",
            root: 55.0,
            tempo: 88,
            line: &[0, 4, 7, 9, 12, 9, 7, 4],
            encodes: "3D Lissajous: x=sin at, y=sin bt, z=sin ct space curve",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE RATIO")
    }

    fn status(&self, t: f64) -> Option<String> {
        let b = ratio(t, None, self.seed);
        Some(format!("b={b:.2}  3D  DRAG:RATIO"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let b = ratio(t, hands.last().copied(), self.seed);
        draw(canvas, b, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let b = ratio(t, hands.last().copied(), self.seed);
        Some(format!("B={b:.3}  liss3d"))
    }

    fn reveal(&self) -> &'static str {
        "A 3D Lissajous curve is (sin a t, sin b t, sin c t). When frequency \
         ratios are rational the path closes; irrational ratios fill a dense \
         tangle in a box. Oscilloscopes and orbital motion both draw cousins."
    }
}

#[cfg(test)]
mod tests {
    use super::Lissajous3d;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Lissajous3d::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("3D"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn ratio_changes() {
        let r = Lissajous3d::new();
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
        Lissajous3d::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

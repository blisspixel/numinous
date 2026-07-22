//! Hypotrochoid: roulette of a circle rolling inside a fixed circle (Spirograph).
//!
//! Ambient phase draws the roulette with a pen. DRAG: TUNE RATIO.
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

fn ratio(_t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    // R/r style via k = R/r
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.1
    };
    if let Some((x, _)) = hand {
        2.0 + x * 6.0 + s
    } else {
        // Ambient ratio holds a readable spirograph; motion lives in the pen.
        4.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, k: f64, show: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let r = (width.min(height) as f64) * 0.12;
    let big = k * r / (k - 1.0).max(1.1); // fixed circle radius style
    let rr = big / k.max(1.1);
    let d = rr
        * (0.6
            + if seed == 0 {
                0.0
            } else {
                (seed % 4) as f64 * 0.1
            });
    // x = (R-r) cos t + d cos((R-r)/r t)
    let r_diff = big - rr;
    let scale = (width.min(height) as f64) * 0.4 / (r_diff + d).max(1.0);
    let show = show.clamp(0.0, 1.0);
    let steps = 800;
    let turns = 4.0;
    let drawn = ((show * steps as f64).round() as usize).min(steps);
    // Soft ghost of the full inner roulette.
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = turns * std::f64::consts::TAU * (i as f64 / steps as f64);
        let x = r_diff * th.cos() + d * ((r_diff / rr) * th).cos();
        let y = r_diff * th.sin() - d * ((r_diff / rr) * th).sin();
        let px = (cx + scale * x).round() as i32;
        let py = (cy - scale * y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // Bright path so far.
    prev = None;
    for i in 0..=drawn {
        let th = turns * std::f64::consts::TAU * (i as f64 / steps as f64);
        let x = r_diff * th.cos() + d * ((r_diff / rr) * th).cos();
        let y = r_diff * th.sin() - d * ((r_diff / rr) * th).sin();
        let px = (cx + scale * x).round() as i32;
        let py = (cy - scale * y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
            canvas.line(ox, oy + 1, px, py + 1, '*');
        }
        prev = Some((px, py));
    }
    let pen_th = show * turns * std::f64::consts::TAU;
    let pen_x =
        (cx + scale * (r_diff * pen_th.cos() + d * ((r_diff / rr) * pen_th).cos())).round() as i32;
    let pen_y =
        (cy - scale * (r_diff * pen_th.sin() - d * ((r_diff / rr) * pen_th).sin())).round() as i32;
    for dy in -2..=2 {
        for dx in -2..=2 {
            if dx * dx + dy * dy <= 5 {
                canvas.plot(pen_x + dx, pen_y + dy, 'o');
            }
        }
    }
}

/// Hypotrochoid room.
#[derive(Debug, Default)]
pub struct Hypotrochoid {
    seed: u64,
}

impl Hypotrochoid {
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

impl Room for Hypotrochoid {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "hypotrochoid",
            title: "Hypotrochoid",
            wing: "Shape & Space",
            blurb: "Spirograph draws itself. Watch the pen; DRAG: TUNE RATIO.",
            accent: [200, 60, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, ratio(t, None, self.seed), phase_unit(t), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "hypotrochoid",
            root: 261.63,
            tempo: 108,
            line: &[0, 5, 10, 12, 7, 2, 9, 0],
            encodes: "inner rolling ratio sets petal count",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE RATIO")
    }

    fn status(&self, t: f64) -> Option<String> {
        let k = ratio(t, None, self.seed);
        let p = (phase_unit(t) * 100.0).round() as i32;
        Some(format!("R/r={k:.1}  draw={p}%  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let k = ratio(t, hands.last().copied(), self.seed);
        let show = hands
            .last()
            .map(|&(_, y)| y)
            .unwrap_or_else(|| phase_unit(t));
        draw(canvas, k, show, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let k = ratio(t, hands.last().copied(), self.seed);
        // Inner rolling: cusp count tracks round(R/r) for d near r (deltoid k=3, astroid k=4).
        let cusps = k.round() as i32;
        let kind = match cusps {
            3 => "deltoid",
            4 => "astroid",
            _ => "hypo",
        };
        Some(format!("R/r={k:.1}  cusps~{cusps}  {kind}"))
    }

    fn reveal(&self) -> &'static str {
        "A hypotrochoid is the path of a point attached to a circle rolling \
         inside a fixed circle. Rational radius ratios close into Spirograph \
         petals; irrationals dense-fill a ring."
    }
}

#[cfg(test)]
mod tests {
    use super::Hypotrochoid;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Hypotrochoid::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("draw"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn ratio_changes() {
        let r = Hypotrochoid::new();
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
    fn ambient_pen_moves_the_plate() {
        let r = Hypotrochoid::new();
        let mut a = Canvas::new(80, 48);
        let mut b = Canvas::new(80, 48);
        r.render(&mut a, 0.15);
        r.render(&mut b, 0.75);
        assert_ne!(a.to_text(), b.to_text(), "pen must walk the spirograph");
        assert!(a.ink_count() > 40);
        assert!(b.ink_count() > 40);
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Hypotrochoid::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

//! Astroid: hypocycloid with four cusps (a circle rolling in a 4x circle).
//!
//! DRAG: TUNE SCALE. See `docs/ROOMS.md`.

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

fn scale(_t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.55 + x * 0.55 + s
    } else {
        0.85 + s * 0.5
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, show: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let rad = (width.min(height) as f64) * 0.48 * a.clamp(0.5, 1.15);
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 11) as f64 * 0.03
    };
    let show = show.clamp(0.0, 1.0);
    let steps = 480;
    let drawn = ((show * steps as f64).round() as usize).min(steps);
    // Ghost full star.
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = rot + std::f64::consts::TAU * (i as f64 / steps as f64);
        let x = rad * th.cos().powi(3);
        let y = rad * th.sin().powi(3);
        let px = (cx + x).round() as i32;
        let py = (cy - y * 0.9).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // Bright path so far.
    prev = None;
    for i in 0..=drawn {
        let th = rot + std::f64::consts::TAU * (i as f64 / steps as f64);
        let x = rad * th.cos().powi(3);
        let y = rad * th.sin().powi(3);
        let px = (cx + x).round() as i32;
        let py = (cy - y * 0.9).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
            canvas.line(ox, oy + 1, px, py + 1, '*');
        }
        prev = Some((px, py));
    }
    // Envelope circle.
    let mut prev_c: Option<(i32, i32)> = None;
    for i in 0..=96 {
        let th = std::f64::consts::TAU * (i as f64 / 96.0);
        let px = (cx + rad * th.cos()).round() as i32;
        let py = (cy - rad * th.sin() * 0.9).round() as i32;
        if let Some(o) = prev_c {
            canvas.line(o.0, o.1, px, py, ':');
        }
        prev_c = Some((px, py));
    }
    // Rolling circle of radius a/4 inside a circle of radius a (classic hypocycloid).
    let pen_th = rot + show * std::f64::consts::TAU;
    let roll_r = rad * 0.25;
    let path_r = rad - roll_r;
    let rcx = cx + path_r * pen_th.cos();
    let rcy = cy - path_r * pen_th.sin() * 0.9;
    let mut prev_r: Option<(i32, i32)> = None;
    for i in 0..=40 {
        let th = std::f64::consts::TAU * (i as f64 / 40.0);
        let px = (rcx + roll_r * th.cos()).round() as i32;
        let py = (rcy - roll_r * th.sin() * 0.9).round() as i32;
        if let Some(o) = prev_r {
            canvas.line(o.0, o.1, px, py, '+');
        }
        prev_r = Some((px, py));
    }
    let pen_x = (cx + rad * pen_th.cos().powi(3)).round() as i32;
    let pen_y = (cy - rad * pen_th.sin().powi(3) * 0.9).round() as i32;
    canvas.line(rcx.round() as i32, rcy.round() as i32, pen_x, pen_y, '-');
    for dy in -2..=2 {
        for dx in -2..=2 {
            if dx * dx + dy * dy <= 5 {
                canvas.plot(pen_x + dx, pen_y + dy, 'o');
            }
        }
    }
}

/// Astroid room.
#[derive(Debug, Default)]
pub struct Astroid {
    seed: u64,
}

impl Astroid {
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

impl Room for Astroid {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "astroid",
            title: "Astroid",
            wing: "Shape & Space",
            blurb: "Four-cusped star from a rolling circle. t and DRAG: TUNE SCALE.",
            accent: [200, 160, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, scale(t, None, self.seed), phase_unit(t), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "astroid",
            root: 369.99,
            tempo: 80,
            line: &[0, 3, 7, 12, 7, 3, 0, 5],
            encodes: "cos cubed and sin cubed draw four cusps",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE SCALE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = scale(t, None, self.seed);
        let p = (phase_unit(t) * 100.0).round() as i32;
        Some(format!("a={a:.2}  draw={p}%  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = scale(t, hands.last().copied(), self.seed);
        let show = hands
            .last()
            .map(|&(_, y)| y)
            .unwrap_or_else(|| phase_unit(t));
        draw(canvas, a, show, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let a = scale(t, hands.last().copied(), self.seed);
        // Hypocycloid astroid: perimeter 6a, area (3/8) pi a^2.
        let perim = 6.0 * a;
        let area = 0.375 * std::f64::consts::PI * a * a;
        Some(format!("a={a:.2}  P=6a={perim:.2}  A={area:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "An astroid is a hypocycloid with four cusps: a circle of radius a/4 \
         rolls inside a circle of radius a. In coordinates it is simply \
         x = a cos^3 t, y = a sin^3 t: a star of envelopes."
    }
}

#[cfg(test)]
mod tests {
    use super::Astroid;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Astroid::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("astroid"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn scale_changes() {
        let r = Astroid::new();
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
        Astroid::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

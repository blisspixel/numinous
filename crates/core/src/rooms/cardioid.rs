//! Cardioid: epicycloid with one cusp (heart-shaped roulette).
//!
//! Ambient phase rolls the generating circle and walks a pen along the heart.
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
        // Ambient scale holds a readable heart; motion lives in the roll.
        0.85 + s * 0.5
    }
}

fn plot_bead(canvas: &mut dyn Surface, px: i32, py: i32, ch: char) {
    for dy in -2..=2 {
        for dx in -2..=2 {
            if dx * dx + dy * dy <= 5 {
                canvas.plot(px + dx, py + dy, ch);
            }
        }
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, show: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let rad = (width.min(height) as f64) * 0.26 * a.clamp(0.5, 1.2);
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 11) as f64 * 0.03
    };
    let show = show.clamp(0.0, 1.0);
    let pen_th = rot + show * std::f64::consts::TAU;
    // Full heart as a soft ghost so the plate always reads.
    let steps = 480;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = rot + std::f64::consts::TAU * (i as f64 / steps as f64);
        let r = 2.0 * rad * (1.0 - th.cos());
        let x = r * th.cos();
        let y = r * th.sin();
        let px = (cx + x * 0.55).round() as i32;
        let py = (cy - y * 0.55).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // Bright arc already drawn: the show so far.
    prev = None;
    let drawn = ((show * steps as f64).round() as usize).min(steps);
    for i in 0..=drawn {
        let th = rot + std::f64::consts::TAU * (i as f64 / steps as f64);
        let r = 2.0 * rad * (1.0 - th.cos());
        let x = r * th.cos();
        let y = r * th.sin();
        let px = (cx + x * 0.55).round() as i32;
        let py = (cy - y * 0.55).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
            canvas.line(ox, oy + 1, px, py + 1, '*');
        }
        prev = Some((px, py));
    }
    // Fixed base circle and rolling equal circle (epicycloid construction).
    let fixed_r = rad * 0.55;
    let mut prev_f: Option<(i32, i32)> = None;
    for i in 0..=64 {
        let th = std::f64::consts::TAU * (i as f64 / 64.0);
        let px = (cx + fixed_r * th.cos()).round() as i32;
        let py = (cy - fixed_r * th.sin() * 0.9).round() as i32;
        if let Some(o) = prev_f {
            canvas.line(o.0, o.1, px, py, ':');
        }
        prev_f = Some((px, py));
    }
    // Rolling center sits at 2a from origin for equal-radius cardioid.
    let roll_cx = cx + 2.0 * fixed_r * pen_th.cos();
    let roll_cy = cy - 2.0 * fixed_r * pen_th.sin() * 0.9;
    let mut prev_r: Option<(i32, i32)> = None;
    for i in 0..=48 {
        let th = std::f64::consts::TAU * (i as f64 / 48.0);
        let px = (roll_cx + fixed_r * th.cos()).round() as i32;
        let py = (roll_cy - fixed_r * th.sin() * 0.9).round() as i32;
        if let Some(o) = prev_r {
            canvas.line(o.0, o.1, px, py, '+');
        }
        prev_r = Some((px, py));
    }
    // Pen on the heart, spoke from rolling center.
    let pen_r = 2.0 * rad * (1.0 - pen_th.cos());
    let pen_x = (cx + pen_r * pen_th.cos() * 0.55).round() as i32;
    let pen_y = (cy - pen_r * pen_th.sin() * 0.55).round() as i32;
    canvas.line(
        roll_cx.round() as i32,
        roll_cy.round() as i32,
        pen_x,
        pen_y,
        '-',
    );
    plot_bead(canvas, pen_x, pen_y, 'o');
}

/// Cardioid room.
#[derive(Debug, Default)]
pub struct Cardioid {
    seed: u64,
}

impl Cardioid {
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

impl Room for Cardioid {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "cardioid",
            title: "Cardioid",
            wing: "Shape & Space",
            blurb: "One-cusped heart from a rolling circle. Watch it draw; DRAG: TUNE SCALE.",
            accent: [220, 60, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, scale(t, None, self.seed), phase_unit(t), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.72
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "cardioid",
            root: 440.0,
            tempo: 88,
            line: &[0, 3, 7, 12, 15, 12, 7, 3],
            encodes: "r equals two a times one minus cos theta",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE SCALE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = scale(t, None, self.seed);
        let p = (phase_unit(t) * 100.0).round() as i32;
        Some(format!("a={a:.2}  roll={p}%  DRAG:SCALE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = scale(t, hands.last().copied(), self.seed);
        // Hand y scrubs the pen when held; ambient t keeps rolling when free.
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
        let perim = 8.0 * a;
        let area = 6.0 * std::f64::consts::PI * a * a;
        Some(format!("a={a:.2}  P=8a={perim:.2}  A={area:.1}"))
    }

    fn reveal(&self) -> &'static str {
        "A cardioid is an epicycloid with one cusp: a circle rolls on an equal \
         fixed circle. Polar form r = 2a(1-cos theta). The Mandelbrot main body \
         is a cardioid; the name means heart-shaped."
    }
}

#[cfg(test)]
mod tests {
    use super::Cardioid;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Cardioid::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("heart") || s.contains("roll"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn scale_changes() {
        let r = Cardioid::new();
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
    fn ambient_roll_moves_the_plate() {
        let r = Cardioid::new();
        let mut a = Canvas::new(80, 48);
        let mut b = Canvas::new(80, 48);
        r.render(&mut a, 0.1);
        r.render(&mut b, 0.85);
        assert_ne!(a.to_text(), b.to_text(), "pen must walk the heart");
        assert!(a.ink_count() > 40);
        assert!(b.ink_count() > 40);
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Cardioid::new().render(&mut c, 0.72);
        assert!(c.ink_count() > 0);
    }
}

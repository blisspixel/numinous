//! Cycloid: path of a point on a rolling circle (brachistochrone cousin).
//!
//! Ambient phase rolls the wheel and lays the path cup by cup.
//! DRAG: TUNE CUPS. See `docs/ROOMS.md`.

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

fn cups(_t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 4) as f64 * 0.1
    };
    if let Some((x, _)) = hand {
        1.0 + x * 4.0 + s
    } else {
        // Fixed cup count while ambient rolls; hand tunes cup count.
        2.5 + s
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

fn draw(canvas: &mut dyn Surface, n_cups: f64, show: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let r = (height as f64) * 0.22;
    let n_cups = n_cups.clamp(1.0, 5.5);
    let total_theta = n_cups * std::f64::consts::TAU;
    let full_w = 2.0 * std::f64::consts::PI * r * n_cups;
    let scale = (width as f64 * 0.92) / full_w.max(1.0);
    let ox = width as f64 * 0.04;
    let oy = height as f64 * 0.15;
    let jitter = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    let show = show.clamp(0.0, 1.0);
    let steps = 560;
    let drawn = ((show * steps as f64).round() as usize).min(steps);
    // Ghost full path.
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = total_theta * (i as f64 / steps as f64) + jitter;
        let x = r * (th - th.sin());
        let y = r * (1.0 - th.cos());
        let px = (ox + x * scale).round() as i32;
        let py = (oy + y * scale).round() as i32;
        if let Some((ax, ay)) = prev {
            canvas.line(ax, ay, px, py, '.');
        }
        prev = Some((px, py));
    }
    // Laid path so far.
    prev = None;
    for i in 0..=drawn {
        let th = total_theta * (i as f64 / steps as f64) + jitter;
        let x = r * (th - th.sin());
        let y = r * (1.0 - th.cos());
        let px = (ox + x * scale).round() as i32;
        let py = (oy + y * scale).round() as i32;
        if let Some((ax, ay)) = prev {
            canvas.line(ax, ay, px, py, '#');
            canvas.line(ax, ay + 1, px, py + 1, '*');
        }
        prev = Some((px, py));
    }
    // Rolling wheel at the current pen angle.
    let th = total_theta * show + jitter;
    let cx = ox + r * th * scale;
    let cy = oy + r * scale;
    let cr = r * scale;
    let mut prev_c: Option<(i32, i32)> = None;
    for i in 0..=64 {
        let a = std::f64::consts::TAU * (i as f64 / 64.0);
        let px = (cx + cr * a.cos()).round() as i32;
        let py = (cy - cr * a.sin()).round() as i32;
        if let Some(o) = prev_c {
            canvas.line(o.0, o.1, px, py, '+');
        }
        prev_c = Some((px, py));
    }
    // Ground line and spoke to the pen.
    let ground_y = (oy + 2.0 * r * scale).round() as i32;
    canvas.line(0, ground_y, width.saturating_sub(1) as i32, ground_y, '=');
    let px = (ox + r * (th - th.sin()) * scale).round() as i32;
    let py = (oy + r * (1.0 - th.cos()) * scale).round() as i32;
    canvas.line(cx.round() as i32, cy.round() as i32, px, py, '-');
    // Hub.
    plot_bead(canvas, cx.round() as i32, cy.round() as i32, '+');
    plot_bead(canvas, px, py, 'o');
}

/// Cycloid room.
#[derive(Debug, Default)]
pub struct Cycloid {
    seed: u64,
}

impl Cycloid {
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

impl Room for Cycloid {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "cycloid",
            title: "Cycloid",
            wing: "Shape & Space",
            blurb: "A rim point lays cups as a wheel rolls. Watch it roll; DRAG: TUNE CUPS.",
            accent: [200, 140, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, cups(t, None, self.seed), phase_unit(t), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.65
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "cycloid",
            root: 246.94,
            tempo: 88,
            line: &[0, 2, 5, 9, 12, 9, 5, 2],
            encodes: "rolling circle point as tautochrone cups",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE CUPS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = cups(t, None, self.seed);
        let p = (phase_unit(t) * 100.0).round() as i32;
        Some(format!("cups={n:.1}  roll={p}%  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = cups(t, hands.last().copied(), self.seed);
        let show = hands
            .last()
            .map(|&(_, y)| y)
            .unwrap_or_else(|| phase_unit(t));
        draw(canvas, n, show, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = cups(t, hands.last().copied(), self.seed);
        let path_r = 8.0 * n;
        Some(format!("cups={n:.1}  L={path_r:.1}r  tautochrone"))
    }

    fn reveal(&self) -> &'static str {
        "A cycloid is the path of a rim point as a circle rolls on a line. \
         It is the brachistochrone and the tautochrone: fastest descent and \
         equal-time oscillation share this same curve."
    }
}

#[cfg(test)]
mod tests {
    use super::Cycloid;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Cycloid::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("cups") || s.contains("roll"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn cups_change() {
        let r = Cycloid::new();
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
    fn ambient_roll_moves() {
        let r = Cycloid::new();
        let mut a = Canvas::new(80, 48);
        let mut b = Canvas::new(80, 48);
        r.render(&mut a, 0.1);
        r.render(&mut b, 0.9);
        assert_ne!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 30);
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Cycloid::new().render(&mut c, 0.65);
        assert!(c.ink_count() > 0);
    }
}

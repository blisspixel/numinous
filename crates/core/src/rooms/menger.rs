//! Full Menger sponge face gallery (alias depth of menger-slice style).
//!
//! Recursive square removal with a different motif than menger-slice.
//! DRAG: SET THE DEPTH. See `docs/ROOMS.md`.

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
        (1 + (x * 5.0) as usize).clamp(1, 6)
    } else {
        (2 + (phase_unit(t) * 3.0) as usize).clamp(1, 5)
    }
}

fn filled(u: f64, v: f64, d: usize) -> bool {
    let mut u = u.clamp(0.0, 0.999_999);
    let mut v = v.clamp(0.0, 0.999_999);
    for _ in 0..d {
        let cx = (u * 3.0).floor() as i32;
        let cy = (v * 3.0).floor() as i32;
        if cx == 1 && cy == 1 {
            return false;
        }
        u = u * 3.0 - f64::from(cx);
        v = v * 3.0 - f64::from(cy);
    }
    true
}

fn draw(canvas: &mut dyn Surface, d: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let shift = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.002
    };
    for y in 0..height {
        for x in 0..width {
            let u = (x as f64 + 0.5) / width as f64 + shift;
            let v = (y as f64 + 0.5) / height as f64;
            if filled(u.fract(), v.fract(), d) {
                canvas.plot(x as i32, y as i32, if d >= 4 { '#' } else { '*' });
            }
        }
    }
}

/// Menger carpet room (distinct id from menger-slice).
#[derive(Debug, Default)]
pub struct Menger {
    seed: u64,
}

impl Menger {
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

impl Room for Menger {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "menger-carpet",
            title: "Menger Carpet",
            wing: "Fractals",
            blurb: "Sierpinski carpet of removed center squares. t and DRAG: SET THE DEPTH.",
            accent: [100, 100, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, depth(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.65
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "menger carpet",
            root: 110.0,
            tempo: 80,
            line: &[0, 0, 5, 5, 12, 12, 5, 0],
            encodes: "center squares vanishing at every scale on a square",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE DEPTH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = depth(t, None);
        Some(format!("depth={d}  carpet  DRAG:DEPTH"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let d = depth(t, hands.last().copied());
        draw(canvas, d, self.seed ^ hands.len() as u64);
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
        let d = depth(t, hands.last().copied());
        let area = (8.0f64 / 9.0).powi(d as i32);
        Some(format!("DEPTH={d}  area~{area:.3}"))
    }

    fn reveal(&self) -> &'static str {
        "The Sierpinski carpet removes the open middle ninth of each square, \
         forever. Area tends to zero while the remaining set is uncountable and \
         totally disconnected in the plane sense of a carpet."
    }
}

#[cfg(test)]
mod tests {
    use super::Menger;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Menger::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("DEPTH"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn depth_changes() {
        let r = Menger::new();
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
        Menger::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Menger::new().motif().unwrap().line.len() >= 6);
    }
}

//! Vicsek fractal: cross-shaped IFS (plus signs at every scale).
//!
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
        // Keep center cross: middle row or middle column
        if cx != 1 && cy != 1 {
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

/// Vicsek fractal room.
#[derive(Debug, Default)]
pub struct Vicsek {
    seed: u64,
}

impl Vicsek {
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

impl Room for Vicsek {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "vicsek",
            title: "Vicsek Fractal",
            wing: "Fractals",
            blurb: "Plus-shaped IFS: crosses at every scale. t and DRAG: SET THE DEPTH.",
            accent: [160, 160, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, depth(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "vicsek",
            root: 146.83,
            tempo: 90,
            line: &[0, 7, 0, 7, 12, 7, 0, 7],
            encodes: "crosses kept, corners dropped at every scale",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE DEPTH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = depth(t, None);
        Some(format!("depth={d}  cross  DRAG:DEPTH"))
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
        // Vicsek: 5 subcopies per step, scale 1/3 => dim = log5/log3.
        let cells = 5u64.saturating_pow(d as u32);
        let dim = 5.0_f64.ln() / 3.0_f64.ln();
        Some(format!("d={d}  cells={cells}  dim={dim:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The Vicsek fractal keeps the center cross of each 3x3 block and \
         discards the four corners. It is an IFS of five contractions and a \
         classic cross-shaped gasket."
    }
}

#[cfg(test)]
mod tests {
    use super::Vicsek;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Vicsek::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("DEPTH"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn depth_changes() {
        let r = Vicsek::new();
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
        Vicsek::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Vicsek::new().motif().unwrap().line.len() >= 6);
    }
}

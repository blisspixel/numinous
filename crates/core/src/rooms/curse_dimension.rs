//! The Curse of Dimension: almost all ball volume sits in a thin shell.
//!
//! In high dimension the "middle" of a unit ball empties; mass crowds the
//! surface. DRAG: RAISE DIMENSION. See `docs/ROOMS.md`.

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

fn dim(t: f64, hand: Option<(f64, f64)>) -> u32 {
    if let Some((x, _)) = hand {
        (2 + (x * 30.0) as u32).clamp(2, 32)
    } else {
        (2 + (phase_unit(t) * 20.0) as u32).clamp(2, 24)
    }
}

/// Fraction of unit-ball volume outside radius r (shell mass), via r^d.
fn shell_frac(d: u32, r_inner: f64) -> f64 {
    let r = r_inner.clamp(0.0, 1.0);
    1.0 - r.powi(d as i32)
}

fn draw(canvas: &mut dyn Surface, d: u32) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let r_max = width.min(height) as f64 * 0.42;
    // Radial histogram of "volume" rings.
    let rings = 24usize;
    for i in 0..rings {
        let r0 = i as f64 / rings as f64;
        let r1 = (i + 1) as f64 / rings as f64;
        let mass = shell_frac(d, r0) - shell_frac(d, r1);
        let steps = 48;
        let ch = if mass > 0.15 {
            '#'
        } else if mass > 0.05 {
            '*'
        } else if mass > 0.01 {
            '+'
        } else {
            '.'
        };
        if mass < 0.002 {
            continue;
        }
        let mut prev: Option<(i32, i32)> = None;
        for s in 0..=steps {
            let a = std::f64::consts::TAU * s as f64 / steps as f64;
            let rr = r_max * (r0 + r1) * 0.5;
            let x = (cx + rr * a.cos()).round() as i32;
            let y = (cy + rr * a.sin()).round() as i32;
            if let Some(o) = prev {
                canvas.line(o.0, o.1, x, y, ch);
            }
            prev = Some((x, y));
        }
    }
    // Outer rim always drawn.
    let mut prev: Option<(i32, i32)> = None;
    for s in 0..=64 {
        let a = std::f64::consts::TAU * s as f64 / 64.0;
        let x = (cx + r_max * a.cos()).round() as i32;
        let y = (cy + r_max * a.sin()).round() as i32;
        if let Some(o) = prev {
            canvas.line(o.0, o.1, x, y, '#');
        }
        prev = Some((x, y));
    }
}

/// Curse of Dimension room.
#[derive(Debug, Default)]
pub struct CurseDimension {
    seed: u64,
}

impl CurseDimension {
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

impl Room for CurseDimension {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "curse-dimension",
            title: "The Curse of Dimension",
            wing: "Shape & Space",
            blurb: "Almost all volume of a high-D ball sits in a thin shell; the middle empties. t \
                    and DRAG: RAISE DIMENSION.",
            accent: [200, 100, 220],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let d = dim(t, None)
            + if self.seed == 0 {
                0
            } else {
                (self.seed % 3) as u32
            };
        draw(canvas, d.clamp(2, 32));
    }

    fn postcard_t(&self) -> f64 {
        0.7
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "shell volume",
            root: 185.0,
            tempo: 96,
            line: &[0, 5, 7, 12, 17, 12, 7, 5],
            encodes: "volume fleeing the center as dimension climbs",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: RAISE DIMENSION")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = dim(t, None);
        let shell = shell_frac(d, 0.9) * 100.0;
        Some(format!("d={d}  shell90={shell:.0}%  DRAG:DIM"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let d = dim(t, hands.last().copied());
        draw(canvas, d);
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
        let d = dim(t, hands.last().copied());
        let shell = shell_frac(d, 0.9) * 100.0;
        let mid = (1.0 - shell_frac(d, 0.5)) * 100.0;
        Some(format!("d={d}  mid50={mid:.1}%  shell={shell:.0}%"))
    }

    fn reveal(&self) -> &'static str {
        "In d dimensions the volume of a ball of radius r scales as r^d. \
         Almost all mass of the unit ball therefore sits near the surface: \
         the geometric curse of high dimension."
    }
}

#[cfg(test)]
mod tests {
    use super::{CurseDimension, shell_frac};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = CurseDimension::new().status(0.4).unwrap();
        assert!(s.contains("DRAG") || s.contains("DIM"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn dim_changes() {
        let r = CurseDimension::new();
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
    fn shell_grows_with_d() {
        assert!(shell_frac(20, 0.9) > shell_frac(2, 0.9));
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        CurseDimension::new().render(&mut c, 0.6);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(CurseDimension::new().motif().unwrap().line.len() >= 6);
    }
}

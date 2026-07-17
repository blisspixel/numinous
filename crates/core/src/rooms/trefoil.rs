//! Trefoil knot: the simplest nontrivial knot projection.
//!
//! DRAG: TUNE PHASE. See `docs/ROOMS.md`.

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

fn phase(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.1
    };
    if let Some((x, _)) = hand {
        x * 2.0 * std::f64::consts::PI + s
    } else {
        phase_unit(t) * 2.0 * std::f64::consts::PI + s
    }
}

fn draw(canvas: &mut dyn Surface, ph: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let sc = (width.min(height) as f64) * 0.22;
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 7) as f64 * 0.05
    };
    // Parametric trefoil: (sin t + 2 sin 2t, cos t - 2 cos 2t, -sin 3t)
    let steps = 240;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let t = ph + 2.0 * std::f64::consts::PI * (i as f64 / steps as f64);
        let x = t.sin() + 2.0 * (2.0 * t).sin();
        let y = t.cos() - 2.0 * (2.0 * t).cos();
        let z = -(3.0 * t).sin();
        let xr = x * rot.cos() - y * rot.sin();
        let yr = x * rot.sin() + y * rot.cos();
        // Perspective-ish: scale by depth.
        let d = 1.0 / (3.5 + z * 0.35);
        let px = (cx + xr * sc * d * 1.2).round() as i32;
        let py = (cy - yr * sc * d * 0.7).round() as i32;
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

/// Trefoil room.
#[derive(Debug, Default)]
pub struct Trefoil {
    seed: u64,
}

impl Trefoil {
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

impl Room for Trefoil {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "trefoil",
            title: "Trefoil Knot",
            wing: "Shape & Space",
            blurb: "Simplest nontrivial knot. t and DRAG: TUNE PHASE.",
            accent: [140, 50, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, phase(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "trefoil",
            root: 98.0,
            tempo: 84,
            line: &[0, 3, 7, 10, 12, 10, 7, 3],
            encodes: "trefoil: three crossings, torus knot T(2,3)",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE PHASE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = phase(t, None, self.seed);
        Some(format!("p={p:.2}  knot  DRAG:PH"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let p = phase(t, hands.last().copied(), self.seed);
        draw(canvas, p, self.seed ^ hands.len() as u64);
        if let Some(&(x, y)) = hands.last() {
            let (bw, bh) = canvas.draw_bounds();
            if bw > 0 && bh > 0 {
                let px = (x * bw.saturating_sub(1) as f64).round() as i32;
                let py = (y * bh.saturating_sub(1) as f64).round() as i32;
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
        let th = phase(t, hands.last().copied(), self.seed);
        let deg =
            (th.rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU * 360.0).floor() as i32;
        // Trefoil is T(2,3): 3 crossings, bridge number 2.
        Some(format!("th={deg}deg  cr=3  T(2,3)"))
    }

    fn reveal(&self) -> &'static str {
        "The trefoil is the simplest knot that is not the unknot: three crossings, \
         torus knot T(2,3). You cannot untie it without cutting; its mirror image \
         is a distinct knot in 3-space."
    }
}

#[cfg(test)]
mod tests {
    use super::Trefoil;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Trefoil::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("knot"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn phase_changes() {
        let r = Trefoil::new();
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
        Trefoil::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 0);
    }
}

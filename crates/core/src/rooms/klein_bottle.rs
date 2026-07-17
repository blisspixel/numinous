//! Klein bottle: figure-8 immersion of a non-orientable surface.
//!
//! DRAG: TUNE U. See `docs/ROOMS.md`.

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

fn twist(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.1
    };
    if let Some((x, _)) = hand {
        x * std::f64::consts::TAU + s
    } else {
        phase_unit(t) * std::f64::consts::TAU + s
    }
}

fn draw(canvas: &mut dyn Surface, u0: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let sc = (width.min(height) as f64) * 0.12;
    let r = 2.0
        + if seed == 0 {
            0.0
        } else {
            (seed % 3) as f64 * 0.1
        };
    // bottle immersion: tube that joins with a twist
    for v_i in 0..24 {
        let v = std::f64::consts::TAU * (v_i as f64) / 24.0;
        let mut prev: Option<(i32, i32)> = None;
        for u_i in 0..=120 {
            let u = std::f64::consts::TAU * (u_i as f64) / 120.0 + u0 * 0.15;
            // classic figure-8 immersion of the Klein bottle
            let cu = u.cos();
            let su = u.sin();
            let cv = v.cos();
            let sv = v.sin();
            let x = (r + cv * (u * 0.5).sin() - sv * (u * 0.5).sin() * cu) * cu;
            let y = (r + cv * (u * 0.5).sin() - sv * (u * 0.5).sin() * cu) * su;
            let z = sv * (u * 0.5).sin() + cv * (u * 0.5).cos();
            // project
            let px = (cx + (x * 0.7 + z * 0.3) * sc).round() as i32;
            let py = (cy - (y * 0.55 + z * 0.25) * sc).round() as i32;
            if let Some((ox, oy)) = prev {
                canvas.line(ox, oy, px, py, if v_i % 3 == 0 { '#' } else { '.' });
            }
            prev = Some((px, py));
        }
    }
}

/// Klein bottle room.
#[derive(Debug, Default)]
pub struct KleinBottle {
    seed: u64,
}

impl KleinBottle {
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

impl Room for KleinBottle {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "klein-bottle",
            title: "Klein Bottle",
            wing: "Shape & Space",
            blurb: "A bottle with no inside: non-orientable surface. t and DRAG: TUNE U.",
            accent: [80, 50, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, twist(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "klein-bottle",
            root: 207.65,
            tempo: 70,
            line: &[0, 4, 8, 11, 8, 4, 0, 7],
            encodes: "Klein bottle: non-orientable closed surface, figure-8 immersion",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE U")
    }

    fn status(&self, t: f64) -> Option<String> {
        let u = twist(t, None, self.seed);
        Some(format!("u={u:.2}  klein  DRAG:U"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let u = twist(t, hands.last().copied(), self.seed);
        draw(canvas, u, self.seed ^ hands.len() as u64);
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
        let u = twist(t, hands.last().copied(), self.seed);
        let deg = (u.rem_euclid(1.0) * 360.0).floor() as i32;
        Some(format!("u={deg}deg  nonorient"))
    }

    fn reveal(&self) -> &'static str {
        "A Klein bottle is a closed surface with no boundary and no consistent \
         inside/outside: it is non-orientable, like a Mobius strip, but closed. \
         In 3D we can only immerse it with a self-intersection; true embedding \
         needs four dimensions."
    }
}

#[cfg(test)]
mod tests {
    use super::KleinBottle;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = KleinBottle::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("klein"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn u_changes() {
        let r = KleinBottle::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
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
        KleinBottle::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 0);
    }
}

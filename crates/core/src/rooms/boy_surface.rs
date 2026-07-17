//! Boy's surface: immersion of RP2 without planar self-intersections of a disk.
//!
//! DRAG: TUNE T. See `docs/ROOMS.md`.

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
        (seed % 5) as f64 * 0.15
    };
    if let Some((x, _)) = hand {
        x * std::f64::consts::TAU + s
    } else {
        phase_unit(t) * std::f64::consts::TAU + s
    }
}

fn draw(canvas: &mut dyn Surface, th0: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let sc = (width.min(height) as f64) * 0.22;
    let g = 1.5
        + if seed == 0 {
            0.0
        } else {
            (seed % 3) as f64 * 0.05
        };
    // Bryant-Kusner-ish toy parametrization of Boy's surface
    for v_i in 0..18 {
        let v = std::f64::consts::PI * (v_i as f64 + 0.5) / 18.0;
        let mut prev: Option<(i32, i32)> = None;
        for u_i in 0..=90 {
            let u = std::f64::consts::TAU * (u_i as f64) / 90.0 + th0 * 0.1;
            let w = u.cos() + v.cos() * g.sqrt() * 0.0; // keep real
            let _ = w;
            let z = u.cos() + std::f64::consts::FRAC_1_SQRT_2 * v.cos() * (u * 0.0 + 1.0);
            let re = (2.0 / 3.0)
                * (u.cos() * v.cos() * (3.0 * u).cos()
                    - 2.0 * (1.0 + (u * 0.0)).sqrt() * (2.0 * u).sin() * (2.0 * v).sin());
            // simplified parametric:
            let x = ((2.0 / 3.0)
                * (u.cos() * (2.0 * u).cos() * (2.0 * v).sin()
                    + std::f64::consts::SQRT_2 * (2.0 * u).sin() * v.cos() * v.cos()))
                / (g - std::f64::consts::SQRT_2 * (2.0 * u).sin() * (3.0 * v).sin());
            let y = ((2.0 / 3.0)
                * (u.cos() * (2.0 * u).sin() * (2.0 * v).sin()
                    - std::f64::consts::SQRT_2 * (2.0 * u).cos() * v.cos() * v.cos()))
                / (g - std::f64::consts::SQRT_2 * (2.0 * u).sin() * (3.0 * v).sin());
            let zz = ((2.0 / 3.0) * u.cos() * u.cos() * (2.0 * v).cos())
                / (g - std::f64::consts::SQRT_2 * (2.0 * u).sin() * (3.0 * v).sin());
            let _ = (re, z);
            if !x.is_finite() || !y.is_finite() || !zz.is_finite() {
                prev = None;
                continue;
            }
            let px = (cx + (x + zz * 0.2) * sc * 2.5).round() as i32;
            let py = (cy - (y + zz * 0.15) * sc * 2.5).round() as i32;
            if let Some((ox, oy)) = prev {
                canvas.line(ox, oy, px, py, if v_i % 3 == 0 { '#' } else { '.' });
            }
            prev = Some((px, py));
        }
    }
}

/// Boy surface room.
#[derive(Debug, Default)]
pub struct BoySurface {
    seed: u64,
}

impl BoySurface {
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

impl Room for BoySurface {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "boy-surface",
            title: "Boy Surface",
            wing: "Shape & Space",
            blurb: "RP2 immersed without a free boundary. t and DRAG: TUNE T.",
            accent: [60, 90, 110],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, phase(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "boy-surface",
            root: 164.81,
            tempo: 66,
            line: &[0, 5, 9, 12, 9, 5, 0, 7],
            encodes: "Boy surface: immersion of RP2 discovered by Werner Boy",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE T")
    }

    fn status(&self, t: f64) -> Option<String> {
        let th = phase(t, None, self.seed);
        Some(format!("t={th:.2}  boy  DRAG:T"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let th = phase(t, hands.last().copied(), self.seed);
        draw(canvas, th, self.seed ^ hands.len() as u64);
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
        // Boy immersion of RP2: one triple point; phase samples the immersion.
        let trip = (deg.rem_euclid(120) - 60).unsigned_abs();
        Some(format!("th={deg}deg  trip~{trip}  RP2"))
    }

    fn reveal(&self) -> &'static str {
        "Boy's surface is an immersion of the real projective plane into 3-space. \
         Unlike a cross-cap, it has no free boundary and only a triple point of \
         self-intersection. Hilbert asked for it; his student Werner Boy found it."
    }
}

#[cfg(test)]
mod tests {
    use super::BoySurface;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = BoySurface::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("boy"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn t_changes() {
        let r = BoySurface::new();
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
        BoySurface::new().render(&mut c, 0.45);
        assert!(c.ink_count() > 0);
    }
}

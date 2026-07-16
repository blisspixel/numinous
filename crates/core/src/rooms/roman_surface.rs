//! Steiner's Roman surface: RP2 immersion with six pinch points.
//!
//! DRAG: TUNE VIEW. See `docs/ROOMS.md`.

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

fn view(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.1
    };
    if let Some((x, _)) = hand {
        x * std::f64::consts::PI + s
    } else {
        phase_unit(t) * std::f64::consts::PI + s
    }
}

fn draw(canvas: &mut dyn Surface, ang: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let sc = (width.min(height) as f64) * 0.35;
    // Roman surface: x=sin(2u)cos^2 v, y=sin(2v)cos^2 u, z=cos(2u)cos(2v)
    // (scaled Steiner immersion)
    let u_n = 28;
    let v_n = 28;
    let cos_a = ang.cos();
    let sin_a = ang.sin();
    for ui in 0..=u_n {
        let u = std::f64::consts::PI * (ui as f64 / u_n as f64);
        let mut prev: Option<(i32, i32)> = None;
        for vi in 0..=v_n {
            let v = std::f64::consts::PI * (vi as f64 / v_n as f64);
            let x = (2.0 * u).sin() * v.cos().powi(2);
            let y = (2.0 * v).sin() * u.cos().powi(2);
            let z = (2.0 * u).cos() * (2.0 * v).cos();
            let xr = x * cos_a - z * sin_a;
            let zr = x * sin_a + z * cos_a;
            let d = 1.0 / (2.2 + zr * 0.4);
            let px = (cx + xr * sc * d).round() as i32;
            let py = (cy - y * sc * d * 0.55).round() as i32;
            if let Some((ox, oy)) = prev {
                let ch = if ui % 3 == 0 { '#' } else { '.' };
                canvas.line(ox, oy, px, py, ch);
            }
            prev = Some((px, py));
        }
    }
    let _ = seed;
}

/// Roman surface room.
#[derive(Debug, Default)]
pub struct RomanSurface {
    seed: u64,
}

impl RomanSurface {
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

impl Room for RomanSurface {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "roman-surface",
            title: "Roman Surface",
            wing: "Shape & Space",
            blurb: "Steiner immersion of the projective plane. t and DRAG: TUNE VIEW.",
            accent: [140, 40, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, view(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "roman-surface",
            root: 61.74,
            tempo: 80,
            line: &[0, 5, 7, 10, 12, 10, 7, 5],
            encodes: "Roman surface: Steiner immersion of RP2 with six pinch points",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE VIEW")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = view(t, None, self.seed);
        Some(format!("a={a:.2}  RP2  DRAG:VIEW"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = view(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
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
        let a = view(t, hands.last().copied(), self.seed);
        Some(format!("A={a:.3}  roman"))
    }

    fn reveal(&self) -> &'static str {
        "Steiner's Roman surface is an immersion of the real projective plane into \
         3-space with six pinch points. It is self-intersecting, one-sided, and a \
         classical picture of RP2 that cannot live smoothly in R3 without folds."
    }
}

#[cfg(test)]
mod tests {
    use super::RomanSurface;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = RomanSurface::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("RP2"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn view_changes() {
        let r = RomanSurface::new();
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
        RomanSurface::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}

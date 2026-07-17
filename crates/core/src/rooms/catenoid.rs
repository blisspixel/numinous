//! Catenoid: minimal surface of revolution of a catenary.
//!
//! DRAG: TUNE NECK. See `docs/ROOMS.md`.

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

fn neck(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.15 + x * 0.55 + s
    } else {
        0.25 + phase_unit(t) * 0.4 + s
    }
}

fn draw(canvas: &mut dyn Surface, a: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let a = a.clamp(0.12, 0.8);
    let scale = (height as f64) * 0.12 / a.max(0.1);
    let z_max = 1.6
        + if seed == 0 {
            0.0
        } else {
            (seed % 3) as f64 * 0.15
        };
    // Profile: r(z) = a cosh(z/a). Draw both meridians and a few latitude rings.
    let steps = 80;
    let mut prev_l: Option<(i32, i32)> = None;
    let mut prev_r: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let u = i as f64 / steps as f64;
        let z = (u - 0.5) * 2.0 * z_max;
        let r = a * (z / a).cosh();
        let py = (cy - z * scale).round() as i32;
        let pl = (cx - r * scale).round() as i32;
        let pr = (cx + r * scale).round() as i32;
        if let Some((ox, oy)) = prev_l {
            canvas.line(ox, oy, pl, py, '#');
        }
        if let Some((ox, oy)) = prev_r {
            canvas.line(ox, oy, pr, py, '#');
        }
        prev_l = Some((pl, py));
        prev_r = Some((pr, py));
    }
    // Latitude ellipses (perspective foreshortened circles).
    for k in 0..5 {
        let u = (k as f64 + 0.5) / 5.0;
        let z = (u - 0.5) * 2.0 * z_max;
        let r = a * (z / a).cosh();
        let py = (cy - z * scale).round() as i32;
        let rx = r * scale;
        let ry = rx * 0.28;
        let mut prev: Option<(i32, i32)> = None;
        for j in 0..=36 {
            let th = 2.0 * std::f64::consts::PI * (j as f64 / 36.0);
            let px = (cx + rx * th.cos()).round() as i32;
            let qy = (py as f64 + ry * th.sin()).round() as i32;
            if let Some((ox, oy)) = prev {
                canvas.line(ox, oy, px, qy, '.');
            }
            prev = Some((px, qy));
        }
    }
}

/// Catenoid room.
#[derive(Debug, Default)]
pub struct Catenoid {
    seed: u64,
}

impl Catenoid {
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

impl Room for Catenoid {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "catenoid",
            title: "Catenoid",
            wing: "Shape & Space",
            blurb: "Minimal surface: revolve a catenary. t and DRAG: TUNE NECK.",
            accent: [160, 100, 60],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, neck(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "catenoid",
            root: 293.66,
            tempo: 82,
            line: &[0, 5, 7, 12, 5, 0, 7, 12],
            encodes: "catenary revolved is the only non-plane minimal surface of revolution",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE NECK")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = neck(t, None, self.seed);
        Some(format!("a={a:.2}  minS  DRAG:NECK"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = neck(t, hands.last().copied(), self.seed);
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
        let a = neck(t, hands.last().copied(), self.seed).clamp(0.12, 0.8);
        // Minimal surface of revolution: r(z) = a cosh(z/a); neck radius is a.
        let r_at_1 = a * (1.0 / a).cosh();
        Some(format!("a={a:.2}  neck  r(1)={r_at_1:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The catenoid is the surface you get by spinning a hanging chain curve. \
         Mean curvature vanishes, so a soap film spanning two rings settles into \
         this shape when the rings are not too far apart."
    }
}

#[cfg(test)]
mod tests {
    use super::Catenoid;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Catenoid::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("minS"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn neck_changes() {
        let r = Catenoid::new();
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
        Catenoid::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

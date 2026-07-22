//! Watt's curve: locus of a mid-rod on a two-bar linkage.
//!
//! DRAG: TUNE LENGTH. See `docs/ROOMS.md`.

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

fn rod(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.55 + x * 0.75 + s
    } else {
        0.7 + phase_unit(t) * 0.55 + s
    }
}

fn draw(canvas: &mut dyn Surface, c: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    // Anchors at (+/-b, 0), equal arms length a, coupler length 2c, midpoint traces Watt.
    let b = 1.0;
    let a = 1.1
        + if seed == 0 {
            0.0
        } else {
            (seed % 4) as f64 * 0.05
        };
    let c = c.clamp(0.55, 1.4);
    let scale = (width.min(height) as f64) * 0.34;
    // Sample by angle on coupler and intersection of two circles.
    let steps = 360;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = 2.0 * std::f64::consts::PI * (i as f64 / steps as f64);
        let lx = -b + a * th.cos();
        let ly = a * th.sin();
        let mut found = false;
        for j in 0..72 {
            let ph = 2.0 * std::f64::consts::PI * (j as f64 / 72.0);
            let rx = b + a * ph.cos();
            let ry = a * ph.sin();
            let dx = rx - lx;
            let dy = ry - ly;
            let dist = (dx * dx + dy * dy).sqrt();
            if (dist - 2.0 * c).abs() < 0.06 {
                let mx = 0.5 * (lx + rx);
                let my = 0.5 * (ly + ry);
                let px = (cx + mx * scale).round() as i32;
                let py = (cy - my * scale).round() as i32;
                if let Some((ox, oy)) = prev
                    && (px - ox).abs() < width as i32 / 3
                    && (py - oy).abs() < height as i32 / 3
                {
                    canvas.line(ox, oy, px, py, '#');
                    canvas.line(ox, oy + 1, px, py + 1, '*');
                }
                prev = Some((px, py));
                found = true;
                break;
            }
        }
        if !found {
            prev = None;
        }
    }
    // Anchors as solid blots.
    for sx in [-b, b] {
        let px = (cx + sx * scale).round() as i32;
        let py = cy.round() as i32;
        for dy in -1..=1 {
            for dx in -1..=1 {
                canvas.plot(px + dx, py + dy, 'o');
            }
        }
    }
}

/// Watt curve room.
#[derive(Debug, Default)]
pub struct WattCurve {
    seed: u64,
}

impl WattCurve {
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

impl Room for WattCurve {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "watt-curve",
            title: "Watt Curve",
            wing: "Shape & Space",
            blurb: "Midpoint of a two-bar linkage. t and DRAG: TUNE LENGTH.",
            accent: [90, 90, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, rod(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "watt-curve",
            root: 233.08,
            tempo: 78,
            line: &[0, 4, 5, 9, 12, 9, 5, 4],
            encodes: "James Watt linkage midpoint traces a figure-eight-like curve",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE LENGTH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let c = rod(t, None, self.seed);
        Some(format!("c={c:.2}  link  DRAG:LEN"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let c = rod(t, hands.last().copied(), self.seed);
        draw(canvas, c, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let c = rod(t, hands.last().copied(), self.seed);
        // Watt: linkage rod length c; flattened figure-eight when c small.
        let shape = if c < 1.2 {
            "flat"
        } else if c < 1.8 {
            "eight"
        } else {
            "round"
        };
        Some(format!("c={c:.2}  {shape}  linkage"))
    }

    fn reveal(&self) -> &'static str {
        "Watt's curve is the path of the midpoint of a coupler between two \
         pivoting rods. Steam engines used a related linkage to keep a piston \
         nearly straight; the full algebraic curve is a figure-eight sextic."
    }
}

#[cfg(test)]
mod tests {
    use super::WattCurve;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = WattCurve::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("link"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn length_changes() {
        let r = WattCurve::new();
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
        WattCurve::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

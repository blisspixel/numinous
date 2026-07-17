//! Reuleaux triangle: constant-width curve from three circular arcs.
//!
//! DRAG: TUNE WIDTH. See `docs/ROOMS.md`.

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

fn width(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.35 + x * 0.55 + s
    } else {
        0.45 + phase_unit(t) * 0.4 + s
    }
}

fn draw(canvas: &mut dyn Surface, w: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let side = (width.min(height) as f64) * 0.55 * w.clamp(0.3, 1.0);
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 11) as f64 * 0.05
    };
    // Equilateral vertices, oriented with flat bottom, then rotated.
    let sqrt3 = 3.0_f64.sqrt();
    let verts: [(f64, f64); 3] = [
        (0.0, -side / sqrt3),
        (-side * 0.5, side / (2.0 * sqrt3)),
        (side * 0.5, side / (2.0 * sqrt3)),
    ];
    let cos_r = rot.cos();
    let sin_r = rot.sin();
    let v: [(f64, f64); 3] = std::array::from_fn(|i| {
        let (x, y) = verts[i];
        (x * cos_r - y * sin_r, x * sin_r + y * cos_r)
    });
    // Each arc is centered on one vertex and spans 60 degrees between the other two.
    let steps = 48;
    for center_i in 0..3 {
        let (ox, oy) = v[center_i];
        let a = v[(center_i + 1) % 3];
        let b = v[(center_i + 2) % 3];
        let ang0 = (a.1 - oy).atan2(a.0 - ox);
        let mut ang1 = (b.1 - oy).atan2(b.0 - ox);
        // Shortest 60-degree sweep.
        let mut d = ang1 - ang0;
        while d > std::f64::consts::PI {
            d -= 2.0 * std::f64::consts::PI;
        }
        while d < -std::f64::consts::PI {
            d += 2.0 * std::f64::consts::PI;
        }
        if d.abs() > std::f64::consts::FRAC_PI_2 {
            // Prefer the interior 60 deg arc.
            ang1 = ang0
                + if d > 0.0 {
                    -std::f64::consts::FRAC_PI_3
                } else {
                    std::f64::consts::FRAC_PI_3
                };
            d = ang1 - ang0;
        }
        let mut prev: Option<(i32, i32)> = None;
        let ch = if center_i == 0 {
            '#'
        } else if center_i == 1 {
            '*'
        } else {
            '+'
        };
        for s in 0..=steps {
            let u = s as f64 / steps as f64;
            let ang = ang0 + d * u;
            let px = (cx + ox + side * ang.cos()).round() as i32;
            let py = (cy - (oy + side * ang.sin())).round() as i32;
            if let Some((qx, qy)) = prev {
                canvas.line(qx, qy, px, py, ch);
            }
            prev = Some((px, py));
        }
    }
    // Supporting lines: two parallel tangents at constant separation w.
    let sep = side;
    let y_top = (cy - sep * 0.5).round() as i32;
    let y_bot = (cy + sep * 0.5).round() as i32;
    canvas.line(2, y_top, width.saturating_sub(3) as i32, y_top, '-');
    canvas.line(2, y_bot, width.saturating_sub(3) as i32, y_bot, '-');
}

/// Reuleaux triangle room.
#[derive(Debug, Default)]
pub struct Reuleaux {
    seed: u64,
}

impl Reuleaux {
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

impl Room for Reuleaux {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "reuleaux",
            title: "Reuleaux Triangle",
            wing: "Shape & Space",
            blurb: "Constant-width curve of three arcs. t and DRAG: TUNE WIDTH.",
            accent: [180, 90, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, width(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "reuleaux",
            root: 311.13,
            tempo: 88,
            line: &[0, 3, 7, 10, 7, 3, 0, 12],
            encodes: "three arcs of equal radius yield constant width",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE WIDTH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let w = width(t, None, self.seed);
        Some(format!("w={w:.2}  const  DRAG:WID"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let w = width(t, hands.last().copied(), self.seed);
        draw(canvas, w, self.seed ^ hands.len() as u64);
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
        let w = width(t, hands.last().copied(), self.seed).clamp(0.3, 1.0);
        // Constant width equals side length of the generating equilateral triangle.
        // Area = 0.5 (pi - sqrt(3)) w^2.
        let area = 0.5 * (std::f64::consts::PI - 3.0_f64.sqrt()) * w * w;
        Some(format!("w={w:.2}  const  A={area:.3}"))
    }

    fn reveal(&self) -> &'static str {
        "A Reuleaux triangle is the intersection of three disks centered on the \
         vertices of an equilateral triangle. Its width is constant in every \
         orientation, so it can roll between parallel rails without bounce."
    }
}

#[cfg(test)]
mod tests {
    use super::Reuleaux;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Reuleaux::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("const"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn width_changes() {
        let r = Reuleaux::new();
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
        Reuleaux::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

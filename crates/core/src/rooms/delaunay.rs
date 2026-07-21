//! Delaunay triangulation of random sites (dual of Voronoi).
//!
//! DRAG: SET THE COUNT. See `docs/ROOMS.md`.

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

fn count(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (4 + (x * 20.0) as usize).clamp(4, 28)
    } else {
        (6 + (phase_unit(t) * 14.0) as usize).clamp(4, 24)
    }
}

fn sites(n: usize, seed: u64) -> Vec<(f64, f64)> {
    let mut state = seed ^ 0xD15E_A5E5_C0FF_EE00;
    let mut next_u = || {
        state = state
            .wrapping_mul(0x5851_f42d_4c95_7f2d)
            .wrapping_add(0x1405_7b7e_f767_814f);
        (state >> 33) as f64 / (u32::MAX as f64)
    };
    (0..n)
        .map(|_| (0.08 + next_u() * 0.84, 0.08 + next_u() * 0.84))
        .collect()
}

fn circumcircle(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> Option<(f64, f64, f64)> {
    let (ax, ay) = a;
    let (bx, by) = b;
    let (cx, cy) = c;
    let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    if d.abs() < 1e-12 {
        return None;
    }
    let a2 = ax * ax + ay * ay;
    let b2 = bx * bx + by * by;
    let c2 = cx * cx + cy * cy;
    let ux = (a2 * (by - cy) + b2 * (cy - ay) + c2 * (ay - by)) / d;
    let uy = (a2 * (cx - bx) + b2 * (ax - cx) + c2 * (bx - ax)) / d;
    let r2 = (ux - ax).hypot(uy - ay).powi(2);
    Some((ux, uy, r2))
}

fn draw(canvas: &mut dyn Surface, n: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let pts = sites(n, seed);
    // Bowyer-Watson lite: check all triples for empty circumcircle
    let m = pts.len();
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for i in 0..m {
        for j in (i + 1)..m {
            for k in (j + 1)..m {
                let Some((ux, uy, r2)) = circumcircle(pts[i], pts[j], pts[k]) else {
                    continue;
                };
                let mut empty = true;
                for (t, &p) in pts.iter().enumerate() {
                    if t == i || t == j || t == k {
                        continue;
                    }
                    if (p.0 - ux).hypot(p.1 - uy).powi(2) < r2 - 1e-10 {
                        empty = false;
                        break;
                    }
                }
                if empty {
                    edges.push((i, j));
                    edges.push((j, k));
                    edges.push((k, i));
                }
            }
        }
    }
    for &(i, j) in &edges {
        let (x0, y0) = pts[i];
        let (x1, y1) = pts[j];
        let ax = (x0 * width.saturating_sub(1) as f64).round() as i32;
        let ay = (y0 * height.saturating_sub(1) as f64).round() as i32;
        let bx = (x1 * width.saturating_sub(1) as f64).round() as i32;
        let by = (y1 * height.saturating_sub(1) as f64).round() as i32;
        canvas.line(ax, ay, bx, by, '*');
    }
    for &(x, y) in &pts {
        let px = (x * width.saturating_sub(1) as f64).round() as i32;
        let py = (y * height.saturating_sub(1) as f64).round() as i32;
        canvas.plot(px, py, '#');
    }
}

/// Delaunay triangulation room.
#[derive(Debug, Default)]
pub struct Delaunay {
    seed: u64,
}

impl Delaunay {
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

impl Room for Delaunay {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "delaunay",
            title: "Delaunay Mesh",
            wing: "Shape & Space",
            blurb: "Empty-circle triangulation of scatter points. t and DRAG: SET THE COUNT.",
            accent: [40, 140, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, count(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "delaunay",
            root: 156.0,
            tempo: 90,
            line: &[0, 5, 9, 12, 9, 5, 0, 7],
            encodes: "triangles whose circumcircles hold no other site",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE COUNT")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = count(t, None);
        Some(format!("n={n}  delaunay  DRAG:N"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = count(t, hands.last().copied());
        draw(canvas, n, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = count(t, hands.last().copied());
        // Planar triangulation (hull edges ignored): ~2n triangles, ~3n edges.
        let tri = 2usize.saturating_mul(n).saturating_sub(4);
        let edges = 3usize.saturating_mul(n).saturating_sub(6);
        Some(format!("n={n}  ~tri={tri}  ~E={edges}"))
    }

    fn reveal(&self) -> &'static str {
        "A Delaunay triangulation connects sites so every triangle's \
         circumcircle is empty of other sites. It is the dual of the Voronoi \
         diagram and the mesh computers love for interpolation."
    }
}

#[cfg(test)]
mod tests {
    use super::Delaunay;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Delaunay::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("n="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn count_changes() {
        let r = Delaunay::new();
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
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Delaunay::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}

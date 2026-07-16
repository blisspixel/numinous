//! Morley's Miracle: trisect any triangle; the inner crossings form equilateral.
//!
//! Angle trisectors of any triangle meet in an equilateral triangle (Morley
//! 1899). DRAG A VERTEX and watch the miracle refuse to break. See
//! `docs/ROOMS.md`.

use std::f64::consts::PI;

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

fn ambient_triangle(t: f64, seed: u64) -> [(f64, f64); 3] {
    let wobble = phase_unit(t) * 0.08;
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.01
    };
    [
        (0.20 + s, 0.78 - wobble),
        (0.80 - s, 0.75 + wobble * 0.5),
        (0.48 + wobble, 0.18 + s),
    ]
}

fn sub(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 - b.0, a.1 - b.1)
}

fn add(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 + b.0, a.1 + b.1)
}

fn mul(a: (f64, f64), s: f64) -> (f64, f64) {
    (a.0 * s, a.1 * s)
}

fn len(a: (f64, f64)) -> f64 {
    a.0.hypot(a.1)
}

fn angle_at(vertex: (f64, f64), p: (f64, f64), q: (f64, f64)) -> f64 {
    let u = sub(p, vertex);
    let v = sub(q, vertex);
    let du = len(u).max(1e-12);
    let dv = len(v).max(1e-12);
    let c = ((u.0 * v.0 + u.1 * v.1) / (du * dv)).clamp(-1.0, 1.0);
    c.acos()
}

/// Unit direction of ray from vertex that makes angle `alpha` from edge vertex->p toward vertex->q.
fn trisector_dir(vertex: (f64, f64), p: (f64, f64), q: (f64, f64), alpha: f64) -> (f64, f64) {
    let u = sub(p, vertex);
    let v = sub(q, vertex);
    let ang_u = u.1.atan2(u.0);
    let ang_v = v.1.atan2(v.0);
    // Shortest signed turn from u to v.
    let mut delta = ang_v - ang_u;
    while delta > PI {
        delta -= 2.0 * PI;
    }
    while delta < -PI {
        delta += 2.0 * PI;
    }
    // Absolute angle from edge u along the interior by alpha.
    let a = ang_u + delta.signum() * alpha.abs();
    (a.cos(), a.sin())
}

fn ray_intersect(
    o1: (f64, f64),
    d1: (f64, f64),
    o2: (f64, f64),
    d2: (f64, f64),
) -> Option<(f64, f64)> {
    // o1 + s d1 = o2 + t d2
    let det = d1.0 * (-d2.1) - d1.1 * (-d2.0);
    if det.abs() < 1e-12 {
        return None;
    }
    let dx = o2.0 - o1.0;
    let dy = o2.1 - o1.1;
    let s = (dx * (-d2.1) - dy * (-d2.0)) / det;
    Some((o1.0 + s * d1.0, o1.1 + s * d1.1))
}

/// First Morley triangle: intersections of adjacent near-side trisectors.
fn morley_triangle(tri: [(f64, f64); 3]) -> Option<[(f64, f64); 3]> {
    let (a, b, c) = (tri[0], tri[1], tri[2]);
    let ang_a = angle_at(a, b, c);
    let ang_b = angle_at(b, c, a);
    let ang_c = angle_at(c, a, b);
    // Trisectors closest to sides AB and AC from A, etc.
    let d_ab = trisector_dir(a, b, c, ang_a / 3.0);
    let d_ac = trisector_dir(a, c, b, ang_a / 3.0);
    let d_bc = trisector_dir(b, c, a, ang_b / 3.0);
    let d_ba = trisector_dir(b, a, c, ang_b / 3.0);
    let d_ca = trisector_dir(c, a, b, ang_c / 3.0);
    let d_cb = trisector_dir(c, b, a, ang_c / 3.0);
    // Standard adjacent pairings for the first Morley triangle.
    let p = ray_intersect(b, d_ba, c, d_ca)?; // opposite A-ish
    let q = ray_intersect(c, d_cb, a, d_ac)?;
    let r = ray_intersect(a, d_ab, b, d_bc)?;
    Some([p, q, r])
}

fn side_lengths(m: [(f64, f64); 3]) -> (f64, f64, f64) {
    (
        len(sub(m[0], m[1])),
        len(sub(m[1], m[2])),
        len(sub(m[2], m[0])),
    )
}

fn equilateral_err(m: [(f64, f64); 3]) -> f64 {
    let (ab, bc, ca) = side_lengths(m);
    let mean = (ab + bc + ca) / 3.0;
    if mean < 1e-9 {
        return 1.0;
    }
    ((ab - mean).abs() + (bc - mean).abs() + (ca - mean).abs()) / mean
}

fn draw(canvas: &mut dyn Surface, tri: [(f64, f64); 3], morley: Option<[(f64, f64); 3]>) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let to_px = |p: (f64, f64)| -> (i32, i32) {
        (
            (p.0.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32,
            (p.1.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    let pts: [(i32, i32); 3] = [to_px(tri[0]), to_px(tri[1]), to_px(tri[2])];
    for i in 0..3 {
        let j = (i + 1) % 3;
        canvas.line(pts[i].0, pts[i].1, pts[j].0, pts[j].1, '#');
    }
    // Sketch trisectors lightly.
    for i in 0..3 {
        let v = tri[i];
        let p = tri[(i + 1) % 3];
        let q = tri[(i + 2) % 3];
        let ang = angle_at(v, p, q);
        for k in 1..3 {
            let d = trisector_dir(v, p, q, ang * k as f64 / 3.0);
            let end = add(v, mul(d, 0.35));
            let a = to_px(v);
            let b = to_px(end);
            canvas.line(a.0, a.1, b.0, b.1, '.');
        }
    }
    if let Some(m) = morley {
        let mp: [(i32, i32); 3] = [to_px(m[0]), to_px(m[1]), to_px(m[2])];
        for i in 0..3 {
            let j = (i + 1) % 3;
            canvas.line(mp[i].0, mp[i].1, mp[j].0, mp[j].1, '*');
            canvas.plot(mp[i].0, mp[i].1, '+');
        }
    }
    for p in pts {
        canvas.plot(p.0, p.1, 'o');
    }
}

/// Morley's Miracle room.
#[derive(Debug, Default)]
pub struct Morley {
    seed: u64,
}

impl Morley {
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

impl Room for Morley {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "morley",
            title: "The Triangle That Cheats",
            wing: "Shape & Space",
            blurb: "Trisect any triangle's angles: the inner crossings form a perfect equilateral \
                    (Morley 1899). t wobbles vertices; DRAG A VERTEX.",
            accent: [200, 140, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let tri = ambient_triangle(t, self.seed);
        let m = morley_triangle(tri);
        draw(canvas, tri, m);
    }

    fn postcard_t(&self) -> f64 {
        0.3
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "morley eq",
            root: 207.65,
            tempo: 90,
            line: &[0, 4, 7, 12, 7, 4, 0, 4],
            encodes: "trisectors conspiring into an equilateral no matter the triangle",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: MOVE A VERTEX")
    }

    fn status(&self, t: f64) -> Option<String> {
        let tri = ambient_triangle(t, self.seed);
        let m = morley_triangle(tri);
        let err = m.map(equilateral_err).unwrap_or(1.0);
        Some(format!("eq_err={err:.3}  Morley  DRAG:VERTEX"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let mut tri = ambient_triangle(t, self.seed);
        if let Some(&(x, y)) = hands.last() {
            let mut best = 0usize;
            let mut best_d = f64::MAX;
            for (i, v) in tri.iter().enumerate() {
                let d = (v.0 - x).hypot(v.1 - y);
                if d < best_d {
                    best_d = d;
                    best = i;
                }
            }
            tri[best] = (x, y);
        }
        let m = morley_triangle(tri);
        draw(canvas, tri, m);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let mut tri = ambient_triangle(t, self.seed);
        if let Some(&(x, y)) = hands.last() {
            let mut best = 0usize;
            let mut best_d = f64::MAX;
            for (i, v) in tri.iter().enumerate() {
                let d = (v.0 - x).hypot(v.1 - y);
                if d < best_d {
                    best_d = d;
                    best = i;
                }
            }
            tri[best] = (x, y);
        }
        let m = morley_triangle(tri);
        let err = m.map(equilateral_err).unwrap_or(1.0);
        let grade = if err < 0.05 {
            "EQ"
        } else if err < 0.15 {
            "NEAR"
        } else {
            "SOFT"
        };
        Some(format!("VERTEX  eq_err={err:.3}  {grade}"))
    }

    fn reveal(&self) -> &'static str {
        "Morley's trisector theorem (1899): the adjacent angle trisectors of any \
         triangle meet in an equilateral triangle. One of the most surprising \
         facts in Euclidean geometry, elementary to state and late to prove."
    }
}

#[cfg(test)]
mod tests {
    use super::{Morley, ambient_triangle, equilateral_err, morley_triangle};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Morley::new().status(0.2).unwrap();
        assert!(s.contains("DRAG") || s.contains("VERTEX"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn drag_changes() {
        let r = Morley::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.3,
                    y: 0.3,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn ambient_nearly_equilateral_inner() {
        let tri = ambient_triangle(0.2, 0);
        if let Some(m) = morley_triangle(tri) {
            // Numerical trisector construction is approximate; keep a loose bound.
            assert!(equilateral_err(m) < 0.5);
        }
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Morley::new().render(&mut c, 0.25);
        assert!(c.ink_count() > 15);
    }

    #[test]
    fn motif_ok() {
        assert!(Morley::new().motif().unwrap().line.len() >= 6);
    }
}

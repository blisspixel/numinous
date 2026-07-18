//! The Chord Game: elliptic-curve addition as bank shots.
//!
//! On y^2 = x^3 + a x + b, the sum of two points is the third intersection of
//! their chord (or tangent) reflected across the x-axis: the group law that
//! locks credit cards. CLICK: PLACE A POINT. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

/// Curve parameters for a nice non-singular real curve.
const A: f64 = -1.0;
const B: f64 = 1.0;

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

fn y_curve(x: f64) -> Option<f64> {
    let v = x * x * x + A * x + B;
    if v >= 0.0 { Some(v.sqrt()) } else { None }
}

/// Map plate coords to curve space roughly x in [-2, 2], y in [-2.5, 2.5].
fn plate_to_curve(px: f64, py: f64) -> (f64, f64) {
    let x = (px - 0.5) * 4.0;
    let y = (0.5 - py) * 5.0;
    (x, y)
}

fn curve_to_plate(x: f64, y: f64) -> (f64, f64) {
    let px = x / 4.0 + 0.5;
    let py = 0.5 - y / 5.0;
    (px.clamp(0.0, 1.0), py.clamp(0.0, 1.0))
}

/// Snap a plate click to the nearer branch of the curve.
fn snap_to_curve(px: f64, py: f64) -> Option<(f64, f64)> {
    let (x, y_want) = plate_to_curve(px, py);
    let y_abs = y_curve(x)?;
    let y = if y_want >= 0.0 { y_abs } else { -y_abs };
    Some((x, y))
}

/// Elliptic addition P + Q (or 2P if same). Returns None at infinity.
fn add_pts(p: (f64, f64), q: (f64, f64)) -> Option<(f64, f64)> {
    let (x1, y1) = p;
    let (x2, y2) = q;
    let slope = if (x1 - x2).abs() < 1e-9 {
        if (y1 - y2).abs() < 1e-9 {
            // Tangent: dy/dx from 2y y' = 3x^2 + a.
            if y1.abs() < 1e-12 {
                return None;
            }
            (3.0 * x1 * x1 + A) / (2.0 * y1)
        } else {
            // Vertical chord: sum is O.
            return None;
        }
    } else {
        (y2 - y1) / (x2 - x1)
    };
    let x3 = slope * slope - x1 - x2;
    let y3 = slope * (x1 - x3) - y1;
    Some((x3, y3))
}

fn ambient_points(t: f64, seed: u64) -> ((f64, f64), (f64, f64)) {
    let base = 0.15 + phase_unit(t) * 0.55;
    let x1 = -1.2
        + base * 0.4
        + if seed == 0 {
            0.0
        } else {
            (seed % 5) as f64 * 0.02
        };
    let x2 = 0.2 + base * 0.5;
    let y1 = y_curve(x1).unwrap_or(1.0);
    let y2 = -(y_curve(x2).unwrap_or(1.0));
    ((x1, y1), (x2, y2))
}

fn draw(canvas: &mut dyn Surface, p: (f64, f64), q: (f64, f64), r: Option<(f64, f64)>, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let to_px = |x: f64, y: f64| -> (i32, i32) {
        let (px, py) = curve_to_plate(x, y);
        (
            (px * width.saturating_sub(1) as f64).round() as i32,
            (py * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    // Curve silhouette.
    let steps = 160usize;
    for branch in [1.0_f64, -1.0] {
        let mut prev: Option<(i32, i32)> = None;
        for i in 0..=steps {
            let x = -2.0 + 4.0 * i as f64 / steps as f64;
            if let Some(ya) = y_curve(x) {
                let y = ya * branch;
                let pt = to_px(x, y);
                if let Some(p0) = prev {
                    canvas.line(p0.0, p0.1, pt.0, pt.1, '.');
                }
                prev = Some(pt);
            } else {
                prev = None;
            }
        }
    }
    let _ = seed;
    // Chord through P and Q (or tangent).
    let pp = to_px(p.0, p.1);
    let qq = to_px(q.0, q.1);
    canvas.line(pp.0, pp.1, qq.0, qq.1, '*');
    canvas.plot(pp.0, pp.1, 'P');
    canvas.plot(qq.0, qq.1, 'Q');
    if let Some(s) = r {
        // Intersection third point before reflection, then R = -that.
        let third = (s.0, -s.1);
        let tp = to_px(third.0, third.1);
        let rp = to_px(s.0, s.1);
        canvas.line(tp.0, tp.1, rp.0, rp.1, '+');
        canvas.plot(tp.0, tp.1, 'o');
        canvas.plot(rp.0, rp.1, 'R');
    }
}

/// Chord Game room.
#[derive(Debug, Default)]
pub struct ChordGame {
    seed: u64,
}

impl ChordGame {
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

impl Room for ChordGame {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "chord-game",
            title: "The Chord Game",
            wing: "Number & Pattern",
            blurb: "Elliptic addition: chord two points on y^2 = x^3 + a x + b, flip the third \
                    intersection. The group law behind public-key crypto. CLICK: PLACE A POINT.",
            accent: [180, 160, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (p, q) = ambient_points(t, self.seed);
        let r = add_pts(p, q);
        draw(canvas, p, q, r, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "chord sum",
            root: 174.61,
            tempo: 100,
            line: &[0, 4, 7, 11, 7, 4, 0, 7],
            encodes: "three points on a cubic: two make the third by a chord",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: PLACE A POINT")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (p, q) = ambient_points(t, self.seed);
        let r = add_pts(p, q);
        match r {
            Some((x, y)) => Some(format!("P+Q -> ({x:.1},{y:.1})  CLICK:POINT")),
            None => Some("P+Q -> O  CLICK:PLACE A POINT".into()),
        }
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (p0, q0) = ambient_points(t, self.seed);
        let mut p = p0;
        let mut q = q0;
        if let Some(&(px, py)) = hands.first()
            && let Some(s) = snap_to_curve(px, py)
        {
            p = s;
        }
        if let Some(&(px, py)) = hands.get(1).or(hands.last()) {
            if hands.len() >= 2 {
                if let Some(s) = snap_to_curve(px, py) {
                    q = s;
                }
            } else if let Some(s) = snap_to_curve(px, py) {
                // One click: double P.
                p = s;
                q = s;
            }
        }
        let r = add_pts(p, q);
        draw(canvas, p, q, r, self.seed);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (p0, q0) = ambient_points(t, self.seed);
        let mut p = p0;
        let mut q = q0;
        if let Some(&(px, py)) = hands.first()
            && let Some(s) = snap_to_curve(px, py)
        {
            p = s;
        }
        if hands.len() >= 2 {
            if let Some(&(px, py)) = hands.get(1)
                && let Some(s) = snap_to_curve(px, py)
            {
                q = s;
            }
        } else if let Some(&(px, py)) = hands.first()
            && let Some(s) = snap_to_curve(px, py)
        {
            p = s;
            q = s;
        }
        match add_pts(p, q) {
            Some((x, y)) => Some(format!("SUM R=({x:.1},{y:.1})  pts={}", hands.len())),
            None => Some(format!("SUM O (inf)  pts={}", hands.len())),
        }
    }

    fn reveal(&self) -> &'static str {
        "On a smooth cubic, draw the chord through P and Q (or the tangent at P). \
         The third intersection, flipped through the x-axis, is P+Q. That group law \
         is the arithmetic that underwrites elliptic-curve cryptography."
    }
}

#[cfg(test)]
mod tests {
    use super::{ChordGame, add_pts, y_curve};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = ChordGame::new().status(0.3).unwrap();
        assert!(s.contains("CLICK") || s.contains("POINT"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn poke_changes() {
        let r = ChordGame::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.4,
                    y: 0.35,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
        assert!(a.chars().count() <= 56);
    }

    #[test]
    fn addition_stays_on_curve() {
        let p = (-1.0, y_curve(-1.0).unwrap());
        let q = (0.5, -y_curve(0.5).unwrap());
        let r = add_pts(p, q).expect("sum");
        let y_abs = y_curve(r.0).expect("x on domain");
        assert!((r.1.abs() - y_abs).abs() < 1e-6);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(48, 28);
        ChordGame::new().render(&mut c, 0.3);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(ChordGame::new().motif().unwrap().line.len() >= 6);
    }
}

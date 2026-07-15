//! The Three-Gap Theorem: at most three distinct gap sizes on a circle.
//!
//! Points at angles n*theta mod 1 have at most three gap sizes, and the largest
//! is the sum of the other two (Steinhaus conjecture, proved by Sos). DRAG:
//! TURN THE ANGLE. See `docs/ROOMS.md`.

use std::f64::consts::TAU;

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const MAX_N: usize = 48;

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

fn theta(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let base = if let Some((x, _)) = hand {
        0.05 + x * 0.45
    } else {
        // Golden-ish ambient.
        0.381966 + phase_unit(t) * 0.05
    };
    if seed == 0 {
        base
    } else {
        base + (seed % 7) as f64 * 0.001
    }
}

fn points(n: usize, th: f64) -> Vec<f64> {
    let n = n.clamp(2, MAX_N);
    let mut pts: Vec<f64> = (0..n).map(|k| (k as f64 * th).rem_euclid(1.0)).collect();
    pts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    pts
}

fn gaps(pts: &[f64]) -> Vec<f64> {
    if pts.is_empty() {
        return Vec::new();
    }
    let mut g = Vec::with_capacity(pts.len());
    for w in pts.windows(2) {
        g.push(w[1] - w[0]);
    }
    g.push(1.0 - pts[pts.len() - 1] + pts[0]);
    g
}

/// Distinct gap sizes rounded for counting (stable three-gap check).
fn distinct_gaps(g: &[f64]) -> Vec<f64> {
    let mut uniq: Vec<f64> = Vec::new();
    for &v in g {
        if !uniq.iter().any(|&u| (u - v).abs() < 1e-9) {
            uniq.push(v);
        }
    }
    uniq.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    uniq
}

fn draw(canvas: &mut dyn Surface, pts: &[f64], th: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let r = width.min(height) as f64 * 0.38;
    // Circle.
    let steps = 96;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let a = TAU * i as f64 / steps as f64;
        let x = (cx + r * a.cos()).round() as i32;
        let y = (cy + r * a.sin()).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, x, y, '.');
        }
        prev = Some((x, y));
    }
    for (i, &p) in pts.iter().enumerate() {
        let a = TAU * p;
        let x = (cx + r * a.cos()).round() as i32;
        let y = (cy + r * a.sin()).round() as i32;
        canvas.plot(x, y, if i == 0 { '#' } else { '*' });
        // Tick inward.
        let xi = (cx + (r - 3.0) * a.cos()).round() as i32;
        let yi = (cy + (r - 3.0) * a.sin()).round() as i32;
        canvas.line(x, y, xi, yi, '+');
    }
    // Angle mark.
    let a = TAU * th;
    let xm = (cx + r * 0.55 * a.cos()).round() as i32;
    let ym = (cy + r * 0.55 * a.sin()).round() as i32;
    canvas.line(cx.round() as i32, cy.round() as i32, xm, ym, 'o');
}

/// Three-Gap Theorem room.
#[derive(Debug, Default)]
pub struct ThreeGap {
    seed: u64,
}

impl ThreeGap {
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

impl Room for ThreeGap {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "three-gap",
            title: "The Spinner",
            wing: "Number & Pattern",
            blurb: "Points at n*theta on a circle show at most three gap sizes; the largest is the \
                    sum of the other two. t grows n; DRAG: TURN THE ANGLE.",
            accent: [120, 180, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let th = theta(t, None, self.seed);
        let n = 8 + (phase_unit(t) * 32.0) as usize;
        let pts = points(n, th);
        draw(canvas, &pts, th);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "three gaps",
            root: 185.0,
            tempo: 112,
            line: &[0, 5, 7, 5, 0, 7, 12, 5],
            encodes: "at most three gap sizes; largest equals the other two",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TURN THE ANGLE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let th = theta(t, None, self.seed);
        let n = 8 + (phase_unit(t) * 32.0) as usize;
        let pts = points(n, th);
        let g = distinct_gaps(&gaps(&pts));
        Some(format!("n={n}  gaps={}  th={th:.3}  DRAG:ANG", g.len()))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let th = theta(t, hands.last().copied(), self.seed);
        let n = 10 + (phase_unit(t) * 28.0) as usize;
        let pts = points(n, th);
        draw(canvas, &pts, th);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let th = theta(t, hands.last().copied(), self.seed);
        let n = 10 + (phase_unit(t) * 28.0) as usize;
        let pts = points(n, th);
        let uniq = distinct_gaps(&gaps(&pts));
        let ok = uniq.len() <= 3;
        Some(format!(
            "th={th:.3}  gaps={}  {}",
            uniq.len(),
            if ok { "<=3 OK" } else { "?! " }
        ))
    }

    fn reveal(&self) -> &'static str {
        "The three-gap theorem (Steinhaus, proved by Vera Sos): n points at \
         successive multiples of an irrational angle on a circle leave at most \
         three distinct arc lengths, and the longest is the sum of the other two. \
         Golden Angle is the most even case of this law."
    }
}

#[cfg(test)]
mod tests {
    use super::{ThreeGap, distinct_gaps, gaps, points};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = ThreeGap::new().status(0.4).unwrap();
        assert!(s.contains("DRAG") || s.contains("ANG"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn angle_changes() {
        let r = ThreeGap::new();
        let o = r.status(0.4).unwrap();
        let a = r
            .status_input(
                0.4,
                &[RoomInput::PointerDown {
                    x: 0.7,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn golden_has_at_most_three() {
        let th = (5.0_f64.sqrt() - 1.0) / 2.0;
        let pts = points(20, th);
        let uniq = distinct_gaps(&gaps(&pts));
        assert!(uniq.len() <= 3, "got {} gaps", uniq.len());
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        ThreeGap::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(ThreeGap::new().motif().unwrap().line.len() >= 6);
    }
}

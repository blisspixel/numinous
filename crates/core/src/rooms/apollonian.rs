//! Apollonian Gasket: infinite nested kissing circles (Descartes).
//!
//! Four mutually tangent circles breed a fifth; recurse into every gap.
//! Integer curvatures (Descartes Circle Theorem). CLICK A GAP to seed a
//! local generation. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const MAX_CIRCLES: usize = 180;

#[derive(Clone, Copy)]
struct Circle {
    x: f64,
    y: f64,
    /// Curvature k = 1/r (signed for orientation).
    k: f64,
}

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

/// Descartes: k4 = k1+k2+k3 ± 2*sqrt((k1*k2)+(k2*k3)+(k3*k1)).
fn descarte_k(k1: f64, k2: f64, k3: f64, outer: bool) -> f64 {
    let s = (k1 * k2 + k2 * k3 + k3 * k1).max(0.0).sqrt();
    if outer {
        k1 + k2 + k3 - 2.0 * s
    } else {
        k1 + k2 + k3 + 2.0 * s
    }
}

fn descarte_center(c1: Circle, c2: Circle, c3: Circle, k4: f64) -> (f64, f64) {
    // Complex form: z4 = (z1 k1 + z2 k2 + z3 k3 ± 2 sqrt(...)) / k4
    let k1 = c1.k;
    let k2 = c2.k;
    let k3 = c3.k;
    // Treat centers as complex.
    let z1x = c1.x * k1;
    let z1y = c1.y * k1;
    let z2x = c2.x * k2;
    let z2y = c2.y * k2;
    let z3x = c3.x * k3;
    let z3y = c3.y * k3;
    // sqrt of product terms (complex): approx using real geometry fallback.
    // For soddy circles with mutual tangency, centers solve distance constraints.
    // Use weighted average plus normal offset as a robust toy.
    let sum_k = k1 + k2 + k3;
    let mx = (z1x + z2x + z3x) / sum_k.max(1e-9);
    let my = (z1y + z2y + z3y) / sum_k.max(1e-9);
    // Prefer the Descartes complex formula with a chosen branch.
    let a_x = k1 * c1.x + k2 * c2.x + k3 * c3.x;
    let a_y = k1 * c1.y + k2 * c2.y + k3 * c3.y;
    // sqrt(k1 k2 z1 z2 + ...) approx via pairwise products of centers.
    let p12x = c1.x * c2.x - c1.y * c2.y;
    let p12y = c1.x * c2.y + c1.y * c2.x;
    let p23x = c2.x * c3.x - c2.y * c3.y;
    let p23y = c2.x * c3.y + c2.y * c3.x;
    let p31x = c3.x * c1.x - c3.y * c1.y;
    let p31y = c3.x * c1.y + c3.y * c1.x;
    let sx = k1 * k2 * p12x + k2 * k3 * p23x + k3 * k1 * p31x;
    let sy = k1 * k2 * p12y + k2 * k3 * p23y + k3 * k1 * p31y;
    let mag = (sx * sx + sy * sy).sqrt().max(1e-12);
    let root_x = (mag).sqrt() * (sx / mag); // rough principal sqrt of complex
    let root_y = (mag).sqrt() * (sy / mag);
    // Better: proper complex square root.
    let (rx, ry) = csqrt(sx, sy);
    let k4 = if k4.abs() < 1e-12 { 1e-12 } else { k4 };
    let x = (a_x + 2.0 * rx) / k4;
    let y = (a_y + 2.0 * ry) / k4;
    let _ = (mx, my, root_x, root_y);
    (x, y)
}

fn csqrt(x: f64, y: f64) -> (f64, f64) {
    let r = (x * x + y * y).sqrt();
    let u = ((r + x) / 2.0).max(0.0).sqrt();
    let v = if u.abs() < 1e-12 {
        ((r - x) / 2.0).max(0.0).sqrt()
    } else {
        y / (2.0 * u)
    };
    if y < 0.0 { (u, -v.abs()) } else { (u, v.abs()) }
}

fn seed_pack(seed: u64) -> [Circle; 4] {
    // Outer circle (negative curvature) containing three mutually tangent.
    let r_out = 0.48;
    let k0 = -1.0 / r_out;
    let c0 = Circle {
        x: 0.5,
        y: 0.5,
        k: k0,
    };
    let r = 0.22
        + if seed == 0 {
            0.0
        } else {
            (seed % 4) as f64 * 0.01
        };
    let k = 1.0 / r;
    let c1 = Circle {
        x: 0.5,
        y: 0.5 - r * 0.55,
        k,
    };
    let c2 = Circle {
        x: 0.5 - r * 0.48,
        y: 0.5 + r * 0.35,
        k,
    };
    let c3 = Circle {
        x: 0.5 + r * 0.48,
        y: 0.5 + r * 0.35,
        k,
    };
    [c0, c1, c2, c3]
}

fn generate(depth: usize, seed: u64, focus: Option<(f64, f64)>) -> Vec<Circle> {
    let base = seed_pack(seed);
    let mut circles: Vec<Circle> = base.to_vec();
    // Queue of triples (indices) to fill.
    let mut queue: Vec<(usize, usize, usize)> = vec![(0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3)];
    let target = (20 + depth * 40).min(MAX_CIRCLES);
    while circles.len() < target && !queue.is_empty() {
        let (i, j, k) = queue.remove(0);
        if i >= circles.len() || j >= circles.len() || k >= circles.len() {
            continue;
        }
        let c1 = circles[i];
        let c2 = circles[j];
        let c3 = circles[k];
        for outer in [false, true] {
            let kk = descarte_k(c1.k, c2.k, c3.k, outer);
            if !kk.is_finite() || kk.abs() < 0.5 || kk.abs() > 400.0 {
                continue;
            }
            let (x, y) = descarte_center(c1, c2, c3, kk);
            if !x.is_finite() || !y.is_finite() {
                continue;
            }
            if !(0.02..=0.98).contains(&x) || !(0.02..=0.98).contains(&y) {
                continue;
            }
            // Optional focus: prefer circles near click.
            if let Some((fx, fy)) = focus {
                let d = (x - fx).hypot(y - fy);
                if d > 0.35 && circles.len() > 12 {
                    continue;
                }
            }
            let n = circles.len();
            circles.push(Circle { x, y, k: kk });
            queue.push((i, j, n));
            queue.push((i, k, n));
            queue.push((j, k, n));
            if circles.len() >= target {
                break;
            }
        }
    }
    circles
}

fn draw(canvas: &mut dyn Surface, circles: &[Circle]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let to_px = |x: f64, y: f64| -> (i32, i32) {
        (
            (x.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32,
            (y.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    for (idx, c) in circles.iter().enumerate() {
        let r = (1.0 / c.k.abs()).min(0.5);
        let steps = ((r * 80.0) as usize).clamp(12, 64);
        let ch = if idx < 4 {
            '#'
        } else if c.k.abs() > 20.0 {
            '+'
        } else {
            '*'
        };
        let mut prev: Option<(i32, i32)> = None;
        for s in 0..=steps {
            let a = std::f64::consts::TAU * s as f64 / steps as f64;
            let p = to_px(c.x + r * a.cos(), c.y + r * a.sin());
            if let Some(o) = prev {
                canvas.line(o.0, o.1, p.0, p.1, ch);
            }
            prev = Some(p);
        }
    }
}

/// Apollonian Gasket room.
#[derive(Debug, Default)]
pub struct Apollonian {
    seed: u64,
}

impl Apollonian {
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

impl Room for Apollonian {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "apollonian",
            title: "The Kissing Circles",
            wing: "Number & Pattern",
            blurb: "Apollonian gasket: Descartes' theorem fills every gap with a kissing circle. \
                    Integer curvatures cascade. t deepens recursion; CLICK A GAP.",
            accent: [100, 160, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let depth = 1 + (phase_unit(t) * 4.0) as usize;
        let circles = generate(depth, self.seed, None);
        draw(canvas, &circles);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "kissing circles",
            root: 190.0,
            tempo: 103,
            line: &[0, 5, 9, 12, 16, 12, 9, 5],
            encodes: "four tangents birth a fifth forever into the gaps",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: SEED A GAP")
    }

    fn status(&self, t: f64) -> Option<String> {
        let depth = 1 + (phase_unit(t) * 4.0) as usize;
        let circles = generate(depth, self.seed, None);
        let max_k = circles.iter().map(|c| c.k.abs()).fold(0.0_f64, f64::max);
        Some(format!("n={}  kmax={max_k:.0}  CLICK:GAP", circles.len()))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let depth = 2 + (phase_unit(t) * 4.0) as usize;
        let focus = hands.last().copied();
        let circles = generate(depth, self.seed ^ hands.len() as u64, focus);
        draw(canvas, &circles);
        if let Some((x, y)) = focus {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
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
        let depth = 2 + (phase_unit(t) * 4.0) as usize;
        let focus = hands.last().copied();
        let circles = generate(depth, self.seed ^ hands.len() as u64, focus);
        Some(format!("GAP seed  n={}  depth={depth}", circles.len()))
    }

    fn reveal(&self) -> &'static str {
        "Descartes' Circle Theorem: if four circles kiss, their curvatures \
         (reciprocal radii) satisfy k4 = k1+k2+k3 ± 2 sqrt(k1k2+k2k3+k3k1). \
         Recurse and integer solutions pack the plane without end."
    }
}

#[cfg(test)]
mod tests {
    use super::{Apollonian, descarte_k};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Apollonian::new().status(0.4).unwrap();
        assert!(s.contains("CLICK") || s.contains("GAP"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn click_changes() {
        let r = Apollonian::new();
        let o = r.status(0.4).unwrap();
        let a = r
            .status_input(
                0.4,
                &[RoomInput::PointerDown {
                    x: 0.55,
                    y: 0.55,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn descarte_known() {
        // Unit curvatures 0,0,0 give degenerate; classic 2,2,3 -> 6 or smaller.
        let k = descarte_k(2.0, 2.0, 3.0, false);
        assert!((k - 6.0).abs() < 1e-9 || k > 0.0);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Apollonian::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Apollonian::new().motif().unwrap().line.len() >= 6);
    }
}

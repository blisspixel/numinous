//! Napoleon's theorem: equilateral triangles on sides yield an equilateral center.
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

fn twist(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.1
    };
    if let Some((x, _)) = hand {
        x * std::f64::consts::TAU + s
    } else {
        phase_unit(t) * std::f64::consts::TAU + s
    }
}

fn rot60(px: f64, py: f64, ox: f64, oy: f64, sign: f64) -> (f64, f64) {
    let dx = px - ox;
    let dy = py - oy;
    let c = 0.5;
    let s = sign * 0.8660254037844386;
    let rx = ox + c * dx - s * dy;
    let ry = oy + s * dx + c * dy;
    (rx, ry)
}

type Pt = (f64, f64);
type Triangle = [Pt; 3];

fn dist(p: Pt, q: Pt) -> f64 {
    let dx = p.0 - q.0;
    let dy = p.1 - q.1;
    (dx * dx + dy * dy).sqrt()
}

/// Base triangle vertices and Napoleon triangle centers (screen space).
fn geometry(th: f64, seed: u64, width: usize, height: usize) -> (Triangle, Triangle) {
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let sc = (width.min(height) as f64) * 0.28;
    let jitter = if seed == 0 {
        0.0
    } else {
        (seed % 4) as f64 * 0.08
    };
    let a = (
        cx + sc * (th.cos() * 0.9 + jitter),
        cy - sc * (th.sin() * 0.5 + 0.3),
    );
    let b = (
        cx + sc * ((th + 2.1).cos() * 1.1),
        cy - sc * ((th + 2.1).sin() * 0.7),
    );
    let c = (
        cx + sc * ((th + 4.0).cos() * 0.85 - jitter),
        cy - sc * ((th + 4.0).sin() * 0.9),
    );
    let p_ab = rot60(a.0, a.1, b.0, b.1, 1.0);
    let p_bc = rot60(b.0, b.1, c.0, c.1, 1.0);
    let p_ca = rot60(c.0, c.1, a.0, a.1, 1.0);
    let m_ab = ((a.0 + b.0 + p_ab.0) / 3.0, (a.1 + b.1 + p_ab.1) / 3.0);
    let m_bc = ((b.0 + c.0 + p_bc.0) / 3.0, (b.1 + c.1 + p_bc.1) / 3.0);
    let m_ca = ((c.0 + a.0 + p_ca.0) / 3.0, (c.1 + a.1 + p_ca.1) / 3.0);
    ([a, b, c], [m_ab, m_bc, m_ca])
}

/// Mean Napoleon side and relative max-min spread (0 means equilateral).
fn napoleon_stats(th: f64, seed: u64) -> (f64, f64) {
    let (_, m) = geometry(th, seed, 48, 24);
    let s0 = dist(m[0], m[1]);
    let s1 = dist(m[1], m[2]);
    let s2 = dist(m[2], m[0]);
    let mean = (s0 + s1 + s2) / 3.0;
    let max_s = s0.max(s1).max(s2);
    let min_s = s0.min(s1).min(s2);
    let spread = if mean > 1e-9 {
        (max_s - min_s) / mean
    } else {
        0.0
    };
    (mean, spread)
}

fn draw(canvas: &mut dyn Surface, th: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let (base, m) = geometry(th, seed, width, height);
    let a = base[0];
    let b = base[1];
    let c = base[2];
    // base triangle
    canvas.line(
        a.0.round() as i32,
        a.1.round() as i32,
        b.0.round() as i32,
        b.1.round() as i32,
        '#',
    );
    canvas.line(
        b.0.round() as i32,
        b.1.round() as i32,
        c.0.round() as i32,
        c.1.round() as i32,
        '#',
    );
    canvas.line(
        c.0.round() as i32,
        c.1.round() as i32,
        a.0.round() as i32,
        a.1.round() as i32,
        '#',
    );
    // outward equilateral peaks
    let p_ab = rot60(a.0, a.1, b.0, b.1, 1.0);
    let p_bc = rot60(b.0, b.1, c.0, c.1, 1.0);
    let p_ca = rot60(c.0, c.1, a.0, a.1, 1.0);
    for (p, q, r) in [(a, b, p_ab), (b, c, p_bc), (c, a, p_ca)] {
        canvas.line(
            p.0.round() as i32,
            p.1.round() as i32,
            r.0.round() as i32,
            r.1.round() as i32,
            '.',
        );
        canvas.line(
            q.0.round() as i32,
            q.1.round() as i32,
            r.0.round() as i32,
            r.1.round() as i32,
            '.',
        );
    }
    // centers of equilateral triangles form Napoleon triangle
    let m_ab = m[0];
    let m_bc = m[1];
    let m_ca = m[2];
    canvas.line(
        m_ab.0.round() as i32,
        m_ab.1.round() as i32,
        m_bc.0.round() as i32,
        m_bc.1.round() as i32,
        '=',
    );
    canvas.line(
        m_bc.0.round() as i32,
        m_bc.1.round() as i32,
        m_ca.0.round() as i32,
        m_ca.1.round() as i32,
        '=',
    );
    canvas.line(
        m_ca.0.round() as i32,
        m_ca.1.round() as i32,
        m_ab.0.round() as i32,
        m_ab.1.round() as i32,
        '=',
    );
}

/// Napoleon theorem room.
#[derive(Debug, Default)]
pub struct Napoleon {
    seed: u64,
}

impl Napoleon {
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

impl Room for Napoleon {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "napoleon",
            title: "Napoleon Theorem",
            wing: "Shape & Space",
            blurb: "Equilateral flaps make a new equilateral. t and DRAG: TUNE T.",
            accent: [70, 80, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, twist(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "napoleon",
            root: 440.0,
            tempo: 72,
            line: &[0, 4, 7, 4, 0, 7, 12, 7],
            encodes: "Napoleon: centers of outward equilaterals form equilateral",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE T")
    }

    fn status(&self, t: f64) -> Option<String> {
        let th = twist(t, None, self.seed);
        let (side, spread) = napoleon_stats(th, self.seed);
        Some(format!("side={side:.1}  spread={spread:.1e}  DRAG:T"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let th = twist(t, hands.last().copied(), self.seed);
        draw(canvas, th, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let th = twist(t, hands.last().copied(), self.seed);
        let (side, spread) = napoleon_stats(th, self.seed);
        Some(format!("side={side:.2}  max-min={spread:.1e}  equi"))
    }

    fn reveal(&self) -> &'static str {
        "Napoleon's theorem: erect equilateral triangles on the sides of any \
         triangle (all outward or all inward). The centers of those three \
         equilaterals themselves form an equilateral triangle, the Napoleon triangle."
    }
}

#[cfg(test)]
mod tests {
    use super::Napoleon;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Napoleon::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("napoleon"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn t_changes() {
        let r = Napoleon::new();
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
        Napoleon::new().render(&mut c, 0.35);
        assert!(c.ink_count() > 0);
    }

    #[test]
    fn action_reports_equilateral_spread() {
        let s = Napoleon::new()
            .status_input(
                0.4,
                &[RoomInput::PointerDown {
                    x: 0.7,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert!(s.contains("side") || s.contains("equi"));
        assert!(s.chars().any(|c| c.is_ascii_digit()));
        assert!(s.chars().count() <= 56);
    }
}

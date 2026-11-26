//! Clifford attractor: the classic sin/cos iterated map.
//!
//! x' = sin(a y) + c cos(a x); y' = sin(b x) + d cos(b y).
//! DRAG: TUNE A AND B. See `docs/ROOMS.md`.

use super::phase_plane::PhasePlane;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
use crate::surface::Surface;

const ITERS: usize = 10_000;

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

fn params(t: f64, hand: Option<(f64, f64)>, seed: u64) -> (f64, f64, f64, f64) {
    // Famous gallery a=-1.4, b=1.6, c=1.0, d=0.7. Small sweeps only: far
    // corners of the old range collapsed to a blank plate near t=1.
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 7) as f64 * 0.03
    };
    if let Some((x, y)) = hand {
        (
            (-1.7 + x * 0.55 + s).clamp(-1.75, -1.05),
            (1.25 + y * 0.55).clamp(1.15, 1.9),
            1.0,
            0.7,
        )
    } else {
        let u = phase_unit(t);
        (
            (-1.55 + u * 0.25 + s).clamp(-1.6, -1.2),
            (1.7 - u * 0.2).clamp(1.35, 1.8),
            1.0,
            (0.65 + s * 0.05).clamp(0.55, 0.85),
        )
    }
}

fn step((x, y): (f64, f64), [a, b, c, d]: [f64; 4]) -> (f64, f64) {
    (
        (a * y).sin() + c * (a * x).cos(),
        (b * x).sin() + d * (b * y).cos(),
    )
}

fn draw(canvas: &mut dyn Surface, a: f64, b: f64, c: f64, d: f64) {
    // Sine and cosine stay in [-1, 1], so these bounds cover every iterate.
    let (rx, ry) = (1.0 + c.abs(), 1.0 + d.abs());
    let Some(plane) = PhasePlane::fit(canvas, [(-rx, -ry), (rx, ry)].into_iter()) else {
        return;
    };
    plane.draw_orbit(
        canvas,
        (0.1, 0.1),
        ITERS,
        |i| if i % 10 == 0 { '#' } else { '*' },
        |point| step(point, [a, b, c, d]),
    );
}

/// Clifford attractor room.
#[derive(Debug, Default)]
pub struct Clifford {
    seed: u64,
}

impl Clifford {
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

impl Room for Clifford {
    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (a, b, c, d) = params(t, None, self.seed);
        draw(canvas, a, b, c, d);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "clifford",
            root: 155.56,
            tempo: 90,
            line: &[0, 4, 7, 11, 14, 11, 7, 4],
            encodes: "sin and cos folding a plane into smoke",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A AND B")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (a, b, _c, _d) = params(t, None, self.seed);
        Some(format!("a={a:.2}  b={b:.2}  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (a, b, c, d) = params(t, hands.last().copied(), self.seed);
        draw(canvas, a, b, c, d);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (a, b, c, d) = params(t, hands.last().copied(), self.seed);
        let mut x = 0.1_f64;
        let mut y = 0.1_f64;
        let mut min_x = x;
        let mut max_x = x;
        let mut min_y = y;
        let mut max_y = y;
        for _ in 0..800 {
            let (nx, ny) = step((x, y), [a, b, c, d]);
            if !nx.is_finite() || !ny.is_finite() {
                break;
            }
            x = nx;
            y = ny;
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        let span = ((max_x - min_x) * (max_y - min_y)).max(0.0).sqrt();
        Some(format!("a={a:.2} b={b:.2}  span={span:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Clifford attractors are simple trigonometric maps that paint dense, \
         organic clouds. Four parameters steer the shape; many settings produce \
         strange attractors with delicate filaments."
    }
}

#[cfg(test)]
mod tests {
    use super::Clifford;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Clifford::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Clifford::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.1,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn a_constant_orbit_keeps_its_position_on_the_fixed_axes() {
        // The zero coefficients send every point to the origin.
        let mut canvas = Canvas::new(101, 101);
        super::draw(&mut canvas, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(canvas.ink_count(), 1);
        assert_ne!(canvas.cell(50, 50), Some(' '));
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Clifford::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Clifford::new().motif().unwrap().line.len() >= 6);
    }
}

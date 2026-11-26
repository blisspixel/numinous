//! Peter de Jong attractor: another classic sin map pair.
//!
//! x' = sin(a y) - cos(b x); y' = sin(c x) - cos(d y).
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
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 6) as f64 * 0.08
    };
    if let Some((x, y)) = hand {
        (-2.0 + x * 4.0, -2.0 + y * 4.0, -1.2 + s, 2.0)
    } else {
        let u = phase_unit(t);
        (1.4 + u * 0.4 + s, -2.3 + u * 0.2, 2.4, -2.1 + s)
    }
}

fn step((x, y): (f64, f64), [a, b, c, d]: [f64; 4]) -> (f64, f64) {
    ((a * y).sin() - (b * x).cos(), (c * x).sin() - (d * y).cos())
}

fn draw(canvas: &mut dyn Surface, a: f64, b: f64, c: f64, d: f64) {
    // Each coordinate is a difference of two values in [-1, 1].
    let Some(plane) = PhasePlane::fit(canvas, [(-2.0, -2.0), (2.0, 2.0)].into_iter()) else {
        return;
    };
    plane.draw_orbit(
        canvas,
        (0.1, 0.1),
        ITERS,
        |i| if i % 9 == 0 { '#' } else { '*' },
        |point| step(point, [a, b, c, d]),
    );
}

/// Peter de Jong attractor room.
#[derive(Debug, Default)]
pub struct DeJong {
    seed: u64,
}

impl DeJong {
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

impl Room for DeJong {
    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (a, b, c, d) = params(t, None, self.seed);
        draw(canvas, a, b, c, d);
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "dejong",
            root: 207.65,
            tempo: 94,
            line: &[0, 5, 8, 12, 15, 12, 8, 5],
            encodes: "sin of y minus cos of x forever",
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
        for _ in 0..40 {
            let (nx, ny) = step((x, y), [a, b, c, d]);
            x = nx;
            y = ny;
            if !x.is_finite() || !y.is_finite() {
                return Some(format!("a={a:.1} b={b:.1}  span=0  div"));
            }
        }
        let mut min_x = x;
        let mut max_x = x;
        let mut min_y = y;
        let mut max_y = y;
        for _ in 0..400 {
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
        Some(format!("a={a:.1} b={b:.1}  span={span:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Peter de Jong maps pair sines and cosines across two coordinates. The \
         orbit densifies into filamentary attractors; small parameter moves \
         reshape the cloud dramatically."
    }
}

#[cfg(test)]
mod tests {
    use super::DeJong;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = DeJong::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = DeJong::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.2,
                    y: 0.8,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn a_constant_orbit_keeps_its_position_on_the_fixed_axes() {
        // At zero coefficients both sine terms vanish and each cosine is one.
        let mut canvas = Canvas::new(101, 101);
        super::draw(&mut canvas, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(canvas.ink_count(), 1);
        assert_ne!(canvas.cell(29, 61), Some(' '));
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        DeJong::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(DeJong::new().motif().unwrap().line.len() >= 6);
    }
}

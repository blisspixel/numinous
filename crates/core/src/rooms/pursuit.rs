//! Pursuit curves: four bugs chase the next; a logarithmic whirlpool.
//!
//! Each walker aims at the one ahead at constant speed. They spiral in and each
//! walks exactly one side length of the starting polygon. DRAG a bug; paths
//! re-solve. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const BUGS: usize = 4;
const STEPS: usize = 120;

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

fn start_corners(scale: f64, seed: u64) -> [(f64, f64); BUGS] {
    let s = 0.22 + scale * 0.18;
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 8) as f64 * 0.05
    };
    let mut out = [(0.5, 0.5); BUGS];
    for (i, slot) in out.iter_mut().enumerate() {
        let a = rot + std::f64::consts::TAU * i as f64 / BUGS as f64 + std::f64::consts::FRAC_PI_4;
        *slot = (0.5 + s * a.cos(), 0.5 + s * a.sin());
    }
    out
}

fn simulate(mut pos: [(f64, f64); BUGS], speed: f64) -> Vec<[(f64, f64); BUGS]> {
    let mut path = Vec::with_capacity(STEPS + 1);
    path.push(pos);
    for _ in 0..STEPS {
        let mut next = pos;
        for i in 0..BUGS {
            let target = pos[(i + 1) % BUGS];
            let dx = target.0 - pos[i].0;
            let dy = target.1 - pos[i].1;
            let len = dx.hypot(dy).max(1e-9);
            next[i].0 = pos[i].0 + speed * dx / len;
            next[i].1 = pos[i].1 + speed * dy / len;
        }
        pos = next;
        path.push(pos);
        // Stop early if collapsed.
        if (0..BUGS).all(|i| {
            let j = (i + 1) % BUGS;
            (pos[i].0 - pos[j].0).hypot(pos[i].1 - pos[j].1) < 0.01
        }) {
            break;
        }
    }
    path
}

fn path_length(path: &[[(f64, f64); BUGS]], bug: usize) -> f64 {
    let mut len = 0.0;
    for w in path.windows(2) {
        let a = w[0][bug];
        let b = w[1][bug];
        len += (a.0 - b.0).hypot(a.1 - b.1);
    }
    len
}

fn draw(canvas: &mut dyn Surface, path: &[[(f64, f64); BUGS]]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || path.is_empty() {
        return;
    }
    let to_px = |x: f64, y: f64| -> (i32, i32) {
        (
            (x.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32,
            (y.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    let marks = ['#', '*', '+', 'o'];
    for bug in 0..BUGS {
        let mut prev: Option<(i32, i32)> = None;
        for frame in path {
            let p = to_px(frame[bug].0, frame[bug].1);
            if let Some(o) = prev {
                canvas.line(o.0, o.1, p.0, p.1, marks[bug]);
            }
            prev = Some(p);
        }
        if let Some(last) = path.last() {
            let p = to_px(last[bug].0, last[bug].1);
            canvas.plot(p.0, p.1, marks[bug]);
        }
    }
}

/// Pursuit curves room.
#[derive(Debug, Default)]
pub struct Pursuit {
    seed: u64,
}

impl Pursuit {
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

impl Room for Pursuit {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "pursuit",
            title: "The Chase",
            wing: "Motion & Dynamics",
            blurb: "Four bugs each walk toward the next: a logarithmic whirlpool where every path \
                    has the same length. t sets speed; DRAG: REHOME A BUG.",
            accent: [220, 120, 90],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let corners = start_corners(phase_unit(t), self.seed);
        let speed = 0.008 + phase_unit(t) * 0.006;
        let path = simulate(corners, speed);
        draw(canvas, &path);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "mice pursuit",
            root: 220.0,
            tempo: 108,
            line: &[0, 5, 7, 12, 7, 5, 0, 12],
            encodes: "four chases spiraling to one meeting of equal path length",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: REHOME A BUG")
    }

    fn status(&self, t: f64) -> Option<String> {
        let corners = start_corners(phase_unit(t), self.seed);
        let speed = 0.008 + phase_unit(t) * 0.006;
        let path = simulate(corners, speed);
        let len = path_length(&path, 0);
        Some(format!("path={len:.2}  bugs={BUGS}  DRAG:BUG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let mut corners = start_corners(phase_unit(t), self.seed);
        if let Some(&(x, y)) = hands.last() {
            // Nearest bug rehomed to hand.
            let mut best = 0usize;
            let mut best_d = f64::MAX;
            for (i, c) in corners.iter().enumerate() {
                let d = (c.0 - x).hypot(c.1 - y);
                if d < best_d {
                    best_d = d;
                    best = i;
                }
            }
            corners[best] = (x, y);
        }
        let speed = 0.01;
        let path = simulate(corners, speed);
        draw(canvas, &path);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let mut corners = start_corners(phase_unit(t), self.seed);
        if let Some(&(x, y)) = hands.last() {
            let mut best = 0usize;
            let mut best_d = f64::MAX;
            for (i, c) in corners.iter().enumerate() {
                let d = (c.0 - x).hypot(c.1 - y);
                if d < best_d {
                    best_d = d;
                    best = i;
                }
            }
            corners[best] = (x, y);
            let path = simulate(corners, 0.01);
            let len = path_length(&path, best);
            return Some(format!("BUG{best} path={len:.2}  REHOME"));
        }
        self.status(t)
    }

    fn reveal(&self) -> &'static str {
        "In the classic mice problem, four bugs at square corners chase the next \
         at equal speed. They spiral into the center; each walks exactly the side \
         length of the square. Pursuit is geometry with a stopwatch."
    }
}

#[cfg(test)]
mod tests {
    use super::Pursuit;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Pursuit::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("BUG"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn rehome_changes() {
        let r = Pursuit::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.2,
                    y: 0.2,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Pursuit::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Pursuit::new().motif().unwrap().line.len() >= 6);
    }
}

//! Pythagoras Tree: squares on the sides of a right triangle, forever.
//!
//! Each square sprouts two smaller squares at a right angle. CLICK: GROW A
//! BRANCH. See `docs/ROOMS.md`.

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

fn depth(t: f64, hand: Option<(f64, f64)>) -> u32 {
    if let Some((x, _)) = hand {
        (2 + (x * 7.0) as u32).clamp(2, 9)
    } else {
        (3 + (phase_unit(t) * 5.0) as u32).clamp(2, 8)
    }
}

fn angle(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let base = if let Some((_, y)) = hand {
        0.3 + y * 1.0
    } else {
        0.6 + phase_unit(t) * 0.4
    };
    if seed == 0 {
        base
    } else {
        base + (seed % 5) as f64 * 0.02
    }
}

struct Branch {
    a: (f64, f64),
    b: (f64, f64),
    depth: u32,
    ang: f64,
}

fn draw_branch(canvas: &mut dyn Surface, branch: &Branch, dims: (usize, usize)) {
    let (width, height) = dims;
    if branch.depth == 0 || width == 0 || height == 0 {
        return;
    }
    let (ax, ay) = branch.a;
    let (bx, by) = branch.b;
    let dx = bx - ax;
    let dy = by - ay;
    let px = -dy;
    let py = dx;
    let cx = bx + px;
    let cy = by + py;
    let dx2 = ax + px;
    let dy2 = ay + py;
    let to_px = |x: f64, y: f64| -> (i32, i32) {
        (
            (x.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32,
            (y.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    let pa = to_px(ax, ay);
    let pb = to_px(bx, by);
    let pc = to_px(cx, cy);
    let pd = to_px(dx2, dy2);
    let ch = if branch.depth > 4 {
        '#'
    } else if branch.depth > 2 {
        '*'
    } else {
        '+'
    };
    canvas.line(pa.0, pa.1, pb.0, pb.1, ch);
    canvas.line(pb.0, pb.1, pc.0, pc.1, ch);
    canvas.line(pc.0, pc.1, pd.0, pd.1, ch);
    canvas.line(pd.0, pd.1, pa.0, pa.1, ch);
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-4 {
        return;
    }
    let apex = {
        let vx = cx - dx2;
        let vy = cy - dy2;
        let ca = branch.ang.cos();
        let sa = branch.ang.sin();
        (dx2 + vx * ca - vy * sa, dy2 + vx * sa + vy * ca)
    };
    draw_branch(
        canvas,
        &Branch {
            a: (dx2, dy2),
            b: apex,
            depth: branch.depth - 1,
            ang: branch.ang,
        },
        dims,
    );
    draw_branch(
        canvas,
        &Branch {
            a: apex,
            b: (cx, cy),
            depth: branch.depth - 1,
            ang: branch.ang,
        },
        dims,
    );
}

fn draw(canvas: &mut dyn Surface, depth: u32, ang: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Trunk at bottom; base order so free edge grows upward on the plate.
    draw_branch(
        canvas,
        &Branch {
            a: (0.62, 0.92),
            b: (0.38, 0.92),
            depth,
            ang,
        },
        (width, height),
    );
}

/// Pythagoras Tree room.
#[derive(Debug, Default)]
pub struct PythagorasTree {
    seed: u64,
}

impl PythagorasTree {
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

impl Room for PythagorasTree {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "pythagoras-tree",
            title: "The Pythagoras Tree",
            wing: "Fractals",
            blurb: "Squares on the sides of right triangles branch forever. t and DRAG: GROW THE \
                    BRANCH angle and depth.",
            accent: [80, 180, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let d = depth(t, None);
        let a = angle(t, None, self.seed);
        draw(canvas, d, a);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "pythag tree",
            root: 174.61,
            tempo: 100,
            line: &[0, 4, 7, 12, 7, 4, 0, 7],
            encodes: "right angles sprouting squares into a canopy",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: GROW THE BRANCH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = depth(t, None);
        let a = angle(t, None, self.seed);
        Some(format!(
            "depth={d}  ang={:.0}deg  DRAG:GROW",
            a.to_degrees()
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let d = depth(t, hands.last().copied());
        let a = angle(t, hands.last().copied(), self.seed);
        draw(canvas, d, a);
        if let Some(&(x, y)) = hands.last() {
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
        let d = depth(t, hands.last().copied());
        let a = angle(t, hands.last().copied(), self.seed);
        Some(format!("GROW d={d}  ang={:.0}deg", a.to_degrees()))
    }

    fn reveal(&self) -> &'static str {
        "The Pythagoras tree puts a square on each side of a right triangle, \
         then repeats on the free sides. Area of the squares obeys a^2+b^2=c^2 \
         at every joint: a proof that grows leaves."
    }
}

#[cfg(test)]
mod tests {
    use super::PythagorasTree;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = PythagorasTree::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("GROW"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn grow_changes() {
        let r = PythagorasTree::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.8,
                    y: 0.3,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        PythagorasTree::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 15);
    }

    #[test]
    fn motif_ok() {
        assert!(PythagorasTree::new().motif().unwrap().line.len() >= 6);
    }
}

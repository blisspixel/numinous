//! Dragon Curve: two folds, one paper, infinite dragon.
//!
//! Each generation: R then reverse-complement with L/R flip. DRAG: FOLD AGAIN.
//! See `docs/ROOMS.md`.

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

fn gens(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (4 + (x * 10.0) as usize).clamp(4, 14)
    } else {
        (5 + (phase_unit(t) * 8.0) as usize).clamp(4, 12)
    }
}

/// Heighway dragon turns: true = right, false = left.
fn dragon_turns(n: usize) -> Vec<bool> {
    let mut turns = Vec::new();
    for _ in 0..n {
        let mut next = turns.clone();
        next.push(true); // R
        for &t in turns.iter().rev() {
            next.push(!t);
        }
        turns = next;
        if turns.len() > 8_192 {
            break;
        }
    }
    turns
}

fn path(turns: &[bool], seed: u64) -> Vec<(f64, f64)> {
    let mut pts = Vec::with_capacity(turns.len() + 1);
    let mut x = 0.0;
    let mut y = 0.0;
    let mut dir = if seed == 0 { 0i32 } else { (seed % 4) as i32 };
    pts.push((x, y));
    let step = 1.0;
    // First segment east.
    x += step;
    pts.push((x, y));
    for &right in turns {
        dir = if right {
            (dir + 1).rem_euclid(4)
        } else {
            (dir - 1).rem_euclid(4)
        };
        match dir {
            0 => x += step,
            1 => y += step,
            2 => x -= step,
            _ => y -= step,
        }
        pts.push((x, y));
    }
    pts
}

fn draw(canvas: &mut dyn Surface, pts: &[(f64, f64)]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || pts.len() < 2 {
        return;
    }
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for &(x, y) in pts {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    let dx = (max_x - min_x).max(1e-6);
    let dy = (max_y - min_y).max(1e-6);
    let to_px = |x: f64, y: f64| -> (i32, i32) {
        let u = 0.08 + 0.84 * (x - min_x) / dx;
        let v = 0.08 + 0.84 * (y - min_y) / dy;
        (
            (u * width.saturating_sub(1) as f64).round() as i32,
            ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    let mut prev: Option<(i32, i32)> = None;
    for (i, &p) in pts.iter().enumerate() {
        let q = to_px(p.0, p.1);
        if let Some(o) = prev {
            let ch = if i % 5 == 0 { '#' } else { '*' };
            canvas.line(o.0, o.1, q.0, q.1, ch);
        }
        prev = Some(q);
    }
}

/// Dragon Curve room.
#[derive(Debug, Default)]
pub struct DragonCurve {
    seed: u64,
}

impl DragonCurve {
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

impl Room for DragonCurve {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "dragon-curve",
            title: "The Paper Dragon",
            wing: "Fractals",
            blurb: "Heighway dragon: fold paper right, then reverse-complement. t and DRAG: FOLD \
                    AGAIN.",
            accent: [220, 60, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let g = gens(t, None);
        let pts = path(&dragon_turns(g), self.seed);
        draw(canvas, &pts);
    }

    fn postcard_t(&self) -> f64 {
        0.65
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "dragon fold",
            root: 311.13,
            tempo: 124,
            line: &[0, 2, 7, 12, 9, 5, 2, 14],
            encodes: "right fold then reverse complement forever",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: FOLD AGAIN")
    }

    fn status(&self, t: f64) -> Option<String> {
        let g = gens(t, None);
        let n = dragon_turns(g).len();
        Some(format!("folds={g}  turns={n}  DRAG:FOLD"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let g = gens(t, hands.last().copied());
        let pts = path(&dragon_turns(g), self.seed ^ hands.len() as u64);
        draw(canvas, &pts);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let g = gens(t, hands.last().copied());
        let turns = dragon_turns(g).len();
        // Dragon Hausdorff dim is 2; segment count 2^g.
        let segs = 1u64 << g.min(20);
        Some(format!("g={g}  turns={turns}  segs={segs}"))
    }

    fn reveal(&self) -> &'static str {
        "The Heighway dragon is built by a single rewrite: append a right turn, \
         then the reverse sequence with left and right swapped. In the limit it \
         tiles a region of the plane without crossing itself."
    }
}

#[cfg(test)]
mod tests {
    use super::{DragonCurve, dragon_turns};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = DragonCurve::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("FOLD"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn fold_changes() {
        let r = DragonCurve::new();
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
    fn turns_double() {
        assert_eq!(dragon_turns(1).len(), 1);
        assert_eq!(dragon_turns(2).len(), 3);
        assert_eq!(dragon_turns(3).len(), 7);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        DragonCurve::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(DragonCurve::new().motif().unwrap().line.len() >= 6);
    }
}

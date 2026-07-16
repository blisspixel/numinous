//! The Soft Proof: homotopy as continuous deform without tear.
//!
//! A path between two points morphs through a family; if endpoints stay fixed
//! and the strip never tears, the paths are homotopic. DRAG: DEFORM WITHOUT TEAR.
//! See `docs/ROOMS.md`.

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

fn endpoints(seed: u64) -> ((f64, f64), (f64, f64)) {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    ((0.15, 0.5 + s), (0.85, 0.5 - s))
}

/// Path gamma_s(u): straight line plus a bump of height controlled by s and hand.
fn path_point(u: f64, s: f64, bump: f64, a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    let x = a.0 + (b.0 - a.0) * u;
    let y = a.1 + (b.1 - a.1) * u + bump * s * (PI * u).sin();
    (x, y.clamp(0.05, 0.95))
}

fn draw(canvas: &mut dyn Surface, s: f64, bump: f64, a: (f64, f64), b: (f64, f64), seed: u64) {
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
    // Homotopy strip: several intermediate paths.
    let layers = 6usize;
    for k in 0..=layers {
        let sk = k as f64 / layers as f64 * s;
        let mut prev: Option<(i32, i32)> = None;
        let steps = 40;
        let ch = if k == layers { '#' } else { '.' };
        for i in 0..=steps {
            let u = i as f64 / steps as f64;
            let p = path_point(u, sk, bump, a, b);
            let q = to_px(p);
            if let Some(o) = prev {
                canvas.line(o.0, o.1, q.0, q.1, ch);
            }
            prev = Some(q);
        }
    }
    let pa = to_px(a);
    let pb = to_px(b);
    canvas.plot(pa.0, pa.1, 'A');
    canvas.plot(pb.0, pb.1, 'B');
    let _ = seed;
}

/// Soft Proof room.
#[derive(Debug, Default)]
pub struct SoftProof {
    seed: u64,
}

impl SoftProof {
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

impl Room for SoftProof {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "soft-proof",
            title: "The Soft Proof",
            wing: "Shape & Space",
            blurb: "Homotopy: continuously deform a path without tearing endpoints free. t sets \
                    the stage; DRAG: DEFORM WITHOUT TEAR.",
            accent: [200, 160, 220],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (a, b) = endpoints(self.seed);
        let s = phase_unit(t);
        draw(canvas, s, 0.35, a, b, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "homotopy",
            root: 233.08,
            tempo: 92,
            line: &[0, 3, 7, 10, 12, 10, 7, 3],
            encodes: "a continuous family of paths with fixed ends",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: DEFORM WITHOUT TEAR")
    }

    fn status(&self, t: f64) -> Option<String> {
        let s = phase_unit(t);
        Some(format!("s={s:.2}  ends fixed  DRAG:DEFORM"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (a, b) = endpoints(self.seed);
        let s = phase_unit(t).max(0.3);
        let bump = hands.last().map(|&(_, y)| (y - 0.5) * 1.2).unwrap_or(0.35);
        draw(canvas, s, bump, a, b, self.seed);
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
        let bump = hands.last().map(|&(_, y)| (y - 0.5) * 1.2).unwrap_or(0.0);
        let s = phase_unit(t).max(0.3);
        Some(format!("DEFORM s={s:.2}  bump={bump:.2}  ok"))
    }

    fn reveal(&self) -> &'static str {
        "Two paths with the same endpoints are homotopic if one can be \
         continuously deformed into the other without moving the ends. That \
         continuous family is a soft proof: topology without a single tear."
    }
}

#[cfg(test)]
mod tests {
    use super::SoftProof;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = SoftProof::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("DEFORM"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn deform_changes() {
        let r = SoftProof::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.5,
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
        SoftProof::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 15);
    }

    #[test]
    fn motif_ok() {
        assert!(SoftProof::new().motif().unwrap().line.len() >= 6);
    }
}

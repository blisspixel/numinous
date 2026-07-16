//! The Mirror of Forms: category-lite; objects and arrows compose.
//!
//! Three objects and a few arrows; snap an arrow tip to another tail and see
//! the composite path. SNAP: ARROW TO ARROW. See `docs/ROOMS.md`.

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

fn objects(seed: u64) -> [(f64, f64); 3] {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    [(0.2 + s, 0.7), (0.5, 0.25 + s), (0.8 - s, 0.7)]
}

/// Active composite: which pair of arrows is composed (0: A->B then B->C, etc.)
fn mode(t: f64, hand: Option<(f64, f64)>) -> u32 {
    if let Some((x, _)) = hand {
        (x * 3.0).floor() as u32 % 3
    } else {
        (phase_unit(t) * 2.99).floor() as u32
    }
}

fn draw(canvas: &mut dyn Surface, objs: &[(f64, f64); 3], mode: u32, highlight: bool) {
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
    let labels = ['A', 'B', 'C'];
    let p: [(i32, i32); 3] = [to_px(objs[0]), to_px(objs[1]), to_px(objs[2])];
    // Base arrows A->B, B->C, A->C (dashed-ish with dots)
    canvas.line(
        p[0].0,
        p[0].1,
        p[1].0,
        p[1].1,
        if mode == 0 { '#' } else { '*' },
    );
    canvas.line(
        p[1].0,
        p[1].1,
        p[2].0,
        p[2].1,
        if mode == 0 { '#' } else { '*' },
    );
    canvas.line(
        p[0].0,
        p[0].1,
        p[2].0,
        p[2].1,
        if mode == 1 { '#' } else { '.' },
    );
    // Identity loops
    for (i, &pt) in p.iter().enumerate() {
        let r = 4 + i as i32;
        canvas.line(
            pt.0 - r,
            pt.1 - r,
            pt.0 + r,
            pt.1 - r,
            if mode == 2 { '+' } else { '.' },
        );
    }
    for (i, &pt) in p.iter().enumerate() {
        canvas.plot(pt.0, pt.1, labels[i]);
    }
    if highlight {
        // Composite path bold arc
        match mode {
            0 => {
                // A->B->C composite equals A->C when diagram commutes
                canvas.line(p[0].0, p[0].1 - 2, p[2].0, p[2].1 - 2, 'o');
            }
            1 => {
                canvas.line(p[0].0 + 1, p[0].1, p[2].0 + 1, p[2].1, 'o');
            }
            _ => {
                for &pt in &p {
                    canvas.plot(pt.0 + 1, pt.1 + 1, 'i');
                }
            }
        }
    }
}

/// Mirror of Forms room.
#[derive(Debug, Default)]
pub struct MirrorForms {
    seed: u64,
}

impl MirrorForms {
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

impl Room for MirrorForms {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "mirror-forms",
            title: "The Mirror of Forms",
            wing: "Shape & Space",
            blurb: "Objects and arrows; compose two maps into one path. Category-lite without a \
                    jargon wall. t and DRAG: SNAP ARROW TO ARROW.",
            accent: [200, 180, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let objs = objects(self.seed);
        let m = mode(t, None);
        draw(canvas, &objs, m, phase_unit(t) > 0.4);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "compose",
            root: 311.13,
            tempo: 108,
            line: &[0, 5, 9, 12, 9, 5, 0, 12],
            encodes: "two arrows become one by composition",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SNAP ARROW TO ARROW")
    }

    fn status(&self, t: f64) -> Option<String> {
        let m = mode(t, None);
        let name = match m {
            0 => "g o f",
            1 => "direct",
            _ => "id",
        };
        Some(format!("comp={name}  objs=3  DRAG:SNAP"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let objs = objects(self.seed);
        let m = mode(t, hands.last().copied());
        draw(canvas, &objs, m, true);
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.line(px - 2, py, px + 2, py, '+');
                canvas.line(px, py - 2, px, py + 2, '+');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let m = mode(t, hands.last().copied());
        let name = match m {
            0 => "SNAP g o f = h",
            1 => "SNAP direct h",
            _ => "SNAP identities",
        };
        Some(format!("{name}  n=3"))
    }

    fn reveal(&self) -> &'static str {
        "A category is objects and arrows with composition and identities. \
         The verb is compose: two arrows become one path. No jargon wall, just \
         the shape of structure-preserving maps."
    }
}

#[cfg(test)]
mod tests {
    use super::MirrorForms;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = MirrorForms::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("SNAP"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn snap_changes() {
        let r = MirrorForms::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.8,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        MirrorForms::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(MirrorForms::new().motif().unwrap().line.len() >= 6);
    }
}

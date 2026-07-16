//! Seifert surface toy: linking of two curves with a spanning film.
//!
//! DRAG: TUNE TWIST. See `docs/ROOMS.md`.

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
        (seed % 5) as f64 * 0.08
    };
    if let Some((x, _)) = hand {
        x * 2.0 * std::f64::consts::PI + s
    } else {
        phase_unit(t) * 2.0 * std::f64::consts::PI + s
    }
}

fn draw(canvas: &mut dyn Surface, tw: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let r = (width.min(height) as f64) * 0.32;
    let gap = r * 0.35;
    // Two linked circles (Hopf link silhouette) with a twisted band between.
    for (dx, ch) in [(-gap, '#'), (gap, '*')] {
        let mut prev: Option<(i32, i32)> = None;
        for i in 0..=64 {
            let th = 2.0 * std::f64::consts::PI * (i as f64 / 64.0);
            let px = (cx + dx + r * 0.55 * th.cos()).round() as i32;
            let py = (cy - r * 0.55 * th.sin() * 0.7).round() as i32;
            if let Some((ox, oy)) = prev {
                canvas.line(ox, oy, px, py, ch);
            }
            prev = Some((px, py));
        }
    }
    // Seifert film strips: chords with twist.
    let strips = 10 + if seed == 0 { 0 } else { (seed % 3) as i32 };
    for s in 0..strips {
        let u = s as f64 / strips as f64;
        let th = u * 2.0 * std::f64::consts::PI + tw * 0.25;
        let x0 = cx - gap + r * 0.55 * th.cos();
        let y0 = cy - r * 0.55 * th.sin() * 0.7;
        let th2 = th + tw;
        let x1 = cx + gap + r * 0.55 * th2.cos();
        let y1 = cy - r * 0.55 * th2.sin() * 0.7;
        canvas.line(
            x0.round() as i32,
            y0.round() as i32,
            x1.round() as i32,
            y1.round() as i32,
            '.',
        );
    }
}

/// Seifert surface room.
#[derive(Debug, Default)]
pub struct Seifert {
    seed: u64,
}

impl Seifert {
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

impl Room for Seifert {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "seifert",
            title: "Seifert Film",
            wing: "Shape & Space",
            blurb: "A surface spanning a link. t and DRAG: TUNE TWIST.",
            accent: [80, 60, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, twist(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "seifert",
            root: 103.83,
            tempo: 80,
            line: &[0, 4, 7, 11, 7, 4, 0, 12],
            encodes: "Seifert surface: oriented film spanning a knot or link",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE TWIST")
    }

    fn status(&self, t: f64) -> Option<String> {
        let tw = twist(t, None, self.seed);
        Some(format!("tw={tw:.2}  film  DRAG:TW"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let tw = twist(t, hands.last().copied(), self.seed);
        draw(canvas, tw, self.seed ^ hands.len() as u64);
        if let Some(&(x, y)) = hands.last() {
            let (bw, bh) = canvas.draw_bounds();
            if bw > 0 && bh > 0 {
                let px = (x * bw.saturating_sub(1) as f64).round() as i32;
                let py = (y * bh.saturating_sub(1) as f64).round() as i32;
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
        let tw = twist(t, hands.last().copied(), self.seed);
        Some(format!("TW={tw:.3}  seifert"))
    }

    fn reveal(&self) -> &'static str {
        "Every knot or link bounds an oriented Seifert surface. From that film you \
         read Seifert genus and build the Seifert matrix; linking of components is \
         written in how the film twists between them."
    }
}

#[cfg(test)]
mod tests {
    use super::Seifert;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Seifert::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("film"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn twist_changes() {
        let r = Seifert::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
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
        Seifert::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}

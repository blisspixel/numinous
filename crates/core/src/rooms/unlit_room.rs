//! The Unlit Room: Tokarsky illumination; one point no light can reach.
//!
//! A polygonal room is almost always illuminateable from any point, but
//! Tokarsky (1995) built a polygon with a dark point. This room shows a
//! simplified dark pocket: rays from the lantern miss a marked cell. DRAG:
//! CRANK THE LANTERN. See `docs/ROOMS.md`.

use std::f64::consts::TAU;

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const RAYS: usize = 72;

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

/// Simple L-shaped polygon vertices in plate coords.
fn walls() -> Vec<(f64, f64)> {
    vec![
        (0.15, 0.15),
        (0.85, 0.15),
        (0.85, 0.45),
        (0.50, 0.45),
        (0.50, 0.85),
        (0.15, 0.85),
    ]
}

/// Dark point that our ray budget systematically undersamples (toy of Tokarsky).
fn dark_point(seed: u64) -> (f64, f64) {
    if seed == 0 {
        (0.72, 0.72)
    } else {
        (0.70 + ((seed % 5) as f64) * 0.01, 0.70)
    }
}

fn draw(canvas: &mut dyn Surface, lantern: (f64, f64), dark: (f64, f64), phase: f64) {
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
    let w = walls();
    let mut prev = to_px(w[0].0, w[0].1);
    for &(x, y) in w.iter().skip(1).chain(std::iter::once(&w[0])) {
        let p = to_px(x, y);
        canvas.line(prev.0, prev.1, p.0, p.1, '#');
        prev = p;
    }
    let lp = to_px(lantern.0, lantern.1);
    canvas.plot(lp.0, lp.1, 'O');
    // Rays; deliberately skip the angular sector aimed at the dark pocket.
    let dark_ang = (dark.1 - lantern.1).atan2(dark.0 - lantern.0);
    for i in 0..RAYS {
        let a = TAU * (i as f64 + phase) / RAYS as f64;
        // Soften rays near the dark angle to keep the pocket dark (toy).
        if (a - dark_ang)
            .rem_euclid(TAU)
            .min(TAU - (a - dark_ang).rem_euclid(TAU))
            < 0.18
        {
            continue;
        }
        let ex = lantern.0 + 0.55 * a.cos();
        let ey = lantern.1 + 0.55 * a.sin();
        let ep = to_px(ex, ey);
        canvas.line(lp.0, lp.1, ep.0, ep.1, '.');
    }
    let dp = to_px(dark.0, dark.1);
    canvas.plot(dp.0, dp.1, 'x');
}

/// Unlit Room.
#[derive(Debug, Default)]
pub struct UnlitRoom {
    seed: u64,
}

impl UnlitRoom {
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

impl Room for UnlitRoom {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "unlit-room",
            title: "The Unlit Room",
            wing: "Shape & Space",
            blurb: "Most rooms light everywhere from any lamp; Tokarsky built one that does not. \
                    A marked dark point stays unlit. t turns the beam; DRAG: CRANK THE LANTERN.",
            accent: [80, 80, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let lantern = (0.28, 0.28);
        draw(canvas, lantern, dark_point(self.seed), phase_unit(t));
    }

    fn postcard_t(&self) -> f64 {
        0.3
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "dark pocket",
            root: 130.81,
            tempo: 84,
            line: &[0, 0, 5, 0, 7, 0, 12, 0],
            encodes: "rays filling a polygon except one stubborn point",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: CRANK THE LANTERN")
    }

    fn status(&self, t: f64) -> Option<String> {
        let _ = t;
        Some("DARK x  LIT rays  DRAG:LANTERN".into())
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let lantern = hands.last().copied().unwrap_or((0.28, 0.28));
        draw(canvas, lantern, dark_point(self.seed), phase_unit(t));
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (x, y) = *hands.last().unwrap();
        let d = dark_point(self.seed);
        let dist = (x - d.0).hypot(y - d.1);
        Some(format!(
            "LAMP@{:.0}%{:.0}%  dDARK={dist:.2}  UNLIT",
            x * 100.0,
            y * 100.0
        ))
    }

    fn reveal(&self) -> &'static str {
        "Tokarsky (1995) constructed a polygonal room with a point that no \
         light from another fixed point can reach, even after any number of \
         reflections. Illumination is not always possible; this room is a toy \
         of that dark pocket."
    }
}

#[cfg(test)]
mod tests {
    use super::UnlitRoom;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = UnlitRoom::new().status(0.0).unwrap();
        assert!(s.contains("DRAG") || s.contains("LANTERN"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn lamp_changes() {
        let r = UnlitRoom::new();
        let o = r.status(0.0).unwrap();
        let a = r
            .status_input(
                0.0,
                &[RoomInput::PointerDown {
                    x: 0.4,
                    y: 0.4,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        UnlitRoom::new().render(&mut c, 0.2);
        assert!(c.ink_count() > 15);
    }

    #[test]
    fn motif_ok() {
        assert!(UnlitRoom::new().motif().unwrap().line.len() >= 6);
    }
}

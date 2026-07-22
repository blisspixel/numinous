//! Hippopede of Proclus: figure-eight from intersecting sphere and cylinder.
//!
//! Ambient phase walks a pen along the fetter. DRAG: TUNE ECC.
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

fn ecc(_t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.03
    };
    if let Some((x, _)) = hand {
        0.3 + x * 1.4 + s
    } else {
        // Ambient ecc holds a readable eight; motion lives in the pen.
        1.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, k: f64, show: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    // Polar: r^2 = 4 a (b - a sin^2 th) with b/a = k related.
    let a = 1.0;
    let b = k.clamp(0.25, 1.9);
    let scale = (width.min(height) as f64) * 0.28;
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 6) as f64 * 0.05
    };
    let show = show.clamp(0.0, 1.0);
    let steps = 320;
    let drawn = ((show * steps as f64).round() as usize).min(steps);
    // Soft ghost of the full fetter.
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = 2.0 * std::f64::consts::PI * (i as f64 / steps as f64);
        let r2 = 4.0 * a * (b - a * th.sin().powi(2));
        if r2 <= 0.0 {
            prev = None;
            continue;
        }
        let r = r2.sqrt();
        let ang = th + rot;
        let x = r * ang.cos();
        let y = r * ang.sin();
        let px = (cx + x * scale).round() as i32;
        let py = (cy - y * scale).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // Bright path so far.
    prev = None;
    let mut tip = (cx.round() as i32, cy.round() as i32);
    for i in 0..=drawn {
        let th = 2.0 * std::f64::consts::PI * (i as f64 / steps as f64);
        let r2 = 4.0 * a * (b - a * th.sin().powi(2));
        if r2 <= 0.0 {
            prev = None;
            continue;
        }
        let r = r2.sqrt();
        let ang = th + rot;
        let x = r * ang.cos();
        let y = r * ang.sin();
        let px = (cx + x * scale).round() as i32;
        let py = (cy - y * scale).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
            canvas.line(ox, oy + 1, px, py + 1, '*');
        }
        tip = (px, py);
        prev = Some((px, py));
    }
    for dy in -2..=2 {
        for dx in -2..=2 {
            if dx * dx + dy * dy <= 5 {
                canvas.plot(tip.0 + dx, tip.1 + dy, 'o');
            }
        }
    }
}

/// Hippopede room.
#[derive(Debug, Default)]
pub struct Hippopede {
    seed: u64,
}

impl Hippopede {
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

impl Room for Hippopede {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "hippopede",
            title: "Hippopede",
            wing: "Shape & Space",
            blurb: "Proclus horse-fetter draws itself. Watch the pen; DRAG: TUNE ECC.",
            accent: [100, 70, 50],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, ecc(t, None, self.seed), phase_unit(t), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "hippopede",
            root: 164.81,
            tempo: 84,
            line: &[0, 4, 7, 11, 7, 4, 0, 12],
            encodes: "hippopede: sphere-cylinder section as polar horse fetter",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE ECC")
    }

    fn status(&self, t: f64) -> Option<String> {
        let k = ecc(t, None, self.seed);
        let p = (phase_unit(t) * 100.0).round() as i32;
        Some(format!("b={k:.2}  draw={p}%  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let k = ecc(t, hands.last().copied(), self.seed);
        let show = hands
            .last()
            .map(|&(_, y)| y)
            .unwrap_or_else(|| phase_unit(t));
        draw(canvas, k, show, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let k = ecc(t, hands.last().copied(), self.seed);
        let shape = if k < 0.85 {
            "oval"
        } else if k < 1.15 {
            "eight"
        } else {
            "wide"
        };
        Some(format!("k={k:.2}  {shape}"))
    }

    fn reveal(&self) -> &'static str {
        "The hippopede (horse fetter) of Proclus is the plane curve from a sphere \
         cut by a cylinder. For the right radii it is a figure-eight, kin to the \
         lemniscate and a classical model of planetary motion."
    }
}

#[cfg(test)]
mod tests {
    use super::Hippopede;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Hippopede::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("draw"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn ecc_changes() {
        let r = Hippopede::new();
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
    fn ambient_pen_moves_the_plate() {
        let r = Hippopede::new();
        let mut a = Canvas::new(80, 48);
        let mut b = Canvas::new(80, 48);
        r.render(&mut a, 0.15);
        r.render(&mut b, 0.75);
        assert_ne!(a.to_text(), b.to_text(), "pen must walk the fetter");
        assert!(a.ink_count() > 20);
        assert!(b.ink_count() > 20);
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Hippopede::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}

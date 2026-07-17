//! Sphere Eversion: turn a sphere inside out without creases (smoothed).
//!
//! A guided morph of a circle (2D shadow of the famous eversion): phases show
//! the halfway model collapsing through itself smoothly. HOLD: PUSH THROUGH.
//! See `docs/ROOMS.md`.

use std::f64::consts::TAU;

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

/// Morph parameter in [0,1]: 0 = outside, 0.5 = halfway (figure-eight-ish), 1 = everted.
fn morph(t: f64, hand: Option<(f64, f64)>) -> f64 {
    if let Some((_, y)) = hand {
        y.clamp(0.0, 1.0)
    } else {
        phase_unit(t)
    }
}

/// Parametric "sphere" silhouette under eversion toy: radius modulated by stage.
fn curve_point(theta: f64, stage: f64) -> (f64, f64) {
    // Outside: circle. Halfway: pinched equator (Morin-inspired silhouette).
    // Everted: circle again with flipped normal (drawn as opposite winding ink).
    let pinch = (stage * TAU).sin().abs();
    let r = 0.32 * (1.0 - 0.45 * pinch * (2.0 * theta).cos().abs());
    let twist = stage * 0.8 * (theta * 2.0).sin();
    let x = 0.5 + r * (theta + twist).cos();
    let y = 0.5 + r * (theta + twist).sin() * (1.0 - 0.25 * pinch);
    (x, y)
}

fn draw(canvas: &mut dyn Surface, stage: f64, seed: u64) {
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
    let layers = 3 + if seed == 0 { 0 } else { (seed % 2) as usize };
    for layer in 0..layers {
        let s = (stage + layer as f64 * 0.08).clamp(0.0, 1.0);
        let steps = 96;
        let mut prev: Option<(i32, i32)> = None;
        let ch = if layer == 0 {
            '#'
        } else if layer == 1 {
            '*'
        } else {
            '+'
        };
        for i in 0..=steps {
            let th = TAU * i as f64 / steps as f64;
            let p = curve_point(th, s);
            let q = to_px(p.0, p.1);
            if let Some(o) = prev {
                canvas.line(o.0, o.1, q.0, q.1, ch);
            }
            prev = Some(q);
        }
    }
    // Stage marker.
    let mx = ((0.1 + stage * 0.8) * width as f64).round() as i32;
    let my = (0.92 * height as f64).round() as i32;
    canvas.line(
        (0.1 * width as f64).round() as i32,
        my,
        (0.9 * width as f64).round() as i32,
        my,
        '.',
    );
    canvas.plot(mx, my, 'o');
}

/// Sphere Eversion room.
#[derive(Debug, Default)]
pub struct SphereEversion {
    seed: u64,
}

impl SphereEversion {
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

impl Room for SphereEversion {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "sphere-eversion",
            title: "Sphere Eversion",
            wing: "Shape & Space",
            blurb: "A sphere can turn inside out without creases if you allow it to pass through \
                    itself smoothly. t and HOLD: PUSH THROUGH the stages.",
            accent: [120, 180, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, morph(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "eversion",
            root: 196.0,
            tempo: 80,
            line: &[0, 4, 7, 11, 14, 11, 7, 4],
            encodes: "inside becomes outside without a crease if passage is smooth",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("HOLD: PUSH THROUGH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let s = morph(t, None);
        let name = if s < 0.25 {
            "OUTSIDE"
        } else if s < 0.75 {
            "THROUGH"
        } else {
            "EVERTED"
        };
        Some(format!("stage={s:.2}  {name}  HOLD:PUSH"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let s = morph(t, hands.last().copied());
        draw(canvas, s, self.seed);
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

    fn render_input(&self, canvas: &mut dyn Surface, t: f64, inputs: &[RoomInput]) {
        self.render_poked(canvas, t, &crate::held_pokes_from_inputs(inputs));
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::held_pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let s = morph(t, hands.last().copied());
        let name = if s < 0.25 {
            "OUTSIDE"
        } else if s < 0.75 {
            "HALFWAY"
        } else {
            "EVERTED"
        };
        Some(format!("PUSH stage={s:.2}  {name}"))
    }

    fn reveal(&self) -> &'static str {
        "Smale proved a sphere can be turned inside out through immersions: \
         it may pass through itself, but never crease. The halfway model is the \
         famous strangest shape in differential topology, worn lightly here."
    }
}

#[cfg(test)]
mod tests {
    use super::SphereEversion;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = SphereEversion::new().status(0.2).unwrap();
        assert!(s.contains("HOLD") || s.contains("PUSH"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn push_changes() {
        let r = SphereEversion::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.5,
                    y: 0.9,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        SphereEversion::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(SphereEversion::new().motif().unwrap().line.len() >= 6);
    }
}

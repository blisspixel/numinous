//! The Loneliness Equation: seven Drake dials; L is drawn longer.
//!
//! N = R* fp ne fl fi fc L. Six factors set a product; L (lifetime of
//! communicative civilizations) stretches the bar. The silence is scheduling,
//! not scarcity. DRAG adjusts dials. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const NAMES: [&str; 7] = ["R*", "fp", "ne", "fl", "fi", "fc", "L"];

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

fn dials(t: f64, seed: u64, hand: Option<(f64, f64)>) -> [f64; 7] {
    let base = [
        1.0,
        0.5,
        0.4,
        0.3,
        0.2,
        0.2,
        0.15 + phase_unit(t) * 0.7, // L sweeps longest
    ];
    let mut d = base;
    if seed != 0 {
        for (i, v) in d.iter_mut().enumerate() {
            *v *= 0.85 + 0.05 * ((seed as usize + i) % 4) as f64;
        }
    }
    if let Some((x, y)) = hand {
        let i = ((x * 6.999) as usize).min(6);
        d[i] = y.clamp(0.05, 1.0);
        // L always visually longest potential.
        if i != 6 {
            d[6] = d[6].max(0.2);
        }
    }
    d
}

fn product(d: &[f64; 7]) -> f64 {
    d.iter().product()
}

fn draw(canvas: &mut dyn Surface, d: &[f64; 7]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    for (i, &v) in d.iter().enumerate() {
        let x0 = ((0.08 + i as f64 * 0.12) * width as f64).round() as i32;
        let max_h = if i == 6 {
            height as f64 * 0.85
        } else {
            height as f64 * 0.55
        };
        let h = (v * max_h).max(2.0);
        let y1 = height.saturating_sub(1) as i32;
        let y0 = (y1 as f64 - h).round() as i32;
        canvas.line(x0, y1, x0, y0, if i == 6 { '#' } else { '*' });
        canvas.plot(x0, y0, '+');
    }
    let _ = NAMES;
}

/// Loneliness Equation room.
#[derive(Debug, Default)]
pub struct Loneliness {
    seed: u64,
}

impl Loneliness {
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

impl Room for Loneliness {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "loneliness",
            title: "The Loneliness Equation",
            wing: "Number & Pattern",
            blurb: "Seven Drake dials multiply to N. L, the lifetime of talkers, is drawn longer: \
                    silence can be scheduling, not scarcity. t grows L; DRAG a dial to retune N.",
            accent: [140, 140, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let d = dials(t, self.seed, None);
        draw(canvas, &d);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "drake product",
            root: 164.81,
            tempo: 80,
            line: &[0, 0, 5, 7, 12, 7, 5, 0],
            encodes: "six quiet factors and one long L stretching N",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A DIAL")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = dials(t, self.seed, None);
        let n = product(&d);
        Some(format!("N={n:.3}  L={:.2}  DRAG:DIAL", d[6]))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let d = dials(t, self.seed, hands.last().copied());
        draw(canvas, &d);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (x, y) = *hands.last().unwrap();
        let d = dials(t, self.seed, Some((x, y)));
        let n = product(&d);
        let i = ((x * 6.999) as usize).min(6);
        Some(format!(
            "TUNE {}={:.2}  N={n:.3}  L={:.2}",
            NAMES[i], d[i], d[6]
        ))
    }

    fn reveal(&self) -> &'static str {
        "Drake's equation multiplies uncertain factors into a census of \
         communicative civilizations. Stretching L, how long they talk, can make \
         the sky empty without making life rare: the silence may be timing."
    }
}

#[cfg(test)]
mod tests {
    use super::Loneliness;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Loneliness::new().status(0.0).unwrap();
        assert!(s.contains("DRAG") || s.contains("DIAL"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = Loneliness::new();
        let o = r.status(0.0).unwrap();
        let a = r
            .status_input(
                0.0,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.8,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
        assert!(a.chars().count() <= 56);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(48, 28);
        Loneliness::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(Loneliness::new().motif().unwrap().line.len() >= 6);
    }
}

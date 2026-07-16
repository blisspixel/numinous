//! Fourteen Beacons: the Pioneer pulsar map as a polyrhythm.
//!
//! Fourteen radial ticks mark pulsars relative to the Sun; one longer mark
//! is the galactic center direction (toy of the Pioneer plaque). DRAG: GUESS
//! WHERE HOME IS. See `docs/ROOMS.md`.

use std::f64::consts::TAU;

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

/// Relative angles (radians) for a stylized 14-pulsar map (not the real plaque data).
const BEACONS: [f64; 14] = [
    0.2, 0.7, 1.3, 1.8, 2.4, 2.9, 3.5, 4.0, 4.6, 5.1, 5.5, 5.9, 0.0, 3.1,
];

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

fn home_angle(seed: u64) -> f64 {
    // Galactic-center-ish direction in our toy map.
    if seed == 0 {
        4.0
    } else {
        BEACONS[(seed as usize) % BEACONS.len()]
    }
}

fn draw(canvas: &mut dyn Surface, guess: Option<f64>, pulse: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let r = width.min(height) as f64 * 0.4;
    // Sun.
    canvas.plot(cx.round() as i32, cy.round() as i32, '#');
    for (i, &ang) in BEACONS.iter().enumerate() {
        let a = ang
            + if seed == 0 {
                0.0
            } else {
                ((seed % 3) as f64) * 0.05
            };
        // Length encodes a fake period rank.
        let len = 0.45 + 0.4 * ((i as f64 * 0.37 + pulse).sin().abs());
        let x0 = cx + r * 0.15 * a.cos();
        let y0 = cy + r * 0.15 * a.sin();
        let x1 = cx + r * len * a.cos();
        let y1 = cy + r * len * a.sin();
        canvas.line(
            x0.round() as i32,
            y0.round() as i32,
            x1.round() as i32,
            y1.round() as i32,
            '*',
        );
    }
    // Home mark (longer).
    let h = home_angle(seed);
    let hx = cx + r * 0.95 * h.cos();
    let hy = cy + r * 0.95 * h.sin();
    canvas.line(
        cx.round() as i32,
        cy.round() as i32,
        hx.round() as i32,
        hy.round() as i32,
        '#',
    );
    if let Some(g) = guess {
        // Full radial stroke so a probe hand changes many cells (challenge pose).
        let gx = cx + r * 0.75 * g.cos();
        let gy = cy + r * 0.75 * g.sin();
        canvas.line(
            cx.round() as i32,
            cy.round() as i32,
            gx.round() as i32,
            gy.round() as i32,
            '+',
        );
    }
}

/// Fourteen Beacons room.
#[derive(Debug, Default)]
pub struct FourteenBeacons {
    seed: u64,
}

impl FourteenBeacons {
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

impl Room for FourteenBeacons {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "fourteen-beacons",
            title: "Fourteen Beacons",
            wing: "Shape & Space",
            blurb: "Fourteen pulsar ticks around the Sun, one longer home mark: a toy of the \
                    Pioneer plaque. t pulses the periods; DRAG: GUESS WHERE HOME IS.",
            accent: [200, 200, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, None, phase_unit(t), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "pulsar map",
            root: 155.56,
            tempo: 140,
            line: &[0, 7, 0, 12, 0, 5, 0, 7],
            encodes: "fourteen ticks and one long home stroke",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: GUESS WHERE HOME IS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let _ = t;
        Some("14 BEACONS  HOME mark  DRAG:GUESS".into())
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let guess = hands.last().map(|&(x, y)| (y - 0.5).atan2(x - 0.5));
        draw(canvas, guess, phase_unit(t), self.seed);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (x, y) = *hands.last().unwrap();
        let g = (y - 0.5).atan2(x - 0.5);
        let h = home_angle(self.seed);
        let err = (g - h).rem_euclid(TAU);
        let err = err.min(TAU - err).to_degrees();
        let grade = if err < 15.0 {
            "NEAR"
        } else if err < 45.0 {
            "WARM"
        } else {
            "FAR"
        };
        Some(format!("GUESS err={err:.0}deg  {grade}"))
    }

    fn reveal(&self) -> &'static str {
        "The Pioneer plaques drew the Sun among fourteen pulsars and a long \
         hash toward the galactic center: a return address in astrophysical units. \
         This room is a playable echo of that map, not a survey catalog."
    }
}

#[cfg(test)]
mod tests {
    use super::FourteenBeacons;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = FourteenBeacons::new().status(0.0).unwrap();
        assert!(s.contains("DRAG") || s.contains("GUESS"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn guess_changes() {
        let r = FourteenBeacons::new();
        let o = r.status(0.0).unwrap();
        let a = r
            .status_input(
                0.0,
                &[RoomInput::PointerDown {
                    x: 0.8,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
        assert!(a.chars().count() <= 56);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        FourteenBeacons::new().render(&mut c, 0.3);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(FourteenBeacons::new().motif().unwrap().line.len() >= 6);
    }
}

//! Capillary meniscus: Young-Laplace curvature of a free surface.
//!
//! DRAG: TUNE CONTACT. See `docs/ROOMS.md`.

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

/// Contact angle factor: <0.5 wetting rise, >0.5 depression.
fn contact(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        (x + s * 0.5).clamp(0.0, 1.0)
    } else {
        (phase_unit(t) + s * 0.5).clamp(0.0, 1.0)
    }
}

fn capillary_state(contact: f64) -> f64 {
    let contact = if contact.is_finite() {
        contact.clamp(0.0, 1.0)
    } else {
        0.5
    };
    (0.5 - contact) * 2.0
}

fn meniscus_offset(contact_cosine: f64, amplitude: f64, edge: f64) -> f64 {
    -contact_cosine * amplitude * edge
}

fn draw(canvas: &mut dyn Surface, c: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Tube walls.
    let left = (width as f64 * 0.2).round() as i32;
    let right = (width as f64 * 0.8).round() as i32;
    canvas.line(left, 1, left, height.saturating_sub(2) as i32, '|');
    canvas.line(right, 1, right, height.saturating_sub(2) as i32, '|');
    // Capillary rise height ~ cos(theta); meniscus shape ~ cosh-ish.
    let cos_th = capillary_state(c);
    let rise = cos_th * (height as f64) * 0.25;
    let mid_y = height as f64 * 0.55 - rise;
    let amp = (height as f64)
        * 0.12
        * (1.0
            + if seed == 0 {
                0.0
            } else {
                (seed % 3) as f64 * 0.05
            });
    // Meniscus: y = mid + amp * ( (2u-1)^2 * sign ) with wetting pulling edges up.
    let steps = right - left;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let u = i as f64 / steps.max(1) as f64;
        let edge = (2.0 * u - 1.0).powi(2);
        // Wetting (cos>0): edges higher; non-wetting: edges lower.
        let y = mid_y + meniscus_offset(cos_th, amp, edge);
        let px = left + i;
        let py = y.round().clamp(1.0, height.saturating_sub(2) as f64) as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
    // Liquid fill below meniscus (sparse).
    for i in (0..=steps).step_by(3) {
        let u = i as f64 / steps.max(1) as f64;
        let edge = (2.0 * u - 1.0).powi(2);
        let y = mid_y + meniscus_offset(cos_th, amp, edge);
        let px = left + i;
        let py0 = y.round().clamp(1.0, height.saturating_sub(2) as f64) as i32;
        let py1 = height.saturating_sub(2) as i32;
        if py1 > py0 {
            canvas.line(px, py0 + 1, px, py1, '.');
        }
    }
}

/// Capillary meniscus room.
#[derive(Debug, Default)]
pub struct Capillary {
    seed: u64,
}

impl Capillary {
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

impl Room for Capillary {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "capillary",
            title: "Capillary Meniscus",
            wing: "Motion & Dynamics",
            blurb: "Young-Laplace rise vs contact angle. t and DRAG: TUNE CONTACT.",
            accent: [40, 120, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, contact(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "capillary",
            root: 261.63,
            tempo: 76,
            line: &[0, 2, 4, 7, 9, 12, 9, 4],
            encodes: "surface tension curves the free surface to meet the wall angle",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE CONTACT")
    }

    fn status(&self, t: f64) -> Option<String> {
        let c = contact(t, None, self.seed);
        let cos = capillary_state(c);
        Some(format!("cos={cos:.2}  men  DRAG:CONT"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let c = contact(t, hands.last().copied(), self.seed);
        draw(canvas, c, self.seed);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let c = contact(t, hands.last().copied(), self.seed);
        let height = capillary_state(c);
        let (wetting, direction) = if height > 0.025 {
            ("WET", "RISE")
        } else if height < -0.025 {
            ("NONWET", "DEPRESS")
        } else {
            ("NEUTRAL", "LEVEL")
        };
        Some(format!(
            "{wetting} {direction} {:.0}%",
            height.abs() * 100.0
        ))
    }

    fn reveal(&self) -> &'static str {
        "Young-Laplace says pressure jump equals surface tension times mean \
         curvature. Against a wall the free surface meets a contact angle; wetting \
         liquids climb, mercury is depressed, and plants drink by this geometry."
    }
}

#[cfg(test)]
mod tests {
    use super::{Capillary, capillary_state, meniscus_offset};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Capillary::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("men"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn contact_changes() {
        let r = Capillary::new();
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
    fn contact_side_determines_rise_or_depression() {
        let wetting = capillary_state(0.1);
        let level = capillary_state(0.5);
        let non_wetting = capillary_state(0.9);
        assert!(wetting > 0.0);
        assert!(level.abs() < 1e-12);
        assert!(non_wetting < 0.0);
        assert!((wetting + non_wetting).abs() < 1e-12);
    }

    #[test]
    fn meniscus_is_continuous_through_neutral_contact() {
        let negative = meniscus_offset(-1e-6, 12.0, 1.0);
        let neutral = meniscus_offset(0.0, 12.0, 1.0);
        let positive = meniscus_offset(1e-6, 12.0, 1.0);
        assert!(negative.abs() < 2e-5);
        assert_eq!(neutral, 0.0);
        assert!(positive.abs() < 2e-5);
        assert!((negative + positive).abs() < 1e-12);
    }

    #[test]
    fn status_distinguishes_rise_from_depression() {
        let room = Capillary::new();
        let status_at = |x| {
            room.status_input(0.0, &[RoomInput::PointerDown { x, y: 0.5, t: 0.0 }])
                .unwrap()
        };
        assert!(status_at(0.1).contains("RISE"));
        assert!(status_at(0.9).contains("DEPRESS"));
    }

    #[test]
    fn duplicate_hand_history_does_not_change_the_meniscus() {
        let room = Capillary::new();
        let hand = (0.2, 0.5);
        let mut single = Canvas::new(48, 24);
        let mut duplicate = Canvas::new(48, 24);
        room.render_poked(&mut single, 0.3, &[hand]);
        room.render_poked(&mut duplicate, 0.3, &[hand, hand]);
        assert_eq!(single.to_text(), duplicate.to_text());
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Capillary::new().render(&mut c, 0.35);
        assert!(c.ink_count() > 0);
    }
}

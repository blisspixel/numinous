//! Aim a ray inside an ellipse and trace its confocal caustic.

use std::f64::consts::{FRAC_PI_2, TAU};

use super::ellipse_orbit::{Ellipse, Ray};
use super::{latest_hand, phase_unit};
use crate::room::{Room, RoomInput, pokes_from_inputs};
use crate::surface::Surface;
use crate::{DisplayNumber, NumericReadout, ParametricSound, ReadoutId, SoundSpec};

const BOUNCES: usize = 48;
const ROOT_HZ: f32 = 207.65;

/// Elliptical billiard room.
#[derive(Debug, Default)]
pub struct EllipticalBilliard {
    seed: u64,
}

impl EllipticalBilliard {
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

    fn table(&self) -> Ellipse {
        Ellipse {
            minor: 0.65 + (self.seed % 5) as f64 * 0.02,
        }
    }

    fn launch(&self, t: f64, hand: Option<(f64, f64)>) -> Ray {
        // The hand selects a place on the rim and an angle from its tangent.
        // Keep the shallowest launch inside the table. The tour returns to
        // its opening angle without a jump when the phase wraps.
        let (rim, degrees) = match hand {
            Some((x, y)) => (x * TAU, 15.0 + 60.0 * y),
            None => (FRAC_PI_2, 32.0 - 12.0 * (TAU * phase_unit(t)).cos()),
        };
        let rim = rim + (self.seed % 9) as f64 * 0.05;
        self.table().launch(rim, FRAC_PI_2 - degrees.to_radians())
    }

    fn draw(&self, canvas: &mut dyn Surface, mut ray: Ray) {
        let table = self.table();
        let b = table.minor;
        let Some(plate) = table.draw_rim(canvas, '.') else {
            return;
        };

        // Later flights supply the envelope. Draw the launch over them so a
        // player can pick out the ray they aimed even on a small text canvas.
        let start = plate.point(ray.point.0, ray.point.1);
        let first_hit = table.bounce(ray);
        let first = plate.point(first_hit.point.0, first_hit.point.1);
        ray = first_hit;
        let mut previous = first;
        for index in 1..BOUNCES {
            ray = table.bounce(ray);
            let next = plate.point(ray.point.0, ray.point.1);
            let ink = if index < 4 { '*' } else { '+' };
            canvas.line(previous.0, previous.1, next.0, next.1, ink);
            previous = next;
        }
        canvas.line(start.0, start.1, first.0, first.1, '*');
        canvas.plot(start.0, start.1, '#');
        let focus = (1.0 - b * b).sqrt();
        for x in [-focus, focus] {
            let (px, py) = plate.point(x, 0.0);
            let (left, _) = plate.point(x - 0.022, 0.0);
            let (right, _) = plate.point(x + 0.022, 0.0);
            let (_, top) = plate.point(x, 0.022);
            let (_, bottom) = plate.point(x, -0.022);
            canvas.line(left, py, right, py, 'o');
            canvas.line(px, top, px, bottom, 'o');
        }
    }

    fn displayed_parameter(lambda: f64) -> DisplayNumber {
        DisplayNumber::fixed(lambda, 4).expect("the bounded launch has a finite caustic")
    }

    fn voice(&self, ray: Ray) -> ParametricSound {
        let table = self.table();
        // One octave per squared table unit. At the focal boundary the two
        // pitches coincide. Crossing the boundary reverses their order.
        let octaves = table.caustic_parameter(ray) - table.minor.powi(2);
        ParametricSound::new(ROOT_HZ, octaves.exp2() as f32, 0.04)
            .expect("a finite billiard launch stays within one octave")
    }

    fn reading(&self, ray: Ray) -> String {
        let table = self.table();
        let lambda = table.caustic_parameter(ray);
        let boundary = table.minor.powi(2);
        let family = if (lambda - boundary).abs() < 1e-12 {
            "through a focus"
        } else if lambda < boundary {
            "ellipse"
        } else {
            "hyperbola"
        };
        format!(
            "{family}  lambda={}  DRAG TO AIM",
            Self::displayed_parameter(lambda)
        )
    }
}

impl Room for EllipticalBilliard {
    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        self.draw(canvas, self.launch(t, None));
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "ellipse billiard",
            root: ROOT_HZ,
            tempo: 88,
            line: &[0, 5, 7, 12, 7, 5, 0, 9],
            encodes: "conserved caustic of an elliptical table",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET THE LAUNCH")
    }

    fn status(&self, t: f64) -> Option<String> {
        Some(self.reading(self.launch(t, None)))
    }

    fn numeric_readouts(&self, t: f64) -> Option<Vec<NumericReadout>> {
        // The old status used channels 0 and 1 for launch angles.
        let lambda = self.table().caustic_parameter(self.launch(t, None));
        Some(vec![
            Self::displayed_parameter(lambda).readout(ReadoutId::new(2), "LAMBDA"),
        ])
    }

    fn parameter_sound(&self, t: f64, inputs: &[RoomInput]) -> Option<ParametricSound> {
        let hand = latest_hand(&pokes_from_inputs(inputs));
        Some(self.voice(self.launch(t, hand)))
    }

    fn sound(&self, t: f64) -> SoundSpec {
        self.voice(self.launch(t, None)).snapshot()
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        self.draw(canvas, self.launch(t, latest_hand(pokes)));
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let hand = latest_hand(&pokes_from_inputs(inputs));
        Some(self.reading(self.launch(t, hand)))
    }

    fn reveal(&self) -> &'static str {
        "Every bounce keeps the same hidden curve. When the ray misses the gap \
         between the foci, its chords skim an ellipse. When it crosses the gap, \
         their lines touch a hyperbola. Aim through one focus and the next bounce \
         sends the ray through the other."
    }
}

#[cfg(test)]
mod tests {
    use super::EllipticalBilliard;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = EllipticalBilliard::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("LAUNCH"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn launch_changes() {
        let r = EllipticalBilliard::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.9,
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
        EllipticalBilliard::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn prediction_reads_the_displayed_caustic_on_its_own_channel() {
        let room = EllipticalBilliard::new_with(4);
        for t in [0.0, 0.25, 0.5] {
            let channels = room.numeric_readouts(t).unwrap();
            let [lambda] = channels.as_slice() else {
                panic!("expected a single caustic reading");
            };
            assert_eq!(lambda.id().get(), 2);
            assert!(
                room.status(t)
                    .unwrap()
                    .contains(&format!("lambda={:.4}", lambda.value()))
            );
        }
    }

    #[test]
    fn moving_the_hand_out_and_back_restores_the_curve_and_voice() {
        let room = EllipticalBilliard::new();
        let oval = (0.25, 0.25); // Top of the table, 30 degrees from the tangent.
        let arms = (0.25, 0.5);
        let focal_y = (0.65_f64.asin().to_degrees() - 15.0) / 60.0;
        let mut pictures = Vec::new();
        for (hand, family, lambda, order) in [
            (oval, "ellipse", "0.2500", std::cmp::Ordering::Less),
            (
                (0.25, focal_y),
                "through a focus",
                "0.4225",
                std::cmp::Ordering::Equal,
            ),
            (arms, "hyperbola", "0.5000", std::cmp::Ordering::Greater),
        ] {
            let inputs = crate::inputs_from_pokes(&[hand], 0.0);
            let reading = room.status_input(0.0, &inputs).unwrap();
            assert!(
                reading.starts_with(family) && reading.contains(lambda),
                "{reading}"
            );
            assert_eq!(
                room.parameter_sound(0.0, &inputs)
                    .unwrap()
                    .ratio()
                    .total_cmp(&1.0),
                order
            );
            let mut picture = Canvas::new(80, 40);
            room.render_poked(&mut picture, 0.0, &[hand]);
            pictures.push(picture.to_text());
        }
        assert_ne!(pictures[0], pictures[2]);
        let history = [oval, arms, oval];
        let mut restored = Canvas::new(80, 40);
        room.render_poked(&mut restored, 0.7, &history);
        assert_eq!(restored.to_text(), pictures[0]);
        assert_eq!(
            room.sound_input(0.7, &crate::inputs_from_pokes(&history, 0.7)),
            room.sound_input(0.0, &crate::inputs_from_pokes(&[oval], 0.0))
        );
    }

    #[test]
    fn motif_ok() {
        assert!(EllipticalBilliard::new().motif().unwrap().line.len() >= 6);
    }
}

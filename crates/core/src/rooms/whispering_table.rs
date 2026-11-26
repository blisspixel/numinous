//! The Whispering Table: elliptic billiards; chaos is impossible here.
//!
//! A ball on an elliptical table reflects with equal angles. Every trajectory
//! is integrable: caustics are smaller confocal ellipses (or hyperbolas). PULL
//! AND RELEASE: SHOOT. See `docs/ROOMS.md`.

use std::f64::consts::TAU;

use super::ellipse_orbit::Ellipse;
use super::{latest_hand, phase_unit};
use crate::room::{Room, RoomInput};
use crate::surface::Surface;

const BOUNCES: usize = 48;
const TABLE: Ellipse = Ellipse { minor: 0.62 };

fn trajectory(theta: f64, aim: f64) -> Vec<(f64, f64)> {
    let launch = TABLE.launch(theta, aim);
    std::iter::successors(Some(launch), |&ray| Some(TABLE.bounce(ray)))
        .take(BOUNCES + 1)
        .map(|ray| ray.point)
        .collect()
}

fn draw(canvas: &mut dyn Surface, path: &[(f64, f64)]) {
    let Some(plate) = TABLE.draw_rim(canvas, '.') else {
        return;
    };
    for segment in path.windows(2) {
        // This room measures its rim angle clockwise on screen.
        let from = plate.point(segment[0].0, -segment[0].1);
        let to = plate.point(segment[1].0, -segment[1].1);
        canvas.line(from.0, from.1, to.0, to.1, '*');
    }
}

/// Whispering Table room.
#[derive(Debug, Default)]
pub struct WhisperingTable {
    seed: u64,
}

impl WhisperingTable {
    /// Create the room with default seed (0).
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed for replayable per-visit novelty.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }
}

impl Room for WhisperingTable {
    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let th = phase_unit(t) * TAU
            + if self.seed == 0 {
                0.0
            } else {
                (self.seed % 7) as f64 * 0.3
            };
        let aim = 0.4 + phase_unit(t) * 0.5;
        let path = trajectory(th, aim);
        draw(canvas, &path);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "confocal caustic",
            root: 196.0,
            tempo: 104,
            line: &[0, 5, 7, 12, 7, 5, 0, 5],
            encodes: "bounces weaving a quiet caustic ellipse",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("PULL AND RELEASE: SHOOT")
    }

    fn status(&self, _t: f64) -> Option<String> {
        Some(format!(
            "BOUNCE {}  a/b={:.2}  PULL:SHOOT",
            BOUNCES,
            1.0 / TABLE.minor
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let Some((x, y)) = latest_hand(pokes) else {
            self.render(canvas, t);
            return;
        };
        let th = ((y - 0.5).atan2(x - 0.5)).rem_euclid(TAU);
        let aim = 0.2 + x * 0.9;
        let path = trajectory(th, aim);
        draw(canvas, &path);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let Some((x, y)) = latest_hand(&pokes) else {
            return self.status(t);
        };
        let th = ((y - 0.5).atan2(x - 0.5)).rem_euclid(TAU);
        let aim = 0.2 + x * 0.9;
        Some(format!(
            "SHOOT th={:.0}  aim={aim:.2}  n={}",
            th.to_degrees(),
            BOUNCES
        ))
    }

    fn reveal(&self) -> &'static str {
        "On an elliptical billiard table every trajectory is regular: the \
         reflection law preserves a caustic confocal with the boundary. There is \
         no chaos here; the whispering gallery is geometry, not magic."
    }
}

#[cfg(test)]
mod tests {
    use super::{BOUNCES, WhisperingTable, trajectory};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn trajectory_bounces() {
        let path = trajectory(0.3, 0.5);
        assert_eq!(path.len(), BOUNCES + 1);
        assert!(path.windows(2).all(|pair| pair[0] != pair[1]));
    }

    #[test]
    fn status_invites() {
        let s = WhisperingTable::new().status(0.0).unwrap();
        assert!(s.contains("PULL") || s.contains("SHOOT"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn shoot_changes() {
        let r = WhisperingTable::new();
        let o = r.status(0.0).unwrap();
        let a = r
            .status_input(
                0.0,
                &[RoomInput::PointerDown {
                    x: 0.7,
                    y: 0.3,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
        assert!(a.chars().count() <= 56);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(48, 32);
        WhisperingTable::new().render(&mut c, 0.3);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(WhisperingTable::new().motif().unwrap().line.len() >= 6);
    }
}

//! Figure-eight knot: the second simplest prime knot.
//!
//! DRAG: TUNE PHASE. See `docs/ROOMS.md`.

use super::{latest_hand, phase_unit};
use crate::room::{Room, RoomInput};
use crate::surface::Surface;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_3, PI, TAU};

fn phase(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let dial = hand.map_or_else(|| phase_unit(t), |(x, _)| x);
    dial * 2.0 * PI + (seed % 5) as f64 * 0.1
}

fn height(t: f64) -> f64 {
    (4.0 * t).sin()
}

// A top view of (r cos(3t), r sin(3t), sin(4t)), with positive z nearer.
fn point(t: f64, rotation: f64) -> (f64, f64, f64) {
    let r = 2.0 + (2.0 * t).cos();
    let (sin, cos) = (3.0 * t + rotation).sin_cos();
    (r * cos, r * sin, height(t))
}

// Two strands meet in this view when their parameters differ by 2pi/3 and
// their radii agree. The four LOWER visits are pi/3 + k*pi/2. Gap just
// those visits, so the continuous strand always passes over the broken one.
fn underpass(t: f64, half_gap: f64) -> bool {
    let offset = (t - FRAC_PI_3).rem_euclid(FRAC_PI_2);
    offset.min(FRAC_PI_2 - offset) < half_gap
}

fn draw(canvas: &mut dyn Surface, ph: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width - 1) as f64 * 0.5;
    let cy = (height - 1) as f64 * 0.5;
    let aspect = canvas.safe_char_aspect();
    let scale = 0.30 * cx.min(cy / aspect);
    let rotation = ph + (seed % 6) as f64 * 0.06;
    // Widen the underpass on small text surfaces so the gap survives rounding.
    let half_gap = (0.025 + 0.6 / (scale * aspect.min(1.0)).max(1.0)).min(0.15);
    let project = |x: f64, y: f64| {
        (
            (cx + x * scale).round() as i32,
            (cy - y * (scale * aspect)).round() as i32,
        )
    };
    let (x, y, _) = point(0.0, rotation);
    let mut prev = project(x, y);
    let steps = 420;
    let step = TAU / f64::from(steps);
    for i in 1..=steps {
        let t = f64::from(i) * step;
        let (x, y, z) = point(t, rotation);
        let next = project(x, y);
        if next != prev && !underpass(t - 0.5 * step, half_gap) {
            canvas.line(
                prev.0,
                prev.1,
                next.0,
                next.1,
                if z > 0.65 { '#' } else { '*' },
            );
        }
        prev = next;
    }
    if !underpass(ph, half_gap) {
        let (x, y, _) = point(ph, rotation);
        let bead = project(x, y);
        let radius = (scale * 0.10).round().clamp(1.0, 4.0) as i32;
        let ry = (f64::from(radius) * aspect).round().clamp(1.0, 4.0) as i32;
        for dy in -ry..=ry {
            for dx in -radius..=radius {
                if (dx * ry).pow(2) + (dy * radius).pow(2) <= (radius * ry).pow(2) {
                    canvas.plot(bead.0 + dx, bead.1 + dy, 'o');
                }
            }
        }
    }
}

/// Figure-eight knot room.
#[derive(Debug, Default)]
pub struct FigureEightKnot {
    seed: u64,
}

impl FigureEightKnot {
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

impl Room for FigureEightKnot {
    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, phase(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "figure-eight-knot",
            root: 82.41,
            tempo: 90,
            line: &[0, 4, 5, 9, 12, 9, 5, 4],
            encodes: "figure-eight knot: four crossings, amphichiral prime",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE PHASE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let p = phase(t, None, self.seed);
        let z = height(p);
        Some(format!("p={p:.2}  4xing  bead z={z:+.2}  DRAG:PH"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let p = phase(t, latest_hand(pokes), self.seed);
        draw(canvas, p, self.seed);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let Some(hand) = latest_hand(&pokes) else {
            return self.status(t);
        };
        let th = phase(t, Some(hand), self.seed);
        let deg = (th.rem_euclid(TAU) / TAU * 360.0).floor() as i32;
        let z = height(th);
        // The hyperbolic volume of the knot complement rounds to 2.03.
        Some(format!("th={deg}deg  cr=4  vol~2.03  bead z={z:+.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Drag sideways to send the bead around one loop. At each gap it ducks \
         behind the unbroken strand. Positive z means the bead is closer to \
         you. The knot needs at least four crossings in a flat diagram. Its \
         mirror can be bent back into the same knot without cutting the loop."
    }
}

#[cfg(test)]
mod tests {
    use super::{FRAC_PI_2, FRAC_PI_3, FigureEightKnot, point, underpass};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn each_crossing_breaks_only_the_lower_visit() {
        for rotation in [0.0, 0.79, 3.57] {
            for k in 0..4 {
                let lower = FRAC_PI_3 + f64::from(k) * FRAC_PI_2;
                let upper = lower - 2.0 * FRAC_PI_3;
                let (x, y, z) = point(lower, rotation);
                let (other_x, other_y, other_z) = point(upper, rotation);
                assert!((x - other_x).hypot(y - other_y) < 1e-12);
                assert!(z < -0.8 && other_z > 0.8);
                for gap in [0.025, 0.15] {
                    assert!(underpass(lower, gap));
                    assert!(!underpass(upper, gap));
                    assert!(!underpass(lower - 1.1 * gap, gap));
                    assert!(!underpass(lower + 1.1 * gap, gap));
                }
            }
        }
    }

    #[test]
    fn bead_passes_behind_each_crossing_and_returns_in_front() {
        let room = FigureEightKnot::new();
        for k in 0..4 {
            let lower = FRAC_PI_3 + f64::from(k) * FRAC_PI_2;
            let upper = (lower - 2.0 * FRAC_PI_3).rem_euclid(super::TAU);
            for (angle, visible, height) in [(lower, false, "-0.87"), (upper, true, "+0.87")] {
                let t = angle / super::TAU;
                let mut canvas = Canvas::new(48, 24);
                room.render(&mut canvas, t);
                assert_eq!(canvas.to_text().contains('o'), visible);
                assert!(
                    room.status(t)
                        .unwrap()
                        .ends_with(&format!("z={height}  DRAG:PH"))
                );
            }
        }
    }

    #[test]
    fn status_invites() {
        let s = FigureEightKnot::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("4xing"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn phase_goals_keep_their_legacy_columns_and_seed_offset() {
        for seed in 0..30 {
            let room = FigureEightKnot::new_with(seed);
            let lookup = crate::readout::ReadoutLookup::new(&room).unwrap();
            for t in [0.0, 0.125, 0.5, 1.0] {
                let angle = t * 2.0 * super::PI + (seed % 5) as f64 * 0.1;
                let displayed = format!("{angle:.2}").parse::<f64>().unwrap();
                assert_eq!(lookup.value(0, t), Some(displayed));
                assert_eq!(lookup.value(1, t), Some(4.0));
            }
        }
    }

    #[test]
    fn a_held_hand_sets_both_turns_independently_of_tour_time_and_event_count() {
        let room = FigureEightKnot::new_with(7);
        let mut early = Canvas::new(48, 24);
        let mut late = Canvas::new(48, 24);
        let mut further = Canvas::new(48, 24);
        let down = RoomInput::PointerDown {
            x: 0.25,
            y: 0.5,
            t: 0.1,
        };
        let repeated = RoomInput::PointerMove {
            x: 0.25,
            y: 0.5,
            t: 0.5,
        };
        room.render_input(&mut early, 0.1, &[down]);
        room.render_input(&mut late, 0.9, &[down, repeated]);
        assert_eq!(early.to_text(), late.to_text());
        room.render_poked(&mut further, 0.9, &[(0.4, 0.5)]);
        assert_ne!(early.to_text(), further.to_text());
        let status = room.status_input(0.9, &[down, repeated]).unwrap();
        assert_eq!(status, room.status_input(0.1, &[down]).unwrap());
        assert!(status.starts_with("th=101deg  cr=4  vol~2.03  bead z="));
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        FigureEightKnot::new().render(&mut c, 0.45);
        assert!(c.ink_count() > 0);
    }
}

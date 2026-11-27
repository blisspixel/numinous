//! Tinkerbell map: a planar quadratic with a butterfly-shaped attractor.
//!
//! x' = x^2 - y^2 + a x + b y; y' = 2 x y + c x + d y.
//! DRAG: TUNE A AND C. See `docs/ROOMS.md`.

use super::{latest_hand, phase_unit};
use crate::room::{Room, RoomInput};
use crate::surface::Surface;

const START: (f64, f64) = (-0.72, -0.64);
const CLASSIC: Map = Map { a: 0.9, c: 2.0 };
const SELECTED_STEPS: usize = 512;
const REFERENCE_STEPS: usize = 2048;

#[derive(Clone, Copy)]
struct Map {
    a: f64,
    c: f64,
}

impl Map {
    fn next(self, (x, y): (f64, f64)) -> (f64, f64) {
        (
            x * x - y * y + self.a * x - 0.6013 * y,
            2.0 * x * y + self.c * x + 0.5 * y,
        )
    }

    // Visit the start and every in-range iterate. In complex coordinates this
    // map is z squared plus a real linear map with Frobenius norm below three
    // throughout the controls. At radius r > 4 the next radius is at least
    // r * (r - 3), so each later radius grows by at least that same factor r - 3 > 1.
    fn trace(self, steps: usize, mut visit: impl FnMut(usize, (f64, f64))) -> Option<usize> {
        let mut point = START;
        visit(0, point);
        for i in 1..=steps {
            point = self.next(point);
            let (x, y) = point;
            if x * x + y * y > 16.0 {
                return Some(i);
            }
            visit(i, point);
        }
        None
    }

    fn readout(self) -> String {
        let (mut xmin, mut xmax) = (START.0, START.0);
        let (mut ymin, mut ymax) = (START.1, START.1);
        let escaped = self.trace(SELECTED_STEPS, |_, (x, y)| {
            xmin = xmin.min(x);
            xmax = xmax.max(x);
            ymin = ymin.min(y);
            ymax = ymax.max(y);
        });
        let result = match escaped {
            Some(step) => format!("escape {step}"),
            None => {
                let span = ((xmax - xmin) * (ymax - ymin)).sqrt();
                format!("span={span:.2}")
            }
        };
        format!(
            "a={:.2} c={:.2}  {result}  * tuned  . classic  DRAG",
            self.a, self.c
        )
    }
}

fn params(t: f64, hand: Option<(f64, f64)>, seed: u64) -> Map {
    let s = (seed % 5) as f64 * 0.008;
    if let Some((x, y)) = hand {
        Map {
            a: (0.82 + x * 0.16 + s).clamp(0.78, 0.98),
            c: (1.7 + y * 0.55).clamp(1.55, 2.35),
        }
    } else {
        let u = phase_unit(t);
        Map {
            a: (0.88 + u * 0.06 + s).clamp(0.85, 0.96),
            c: (1.85 + u * 0.25).clamp(1.75, 2.2),
        }
    }
}

fn draw(canvas: &mut dyn Surface, map: Map) {
    let (w, h) = canvas.draw_bounds();
    if w == 0 || h == 0 {
        return;
    }
    let (w, h) = ((w - 1) as f64, (h - 1) as f64);
    let aspect = canvas.safe_char_aspect();
    let scale = (w / 2.0).min(h / (2.2 * aspect));
    let project = |(x, y): (f64, f64)| {
        (
            (w * 0.5 + (x + 0.4) * scale).round() as i32,
            (h * 0.5 - (y + 0.5) * scale * aspect).round() as i32,
        )
    };

    // One dot per cell keeps the classic butterfly faint even where it folds
    // back on itself. Both orbits share a fixed camera and initial point.
    let mut reference = Vec::with_capacity(REFERENCE_STEPS + 1);
    CLASSIC.trace(REFERENCE_STEPS, |_, p| reference.push(project(p)));
    reference.sort_unstable();
    reference.dedup();
    for (x, y) in reference {
        canvas.plot(x, y, '.');
    }

    let mut previous = None;
    map.trace(SELECTED_STEPS, |i, p| {
        let (x, y) = project(p);
        if let Some((px, py)) = previous
            && i <= 5
        {
            canvas.line(px, py, x, y, '.');
        }
        previous = Some((x, y));
        canvas.plot(x, y, if i < 8 || i % 16 == 0 { '#' } else { '*' });
    });
}

/// Tinkerbell map room.
#[derive(Debug, Default)]
pub struct Tinkerbell {
    seed: u64,
}

impl Tinkerbell {
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

impl Room for Tinkerbell {
    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, params(t, None, self.seed));
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "tinkerbell",
            root: 196.00,
            tempo: 118,
            line: &[0, 7, 3, 10, 5, 12, 7, 14],
            encodes: "quadratic wings of a planar attractor",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE A AND C")
    }

    fn status(&self, t: f64) -> Option<String> {
        Some(params(t, None, self.seed).readout())
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        draw(canvas, params(t, latest_hand(pokes), self.seed));
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let hand = latest_hand(&crate::pokes_from_inputs(inputs));
        Some(params(t, hand, self.seed).readout())
    }

    fn reveal(&self) -> &'static str {
        "The dotted butterfly follows the classic Tinkerbell map. The bright stars follow \
         up to 512 jumps from the same starting point at your settings. Lines join the first five. \
         Drag to see the stars gather or scatter. Escape marks the first step beyond \
         radius four, where the orbit must keep growing."
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Canvas;
    use crate::room::MAX_ROOM_POKES;

    #[test]
    fn status_invites() {
        let s = Tinkerbell::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn the_classic_orbit_starts_at_the_documented_point_and_uses_both_equations() {
        let mut points = Vec::new();
        assert_eq!(CLASSIC.trace(1, |i, p| points.push((i, p))), None);
        assert_eq!(points[0], (0, (-0.72, -0.64)));
        assert_eq!(points[1].0, 1);
        let (x, y) = points[1].1;
        assert!((x + 0.154368).abs() < 1e-14 && (y + 0.8384).abs() < 1e-14);
        assert_eq!(CLASSIC.trace(REFERENCE_STEPS, |_, _| {}), None);
    }

    #[test]
    fn the_escape_step_is_the_first_certified_crossing() {
        let upper = params(1.0, Some((1.0, 1.0)), 4);
        let norm_sq = upper.a.powi(2) + 0.6013_f64.powi(2) + upper.c.powi(2) + 0.5_f64.powi(2);
        assert!(norm_sq < 9.0);
        let map = params(0.9, None, 0);
        let mut last = None;
        assert_eq!(
            map.trace(SELECTED_STEPS, |i, (x, y)| {
                assert!(x * x + y * y <= 16.0);
                last = Some((i, (x, y)));
            }),
            Some(27)
        );
        let (i, p) = last.unwrap();
        assert_eq!(i, 26);
        let (x, y) = map.next(p);
        assert!(x * x + y * y > 16.0);
        assert!(map.readout().contains("escape 27"));
    }

    #[test]
    fn seed_and_phase_keep_the_parameter_targets_and_a_touch_selects_both_dials() {
        let room = Tinkerbell::new_with(4);
        assert!(room.status(f64::NAN).unwrap().starts_with("a=0.91 c=1.85"));
        assert!(room.status(1.5).unwrap().starts_with("a=0.96 c=2.10"));
        let input = RoomInput::PointerDown {
            x: 0.0,
            y: 1.0,
            t: 0.0,
        };
        let room = Tinkerbell::new();
        let status = room.status_input(0.3, &[input]).unwrap();
        assert!(status.starts_with("a=0.82 c=2.25  escape 141"));
        assert!(room.status(0.3).unwrap().contains("span="));
        assert_eq!(room.status_input(0.3, &[]), room.status(0.3));
        assert_eq!(
            latest_hand(&[(0.1, 0.4), (-1.0, 2.0), (0.5, f64::NAN)]),
            Some((0.0, 1.0))
        );
        let mut history = vec![(0.5, 0.5)];
        history.extend(vec![(f64::NAN, 0.5); MAX_ROOM_POKES]);
        assert_eq!(latest_hand(&history), None);
    }

    #[test]
    fn a_compact_footer_keeps_the_measurement_before_the_legend() {
        let room = Tinkerbell::new();
        let hand = RoomInput::PointerDown {
            x: 0.0,
            y: 1.0,
            t: 0.0,
        };
        let tuning = room.status_input(0.5, &[hand]).unwrap();
        let postcard = room.status(room.postcard_t()).unwrap();
        // At 360 pixels the generic-controller footer can keep 25 characters
        // before its ellipsis. Both dials and the result belong in that space.
        assert_eq!(&tuning[..25], "a=0.82 c=2.25  escape 141");
        assert!(postcard[..25].starts_with("a=0.91 c=1.96  span=1.69"));
        for status in [tuning, postcard] {
            assert!(status.ends_with("* tuned  . classic  DRAG"));
        }
    }

    #[test]
    fn the_reference_stays_faint_and_the_escape_draws_only_real_iterates() {
        #[derive(Default)]
        struct Plate {
            dots: Vec<(i32, i32)>,
            live: Vec<(i32, i32, char)>,
            links: usize,
        }

        impl Surface for Plate {
            fn width(&self) -> usize {
                241
            }
            fn height(&self) -> usize {
                141
            }
            fn plot(&mut self, x: i32, y: i32, mark: char) {
                match mark {
                    '.' => self.dots.push((x, y)),
                    _ => self.live.push((x, y, mark)),
                }
            }
            fn line(&mut self, _: i32, _: i32, _: i32, _: i32, mark: char) {
                assert_eq!(mark, '.');
                self.links += 1;
            }
        }

        let mut wings = Plate::default();
        let mut escape = Plate::default();
        let room = Tinkerbell::new();
        room.render(&mut wings, room.postcard_t());
        room.render(&mut escape, 0.9);
        assert!(wings.dots.len() > 300);
        assert_eq!(wings.dots, escape.dots);
        assert!(
            wings.dots.windows(2).all(|w| w[0] < w[1]),
            "one faint dot per cell"
        );
        assert_eq!(wings.live.len(), SELECTED_STEPS + 1);
        assert_eq!(
            escape.live.len(),
            27,
            "includes the starting point at step zero"
        );
        assert_eq!(escape.live[0], (100, 79, '#'));
        assert_eq!(wings.live[0], escape.live[0], "one fixed coordinate frame");
        assert_eq!((wings.links, escape.links), (5, 5));
        assert!(escape.live.iter().all(|(_, _, m)| matches!(m, '*' | '#')));
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Tinkerbell::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 80, "butterfly must fill the plate");
        for t in [0.0, 0.35, 0.55, 0.9] {
            let mut large = Canvas::new(120, 70);
            Tinkerbell::new().render(&mut large, t);
            assert!(
                large.ink_count() > 100,
                "t={t} must not collapse to blank: {}",
                large.ink_count()
            );
        }
    }

    #[test]
    fn motif_ok() {
        assert!(Tinkerbell::new().motif().unwrap().line.len() >= 6);
    }
}

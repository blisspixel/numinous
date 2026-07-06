//! Voronoi: drop wells in a desert and watch the territories crystallize.
//!
//! Every point belongs to its nearest well; the borders are where two wells
//! tie. That single rule draws giraffe patches, dragonfly wings, mud cracks,
//! and cell walls. `t` lets the wells drift, and the whole map of borders
//! renegotiates itself continuously. See `docs/ROOMS.md`.

use std::f64::consts::TAU;

use crate::rng::SplitMix64;
use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// Fixed seed so the desert is the same desert every time.
const SEED: u64 = 0xB0B0_0000_5EED_0011;
/// How many wells.
const SITES: usize = 14;

/// The wells at phase `t`: seeded homes, each drifting on its own small orbit.
fn sites(t: f64) -> Vec<(f64, f64)> {
    let mut rng = SplitMix64::new(SEED);
    (0..SITES)
        .map(|_| {
            let home = (rng.next_f64(), rng.next_f64());
            let orbit = 0.04 + rng.next_f64() * 0.05;
            let phase = rng.next_f64() * TAU;
            let angle = phase + TAU * t;
            (
                (home.0 + orbit * angle.cos()).rem_euclid(1.0),
                (home.1 + orbit * angle.sin()).rem_euclid(1.0),
            )
        })
        .collect()
}

/// The nearest and second-nearest squared distances from `(x, y)` to the wells.
fn two_nearest(wells: &[(f64, f64)], x: f64, y: f64) -> (f64, f64) {
    let (mut best, mut second) = (f64::INFINITY, f64::INFINITY);
    for &(wx, wy) in wells {
        let d = (x - wx).powi(2) + (y - wy).powi(2);
        if d < best {
            second = best;
            best = d;
        } else if d < second {
            second = d;
        }
    }
    (best, second)
}

/// The Voronoi room.
#[derive(Debug, Default)]
pub struct Voronoi;

impl Voronoi {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Room for Voronoi {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "voronoi",
            title: "Voronoi Territories",
            wing: "Shape & Space",
            blurb: "Fourteen wells in a desert; every point belongs to its nearest one. The \
                    borders are the ties. Giraffes, dragonflies, and mud cracks all know this map.",
            accent: [235, 180, 90],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let wells = sites(t);
        // Edge thickness scales with resolution so borders read at any size.
        let eps = 2.4 / width.max(height) as f64;
        for py in 0..height {
            for px in 0..width {
                let x = (px as f64 + 0.5) / width as f64;
                let y = (py as f64 + 0.5) / height as f64;
                let (best, second) = two_nearest(&wells, x, y);
                // A tie, within tolerance, is a border.
                if second.sqrt() - best.sqrt() < eps {
                    canvas.plot(px as i32, py as i32, '*');
                }
            }
        }
        // The wells themselves, bright.
        for &(wx, wy) in &wells {
            let px = (wx * width as f64) as i32;
            let py = (wy * height as f64) as i32;
            canvas.plot(px, py, '#');
            canvas.plot(px + 1, py, '#');
            canvas.plot(px, py + 1, '#');
        }
    }

    fn reveal(&self) -> &'static str {
        "One rule, nearest well wins, draws this entire map, and nature uses it \
         everywhere two things grow toward each other: giraffe patches, \
         dragonfly wings, drying mud, soap froth, cell walls. In 1854 John Snow \
         drew this exact diagram around London's water pumps and found the one \
         spreading cholera."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Descartes sketched these cells for the heavens in 1644, two centuries \
             before Voronoi; astronomers now draw them at the largest scale there \
             is, carving the universe into galactic territories around voids.",
            "Every airport, cell tower, and delivery depot quietly owns a Voronoi \
             cell: the region it serves before a rival is closer. Logistics is \
             this room with trucks.",
        ]
    }

    fn postcard_t(&self) -> f64 {
        0.3
    }
}

#[cfg(test)]
mod tests {
    use super::{SITES, Voronoi, sites, two_nearest};
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn wells_stay_in_the_desert_and_drift_smoothly() {
        for &t in &[0.0, 0.3, 0.7, 1.0] {
            let wells = sites(t);
            assert_eq!(wells.len(), SITES);
            for &(x, y) in &wells {
                assert!((0.0..1.0).contains(&x) && (0.0..1.0).contains(&y));
            }
        }
        assert_ne!(sites(0.0), sites(0.5), "the map renegotiates over time");
    }

    #[test]
    fn nearest_bookkeeping_is_ordered() {
        let wells = vec![(0.2, 0.2), (0.8, 0.8), (0.5, 0.1)];
        let (best, second) = two_nearest(&wells, 0.21, 0.2);
        assert!(best <= second);
        assert!(best < 0.001, "the near well is near");
    }

    #[test]
    fn render_draws_borders_and_wells_deterministically() {
        let room = Voronoi::new();
        let mut a = Canvas::new(60, 40);
        let mut b = Canvas::new(60, 40);
        room.render(&mut a, 0.3);
        room.render(&mut b, 0.3);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 60, "borders cover the desert");
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Voronoi::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_tells_the_snow_story() {
        assert!(Voronoi::new().reveal().contains("cholera"));
    }
}

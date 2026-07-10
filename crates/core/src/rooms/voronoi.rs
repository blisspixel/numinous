//! Voronoi: drop wells in a desert and watch the territories crystallize.
//!
//! Every point belongs to its nearest well; the borders are where two wells
//! tie. That single rule draws giraffe patches, dragonfly wings, mud cracks,
//! and cell walls. `t` lets the wells drift, and the whole map of borders
//! renegotiates itself continuously. See `docs/ROOMS.md`.

use std::f64::consts::TAU;

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomMeta};
use crate::surface::Surface;

/// Fixed seed so the desert is the same desert every time.
const SEED: u64 = 0xB0B0_0000_5EED_0011;
/// How many wells.
const SITES: usize = 14;

/// The wells at phase `t`: seeded homes, each drifting on its own small orbit.
fn sites(t: f64, variation: u64) -> Vec<(f64, f64)> {
    let mut rng = SplitMix64::new(SEED ^ variation);
    let t = if t.is_finite() {
        t.rem_euclid(1.0)
    } else {
        0.0
    };
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
        let dx = x - wx;
        let dy = y - wy;
        let d = dx * dx + dy * dy;
        if d < best {
            second = best;
            best = d;
        } else if d < second {
            second = d;
        }
    }
    (best, second)
}

fn player_wells(pokes: &[(f64, f64)]) -> impl Iterator<Item = (f64, f64)> + '_ {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..].iter().filter_map(|&(x, y)| {
        if x.is_finite() && y.is_finite() {
            Some((x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)))
        } else {
            None
        }
    })
}

fn extend_player_wells(wells: &mut Vec<(f64, f64)>, pokes: &[(f64, f64)]) {
    for well in player_wells(pokes) {
        if !wells.contains(&well) {
            wells.push(well);
        }
    }
}

fn plot_well(canvas: &mut dyn Surface, width: usize, height: usize, wx: f64, wy: f64) {
    let px = (wx * width.saturating_sub(1) as f64) as i32;
    let py = (wy * height.saturating_sub(1) as f64) as i32;
    canvas.plot(px, py, '#');
    canvas.plot(px + 1, py, '#');
    canvas.plot(px, py + 1, '#');
}

/// The Voronoi room.
#[derive(Debug, Default)]
pub struct Voronoi {
    seed: u64,
}

impl Voronoi {
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
        let wells = sites(t, self.seed);
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
            plot_well(canvas, width, height, wx, wy);
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

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "open fifths",
            root: 98.0,
            tempo: 72,
            line: &[0, 7, 12, 7, 0, 7, 12, 19],
            encodes: "territories ringing against each other: nearest wins",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: DROP A WELL")
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        // The player's wells join the desert: every border renegotiates.
        let mut wells = sites(t, self.seed);
        wells.reserve(MAX_ROOM_POKES);
        extend_player_wells(&mut wells, pokes);
        let eps = 2.4 / width.max(height) as f64;
        for py in 0..height {
            for px in 0..width {
                let x = (px as f64 + 0.5) / width as f64;
                let y = (py as f64 + 0.5) / height as f64;
                let (best, second) = two_nearest(&wells, x, y);
                if second.sqrt() - best.sqrt() < eps {
                    canvas.plot(px as i32, py as i32, '*');
                }
            }
        }
        for &(wx, wy) in &wells {
            plot_well(canvas, width, height, wx, wy);
        }
    }

    fn postcard_t(&self) -> f64 {
        0.3
    }
}

#[cfg(test)]
mod tests {
    use super::{SITES, Voronoi, extend_player_wells, player_wells, sites, two_nearest};
    use crate::canvas::Canvas;
    use crate::room::Room;

    fn char_at(canvas: &Canvas, x: usize, y: usize) -> char {
        canvas
            .to_text()
            .lines()
            .nth(y)
            .and_then(|line| line.chars().nth(x))
            .unwrap_or(' ')
    }

    fn border_text_without_wells(canvas: &Canvas) -> String {
        canvas
            .to_text()
            .chars()
            .map(|ch| if ch == '#' { ' ' } else { ch })
            .collect()
    }

    #[test]
    fn wells_stay_in_the_desert_and_drift_smoothly() {
        for &t in &[0.0, 0.3, 0.7, 1.0] {
            let wells = sites(t, 0);
            assert_eq!(wells.len(), SITES);
            for &(x, y) in &wells {
                assert!((0.0..1.0).contains(&x) && (0.0..1.0).contains(&y));
            }
        }
        assert_ne!(
            sites(0.0, 0),
            sites(0.5, 0),
            "the map renegotiates over time"
        );
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
    fn new_with_zero_matches_default_and_nonzero_differs() {
        let r0 = Voronoi::new_with(0);
        let r_def = Voronoi::new();
        let mut a = Canvas::new(60, 40);
        let mut b = Canvas::new(60, 40);
        r0.render(&mut a, 0.3);
        r_def.render(&mut b, 0.3);
        assert_eq!(a.to_text(), b.to_text());
        let r42 = Voronoi::new_with(42);
        let mut c = Canvas::new(60, 40);
        r42.render(&mut c, 0.3);
        assert_ne!(a.to_text(), c.to_text());
    }

    #[test]
    fn player_wells_preserve_order_clamp_and_filter() {
        let pokes = [(0.2, 0.8), (f64::NAN, 0.1), (2.0, -1.0), (0.6, 0.4)];

        let wells: Vec<_> = player_wells(&pokes).collect();

        assert_eq!(wells, vec![(0.2, 0.8), (1.0, 0.0), (0.6, 0.4)]);
    }

    #[test]
    fn dropped_well_is_visible_and_renegotiates_borders() {
        let room = Voronoi::new();
        let mut base = Canvas::new(60, 40);
        let mut poked = Canvas::new(60, 40);

        room.render(&mut base, 0.3);
        room.render_poked(&mut poked, 0.3, &[(0.5, 0.5)]);

        assert_ne!(base.to_text(), poked.to_text());
        assert_ne!(
            border_text_without_wells(&base),
            border_text_without_wells(&poked)
        );
        assert_eq!(char_at(&poked, 29, 19), '#');
    }

    #[test]
    fn dropped_edge_well_remains_visible() {
        let room = Voronoi::new();
        let mut poked = Canvas::new(20, 10);

        room.render_poked(&mut poked, 0.3, &[(1.0, 1.0)]);

        assert_eq!(char_at(&poked, 19, 9), '#');
    }

    #[test]
    fn duplicate_dropped_wells_are_collapsed_before_rendering() {
        let room = Voronoi::new();
        let mut duplicate = Canvas::new(60, 40);
        let mut single = Canvas::new(60, 40);

        room.render_poked(&mut duplicate, 0.3, &[(0.5, 0.5), (0.5, 0.5)]);
        room.render_poked(&mut single, 0.3, &[(0.5, 0.5)]);

        assert_eq!(duplicate.to_text(), single.to_text());
    }

    #[test]
    fn player_wells_do_not_duplicate_existing_sites() {
        let mut wells = vec![(0.5, 0.5)];

        extend_player_wells(&mut wells, &[(0.5, 0.5), (0.25, 0.75)]);

        assert_eq!(wells, vec![(0.5, 0.5), (0.25, 0.75)]);
    }

    #[test]
    fn dropped_wells_use_the_newest_bounded_finite_points() {
        let room = Voronoi::new();
        let newest = vec![(0.7, 0.3); crate::MAX_ROOM_POKES];
        let mut old = vec![(0.2, 0.8); crate::MAX_ROOM_POKES + 12];
        old.extend(newest.clone());
        let mut expected = Canvas::new(60, 40);
        let mut actual = Canvas::new(60, 40);

        room.render_poked(&mut expected, 0.3, &newest);
        room.render_poked(&mut actual, 0.3, &old);

        assert_eq!(expected.to_text(), actual.to_text());
    }

    #[test]
    fn nonfinite_pokes_do_not_consume_well_identity() {
        let room = Voronoi::new();
        let finite = [(0.4, 0.6)];
        let mut with_bad_points = vec![(f64::NAN, f64::INFINITY); 8];
        with_bad_points.push(finite[0]);
        let mut expected = Canvas::new(60, 40);
        let mut actual = Canvas::new(60, 40);

        room.render_poked(&mut expected, 0.3, &finite);
        room.render_poked(&mut actual, 0.3, &with_bad_points);

        assert_eq!(expected.to_text(), actual.to_text());
    }

    #[test]
    fn raw_newest_tail_is_capped_before_nonfinite_filtering() {
        let room = Voronoi::new();
        let finite = vec![(0.4, 0.6); crate::MAX_ROOM_POKES];
        let mut with_invalid_tail = finite;
        with_invalid_tail.extend(vec![(f64::NAN, f64::INFINITY); crate::MAX_ROOM_POKES + 5]);
        let mut expected = Canvas::new(60, 40);
        let mut actual = Canvas::new(60, 40);

        room.render(&mut expected, 0.3);
        room.render_poked(&mut actual, 0.3, &with_invalid_tail);

        assert_eq!(expected.to_text(), actual.to_text());
    }

    #[test]
    fn nonzero_variation_changes_poked_borders() {
        let r0 = Voronoi::new_with(0);
        let r42 = Voronoi::new_with(42);
        let pokes = [(0.4, 0.6)];
        let mut a = Canvas::new(60, 40);
        let mut b = Canvas::new(60, 40);

        r0.render_poked(&mut a, 0.3, &pokes);
        r42.render_poked(&mut b, 0.3, &pokes);

        assert_ne!(a.to_text(), b.to_text());
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Voronoi::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0, f64::NAN, f64::INFINITY] {
            room.render(&mut canvas, t);
            room.render_poked(&mut canvas, t, &[(0.5, 0.5)]);
        }
    }

    #[test]
    fn reveal_tells_the_snow_story() {
        assert!(Voronoi::new().reveal().contains("cholera"));
    }
}

//! The Mobius strip: one side, one edge, and it takes two laps to get home.
//!
//! A band with a half twist. An ant walks the centerline; after one full lap
//! it is on the "other side" without ever crossing an edge, and only after two
//! laps is it home. The boundary, traced, turns out to be a single closed
//! curve. `t` walks the ant. See the Full Map in `docs/ROOMS.md`.

use std::f64::consts::TAU;

use super::variation_unit;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

/// The ring radius.
const R: f64 = 1.0;
/// The half-width of the band.
const W: f64 = 0.35;

fn paint_spread(t: f64) -> f64 {
    let phase = if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    };
    (0.14 + phase * 0.56) * TAU
}

fn painted_edge_percent(t: f64) -> f64 {
    (2.0 * paint_spread(t) / TAU).min(1.0) * 100.0
}

/// A point on the strip at ring angle `u` and width position `v` in [-W, W],
/// with the half twist that makes it what it is.
fn strip(u: f64, v: f64) -> (f64, f64, f64) {
    let x = (R + v * (u / 2.0).cos()) * u.cos();
    let y = (R + v * (u / 2.0).cos()) * u.sin();
    let z = v * (u / 2.0).sin();
    (x, y, z)
}

/// Project 3D to 2D: a fixed gentle tilt, so the twist reads.
fn project(x: f64, y: f64, z: f64) -> (f64, f64) {
    let tilt = 0.9_f64;
    (x, y * tilt.cos() * 0.55 + z * tilt.sin())
}

/// The Mobius strip room.
#[derive(Debug, Default)]
pub struct Mobius {
    seed: u64,
}

impl Mobius {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed for replayable per-visit novelty.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    fn phase_for(&self, t: f64) -> f64 {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if self.seed == 0 {
            t
        } else {
            (t + variation_unit(self.seed, 0x4D4F_4249_5553_0001) * 0.5).fract()
        }
    }
}

/// The single edge, sampled: each entry is (screen point, edge parameter w in
/// [0, 2*TAU)). The edge closes only after two laps; w walks all of it.
fn edge_points(width: usize, height: usize, aspect: f64) -> Vec<((i32, i32), f64)> {
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let scale = (width as f64 / 2.0).min(height as f64 / (2.0 * aspect)) * 0.6;
    let steps = 260;
    // Half-open sampling: w = 2*TAU is the same edge point as w = 0, so the
    // closing sample is excluded (drawers close the polyline themselves).
    // Including it would give the w = 0 location a phantom twin whose
    // parameter sits a full lap away.
    (0..steps)
        .map(|i| {
            let w = 2.0 * TAU * f64::from(i) / f64::from(steps);
            let (x, y, z) = strip(w % TAU, if w < TAU { W } else { -W });
            let (px, py) = project(x, y, z);
            (
                ((cx + px * scale) as i32, (cy + py * scale * aspect) as i32),
                w,
            )
        })
        .collect()
}

impl Room for Mobius {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "mobius",
            title: "Mobius Strip",
            wing: "Shape & Space",
            blurb: "A band with a half twist: one side, one edge. The ant walks a full lap and \
                    arrives on the other side without crossing anything. Two laps to get home.",
            accent: [120, 200, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let aspect = canvas.safe_char_aspect();
        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;
        let scale = (width as f64 / 2.0).min(height as f64 / (2.0 * aspect)) * 0.6;
        let to_screen = |x: f64, y: f64, z: f64| {
            let (px, py) = project(x, y, z);
            ((cx + px * scale) as i32, (cy + py * scale * aspect) as i32)
        };

        // The band: rungs across the strip, dim, so the twist reads as form.
        let rungs = 90;
        for i in 0..rungs {
            let u = TAU * f64::from(i) / f64::from(rungs);
            let (x0, y0, z0) = strip(u, -W);
            let (x1, y1, z1) = strip(u, W);
            let a = to_screen(x0, y0, z0);
            let b = to_screen(x1, y1, z1);
            canvas.line(a.0, a.1, b.0, b.1, '-');
        }
        // The single edge: follow v = +W around TWICE and it closes. One curve.
        let edge = edge_points(width, height, aspect);
        let mut previous: Option<(i32, i32)> = None;
        for &(point, _) in &edge {
            if let Some((px, py)) = previous {
                canvas.line(px, py, point.0, point.1, '*');
            }
            previous = Some(point);
        }
        // Close the two-lap loop back to its first point.
        if let (Some((px, py)), Some(&(first, _))) = (previous, edge.first()) {
            canvas.line(px, py, first.0, first.1, '*');
        }
        // The ant: two laps of the centerline to get home. Its trail shows
        // where it has been; at t = 0.5 it is "underneath", upside down.
        let ant_u = 2.0 * TAU * self.phase_for(t);
        let trail = 40;
        for i in 0..trail {
            let u = ant_u - 0.06 * f64::from(i);
            if u < 0.0 {
                break;
            }
            let (x, y, z) = strip(u % TAU, 0.0);
            let (px, py) = to_screen(x, y, z);
            canvas.plot(px, py, '*');
        }
        let (x, y, z) = strip(ant_u % TAU, 0.0);
        let (px, py) = to_screen(x, y, z);
        for dx in -1..=1 {
            for dy in -1..=1 {
                canvas.plot(px + dx, py + dy, '#');
            }
        }
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: PAINT THE EDGE")
    }

    fn status(&self, t: f64) -> Option<String> {
        Some(format!(
            "ONE EDGE   {:.0}% LIT BY PHASE   CLICK: PAINT THE EDGE",
            painted_edge_percent(t)
        ))
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let brushes = inputs
            .iter()
            .filter(|input| {
                matches!(
                    input,
                    RoomInput::PointerDown { x, y, .. } | RoomInput::PointerMove { x, y, .. }
                        if x.is_finite() && y.is_finite()
                )
            })
            .count()
            .min(MAX_ROOM_POKES);
        if brushes == 0 {
            return self.status(t);
        }
        Some(format!(
            "{brushes} BRUSH POINT(S)   {:.0}% OF THE ONE EDGE LIT",
            painted_edge_percent(t)
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        // The newest bounded raw tail first, finite filtering after, matching
        // the catalog input contract.
        let start = pokes.len().saturating_sub(crate::room::MAX_ROOM_POKES);
        let sources: Vec<(f64, f64)> = pokes[start..]
            .iter()
            .copied()
            .filter(|&(x, y)| x.is_finite() && y.is_finite())
            .collect();
        self.render(canvas, t);
        if sources.is_empty() {
            return;
        }
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let aspect = canvas.safe_char_aspect();
        let edge = edge_points(width, height, aspect);
        // Paint spreads along the one edge from each clicked point; the reach
        // grows with the sweep. By the end of the sweep the paint has flowed
        // onto the "other" edge without ever jumping, because there is only
        // one edge. That is the room's whole truth, now under the hand.
        let spread = paint_spread(t);
        for &(x, y) in &sources {
            let px = (x.clamp(0.0, 1.0) * (width - 1) as f64).round() as i32;
            let py = (y.clamp(0.0, 1.0) * (height - 1) as f64).round() as i32;
            // The nearest sampled edge point to the hand is where the brush
            // lands.
            let Some(&(_, w_start)) = edge.iter().min_by_key(|((ex, ey), _)| {
                let dx = i64::from(*ex) - i64::from(px);
                let dy = i64::from(*ey) - i64::from(py);
                dx * dx + dy * dy
            }) else {
                continue;
            };
            for &((ex, ey), w) in &edge {
                let raw = (w - w_start).abs();
                let around = (2.0 * TAU - raw).abs();
                if raw.min(around) <= spread {
                    canvas.plot(ex, ey, '#');
                }
            }
            let marker = (width.min(height) / 40).clamp(4, 12) as i32;
            canvas.line(px - marker, py - marker, px + marker, py - marker, '#');
            canvas.line(px - marker, py + marker, px + marker, py + marker, '#');
            canvas.line(px - marker, py - marker, px - marker, py + marker, '#');
            canvas.line(px + marker, py - marker, px + marker, py + marker, '#');
            canvas.line(px - marker, py, px + marker, py, '#');
            canvas.line(px, py - marker, px, py + marker, '#');
        }
    }

    fn reveal(&self) -> &'static str {
        "Cut a paper strip, give it a half twist, tape the ends: you have made \
         a shape with one side and one edge. Run a finger along the edge and it \
         visits everything before returning. The ant's lap that ends upside \
         down is not a trick; sidedness itself is a property a surface can \
         simply decline to have."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Cut the strip down its centerline and it does not fall into two \
             pieces: it becomes one longer band with a full twist. Cut a third \
             of the way in and you get two linked bands. Scissors are the best \
             topology teacher there is.",
            "Every conveyor belt and typewriter ribbon built as a Mobius band \
             wears both faces evenly, doubling its life, a patent that exists \
             because sidedness is optional.",
        ]
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "E half twist",
            root: 164.81,
            tempo: 104,
            line: &[0, 5, 7, 12, 7, 5, -12, 0],
            encodes: "one lap turns over and the second lap returns home",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Mobius, W, strip};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};
    use std::f64::consts::TAU;

    #[test]
    fn status_counts_finite_brushes_and_handles_unknown_phase() {
        let room = Mobius::new();
        assert_eq!(
            room.status_input(0.5, &[]).as_deref(),
            room.status(0.5).as_deref()
        );
        let inputs = [
            RoomInput::PointerDown {
                x: 0.2,
                y: 0.3,
                t: 0.0,
            },
            RoomInput::PointerMove {
                x: 0.4,
                y: 0.5,
                t: 0.1,
            },
            RoomInput::PointerMove {
                x: f64::NAN,
                y: 0.5,
                t: 0.2,
            },
        ];
        assert_eq!(
            room.status_input(f64::NAN, &inputs).as_deref(),
            Some("2 BRUSH POINT(S)   28% OF THE ONE EDGE LIT")
        );
        assert_eq!(
            room.status_input(0.5, &inputs).as_deref(),
            Some("2 BRUSH POINT(S)   84% OF THE ONE EDGE LIT")
        );
    }

    #[test]
    fn entry_click_paints_a_visible_edge_segment() {
        let room = Mobius::new();
        let mut base = Canvas::new(80, 40);
        let mut poked = Canvas::new(80, 40);
        room.render(&mut base, 0.0);
        room.render_poked(&mut poked, 0.0, &[(0.53, 0.47)]);

        assert_ne!(base.to_text(), poked.to_text());
        assert!(poked.ink_count() >= base.ink_count());
    }

    #[test]
    fn the_half_twist_swaps_the_sides_after_one_lap() {
        // The point at (u=0, v=+W) and the point one lap on at (u=TAU, v=-W)
        // coincide: the two "sides" are the same side.
        let (x0, y0, z0) = strip(0.0, W);
        let (x1, y1, z1) = strip(TAU, -W);
        // strip() takes u mod TAU in render; compare the raw formula at TAU.
        assert!((x0 - x1).abs() < 1e-9, "{x0} vs {x1}");
        assert!((y0 - y1).abs() < 1e-9, "{y0} vs {y1}");
        assert!((z0 - z1).abs() < 1e-9, "{z0} vs {z1}");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = Mobius::new();
        let mut a = Canvas::new(50, 30);
        let mut b = Canvas::new(50, 30);
        room.render(&mut a, 0.35);
        room.render(&mut b, 0.35);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 40);
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Mobius::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 0.5, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_declines_sidedness() {
        assert!(Mobius::new().reveal().contains("one side"));
    }

    #[test]
    fn paint_spreads_with_the_sweep_and_flows_across_the_twist() {
        let room = Mobius::new();
        let count_paint = |t: f64| {
            let mut canvas = Canvas::new(60, 30);
            room.render_poked(&mut canvas, t, &[(0.5, 0.15)]);
            canvas.to_text().chars().filter(|&c| c == '#').count()
        };
        let early = count_paint(0.05);
        let late = count_paint(0.6);
        assert!(early > 0, "the brush lands immediately");
        assert!(late > early, "paint keeps flowing as the sweep advances");

        // The discriminating cross-lap proof: pick the click's own edge
        // point, then a target sample on the OTHER lap whose circular
        // distance requires crossing the twist, at a screen cell no
        // same-lap sample shares. It must be unpainted while the spread is
        // short and painted once the spread reaches it: paint that only
        // covered one lap could never touch it.
        let edge = super::edge_points(60, 30, 0.5);
        let hand = (0.5_f64, 0.15_f64);
        let (px, py) = (
            (hand.0 * 59.0).round() as i32,
            (hand.1 * 29.0).round() as i32,
        );
        let &(_, w_start) = edge
            .iter()
            .min_by_key(|((ex, ey), _)| {
                let dx = i64::from(*ex) - i64::from(px);
                let dy = i64::from(*ey) - i64::from(py);
                dx * dx + dy * dy
            })
            .expect("edge samples exist");
        let same_lap_cells: std::collections::HashSet<(i32, i32)> = edge
            .iter()
            .filter(|&&(_, w)| (w < TAU) == (w_start < TAU))
            .map(|&(cell, _)| cell)
            .collect();
        let circular = |w: f64| {
            let raw = (w - w_start).abs();
            raw.min(2.0 * TAU - raw)
        };
        let target = edge
            .iter()
            .find(|&&(cell, w)| {
                (w < TAU) != (w_start < TAU)
                    && !same_lap_cells.contains(&cell)
                    && circular(w) > 0.55 * TAU
                    && circular(w) < 0.85 * TAU
            })
            .expect("a distinguishable other-lap sample exists");
        let paint_at = |t: f64| {
            let mut canvas = Canvas::new(60, 30);
            room.render_poked(&mut canvas, t, &[hand]);
            canvas.cell(target.0.0 as usize, target.0.1 as usize)
        };
        assert_ne!(
            paint_at(0.3),
            Some('#'),
            "a short spread has not crossed the twist yet"
        );
        assert_eq!(
            paint_at(0.9),
            Some('#'),
            "the spread crosses the twist onto the other lap without a jump"
        );
    }

    #[test]
    fn pokes_use_the_newest_raw_tail_before_filtering() {
        let room = Mobius::new();
        let mut flood: Vec<(f64, f64)> = (0..200).map(|i| (i as f64 / 200.0, 0.8)).collect();
        flood.push((f64::NAN, 0.5));
        flood.push((0.3, 0.2));
        let start = flood.len() - crate::room::MAX_ROOM_POKES;
        let tail = flood[start..].to_vec();
        let mut via_flood = Canvas::new(60, 30);
        room.render_poked(&mut via_flood, 0.4, &flood);
        let mut via_tail = Canvas::new(60, 30);
        room.render_poked(&mut via_tail, 0.4, &tail);
        assert_eq!(via_flood.to_text(), via_tail.to_text());
    }

    #[test]
    fn all_invalid_pokes_render_the_bare_room() {
        let room = Mobius::new();
        let mut bare = Canvas::new(60, 30);
        room.render(&mut bare, 0.4);
        let mut invalid = Canvas::new(60, 30);
        room.render_poked(&mut invalid, 0.4, &[(f64::NAN, 0.5), (0.5, f64::INFINITY)]);
        assert_eq!(bare.to_text(), invalid.to_text());
    }

    #[test]
    fn seed_variation_changes_poked_renders_and_seed_zero_stays_exact() {
        let mut a = Canvas::new(60, 30);
        Mobius::new().render_poked(&mut a, 0.4, &[(0.5, 0.15)]);
        let mut b = Canvas::new(60, 30);
        Mobius::new_with(11).render_poked(&mut b, 0.4, &[(0.5, 0.15)]);
        assert_ne!(a.to_text(), b.to_text(), "the ant phase varies with seed");
        let mut exact = Canvas::new(60, 30);
        Mobius::new_with(0).render_poked(&mut exact, 0.4, &[(0.5, 0.15)]);
        assert_eq!(a.to_text(), exact.to_text());
    }

    #[test]
    fn hostile_surfaces_and_phase_stay_bounded() {
        struct Weird(Canvas);
        impl crate::surface::Surface for Weird {
            fn width(&self) -> usize {
                self.0.width()
            }
            fn height(&self) -> usize {
                self.0.height()
            }
            fn char_aspect(&self) -> f64 {
                f64::NEG_INFINITY
            }
            fn plot(&mut self, x: i32, y: i32, mark: char) {
                self.0.plot(x, y, mark);
            }
        }
        let room = Mobius::new();
        let mut weird = Weird(Canvas::new(30, 15));
        room.render_poked(&mut weird, f64::NAN, &[(0.5, 0.5)]);
        assert!(weird.0.ink_count() > 0);
        let mut nan_phase = Canvas::new(30, 15);
        room.render(&mut nan_phase, f64::NAN);
        let mut zero_phase = Canvas::new(30, 15);
        room.render(&mut zero_phase, 0.0);
        assert_eq!(nan_phase.to_text(), zero_phase.to_text());
    }
}

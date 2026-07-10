//! Zeno's Square: half, then half of what's left, forever, and it adds to one.
//!
//! The proof without words: a unit square filled by rectangles of area 1/2,
//! 1/4, 1/8, ... alternating vertical and horizontal. Zeno said the runner
//! never arrives because infinitely many steps remain; the square says the
//! infinitely many steps fit exactly inside one tile of floor. `t` lays the
//! tiles. See the Full Map in `docs/ROOMS.md`.

use super::variation_unit;
use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// How many halvings `t` reaches (past ~14 the tiles are subpixel anyway).
const MAX_TILES: usize = 14;

/// The tiles: each is (x, y, w, h) in unit-square coordinates, tile `i`
/// having area 2^-(i+1), alternating vertical and horizontal cuts.
fn tiles() -> Vec<(f64, f64, f64, f64)> {
    let (mut x, mut y) = (0.0, 0.0);
    let (mut w, mut h) = (1.0, 1.0);
    let mut out = Vec::with_capacity(MAX_TILES);
    for i in 0..MAX_TILES {
        if i % 2 == 0 {
            // Take the left half of what remains.
            out.push((x, y, w / 2.0, h));
            x += w / 2.0;
            w /= 2.0;
        } else {
            // Take the bottom half of what remains.
            out.push((x, y, w, h / 2.0));
            y += h / 2.0;
            h /= 2.0;
        }
    }
    out
}

/// Zeno's Square.
#[derive(Debug, Default)]
pub struct Zeno {
    seed: u64,
}

impl Zeno {
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
            (t + variation_unit(self.seed, 0x5A45_4E4F_0000_0001) * 0.4).fract()
        }
    }
}

/// The runner's hops from the square's left edge toward a target: hop `k`
/// lands at the point with exactly half the previous remaining distance
/// left, so the positions are start + (1 - 2^-k) of the way. Zeno's paradox,
/// walked: infinitely many hops, one finite arrival.
fn runner_hops(target: (f64, f64), hops: usize) -> Vec<(f64, f64)> {
    let start = (0.0, target.1);
    (1..=hops.min(MAX_TILES))
        .map(|k| {
            let progress = 1.0 - 0.5_f64.powi(k as i32);
            (
                start.0 + (target.0 - start.0) * progress,
                start.1 + (target.1 - start.1) * progress,
            )
        })
        .collect()
}

/// The safe drawing aspect for any surface.
fn safe_aspect(canvas: &dyn Surface) -> f64 {
    let aspect = canvas.char_aspect();
    if aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        0.5
    }
}

impl Room for Zeno {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "zeno",
            title: "Zeno's Square",
            wing: "Change",
            blurb: "Half the square, then half of what's left, then half of that, forever. \
                    Infinitely many tiles, and they fit exactly. The sum of the halves is one.",
            accent: [200, 160, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let aspect = safe_aspect(canvas);
        let side = (width as f64 * 0.8).min(height as f64 * aspect * 0.8);
        let left = (width as f64 - side) / 2.0;
        let top = (height as f64 - side / aspect) / 2.0;
        let to_screen = |x: f64, y: f64| {
            (
                (left + x * side) as i32,
                (top + (1.0 - y) * side / aspect) as i32,
            )
        };
        // The square itself: the destination.
        let (x0, y0) = to_screen(0.0, 1.0);
        let (x1, y1) = to_screen(1.0, 0.0);
        canvas.line(x0, y0, x1, y0, '*');
        canvas.line(x0, y1, x1, y1, '*');
        canvas.line(x0, y0, x0, y1, '*');
        canvas.line(x1, y0, x1, y1, '*');

        // The tiles laid so far: 1/2, then 1/4, then 1/8, ...
        let laid = ((self.phase_for(t) * (MAX_TILES as f64 + 1.0)) as usize).min(MAX_TILES);
        for (i, &(tx, ty, tw, th)) in tiles().iter().take(laid).enumerate() {
            let (px0, py0) = to_screen(tx, ty + th);
            let (px1, py1) = to_screen(tx + tw, ty);
            // Outline bright, fill dithered; later tiles get denser fill.
            canvas.line(px0, py0, px1, py0, '#');
            canvas.line(px0, py1, px1, py1, '#');
            canvas.line(px0, py0, px0, py1, '#');
            canvas.line(px1, py0, px1, py1, '#');
            let step = if i < 4 { 3 } else { 2 };
            let mut py = py0.min(py1);
            while py <= py0.max(py1) {
                let mut px = px0.min(px1);
                while px <= px0.max(px1) {
                    if (px + py) % step == 0 {
                        canvas.plot(px, py, '-');
                    }
                    px += 1;
                }
                py += 1;
            }
        }
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: SEND THE RUNNER")
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        // The newest bounded raw tail first, finite filtering after, matching
        // the catalog input contract.
        let start = pokes.len().saturating_sub(crate::room::MAX_ROOM_POKES);
        let targets: Vec<(f64, f64)> = pokes[start..]
            .iter()
            .copied()
            .filter(|&(x, y)| x.is_finite() && y.is_finite())
            .collect();
        self.render(canvas, t);
        let Some((&newest, older)) = targets.split_last() else {
            return;
        };
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let aspect = safe_aspect(canvas);
        let side = (width as f64 * 0.8).min(height as f64 * aspect * 0.8);
        let left = (width as f64 - side) / 2.0;
        let top = (height as f64 - side / aspect) / 2.0;
        let to_screen = |x: f64, y: f64| {
            (
                (left + x * side) as i32,
                (top + (1.0 - y) * side / aspect) as i32,
            )
        };
        // Every click sends a runner from the left edge: each hop lands with
        // half the remaining distance still to go, so the hops crowd the
        // target without end, and the sweep lays them one by one. The paradox
        // and its answer, walked by hand.
        let phase = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let laid = ((phase * (MAX_TILES as f64 + 1.0)) as usize).clamp(1, MAX_TILES);
        let mut draw_runner = |target: (f64, f64), mark: char| {
            let clamped = (target.0.clamp(0.0, 1.0), target.1.clamp(0.0, 1.0));
            for hop in runner_hops(clamped, laid) {
                let (px, py) = to_screen(hop.0, hop.1);
                canvas.plot(px, py, mark);
            }
            let (tx, ty) = to_screen(clamped.0, clamped.1);
            canvas.plot(tx, ty, '+');
        };
        for &target in older {
            draw_runner(target, '.');
        }
        draw_runner(newest, 'o');
    }

    fn reveal(&self) -> &'static str {
        "Zeno argued the runner never arrives: always half the remaining \
         distance to go, infinitely many steps, so motion is impossible. The \
         square is the answer he did not live to see: one half plus one quarter \
         plus one eighth, infinitely many terms, land exactly inside one square \
         and fill it. An infinite sum can be a finite thing. That single idea is \
         the gate to calculus."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "It took humanity two thousand years to answer Zeno properly: the \
             epsilon-delta limit, built by Cauchy and Weierstrass in the 1800s, \
             is the machinery that says precisely when infinitely many steps \
             arrive somewhere. Your phone computes with it constantly.",
            "Not every infinite sum behaves: one half plus one third plus one \
             quarter plus one fifth, the harmonic series, grows without bound, \
             passing any number you name, given time. Which infinities settle \
             and which explode is a genuine craft, and it has a name: analysis.",
        ]
    }

    fn postcard_t(&self) -> f64 {
        0.75
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "D convergent halves",
            root: 146.83,
            tempo: 80,
            line: &[12, 7, 5, 3, 2, 1, 0, 0],
            encodes: "halves shrinking toward a finite arrival",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_TILES, Zeno, tiles};
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn the_tiles_halve_and_sum_toward_one() {
        let all = tiles();
        assert_eq!(all.len(), MAX_TILES);
        let mut sum = 0.0;
        for (i, &(_, _, w, h)) in all.iter().enumerate() {
            let area = w * h;
            assert!(
                (area - 0.5_f64.powi(i as i32 + 1)).abs() < 1e-12,
                "tile {i} has area 2^-(i+1)"
            );
            sum += area;
        }
        assert!((sum - (1.0 - 0.5_f64.powi(MAX_TILES as i32))).abs() < 1e-12);
        assert!(sum > 0.9999, "the square is all but filled");
    }

    #[test]
    fn the_tiles_do_not_overlap_and_stay_inside() {
        let all = tiles();
        for &(x, y, w, h) in &all {
            assert!(x >= -1e-12 && y >= -1e-12 && x + w <= 1.0 + 1e-12 && y + h <= 1.0 + 1e-12);
        }
        for (i, &(ax, ay, aw, ah)) in all.iter().enumerate() {
            for &(bx, by, bw, bh) in all.iter().skip(i + 1) {
                let overlap_w = (ax + aw).min(bx + bw) - ax.max(bx);
                let overlap_h = (ay + ah).min(by + bh) - ay.max(by);
                assert!(
                    overlap_w <= 1e-9 || overlap_h <= 1e-9,
                    "tiles must not overlap"
                );
            }
        }
    }

    #[test]
    fn render_is_deterministic_and_fills_over_time() {
        let room = Zeno::new();
        let mut early = Canvas::new(50, 30);
        let mut late = Canvas::new(50, 30);
        room.render(&mut early, 0.15);
        room.render(&mut late, 0.9);
        assert!(late.ink_count() > early.ink_count(), "more tiles, more ink");
        let mut again = Canvas::new(50, 30);
        room.render(&mut again, 0.9);
        assert_eq!(late.to_text(), again.to_text());
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Zeno::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_opens_the_gate_to_calculus() {
        assert!(Zeno::new().reveal().contains("calculus"));
    }

    #[test]
    fn the_runner_halves_the_remaining_distance_every_hop() {
        let target = (0.8, 0.4);
        let hops = super::runner_hops(target, MAX_TILES);
        assert_eq!(hops.len(), MAX_TILES);
        let mut remaining = target.0 - 0.0;
        for hop in &hops {
            let left = target.0 - hop.0;
            assert!(
                (left - remaining / 2.0).abs() < 1e-12,
                "each hop leaves exactly half the previous remainder"
            );
            remaining = left;
        }
        let last = hops.last().expect("hops exist");
        assert!(
            (target.0 - last.0).abs() < 0.8 * 0.5_f64.powi(MAX_TILES as i32 - 1),
            "the runner is provably within any epsilon"
        );
    }

    #[test]
    fn a_click_sends_a_visible_runner_and_the_sweep_lays_the_hops() {
        let room = Zeno::new();
        let mut bare = Canvas::new(50, 30);
        room.render(&mut bare, 0.5);
        let mut poked = Canvas::new(50, 30);
        room.render_poked(&mut poked, 0.5, &[(0.85, 0.3)]);
        assert_ne!(bare.to_text(), poked.to_text());
        assert!(poked.to_text().contains('o'), "the hops are visible");
        assert!(poked.to_text().contains('+'), "the target is marked");
        let count_hop_ink = |t: f64| {
            let mut canvas = Canvas::new(50, 30);
            room.render_poked(&mut canvas, t, &[(0.85, 0.3)]);
            canvas.to_text().chars().filter(|&c| c == 'o').count()
        };
        assert!(
            count_hop_ink(0.9) >= count_hop_ink(0.1),
            "the sweep lays more hops"
        );
    }

    #[test]
    fn pokes_use_the_newest_raw_tail_before_filtering() {
        let room = Zeno::new();
        let mut flood: Vec<(f64, f64)> = (0..200).map(|i| (i as f64 / 200.0, 0.7)).collect();
        flood.push((f64::NAN, 0.5));
        flood.push((0.6, 0.2));
        let start = flood.len() - crate::room::MAX_ROOM_POKES;
        let tail = flood[start..].to_vec();
        let mut via_flood = Canvas::new(50, 30);
        room.render_poked(&mut via_flood, 0.5, &flood);
        let mut via_tail = Canvas::new(50, 30);
        room.render_poked(&mut via_tail, 0.5, &tail);
        assert_eq!(via_flood.to_text(), via_tail.to_text());
    }

    #[test]
    fn all_invalid_pokes_render_the_bare_room_and_older_runners_linger() {
        let room = Zeno::new();
        let mut bare = Canvas::new(50, 30);
        room.render(&mut bare, 0.5);
        let mut invalid = Canvas::new(50, 30);
        room.render_poked(&mut invalid, 0.5, &[(f64::NAN, 0.5), (0.5, f64::INFINITY)]);
        assert_eq!(bare.to_text(), invalid.to_text());
        let mut layered = Canvas::new(50, 30);
        room.render_poked(&mut layered, 0.5, &[(0.9, 0.9), (0.85, 0.15)]);
        let text = layered.to_text();
        assert!(text.contains('.'), "the older runner lingers dim");
        assert!(text.contains('o'), "the newest runner is bright");
    }

    #[test]
    fn seed_variation_changes_poked_renders_and_seed_zero_stays_exact() {
        let mut a = Canvas::new(50, 30);
        Zeno::new().render_poked(&mut a, 0.5, &[(0.85, 0.3)]);
        let mut b = Canvas::new(50, 30);
        Zeno::new_with(13).render_poked(&mut b, 0.5, &[(0.85, 0.3)]);
        assert_ne!(a.to_text(), b.to_text(), "tile phase varies with seed");
        let mut exact = Canvas::new(50, 30);
        Zeno::new_with(0).render_poked(&mut exact, 0.5, &[(0.85, 0.3)]);
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
                f64::NAN
            }
            fn plot(&mut self, x: i32, y: i32, mark: char) {
                self.0.plot(x, y, mark);
            }
        }
        let room = Zeno::new();
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

//! Golden Angle: how a sunflower packs its seeds.
//!
//! Place seeds one at a time (Vogel's model): seed `k` sits at angle
//! `k * step` and radius proportional to `sqrt(k)`. At the golden angle
//! (about 137.5 degrees) the seeds pack into a flawless spiral; nudge the angle
//! and the packing shatters into spokes and gaps. `t` detunes the angle. See
//! `docs/ROOMS.md` and `docs/INSIGHTS.md`.

use std::f64::consts::PI;

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

/// How far `t` can push the angle away from golden, in radians. A small nudge is
/// enough to visibly break the packing.
const MAX_DETUNE: f64 = 0.20;
/// Extra seeds planted from one hand point.
const POKE_SEEDS: usize = 18;
/// Domain separator for deterministic per-visit variation.
const VARIATION_SEED: u64 = 0x601D_EA91_5EED_0001;

/// The Golden Angle room.
#[derive(Debug, Default)]
pub struct GoldenAngle {
    seed: u64,
}

impl GoldenAngle {
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

    /// The golden angle in radians: `pi * (3 - sqrt(5))`, about 2.39996.
    fn golden_angle() -> f64 {
        PI * (3.0 - 5.0_f64.sqrt())
    }

    /// The angle between successive seeds at phase `t`; exactly golden at `t = 0`.
    fn angle_step_for(t: f64) -> f64 {
        Self::golden_angle() + Self::phase_for(t) * MAX_DETUNE
    }

    fn phase_for(t: f64) -> f64 {
        if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    fn seed_count(width: usize, height: usize, seed: u64) -> usize {
        let seeds = (width.saturating_mul(height) / 3).clamp(50, 4000);
        let (_, jitter) = Self::variation_offsets(seed);
        (seeds as i32 + jitter).clamp(50, 4000) as usize
    }

    fn variation_offsets(seed: u64) -> (f64, i32) {
        if seed == 0 {
            return (0.0, -5);
        }
        let mut rng = SplitMix64::new(VARIATION_SEED ^ seed);
        let phase_offset = rng.next_f64();
        let count_jitter = rng.below(11) as i32 - 5;
        (phase_offset, count_jitter)
    }

    fn seed_radius(width: usize) -> i32 {
        (width / 250).min(16) as i32
    }

    /// Plot a small filled disc (used for both base seeds and poked extras).
    fn plot_disc(canvas: &mut dyn Surface, cx: i32, cy: i32, r: i32, ch: char) {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r {
                    let x = i64::from(cx) + i64::from(dx);
                    let y = i64::from(cy) + i64::from(dy);
                    if let (Ok(x), Ok(y)) = (i32::try_from(x), i32::try_from(y)) {
                        canvas.plot(x, y, ch);
                    }
                }
            }
        }
    }

    fn screen_cell(px: f64, py: f64, width: usize, height: usize) -> Option<(i32, i32, f64, f64)> {
        if width == 0 || height == 0 || !px.is_finite() || !py.is_finite() {
            return None;
        }
        let px = px.clamp(0.0, 1.0);
        let py = py.clamp(0.0, 1.0);
        let sx = (px * width.saturating_sub(1) as f64).round() as usize;
        let sy = (py * height.saturating_sub(1) as f64).round() as usize;
        let sx = sx.min(width - 1).min(i32::MAX as usize) as i32;
        let sy = sy.min(height - 1).min(i32::MAX as usize) as i32;
        Some((sx, sy, px, py))
    }

    fn planted_seeds(
        pokes: &[(f64, f64)],
        width: usize,
        height: usize,
    ) -> impl Iterator<Item = (i32, i32, f64, f64)> + '_ {
        let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
        pokes[start..]
            .iter()
            .filter_map(move |&(px, py)| Self::screen_cell(px, py, width, height))
    }

    fn mark_origin(canvas: &mut dyn Surface, sx: i32, sy: i32, width: usize, height: usize) {
        let radius = (width.min(height) / 60).clamp(3, 10) as i32;
        canvas.line(
            sx.saturating_sub(radius),
            sy,
            sx.saturating_add(radius),
            sy,
            '#',
        );
        canvas.line(
            sx,
            sy.saturating_sub(radius),
            sx,
            sy.saturating_add(radius),
            '#',
        );
        for offset in -radius..=radius {
            let edge = radius - offset.abs();
            canvas.plot(sx.saturating_add(offset), sy.saturating_sub(edge), '#');
            canvas.plot(sx.saturating_add(offset), sy.saturating_add(edge), '#');
        }
    }
}

impl Room for GoldenAngle {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "golden-angle",
            title: "Golden Angle",
            wing: "Number & Pattern",
            blurb: "Place seeds one at a time, each turned a fixed angle from the last; at the \
                    golden angle they pack into a flawless sunflower, and a nudge shatters it. \
                    t detunes the angle.",
            accent: [210, 160, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let step = Self::angle_step_for(t);
        let (fw, fh) = (width as f64, height as f64);
        let (cx, cy) = (fw / 2.0, fh / 2.0);

        // Scale so the outermost seed just fits both extents (y uses the surface
        // aspect: 0.5 for tall terminal cells, 1.0 for square pixels).
        let aspect = canvas.char_aspect();
        // Use variation for replayable per-visit novelty: rotation + count jitter.
        // Stronger offset than minimal to ensure visible diffs even on small test canvases.
        let (phase_off, _) = Self::variation_offsets(self.seed);
        let seeds = Self::seed_count(width, height, self.seed);
        let scale = (fw / 2.0).min(fh / (2.0 * aspect)) / (seeds as f64).sqrt();

        for k in 0..seeds {
            let theta = k as f64 * step + phase_off;
            let radius = scale * (k as f64).sqrt();
            let x = cx + radius * theta.cos();
            let y = cy + radius * theta.sin() * aspect;
            // Seeds are small discs that scale with the surface, so the
            // sunflower reads at terminal size and at window size alike.
            let seed_radius = Self::seed_radius(width);
            let (sx, sy) = (x.round() as i32, y.round() as i32);
            Self::plot_disc(canvas, sx, sy, seed_radius, '*');
        }
    }

    fn reveal(&self) -> &'static str {
        "Sunflowers, pinecones, and pineapples all use this exact angle, about \
         137.5 degrees, built from the golden ratio, the most irrational number, \
         so the seeds never line up and never waste space. Evolution found the \
         same number the mathematicians did."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Phi's continued fraction is one over one plus one over one plus one over \
             one, all ones, forever, which makes it provably the hardest number to \
             approximate with fractions. That is what most irrational actually means, \
             and why the seeds never line up.",
            "Count the spirals in a real sunflower and you get consecutive Fibonacci \
             numbers, 34 one way and 55 the other, because ratios of consecutive \
             Fibonacci numbers are the best rational approximations to phi. The \
             flower is doing number theory.",
        ]
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "F# phyllotaxis cycle",
            root: 185.0,
            tempo: 118,
            line: &[0, 7, 2, 9, 4, 11, 6, 13],
            encodes: "seeds stepping by an almost-never-repeating turn",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: PLANT A SEED")
    }

    fn status(&self, _t: f64) -> Option<String> {
        Some("GOLDEN ANGLE 137.5 DEG   CLICK: PLANT A SEED".into())
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let planted = inputs
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
        if planted == 0 {
            return self.status(t);
        }
        Some(format!(
            "{planted} BRIGHT SEED {} PLANTED",
            if planted == 1 { "CLUSTER" } else { "CLUSTERS" }
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        if pokes.is_empty() {
            self.render(canvas, t);
            return;
        }
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        // base
        self.render(canvas, t);
        // Poked: plant small local phyllotaxis patches centered on the clicked cells.
        let step = Self::angle_step_for(t);
        let aspect = canvas.char_aspect();
        let (phase_off, _) = Self::variation_offsets(self.seed);
        let scale = (width.min(height) as f64 * 0.16).max(2.0) / (POKE_SEEDS as f64).sqrt();
        let seed_radius = Self::seed_radius(width);
        for (sx, sy, _px, py) in Self::planted_seeds(pokes, width, height) {
            let local_detune = (py - 0.5) * 0.1;
            let local_step = step + local_detune;
            Self::plot_disc(canvas, sx, sy, seed_radius, '#');
            for k in 1..=POKE_SEEDS {
                let theta = k as f64 * local_step + phase_off;
                let radius = scale * (k as f64).sqrt();
                let x = sx as f64 + radius * theta.cos();
                let y = sy as f64 + radius * theta.sin() * aspect;
                Self::plot_disc(canvas, x.round() as i32, y.round() as i32, seed_radius, '#');
            }
            Self::mark_origin(canvas, sx, sy, width, height);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GoldenAngle;
    use crate::MAX_ROOM_POKES;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_uses_clear_singular_and_plural_seed_counts() {
        let room = GoldenAngle::new();
        assert_eq!(
            room.status_input(0.0, &[]).as_deref(),
            room.status(0.0).as_deref()
        );
        let one = [RoomInput::PointerDown {
            x: 0.4,
            y: 0.5,
            t: 0.0,
        }];
        assert_eq!(
            room.status_input(0.0, &one).as_deref(),
            Some("1 BRIGHT SEED CLUSTER PLANTED")
        );
        let mixed = [
            one[0],
            RoomInput::PointerMove {
                x: 0.6,
                y: 0.7,
                t: 0.1,
            },
            RoomInput::PointerDown {
                x: f64::INFINITY,
                y: 0.5,
                t: 0.2,
            },
        ];
        assert_eq!(
            room.status_input(0.0, &mixed).as_deref(),
            Some("2 BRIGHT SEED CLUSTERS PLANTED")
        );
    }

    #[test]
    fn step_is_the_golden_angle_at_zero() {
        assert!((GoldenAngle::angle_step_for(0.0) - 2.399_963_2).abs() < 1e-6);
    }

    #[test]
    fn detuning_increases_the_step() {
        assert!(GoldenAngle::angle_step_for(1.0) > GoldenAngle::angle_step_for(0.0));
    }

    #[test]
    fn nonfinite_phase_falls_back_to_golden_angle() {
        assert_eq!(
            GoldenAngle::angle_step_for(f64::NAN),
            GoldenAngle::angle_step_for(0.0)
        );
        assert_eq!(
            GoldenAngle::angle_step_for(f64::INFINITY),
            GoldenAngle::angle_step_for(0.0)
        );
    }

    #[test]
    fn render_is_deterministic() {
        let room = GoldenAngle::new();
        let mut a = Canvas::new(40, 30);
        let mut b = Canvas::new(40, 30);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = GoldenAngle::new();
        let mut canvas = Canvas::new(40, 30);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 10);
    }

    #[test]
    fn zero_sized_and_extreme_inputs_do_not_panic() {
        let room = GoldenAngle::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(6, 6);
        for t in [f64::NAN, f64::INFINITY, -2.0, 0.0, 0.999, 3.0] {
            room.render(&mut canvas, t);
        }
        room.render_poked(&mut canvas, f64::NAN, &[(f64::INFINITY, f64::NAN)]);
    }

    #[test]
    fn reveal_names_the_angle() {
        assert!(GoldenAngle::new().reveal().contains("137.5"));
    }

    #[test]
    fn new_with_zero_matches_default_and_poked_changes() {
        let r0 = GoldenAngle::new_with(0);
        let r_def = GoldenAngle::new();
        let mut a = Canvas::new(40, 30);
        let mut b = Canvas::new(40, 30);
        r0.render(&mut a, 0.0);
        r_def.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
        let mut cp = Canvas::new(40, 30);
        r0.render_poked(&mut cp, 0.0, &[(0.5, 0.5)]);
        assert!(cp.ink_count() >= a.ink_count());
    }

    #[test]
    fn new_with_nonzero_produces_variation() {
        let r0 = GoldenAngle::new_with(0);
        let r42 = GoldenAngle::new_with(42);
        let mut a = Canvas::new(40, 30);
        let mut c = Canvas::new(40, 30);
        r0.render(&mut a, 0.0);
        r42.render(&mut c, 0.0);
        assert_ne!(
            a.to_text(),
            c.to_text(),
            "variation seed must affect render for replayable visits"
        );
    }

    #[test]
    fn variation_uses_more_than_small_modulo_identity() {
        let r0 = GoldenAngle::new_with(0);
        let r220 = GoldenAngle::new_with(220);
        let mut a = Canvas::new(40, 30);
        let mut b = Canvas::new(40, 30);
        r0.render(&mut a, 0.0);
        r220.render(&mut b, 0.0);
        assert_ne!(
            a.to_text(),
            b.to_text(),
            "variation seeds that collide under simple modulo arithmetic must still differ"
        );
    }

    #[test]
    fn variation_affects_poked_output() {
        let r0 = GoldenAngle::new_with(0);
        let r42 = GoldenAngle::new_with(42);
        let mut base = Canvas::new(40, 30);
        let mut p0 = Canvas::new(40, 30);
        let mut p42 = Canvas::new(40, 30);
        r0.render(&mut base, 0.0);
        r0.render_poked(&mut p0, 0.0, &[(0.3, 0.7)]);
        r42.render_poked(&mut p42, 0.0, &[(0.3, 0.7)]);
        assert_ne!(p0.to_text(), base.to_text());
        assert_ne!(
            p0.to_text(),
            p42.to_text(),
            "poked output must differ under variation"
        );
    }

    #[test]
    fn planted_seeds_preserve_order_clamp_and_filter() {
        let points: Vec<_> = GoldenAngle::planted_seeds(
            &[
                (-1.0, 0.0),
                (f64::NAN, 0.5),
                (0.5, f64::INFINITY),
                (0.5, 0.5),
                (2.0, 1.0),
            ],
            40,
            20,
        )
        .map(|(sx, sy, px, py)| {
            assert!(px.is_finite() && py.is_finite());
            (sx, sy)
        })
        .collect();

        assert_eq!(points, vec![(0, 0), (20, 10), (39, 19)]);
    }

    #[test]
    fn planted_seeds_are_screen_space_faithful_at_edges() {
        let points: Vec<_> =
            GoldenAngle::planted_seeds(&[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)], 40, 20)
                .map(|(sx, sy, _, _)| (sx, sy))
                .collect();

        assert_eq!(points, vec![(0, 0), (39, 0), (0, 19), (39, 19)]);
    }

    #[test]
    fn render_poked_marks_the_clicked_cell() {
        let room = GoldenAngle::new();
        let mut canvas = Canvas::new(40, 20);

        room.render_poked(&mut canvas, 0.0, &[(1.0, 0.0)]);

        let text = canvas.to_text();
        let top_row = text.lines().next().expect("top row");
        assert_eq!(top_row.as_bytes()[39], b'#');
    }

    #[test]
    fn planted_seeds_use_the_newest_bounded_raw_tail() {
        let mut many = vec![(0.0, 0.0); MAX_ROOM_POKES + 3];
        let newest: Vec<_> = many[many.len() - MAX_ROOM_POKES..].to_vec();
        many[0] = (1.0, 1.0);

        let expected: Vec<_> = GoldenAngle::planted_seeds(&newest, 40, 20).collect();
        let actual: Vec<_> = GoldenAngle::planted_seeds(&many, 40, 20).collect();

        assert_eq!(actual, expected);
        assert_eq!(actual.len(), MAX_ROOM_POKES);
    }

    #[test]
    fn nonfinite_pokes_do_not_consume_planted_seed_identity() {
        let room = GoldenAngle::new();
        let finite = [(0.25, 0.25), (0.75, 0.75)];
        let with_bad_points = [(f64::NAN, 0.0), finite[0], (0.0, f64::INFINITY), finite[1]];

        let mut expected = Canvas::new(40, 20);
        let mut actual = Canvas::new(40, 20);
        room.render_poked(&mut expected, 0.5, &finite);
        room.render_poked(&mut actual, 0.5, &with_bad_points);

        assert_eq!(actual.to_text(), expected.to_text());
    }

    #[test]
    fn raw_newest_tail_is_capped_before_nonfinite_filtering() {
        let mut with_invalid_tail = vec![(0.25, 0.25); MAX_ROOM_POKES];
        with_invalid_tail.push((f64::NAN, f64::INFINITY));

        let points: Vec<_> = GoldenAngle::planted_seeds(&with_invalid_tail, 40, 20).collect();

        assert_eq!(points.len(), MAX_ROOM_POKES - 1);
        assert!(points.iter().all(|&(sx, sy, _, _)| (sx, sy) == (10, 5)));
    }

    #[test]
    fn oversized_poke_slices_render_like_their_newest_bounded_tail() {
        let room = GoldenAngle::new();
        let discarded_prefix = vec![(1.0, 1.0), (0.9, 0.1), (0.8, 0.2)];
        let newest: Vec<_> = (0..MAX_ROOM_POKES)
            .map(|i| {
                (
                    (f64::from((i % 7) as u32) + 0.5) / 7.0,
                    (f64::from((i % 5) as u32) + 0.5) / 5.0,
                )
            })
            .collect();
        let mut all = discarded_prefix.clone();
        all.extend_from_slice(&newest);

        let mut expected = Canvas::new(40, 20);
        let mut actual = Canvas::new(40, 20);
        let mut prefix_only = Canvas::new(40, 20);
        room.render_poked(&mut expected, 0.5, &newest);
        room.render_poked(&mut actual, 0.5, &all);
        room.render_poked(&mut prefix_only, 0.5, &discarded_prefix);

        assert_eq!(actual.to_text(), expected.to_text());
        assert_ne!(prefix_only.to_text(), expected.to_text());
    }

    #[test]
    fn huge_surface_dimensions_do_not_panic() {
        struct HugeSurface {
            plots: usize,
        }

        impl crate::surface::Surface for HugeSurface {
            fn width(&self) -> usize {
                usize::MAX
            }

            fn height(&self) -> usize {
                2
            }

            fn plot(&mut self, _x: i32, _y: i32, _mark: char) {
                self.plots = self.plots.saturating_add(1);
            }
        }

        let room = GoldenAngle::new();
        let mut surface = HugeSurface { plots: 0 };
        room.render(&mut surface, 0.0);
        room.render_poked(&mut surface, f64::NAN, &[(1.0, 1.0)]);
        assert!(surface.plots > 0);
    }
}

//! Lissajous: two perpendicular oscillations tracing a curve.
//!
//! One oscillation drives the x axis and another the y axis. When their
//! frequencies form a simple ratio the figure is stable and closed; off-ratio it
//! tumbles. A stable figure is a musical interval you can see. `t` sweeps the
//! second frequency. See `docs/ROOMS.md` and `docs/INSIGHTS.md`.

use std::f64::consts::{FRAC_PI_2, TAU};

use super::variation_unit;
use crate::room::{Room, RoomInput, RoomMeta, pokes_from_inputs};
use crate::sound::SoundSpec;
use crate::surface::Surface;

/// The fixed x-axis frequency; `t` sweeps the y-axis frequency against it.
const FREQ_X: f64 = 3.0;
/// The y-axis frequency at `t = 0` (a 2:3 ratio, a perfect fifth).
const FREQ_Y_MIN: f64 = 2.0;
/// How far `t` sweeps the y-axis frequency.
const FREQ_Y_SWEEP: f64 = 3.0;
/// Number of samples along the curve; consecutive samples are connected.
const SAMPLES: usize = 1500;
/// The largest whole number either oscillator can be tuned to by hand.
const MAX_TUNE: f64 = 8.0;

/// The Lissajous room.
#[derive(Debug, Default)]
pub struct Lissajous {
    seed: u64,
}

impl Lissajous {
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

    /// The y-axis frequency selected by phase `t`.
    fn freq_y_for(t: f64) -> f64 {
        FREQ_Y_MIN + FREQ_Y_SWEEP * t.clamp(0.0, 1.0)
    }

    fn point_normalized_shifted(theta: f64, freq_y: f64, phase_x: f64, phase_y: f64) -> (f64, f64) {
        Self::point_tuned(theta, FREQ_X, freq_y, phase_x, phase_y)
    }

    /// One curve point for any pair of oscillator frequencies.
    fn point_tuned(theta: f64, freq_x: f64, freq_y: f64, phase_x: f64, phase_y: f64) -> (f64, f64) {
        let base_x = (freq_x * theta + FRAC_PI_2).sin();
        let y = (freq_y * theta + phase_y).sin();
        let x = if phase_x == 0.0 {
            base_x
        } else {
            (base_x + (freq_x * theta + FRAC_PI_2 + phase_x).sin() * 0.15).clamp(-1.0, 1.0)
        };
        (x, y)
    }

    fn phase_offsets(&self) -> (f64, f64) {
        (
            variation_unit(self.seed, 0x4C49_5353_584A_0001) * TAU,
            variation_unit(self.seed, 0x4C49_5353_584A_0002) * TAU,
        )
    }

    /// The whole-number frequencies a hand point tunes: x picks the y-axis
    /// count, y picks the x-axis count, both 1 through 8. Every click is an
    /// exact integer ratio, so every figure the hand makes closes: the hand
    /// plays intervals, never noise.
    fn tuned_freqs(x: f64, y: f64) -> (f64, f64) {
        let fy = 1.0 + (x.clamp(0.0, 1.0) * (MAX_TUNE - 1.0)).round();
        let fx = 1.0 + (y.clamp(0.0, 1.0) * (MAX_TUNE - 1.0)).round();
        (fx, fy)
    }

    /// Draw one full closed curve at the given frequencies.
    fn draw_tuned(&self, canvas: &mut dyn Surface, freq_x: f64, freq_y: f64, t: f64, mark: char) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let (fw, fh) = (width as f64, height as f64);
        let (cx, cy) = (fw / 2.0, fh / 2.0);
        let rx = (fw / 2.0 - 1.0).max(0.0);
        let ry = (fh / 2.0 - 1.0).max(0.0);
        let (phase_x, phase_y) = self.phase_offsets();
        let motion = if t.is_finite() {
            t.clamp(0.0, 1.0) * TAU
        } else {
            0.0
        };
        let to_pixel = |theta: f64| -> (i32, i32) {
            let (nx, ny) = Self::point_tuned(theta, freq_x, freq_y, phase_x, phase_y + motion);
            ((cx + nx * rx).round() as i32, (cy + ny * ry).round() as i32)
        };
        let mut prev = to_pixel(0.0);
        for i in 1..=SAMPLES {
            let theta = (i as f64 / SAMPLES as f64) * TAU;
            let current = to_pixel(theta);
            canvas.line(prev.0, prev.1, current.0, current.1, mark);
            prev = current;
        }
    }
}

impl Room for Lissajous {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "lissajous",
            title: "Lissajous",
            wing: "Waves & Sound",
            blurb: "Two perpendicular oscillations, one per axis; a simple frequency ratio traces a \
                    stable figure and off-ratio it tumbles. t sweeps the second frequency.",
            accent: [230, 90, 130],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let freq_y = Self::freq_y_for(if t.is_finite() { t } else { 0.0 });
        let (fw, fh) = (width as f64, height as f64);
        let (cx, cy) = (fw / 2.0, fh / 2.0);
        let rx = (fw / 2.0 - 1.0).max(0.0);
        let ry = (fh / 2.0 - 1.0).max(0.0);
        let (phase_x, phase_y) = self.phase_offsets();

        let to_pixel = |theta: f64| -> (i32, i32) {
            let (nx, ny) = Self::point_normalized_shifted(theta, freq_y, phase_x, phase_y);
            ((cx + nx * rx).round() as i32, (cy + ny * ry).round() as i32)
        };

        let mut prev = to_pixel(0.0);
        for i in 1..=SAMPLES {
            let theta = (i as f64 / SAMPLES as f64) * TAU;
            let current = to_pixel(theta);
            canvas.line(prev.0, prev.1, current.0, current.1, '*');
            prev = current;
        }
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: TUNE THE INTERVAL")
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        // The newest bounded raw tail first, finite filtering after, matching
        // the catalog input contract.
        let start = pokes.len().saturating_sub(crate::room::MAX_ROOM_POKES);
        let tuned: Vec<(f64, f64)> = pokes[start..]
            .iter()
            .copied()
            .filter(|&(x, y)| x.is_finite() && y.is_finite())
            .collect();
        let Some((&newest, older)) = tuned.split_last() else {
            self.render(canvas, t);
            return;
        };
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        // The hand tunes the instrument: clicked ratios replace the sweep.
        // Older intervals linger dim, the newest plays bright, and the
        // clicked cell is marked so the hand stays visible. Tunings quantize
        // to whole numbers, so a drag trail mostly repeats the same interval;
        // drawing each distinct older tuning once keeps a full trail inside
        // the frame budget without changing a single pixel.
        let (fx, fy) = Self::tuned_freqs(newest.0, newest.1);
        let mut drawn: Vec<(f64, f64)> = vec![(fx, fy)];
        for &(x, y) in older {
            let tuning = Self::tuned_freqs(x, y);
            if !drawn.contains(&tuning) {
                drawn.push(tuning);
                self.draw_tuned(canvas, tuning.0, tuning.1, t, '.');
            }
        }
        self.draw_tuned(canvas, fx, fy, t, '*');
        for &(x, y) in &tuned {
            let px = (x.clamp(0.0, 1.0) * (width - 1) as f64).round() as i32;
            let py = (y.clamp(0.0, 1.0) * (height - 1) as f64).round() as i32;
            canvas.plot(px, py, '+');
        }
    }

    fn status(&self, t: f64) -> Option<String> {
        let freq_y = Self::freq_y_for(if t.is_finite() { t } else { 0.0 });
        Some(format!("X:Y = {FREQ_X:.0}:{freq_y:.2}"))
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = pokes_from_inputs(inputs);
        let Some((x, y)) = pokes
            .iter()
            .rev()
            .copied()
            .find(|(x, y)| x.is_finite() && y.is_finite())
        else {
            return self.status(t);
        };
        let (fx, fy) = Self::tuned_freqs(x, y);
        let a = fx.round() as i32;
        let b = fy.round() as i32;
        let g = {
            let mut xg = a.unsigned_abs();
            let mut yg = b.unsigned_abs();
            while yg != 0 {
                let t = yg;
                yg = xg % yg;
                xg = t;
            }
            xg.max(1)
        };
        let ra = a / g as i32;
        let rb = b / g as i32;
        let interval = match (ra.unsigned_abs(), rb.unsigned_abs()) {
            (u, v) if u == v => "UNISON",
            (1, 2) | (2, 1) => "OCTAVE",
            (2, 3) | (3, 2) => "FIFTH",
            (3, 4) | (4, 3) => "FOURTH",
            (3, 5) | (5, 3) => "SIXTH",
            (4, 5) | (5, 4) => "THIRD",
            _ => "RATIO",
        };
        Some(format!("TUNED {fx:.0}:{fy:.0}  {interval}  MOVING"))
    }

    fn reveal(&self) -> &'static str {
        "A rational frequency ratio closes the figure, and small-integer ratios \
         can also sound consonant. The 2:3 ratio is a perfect fifth. You are not \
         just drawing a curve: old oscilloscopes made the same connection between \
         shape and interval visible."
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "G visible fifth",
            root: 196.0,
            tempo: 120,
            line: &[0, 7, 12, 7, 0, 5, 7, 12],
            encodes: "the two oscillator axes locking into a visible chord",
        })
    }

    fn sound(&self, t: f64) -> SoundSpec {
        // The two axis frequencies as a chord. The y axis snaps to its nearest
        // whole number, so the interval you hear is always a clean integer
        // ratio, the room's own lesson that only whole-number tunings close
        // into a figure and ring true. A swept, non-integer value would sound
        // a sour near-unison, which is exactly the noise the room is not about.
        let fy = (Self::freq_y_for(t).round() as f32).max(1.0);
        SoundSpec::chord(&[110.0 * FREQ_X as f32, 110.0 * fy], 1.5, 0.25)
    }
}

#[cfg(test)]
mod tests {
    use super::Lissajous;
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn freq_y_starts_at_two() {
        assert!((Lissajous::freq_y_for(0.0) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn normalized_points_stay_in_range() {
        for i in 0..1000 {
            let theta = f64::from(i) * 0.017;
            let (x, y) = Lissajous::point_normalized_shifted(theta, 2.0, 0.0, 0.0);
            assert!((-1.0..=1.0).contains(&x));
            assert!((-1.0..=1.0).contains(&y));
        }
    }

    #[test]
    fn render_is_deterministic() {
        let room = Lissajous::new();
        let mut a = Canvas::new(40, 24);
        let mut b = Canvas::new(40, 24);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = Lissajous::new();
        let mut canvas = Canvas::new(40, 24);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 10);
    }

    #[test]
    fn zero_sized_and_extreme_inputs_do_not_panic() {
        let room = Lissajous::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(4, 4);
        for t in [-2.0, 0.0, 0.999, 3.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_names_the_interval() {
        let reveal = Lissajous::new().reveal();
        assert!(reveal.contains("rational frequency ratio closes"));
        assert!(reveal.contains("small-integer ratios"));
        assert!(reveal.contains("2:3 ratio is a perfect fifth"));
        assert!(!reveal.contains("stable figure means"));
    }

    #[test]
    fn sound_is_a_two_note_chord() {
        let spec = Lissajous::new().sound(0.0);
        assert_eq!(spec.notes.len(), 2);
    }

    #[test]
    fn a_click_tunes_a_whole_number_interval() {
        // Corners and center map to exact whole-number oscillator counts.
        assert_eq!(Lissajous::tuned_freqs(0.0, 0.0), (1.0, 1.0));
        assert_eq!(Lissajous::tuned_freqs(1.0, 1.0), (8.0, 8.0));
        assert_eq!(Lissajous::tuned_freqs(0.5, 0.0), (1.0, 5.0));
        // Out-of-range input clamps instead of escaping the tuning range.
        assert_eq!(Lissajous::tuned_freqs(9.0, -3.0), (1.0, 8.0));
    }

    #[test]
    fn interaction_status_reports_the_persistent_tuning() {
        let room = Lissajous::new();
        let inputs = crate::room::inputs_from_pokes(&[(0.72, 0.35)], 0.2);
        let early = room.status_input(0.2, &inputs).expect("tuned status");
        let late = room.status_input(0.8, &inputs).expect("tuned status");
        assert_eq!(early, late);
        assert!(early.contains("TUNED 3:6"), "{early}");
        assert!(
            early.contains("OCTAVE") || early.contains("RATIO"),
            "{early}"
        );
        assert!(early.contains("MOVING"), "{early}");
    }

    #[test]
    fn a_poke_changes_the_figure_and_marks_the_hand() {
        let room = Lissajous::new();
        let mut bare = Canvas::new(48, 24);
        room.render(&mut bare, 0.3);
        let mut poked = Canvas::new(48, 24);
        room.render_poked(&mut poked, 0.3, &[(0.9, 0.1)]);
        assert_ne!(bare.to_text(), poked.to_text(), "the tuned figure differs");
        // The clicked cell carries the hand marker.
        assert_eq!(poked.cell((0.9_f64 * 47.0).round() as usize, 2), Some('+'));
    }

    #[test]
    fn a_tuned_interval_keeps_moving_after_the_click() {
        let room = Lissajous::new();
        let click = [(0.83, 0.21)];
        let mut early = Canvas::new(64, 40);
        room.render_poked(&mut early, 0.2, &click);
        let mut late = Canvas::new(64, 40);
        room.render_poked(&mut late, 0.7, &click);

        assert_ne!(early.to_text(), late.to_text());
        let px = (click[0].0 * 63.0_f64).round() as usize;
        let py = (click[0].1 * 39.0_f64).round() as usize;
        assert_eq!(early.cell(px, py), Some('+'));
        assert_eq!(late.cell(px, py), Some('+'));
    }

    #[test]
    fn tuned_motion_is_continuous_at_the_phase_boundary() {
        let room = Lissajous::new();
        let click = [(0.73, 0.36)];
        let mut start = Canvas::new(64, 40);
        room.render_poked(&mut start, 0.0, &click);
        let mut end = Canvas::new(64, 40);
        room.render_poked(&mut end, 1.0, &click);

        assert_eq!(start.to_text(), end.to_text());
        assert_eq!(Lissajous::tuned_freqs(click[0].0, click[0].1), (4.0, 6.0));
    }

    #[test]
    fn pokes_use_the_newest_raw_tail_before_filtering() {
        let room = Lissajous::new();
        // A flood of old points then bad newest entries: the raw tail is
        // capped first, so surviving finite points are honored while the
        // rest are ignored without panicking.
        let mut flood: Vec<(f64, f64)> = (0..200).map(|i| (i as f64 / 200.0, 0.2)).collect();
        flood.push((f64::NAN, 0.5));
        flood.push((0.4, 0.6));
        let start = flood.len() - crate::room::MAX_ROOM_POKES;
        let tail = flood[start..].to_vec();
        let mut via_flood = Canvas::new(48, 24);
        room.render_poked(&mut via_flood, 0.3, &flood);
        let mut via_tail = Canvas::new(48, 24);
        room.render_poked(&mut via_tail, 0.3, &tail);
        assert_eq!(via_flood.to_text(), via_tail.to_text());
    }

    #[test]
    fn all_invalid_pokes_render_the_bare_room_and_older_intervals_linger() {
        let room = Lissajous::new();
        let mut bare = Canvas::new(48, 24);
        room.render(&mut bare, 0.3);
        let mut invalid = Canvas::new(48, 24);
        room.render_poked(&mut invalid, 0.3, &[(f64::NAN, 0.5), (0.5, f64::INFINITY)]);
        assert_eq!(bare.to_text(), invalid.to_text());
        // Two clicks: the older interval lingers dim beneath the newest.
        let mut layered = Canvas::new(48, 24);
        room.render_poked(&mut layered, 0.3, &[(0.1, 0.9), (0.9, 0.1)]);
        let text = layered.to_text();
        assert!(text.contains('.'), "the older interval lingers dim");
        assert!(text.contains('*'), "the newest interval plays bright");
    }

    #[test]
    fn seed_variation_changes_poked_renders_too() {
        let base = Lissajous::new();
        let varied = Lissajous::new_with(7);
        let mut a = Canvas::new(48, 24);
        base.render_poked(&mut a, 0.3, &[(0.7, 0.7)]);
        let mut b = Canvas::new(48, 24);
        varied.render_poked(&mut b, 0.3, &[(0.7, 0.7)]);
        assert_ne!(a.to_text(), b.to_text());
        let mut exact = Canvas::new(48, 24);
        Lissajous::new_with(0).render_poked(&mut exact, 0.3, &[(0.7, 0.7)]);
        assert_eq!(a.to_text(), exact.to_text(), "seed 0 stays the exact path");
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
        let room = Lissajous::new();
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

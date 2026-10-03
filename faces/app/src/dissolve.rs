//! The room dissolve: every change of room passes through the near-black stage.
//!
//! `docs/DESIGN.md` asks that rooms change by dissolving through black, never by
//! a hard cut. The App does it in one place, on the composed frame just before
//! presentation, for every way of changing rooms (`App::enter_room`). The
//! leaving room is held as the last frame it composed and dims to the stage;
//! the arriving room then rises from the stage, live. No frame mixes two rooms,
//! so two additive pictures never double-expose, and the fade only ever lowers
//! a channel toward the stage, so no frame of a change is brighter than the room
//! it shows.
//!
//! The fade runs on presentation time rather than on a frame count, so it lasts
//! the same at any refresh rate. It scales sRGB values, which track lightness
//! closely enough that equal steps of the curve read as equal steps of light.

use numinous_core::Motion;

/// The near-black stage every room is drawn on (`docs/VISUALS.md`). A channel
/// above it dims toward it; a channel already at or below it is left alone, so
/// an era whose stage is darker keeps its own black.
pub(crate) const STAGE: [u8; 3] = [10, 11, 15];

/// The flash guard. An arriving room may begin to rise at most once in this
/// many seconds, however fast rooms are changed.
///
/// A change of room on a bright picture is a fall to the stage and a rise
/// back: one flash in WCAG 2.3.1's terms. Two arrivals a second cannot add up
/// to more than the three flashes a second the standard allows inside any
/// window, so a held arrow key or a fast hand rests on the dark stage instead
/// of strobing. Changing rooms about twice a second or slower never meets the
/// guard; only faster changes hold the stage a little longer.
const ARRIVAL_SPACING: f64 = 0.5;

/// The two halves of a change of room.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Timing {
    /// Seconds for the leaving room to reach the stage.
    leave: f64,
    /// Seconds for the arriving room to rise to full light.
    arrive: f64,
    /// Eased curves, or straight lines.
    eased: bool,
}

/// Full motion. The leaving room accelerates into the stage, so it moves off
/// without lurching, and the arriving room decelerates into full light,
/// settling rather than switching on. Tuned on rendered strips of real rooms
/// at 60 frames a second: a quarter second to leave, as first proposed, barely
/// dimmed in its first 100 ms and read as lag, so the leaving half is a fifth
/// of a second and has lost a quarter of its light by 100 ms. The longer
/// arrival lets the new room bloom; it is half lit about 0.3 s after the
/// press and whole at 0.55 s.
const FULL: Timing = Timing {
    leave: 0.2,
    arrive: 0.35,
    eased: true,
};

/// Reduced motion: a brief plain fade, by the founder's ruling. Straight lines
/// and a fifth of a second in all, so a change of room still never cuts but
/// spends as little time in transition as it can.
const REDUCED: Timing = Timing {
    leave: 0.08,
    arrive: 0.12,
    eased: false,
};

impl Timing {
    fn for_motion(motion: Motion) -> Self {
        if motion.animates() { FULL } else { REDUCED }
    }

    /// Light left in the leaving room, from 1 at `progress` 0 to 0 at 1.
    fn leaving_level(self, progress: f64) -> f64 {
        let p = progress.clamp(0.0, 1.0);
        if self.eased { 1.0 - p * p } else { 1.0 - p }
    }

    /// Where on the leaving curve a given level of light sits.
    fn leaving_progress(self, level: f64) -> f64 {
        let dark = 1.0 - level.clamp(0.0, 1.0);
        if self.eased { dark.sqrt() } else { dark }
    }

    /// Light in the arriving room, from 0 at `progress` 0 to 1 at 1.
    fn arriving_level(self, progress: f64) -> f64 {
        let q = progress.clamp(0.0, 1.0);
        if self.eased {
            1.0 - (1.0 - q) * (1.0 - q)
        } else {
            q
        }
    }
}

/// Where a change of room stands.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    /// No change in progress: frames pass through untouched.
    Steady,
    /// The leaving room dims toward the stage. `progress` runs from 0 to 1 and
    /// then waits at the stage until the flash guard lets the next room rise.
    Leaving { progress: f64 },
    /// The arriving room rises from the stage. `progress` runs from 0 to 1.
    Arriving { progress: f64 },
}

/// The App's one room transition, driven by presentation time.
#[derive(Debug)]
pub(crate) struct Dissolve {
    phase: Phase,
    timing: Timing,
    /// Seconds since the last arrival began to rise, for the flash guard.
    since_arrival: f64,
    /// The last composed frame, undimmed: the leaving room once a change begins.
    held: Vec<u8>,
    held_size: (usize, usize),
    /// The dimmed frame presented while a change is in progress.
    shown: Vec<u8>,
}

impl Default for Dissolve {
    fn default() -> Self {
        Self {
            phase: Phase::Steady,
            timing: FULL,
            since_arrival: f64::INFINITY,
            held: Vec::new(),
            held_size: (0, 0),
            shown: Vec::new(),
        }
    }
}

impl Dissolve {
    /// Begin a change of room. The room itself has already changed; only the
    /// picture is staged.
    ///
    /// A change during the leaving half keeps going to the stage, so rooms
    /// passed through on the way are never shown. A change while a room is
    /// still arriving turns back toward the stage from the light it has
    /// reached, so the level never jumps.
    pub(crate) fn begin(&mut self, motion: Motion) {
        let level = self.level();
        self.timing = Timing::for_motion(motion);
        self.phase = match self.phase {
            Phase::Steady if self.held.is_empty() => {
                self.since_arrival = 0.0;
                Phase::Arriving { progress: 0.0 }
            }
            Phase::Steady => Phase::Leaving { progress: 0.0 },
            Phase::Leaving { .. } | Phase::Arriving { .. } => Phase::Leaving {
                progress: self.timing.leaving_progress(level),
            },
        };
    }

    /// The ordinary audio wash shares the visual fade's nominal duration.
    /// Rapid changes use the player's short, interruption-safe fade instead
    /// of queuing a second long wash behind a picture already leaving.
    pub(crate) fn audio_transition(&self, motion: Motion) -> numinous_audio::Transition {
        if self.phase != Phase::Steady {
            return numinous_audio::Transition::QUICK;
        }
        let timing = Timing::for_motion(motion);
        numinous_audio::Transition::wash((timing.leave + timing.arrive) as f32)
            .expect("the room fade duration is a valid audio wash")
    }

    /// Whether no change of room is in progress.
    #[cfg(test)]
    pub(crate) fn is_steady(&self) -> bool {
        self.phase == Phase::Steady
    }

    /// How much of its own light the presented frame keeps, from 0 (the stage)
    /// to 1 (untouched).
    pub(crate) fn level(&self) -> f64 {
        match self.phase {
            Phase::Steady => 1.0,
            Phase::Leaving { progress } => self.timing.leaving_level(progress),
            Phase::Arriving { progress } => self.timing.arriving_level(progress),
        }
    }

    /// Let `seconds` of presentation time pass. Time that is negative or not
    /// a number moves nothing; unbounded time completes the change.
    pub(crate) fn advance(&mut self, seconds: f64) {
        if seconds.is_nan() || seconds <= 0.0 {
            return;
        }
        let mut left = seconds;
        loop {
            match self.phase {
                Phase::Steady => {
                    self.since_arrival += left;
                    return;
                }
                Phase::Leaving { progress } => {
                    let to_stage = (1.0 - progress) * self.timing.leave;
                    let guard = (ARRIVAL_SPACING - self.since_arrival).max(0.0);
                    let until_arrival = to_stage.max(guard);
                    if left < until_arrival {
                        self.phase = Phase::Leaving {
                            progress: (progress + left / self.timing.leave).min(1.0),
                        };
                        self.since_arrival += left;
                        return;
                    }
                    left -= until_arrival;
                    self.phase = Phase::Arriving { progress: 0.0 };
                    self.since_arrival = 0.0;
                }
                Phase::Arriving { progress } => {
                    let to_full = (1.0 - progress) * self.timing.arrive;
                    if left < to_full {
                        self.phase = Phase::Arriving {
                            progress: progress + left / self.timing.arrive,
                        };
                        self.since_arrival += left;
                        return;
                    }
                    left -= to_full;
                    self.since_arrival += to_full;
                    self.phase = Phase::Steady;
                }
            }
        }
    }

    /// The frame to present for this composed frame.
    ///
    /// Steady frames pass through without a copy and are kept as the frame a
    /// change would leave from. While a room leaves, the live frames of the
    /// room that replaced it wait behind the stage. A window resized in the
    /// middle of a change has no held frame of its size, so the live frame
    /// dims in its place.
    pub(crate) fn frame(&mut self, composed: Vec<u8>, width: usize, height: usize) -> &[u8] {
        match self.phase {
            Phase::Steady => {
                self.held = composed;
                self.held_size = (width, height);
                &self.held
            }
            Phase::Leaving { .. } => {
                let table = level_table(self.level());
                let mut shown = composed;
                if self.held_size == (width, height) && self.held.len() == shown.len() {
                    shown.copy_from_slice(&self.held);
                }
                dim(&mut shown, &table);
                self.shown = shown;
                &self.shown
            }
            Phase::Arriving { .. } => {
                let table = level_table(self.level());
                self.held = composed;
                self.held_size = (width, height);
                self.shown.clear();
                self.shown.extend_from_slice(&self.held);
                dim(&mut self.shown, &table);
                &self.shown
            }
        }
    }
}

/// One lookup per channel for a level of light: channels above the stage move
/// toward it in proportion, channels at or below it stay where they are.
fn level_table(level: f64) -> [[u8; 256]; 3] {
    let level = level.clamp(0.0, 1.0);
    let mut table = [[0_u8; 256]; 3];
    for (channel, row) in table.iter_mut().enumerate() {
        let floor = STAGE[channel];
        for (value, slot) in row.iter_mut().enumerate() {
            let value = value as u8;
            *slot = if value <= floor {
                value
            } else {
                let lit = f64::from(value - floor) * level;
                floor + lit.round() as u8
            };
        }
    }
    table
}

/// Apply a level table to the color channels of an RGBA frame.
fn dim(rgba: &mut [u8], table: &[[u8; 256]; 3]) {
    for pixel in rgba.chunks_exact_mut(4) {
        pixel[0] = table[0][usize::from(pixel[0])];
        pixel[1] = table[1][usize::from(pixel[1])];
        pixel[2] = table[2][usize::from(pixel[2])];
    }
}

#[cfg(test)]
mod tests {
    use super::{ARRIVAL_SPACING, Dissolve, FULL, Phase, REDUCED, STAGE, Timing, level_table};
    use numinous_core::Motion;

    #[test]
    fn audio_washes_share_the_fade_duration_and_interrupt_quickly() {
        for (motion, seconds) in [(Motion::Full, 0.55), (Motion::Reduced, 0.2)] {
            let mut dissolve = Dissolve::default();
            assert_eq!(
                dissolve.audio_transition(motion),
                numinous_audio::Transition::wash(seconds).expect("valid wash")
            );
            dissolve.frame(vec![255; 16], 2, 2);
            dissolve.begin(motion);
            assert_eq!(
                dissolve.audio_transition(motion),
                numinous_audio::Transition::QUICK
            );
            dissolve.advance(10.0);
            assert_eq!(
                dissolve.audio_transition(motion),
                numinous_audio::Transition::wash(seconds).expect("valid wash")
            );
        }
    }
    const FPS: f64 = 60.0;

    fn uniform(rgb: [u8; 3], pixels: usize) -> Vec<u8> {
        [rgb[0], rgb[1], rgb[2], 255].repeat(pixels)
    }

    /// A dissolve that has already presented one steady frame.
    fn presented(motion: Motion) -> Dissolve {
        let mut dissolve = Dissolve::default();
        let _ = dissolve.frame(uniform([200, 180, 160], 4), 2, 2);
        dissolve.begin(motion);
        dissolve.advance(10.0);
        assert!(dissolve.is_steady());
        dissolve
    }

    fn steepest(timing: Timing) -> f64 {
        if timing.eased {
            (2.0 / timing.leave).max(2.0 / timing.arrive)
        } else {
            (1.0 / timing.leave).max(1.0 / timing.arrive)
        }
    }

    #[test]
    fn the_table_is_untouched_at_full_light_and_the_stage_at_none() {
        let full = level_table(1.0);
        let none = level_table(0.0);
        for value in 0..=255_u8 {
            for channel in 0..3 {
                assert_eq!(full[channel][usize::from(value)], value);
                assert_eq!(
                    none[channel][usize::from(value)],
                    value.min(STAGE[channel]),
                    "channel {channel} value {value}"
                );
            }
        }
    }

    #[test]
    fn no_level_brightens_any_channel() {
        for step in 0..=100 {
            let table = level_table(f64::from(step) / 100.0);
            for value in 0..=255_u8 {
                for row in &table {
                    assert!(row[usize::from(value)] <= value);
                }
            }
        }
    }

    #[test]
    fn each_half_is_monotone_and_the_halves_meet_at_the_stage() {
        for timing in [FULL, REDUCED] {
            let mut previous = timing.leaving_level(0.0);
            assert_eq!(previous, 1.0);
            for step in 1..=100 {
                let level = timing.leaving_level(f64::from(step) / 100.0);
                assert!(level <= previous);
                previous = level;
            }
            assert_eq!(previous, 0.0);
            let mut previous = timing.arriving_level(0.0);
            assert_eq!(previous, 0.0);
            for step in 1..=100 {
                let level = timing.arriving_level(f64::from(step) / 100.0);
                assert!(level >= previous);
                previous = level;
            }
            assert_eq!(previous, 1.0);
            for step in 0..=100 {
                let level = f64::from(step) / 100.0;
                let back = timing.leaving_level(timing.leaving_progress(level));
                assert!((back - level).abs() < 1e-12, "{timing:?} {level}");
            }
        }
    }

    #[test]
    fn a_change_leaves_to_the_stage_then_arrives_and_settles() {
        let mut dissolve = presented(Motion::Full);
        dissolve.begin(Motion::Full);
        assert_eq!(dissolve.level(), 1.0, "a change must not jump at its start");
        dissolve.advance(FULL.leave / 2.0);
        assert!(matches!(dissolve.phase, Phase::Leaving { .. }));
        assert!(dissolve.level() < 1.0 && dissolve.level() > 0.0);
        dissolve.advance(FULL.leave / 2.0);
        assert!(matches!(dissolve.phase, Phase::Arriving { .. }));
        assert!(dissolve.level() < 1e-12);
        dissolve.advance(FULL.arrive - 1e-9);
        assert!(!dissolve.is_steady());
        dissolve.advance(1e-6);
        assert!(dissolve.is_steady());
        assert_eq!(dissolve.level(), 1.0);
    }

    #[test]
    fn reduced_motion_takes_a_brief_plain_fade() {
        let reduced = Timing::for_motion(Motion::Reduced);
        let full = Timing::for_motion(Motion::Full);
        assert!(!reduced.eased);
        assert!(reduced.leave + reduced.arrive < (full.leave + full.arrive) / 2.0);
        let mut dissolve = presented(Motion::Reduced);
        dissolve.begin(Motion::Reduced);
        dissolve.advance(REDUCED.leave / 2.0);
        assert!(
            (dissolve.level() - 0.5).abs() < 1e-12,
            "a plain fade is a line"
        );
        dissolve.advance(REDUCED.leave + REDUCED.arrive);
        assert!(dissolve.is_steady());
    }

    #[test]
    fn a_first_change_with_nothing_presented_rises_from_the_stage() {
        let mut dissolve = Dissolve::default();
        dissolve.begin(Motion::Full);
        assert!(matches!(dissolve.phase, Phase::Arriving { progress } if progress == 0.0));
        assert_eq!(dissolve.level(), 0.0);
    }

    #[test]
    fn the_first_arrival_also_counts_toward_the_flash_guard() {
        let mut dissolve = Dissolve::default();
        dissolve.begin(Motion::Reduced);
        dissolve.advance(REDUCED.arrive);
        let _ = dissolve.frame(uniform([255, 255, 255], 4), 2, 2);
        dissolve.begin(Motion::Reduced);
        dissolve.advance(REDUCED.leave);
        assert_eq!(dissolve.level(), 0.0);
        assert!(matches!(dissolve.phase, Phase::Leaving { .. }));
        dissolve.advance(ARRIVAL_SPACING - REDUCED.arrive - REDUCED.leave + 0.01);
        assert!(matches!(dissolve.phase, Phase::Arriving { .. }));
        assert!(dissolve.level() > 0.0);
    }

    #[test]
    fn changing_motion_during_departure_preserves_the_presented_light() {
        for (from, to) in [
            (Motion::Full, Motion::Reduced),
            (Motion::Reduced, Motion::Full),
        ] {
            let mut dissolve = presented(from);
            dissolve.begin(from);
            dissolve.advance(Timing::for_motion(from).leave * 0.5);
            let before = dissolve.level();
            dissolve.begin(to);
            assert!((dissolve.level() - before).abs() < 1e-12);
            dissolve.advance(Timing::for_motion(to).leave * 0.1);
            assert!(dissolve.level() < before);
        }
    }

    #[test]
    fn steady_frames_pass_through_without_a_copy() {
        let mut dissolve = Dissolve::default();
        let frame = uniform([90, 120, 200], 6);
        let address = frame.as_ptr();
        let shown = dissolve.frame(frame.clone(), 3, 2);
        assert_eq!(shown, frame.as_slice());
        let mut dissolve = Dissolve::default();
        assert_eq!(dissolve.frame(frame, 3, 2).as_ptr(), address);
    }

    #[test]
    fn the_leaving_room_is_the_frame_it_showed_and_the_next_waits_behind_the_stage() {
        let leaving = uniform([240, 40, 40], 4);
        let arriving = uniform([40, 40, 240], 4);
        let mut dissolve = Dissolve::default();
        let _ = dissolve.frame(leaving.clone(), 2, 2);
        dissolve.begin(Motion::Full);
        dissolve.advance(FULL.leave / 2.0);
        let shown = dissolve.frame(arriving.clone(), 2, 2).to_vec();
        let table = level_table(dissolve.level());
        assert_eq!(shown[0], table[0][240], "the leaving room is what dims");
        assert_eq!(shown[2], table[2][40]);
        dissolve.advance(FULL.leave);
        dissolve.advance(FULL.arrive / 2.0);
        let shown = dissolve.frame(arriving, 2, 2).to_vec();
        let table = level_table(dissolve.level());
        assert_eq!(shown[2], table[2][240], "then the arriving room rises live");
        assert_eq!(shown[3], 255, "alpha is never touched");
    }

    #[test]
    fn a_window_resized_mid_change_dims_its_live_frame() {
        let mut dissolve = Dissolve::default();
        let _ = dissolve.frame(uniform([200, 200, 200], 4), 2, 2);
        dissolve.begin(Motion::Full);
        dissolve.advance(FULL.leave / 2.0);
        let level = dissolve.level();
        let shown = dissolve.frame(uniform([100, 100, 100], 9), 3, 3).to_vec();
        assert_eq!(shown.len(), 36);
        assert_eq!(shown[0], level_table(level)[0][100]);
    }

    /// Present one frame per tick under a schedule of changes and collect
    /// every level shown.
    fn levels_under(motion: Motion, change_every: usize, frames: usize) -> Vec<f64> {
        let mut dissolve = presented(motion);
        let mut levels = vec![dissolve.level()];
        for frame in 1..=frames {
            if change_every > 0 && frame % change_every == 0 {
                dissolve.begin(motion);
            }
            dissolve.advance(1.0 / FPS);
            levels.push(dissolve.level());
        }
        levels
    }

    #[test]
    fn no_cadence_of_changes_makes_the_light_jump() {
        for motion in [Motion::Full, Motion::Reduced] {
            let bound = steepest(Timing::for_motion(motion)) / FPS + 1e-9;
            for change_every in 1..=90 {
                let levels = levels_under(motion, change_every, 600);
                for pair in levels.windows(2) {
                    assert!(
                        (pair[1] - pair[0]).abs() <= bound,
                        "{motion:?} every {change_every} frames: {} to {}",
                        pair[0],
                        pair[1]
                    );
                }
            }
        }
    }

    #[test]
    fn no_cadence_of_changes_flashes_past_the_photosensitivity_budget() {
        // The worst picture for this: full white on both sides of every
        // change, so each dip to the stage is as deep as a dip can be.
        let white = uniform([255, 255, 255], 4);
        for motion in [Motion::Full, Motion::Reduced] {
            for change_every in 1..=120 {
                let mut dissolve = presented(motion);
                let mut series = Vec::new();
                for frame in 1..=(6 * FPS as usize) {
                    if frame % change_every == 0 {
                        dissolve.begin(motion);
                    }
                    dissolve.advance(1.0 / FPS);
                    let shown = dissolve.frame(white.clone(), 2, 2);
                    series.push(numinous_core::photosensitivity::frame_luminance(shown));
                }
                let peak = numinous_core::photosensitivity::peak_flashes_per_second(&series, FPS);
                assert!(
                    peak <= numinous_core::photosensitivity::MAX_FLASHES_PER_SECOND,
                    "{motion:?} changing every {change_every} frames flashes {peak} a second"
                );
            }
        }
    }

    #[test]
    fn the_flash_guard_holds_the_stage_until_the_spacing_has_passed() {
        let mut dissolve = presented(Motion::Reduced);
        dissolve.begin(Motion::Reduced);
        dissolve.advance(REDUCED.leave);
        assert!(matches!(dissolve.phase, Phase::Arriving { .. }));
        dissolve.advance(REDUCED.arrive / 2.0);
        // Changed again while arriving: back to the stage, where it waits.
        dissolve.begin(Motion::Reduced);
        dissolve.advance(REDUCED.leave);
        assert!(matches!(dissolve.phase, Phase::Leaving { progress } if progress == 1.0));
        assert_eq!(dissolve.level(), 0.0);
        let waited = REDUCED.arrive / 2.0 + REDUCED.leave;
        dissolve.advance(ARRIVAL_SPACING - waited - 1e-6);
        assert_eq!(dissolve.level(), 0.0, "rose before the guard allowed");
        dissolve.advance(2e-6);
        assert!(matches!(dissolve.phase, Phase::Arriving { .. }));
    }

    #[test]
    fn hostile_time_moves_nothing_and_unbounded_time_completes() {
        let mut dissolve = presented(Motion::Full);
        dissolve.begin(Motion::Full);
        dissolve.advance(0.05);
        let level = dissolve.level();
        for seconds in [f64::NAN, -1.0, 0.0, f64::NEG_INFINITY] {
            dissolve.advance(seconds);
            assert_eq!(dissolve.level(), level);
        }
        dissolve.advance(f64::INFINITY);
        assert!(dissolve.is_steady());
    }
}

//! Time-budgeted adaptive resolution for the live room view.
//!
//! CPU rooms pay per pixel: the round-3 audit measured Mandelbrot's CPU
//! fallback at 939ms per frame at 2560x1440 on the dev machine, with Julia
//! (78ms) and Voronoi (60ms) behind it, while most rooms render in under
//! two milliseconds at the same size. A fixed pixel cap would therefore
//! either leave Mandelbrot janky or shrink rooms that were never slow, so
//! the app watches each frame's room render time (the budget bounds that
//! render, not the whole frame; the upscale, era, and blit run outside the
//! measurement and are cheap and constant per window size) and picks an
//! integer downscale factor per frame: a grossly slow frame jumps straight
//! to the predicted factor, mildly slow frames climb one notch after a
//! short streak, and a short streak of comfortably fast frames steps back
//! toward full resolution one notch at a time, with the prediction gap as
//! hysteresis so a borderline room settles on one factor instead of
//! flapping between two. The factor applies only to the room raster in the
//! live window; exports, postcards, modal game frames, the Studio, and the
//! GPU path never pass through it, and the HUD draws after the upscale so
//! text stays window-crisp at any factor.

/// The room render may burn this much per frame before the factor climbs.
/// Two consecutive frames over keeps a lone scheduler hiccup from degrading
/// the picture.
const RAISE_ABOVE_MS: f64 = 33.0;

/// A frame this slow skips the streak and jumps straight to the factor
/// predicted to make budget, so a heavy room costs one hitch, not a stutter
/// ramp. The deliberate tradeoff: a lone OS stall on a fast room also jumps,
/// costing a few quickly-recovered soft frames, while demanding a
/// confirming second frame here would cost seconds of jank on entry to a
/// genuinely heavy room. The cheap mistake is accepted to avoid the
/// expensive one.
const JUMP_ABOVE_MS: f64 = 2.0 * RAISE_ABOVE_MS;

/// How many consecutive mildly slow frames climb the factor one notch.
const RAISE_STREAK: u8 = 2;

/// Step toward full resolution only when the next-lower factor is predicted
/// to stay under this. The gap below `RAISE_ABOVE_MS` is the hysteresis that
/// keeps a borderline room from flapping between two factors.
const LOWER_PREDICTED_BELOW_MS: f64 = 22.0;

/// How many consecutive frames must predict headroom before stepping down.
/// The prediction assumes cost scales with pixel count; for a room whose
/// per-pixel cost varies with content, one anomalously cheap frame (an
/// almost-empty transition, say) must not be trusted on its own, or the
/// factor saws between climb and recovery.
const LOWER_STREAK: u8 = 2;

/// Jumps aim at the recovery threshold rather than the raise threshold so a
/// jump lands with headroom instead of on the line.
const TARGET_MS: f64 = LOWER_PREDICTED_BELOW_MS;

/// Past 8x the picture is mush in every era; a room that cannot make budget
/// at 8x is simply slow, and an honest slow frame beats an unreadable fast
/// one.
const MAX_FACTOR: usize = 8;

/// Chooses how far below window resolution the live room raster renders.
///
/// One instance lives in the app and persists across rooms, so a heavy room
/// hands its factor to the next room, which walks it back one notch per
/// confirmed-fast frame (a handful of soft frames), rather than paying a
/// full-resolution hitch to rediscover what the last frame already measured.
#[derive(Debug)]
pub(crate) struct LiveScale {
    factor: usize,
    slow_streak: u8,
    fast_streak: u8,
}

impl LiveScale {
    pub(crate) fn new() -> Self {
        Self {
            factor: 1,
            slow_streak: 0,
            fast_streak: 0,
        }
    }

    /// Window pixels covered by one rendered pixel along each axis.
    pub(crate) fn factor(&self) -> usize {
        self.factor
    }

    /// The raster size to render for a `width` x `height` window: ceiling
    /// division, so the upscaled image always covers the window.
    pub(crate) fn render_size(&self, width: usize, height: usize) -> (usize, usize) {
        (
            width.div_ceil(self.factor).max(1),
            height.div_ceil(self.factor).max(1),
        )
    }

    /// Feed one frame's room render time in milliseconds; the factor this
    /// adjusts applies from the next frame.
    pub(crate) fn observe(&mut self, render_ms: f64) {
        if !render_ms.is_finite() || render_ms < 0.0 {
            return;
        }
        if render_ms > JUMP_ABOVE_MS {
            self.factor = predicted_factor(render_ms, self.factor);
            self.slow_streak = 0;
            self.fast_streak = 0;
        } else if render_ms > RAISE_ABOVE_MS {
            self.fast_streak = 0;
            self.slow_streak = self.slow_streak.saturating_add(1);
            if self.slow_streak >= RAISE_STREAK {
                self.factor = (self.factor + 1).min(MAX_FACTOR);
                self.slow_streak = 0;
            }
        } else {
            self.slow_streak = 0;
            if self.factor > 1 {
                let down = self.factor - 1;
                if render_ms * ratio_squared(self.factor, down) < LOWER_PREDICTED_BELOW_MS {
                    self.fast_streak = self.fast_streak.saturating_add(1);
                    if self.fast_streak >= LOWER_STREAK {
                        self.factor = down;
                        self.fast_streak = 0;
                    }
                } else {
                    self.fast_streak = 0;
                }
            }
        }
    }
}

/// How much more a frame costs at `to` than at `from`, assuming render time
/// scales with pixel count (both dimensions divide by the factor).
fn ratio_squared(from: usize, to: usize) -> f64 {
    let ratio = from as f64 / to as f64;
    ratio * ratio
}

/// The smallest factor predicted to bring a `render_ms` frame measured at
/// `factor` under `TARGET_MS`, assuming cost scales with pixel count.
fn predicted_factor(render_ms: f64, factor: usize) -> usize {
    if factor >= MAX_FACTOR {
        return MAX_FACTOR;
    }
    let needed = (factor as f64 * (render_ms / TARGET_MS).sqrt()).ceil() as usize;
    needed.clamp(factor + 1, MAX_FACTOR)
}

#[cfg(test)]
mod tests {
    use super::{LiveScale, MAX_FACTOR};

    #[test]
    fn starts_at_full_resolution_and_stays_there_when_fast() {
        let mut scale = LiveScale::new();
        assert_eq!(scale.factor(), 1);
        assert_eq!(scale.render_size(2560, 1440), (2560, 1440));
        for _ in 0..100 {
            scale.observe(2.0);
        }
        assert_eq!(scale.factor(), 1, "fast frames never leave full resolution");
    }

    #[test]
    fn a_grossly_slow_frame_jumps_straight_to_the_predicted_factor() {
        let mut scale = LiveScale::new();
        // The measured Mandelbrot CPU frame: 939ms at 2560x1440. The
        // predicted factor is ceil(sqrt(939 / 22)) = 7, which lands the next
        // frame near 939 / 49 = 19ms, under the recovery threshold.
        scale.observe(939.0);
        assert_eq!(scale.factor(), 7);
        assert_eq!(scale.render_size(2560, 1440), (366, 206));
    }

    #[test]
    fn mildly_slow_frames_need_a_streak_and_climb_one_notch() {
        let mut scale = LiveScale::new();
        scale.observe(40.0);
        assert_eq!(scale.factor(), 1, "one mildly slow frame is a hiccup");
        scale.observe(40.0);
        assert_eq!(scale.factor(), 2, "two in a row climb one notch");
        // 10ms at factor 2 predicts 40ms at factor 1, so recovery holds,
        // but the fast frame still resets the slow streak: the next lone
        // slow frame must not climb.
        scale.observe(40.0);
        scale.observe(10.0);
        scale.observe(40.0);
        assert_eq!(scale.factor(), 2, "a fast frame resets the streak");
    }

    #[test]
    fn recovery_steps_down_one_notch_only_with_confirmed_headroom() {
        let mut scale = LiveScale::new();
        scale.observe(939.0);
        assert_eq!(scale.factor(), 7);
        // 19ms at factor 7 predicts 19 * (7/6)^2 = 25.9ms at factor 6:
        // over the 22ms recovery threshold, so the factor holds.
        scale.observe(19.0);
        scale.observe(19.0);
        assert_eq!(scale.factor(), 7, "no headroom, no step down");
        // 15ms predicts 20.4ms at factor 6: under the threshold, but one
        // cheap frame could be an empty transition, so it takes two.
        scale.observe(15.0);
        assert_eq!(scale.factor(), 7, "one fast frame is not trusted alone");
        scale.observe(15.0);
        assert_eq!(scale.factor(), 6, "a confirmed streak steps down once");
        scale.observe(15.0);
        scale.observe(15.0);
        assert_eq!(scale.factor(), 5, "recovery walks, it does not jump");
    }

    #[test]
    fn a_lone_cheap_frame_between_slow_ones_does_not_step_down() {
        let mut scale = LiveScale::new();
        scale.observe(939.0);
        assert_eq!(scale.factor(), 7);
        // A varying-cost room: an anomalously cheap transition frame
        // surrounded by frames with no headroom must leave the factor
        // alone, or the controller saws between climb and recovery.
        scale.observe(15.0);
        scale.observe(25.0);
        scale.observe(15.0);
        scale.observe(25.0);
        assert_eq!(scale.factor(), 7, "unconfirmed headroom never steps down");
    }

    #[test]
    fn a_borderline_room_settles_instead_of_flapping() {
        let mut scale = LiveScale::new();
        // 40ms at full resolution: climbs to factor 2 after the streak,
        // where the frame costs about a quarter of that. Stepping back to 1
        // predicts 40ms again, over the recovery threshold, so it stays.
        scale.observe(40.0);
        scale.observe(40.0);
        assert_eq!(scale.factor(), 2);
        for _ in 0..50 {
            scale.observe(10.0);
        }
        assert_eq!(scale.factor(), 2, "hysteresis holds the settled factor");
    }

    #[test]
    fn the_factor_is_capped_and_bad_samples_are_ignored() {
        let mut scale = LiveScale::new();
        scale.observe(1_000_000.0);
        assert_eq!(scale.factor(), MAX_FACTOR);
        scale.observe(1_000_000.0);
        assert_eq!(scale.factor(), MAX_FACTOR, "already at the cap");
        scale.observe(f64::NAN);
        scale.observe(f64::INFINITY);
        scale.observe(-5.0);
        assert_eq!(scale.factor(), MAX_FACTOR, "bad samples change nothing");
    }

    #[test]
    fn render_size_covers_the_window_and_never_hits_zero() {
        let mut scale = LiveScale::new();
        scale.observe(939.0); // factor 7
        let (rw, rh) = scale.render_size(2560, 1440);
        assert!(
            rw * 7 >= 2560 && rh * 7 >= 1440,
            "upscale covers the window"
        );
        assert_eq!(scale.render_size(3, 2), (1, 1));
    }
}

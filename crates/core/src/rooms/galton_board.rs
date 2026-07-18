//! Galton Board: pure chance piling into a bell curve.
//!
//! Each ball falls through a field of pegs, taking a left/right coin flip at
//! every row; its final bin is how many times it went right. One probability
//! does not identify the next landing, yet repeated waves at that fixed
//! probability build an empirical binomial distribution. See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::sound::ParametricSound;
use crate::surface::{MAX_DIM, Surface};

mod path_sound;

/// Fixed seed so the pile reproduces exactly (determinism, see `docs/QUALITY.md`).
const SEED: u64 = 0x6A17_0B04_5EED_ABCD;
/// One touch drops enough balls to make progress visible without hiding the
/// early noise that repeated sampling gradually settles.
const BALLS_PER_WAVE: usize = 64;
/// A physical board has one stable geometry at every viewport size.
const BOARD_ROWS: usize = 16;
/// Exact newest-wave mass at every reachable row and right-turn count.
/// Each cell fits in `u8` because one wave contains exactly 64 balls.
type WaveProfile = [[u8; BOARD_ROWS + 1]; BOARD_ROWS + 1];
/// Cap ad hoc trace requests in tests and future callers independently of the
/// viewport. Production experiments always use [`BOARD_ROWS`].
const MAX_TRACE_ROWS: usize = 255;
const SELECTOR_Y: f64 = 0.13;
const BOARD_TOP: f64 = 0.18;
const BOARD_BOTTOM: f64 = 0.55;
const LANDING_Y: f64 = 0.565;
const PILE_TOP: f64 = 0.58;
const PILE_BOTTOM: f64 = 0.74;

fn drawing_dims(canvas: &dyn Surface) -> Option<(usize, usize)> {
    let width = canvas.width();
    let height = canvas.height();
    if width == 0 || height == 0 {
        None
    } else {
        Some((width.min(MAX_DIM), height.min(MAX_DIM)))
    }
}

/// Five deliberately coarse coins keep each experiment repeatable. The fixed
/// probability within a run is part of the binomial model, not presentation.
const COIN_PROBABILITIES: [f64; 5] = [0.30, 0.40, 0.50, 0.60, 0.70];
const COIN_LABELS: [&str; 5] = [".3", ".4", ".5", ".6", ".7"];
/// C, D, E, G, A: one ordered major-pentatonic root for each fixed coin.
const COIN_ROOT_STEPS: [i32; 5] = [0, 2, 4, 7, 9];
const VOICE_ROOT_HZ: f32 = 130.81;
const VOICE_GAIN: f32 = 0.04;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DropWave {
    coin: usize,
}

fn coin_at(x: f64) -> usize {
    ((unit(x, 0.5) * COIN_PROBABILITIES.len() as f64).floor() as usize)
        .min(COIN_PROBABILITIES.len() - 1)
}

/// Map a horizontal hand position onto a landing bin (rights count 0..=rows).
fn bet_bin_at(x: f64) -> usize {
    let bins = BOARD_ROWS + 1;
    ((unit(x, 0.5) * bins as f64).floor() as usize).min(bins - 1)
}

fn drop_waves(pokes: &[(f64, f64)]) -> Vec<DropWave> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .filter_map(|&(px, py)| {
            if px.is_finite() && py.is_finite() {
                Some((px.clamp(0.0, 1.0), py.clamp(0.0, 1.0)))
            } else {
                None
            }
        })
        .map(|(px, _py)| DropWave { coin: coin_at(px) })
        .collect()
}

fn drop_waves_from_inputs(inputs: &[RoomInput]) -> Vec<DropWave> {
    let points: Vec<_> = inputs
        .iter()
        .filter_map(|input| match *input {
            RoomInput::PointerDown { x, y, .. } => Some((x, y)),
            _ => None,
        })
        .collect();
    drop_waves(&points)
}

/// The newest finite pointer-move commits a one-ball landing bet. Downs never
/// set the bet: a click still only drops waves, so the wager is a separate
/// gesture beat before the result is graded against the last ball path.
fn bet_bin_from_inputs(inputs: &[RoomInput]) -> Option<usize> {
    inputs.iter().rev().find_map(|input| match *input {
        RoomInput::PointerMove { x, y, .. } if x.is_finite() && y.is_finite() => {
            Some(bet_bin_at(x))
        }
        _ => None,
    })
}

fn selected_run(waves: &[DropWave]) -> Option<(usize, usize)> {
    let selected = waves.last()?.coin;
    let wave_count = waves
        .iter()
        .rev()
        .take_while(|wave| wave.coin == selected)
        .count();
    Some((selected, wave_count))
}

fn probability_voice(coin: usize) -> ParametricSound {
    let coin = coin.min(COIN_PROBABILITIES.len() - 1);
    let p = COIN_PROBABILITIES[coin];
    let q = 1.0 - p;
    let root_hz = crate::chiptune::pitch(VOICE_ROOT_HZ, COIN_ROOT_STEPS[coin]);
    let odds = (p.max(q) / p.min(q)) as f32;
    ParametricSound::new(root_hz, odds, VOICE_GAIN)
        .expect("fixed Galton probabilities make a valid voice")
}

/// The Galton Board room.
#[derive(Debug, Default)]
pub struct GaltonBoard {
    seed: u64,
}

impl GaltonBoard {
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

    fn experiment_variation(variation: u64, coin: usize) -> u64 {
        variation ^ (coin as u64).wrapping_mul(0xD1B5_4A32_D192_ED03)
    }

    fn experiment_histogram(coin: usize, wave_count: usize, variation: u64) -> Vec<u64> {
        let mut counts = vec![0u64; BOARD_ROWS + 1];
        let coin = coin.min(COIN_PROBABILITIES.len() - 1);
        let p_right = COIN_PROBABILITIES[coin];
        let variation = Self::experiment_variation(variation, coin);
        let ball_count = wave_count
            .min(MAX_ROOM_POKES)
            .saturating_mul(BALLS_PER_WAVE);
        for ball in 0..ball_count {
            let bin = Self::landing_bin(BOARD_ROWS, p_right, variation, ball);
            counts[bin] = counts[bin].saturating_add(1);
        }
        counts
    }

    fn trace_seed(variation: u64, which: usize) -> u64 {
        SEED ^ variation ^ 0xB411_7ACE_u64 ^ (which as u64).wrapping_mul(0x9E37_79B9)
    }

    fn landing_bin(rows: usize, p_right: f64, variation: u64, which: usize) -> usize {
        Self::walk_ball(rows, p_right, variation, which, |_, _| {})
    }

    fn walk_ball(
        rows: usize,
        p_right: f64,
        variation: u64,
        which: usize,
        mut visit: impl FnMut(usize, usize),
    ) -> usize {
        let rows = rows.clamp(1, MAX_TRACE_ROWS);
        let p_right = unit(p_right, 0.5);
        let mut rng = SplitMix64::new(Self::trace_seed(variation, which));
        let mut rights = 0;
        visit(0, rights);
        for row in 1..=rows {
            rights += usize::from(rng.next_f64() < p_right);
            visit(row, rights);
        }
        rights
    }

    fn ball_trace(rows: usize, p_right: f64, variation: u64, which: usize) -> Vec<usize> {
        let rows = rows.clamp(1, MAX_TRACE_ROWS);
        let mut trace = Vec::with_capacity(rows + 1);
        Self::walk_ball(rows, p_right, variation, which, |_, rights| {
            trace.push(rights);
        });
        trace
    }

    fn newest_ball_trace(&self, waves: &[DropWave]) -> Option<(usize, usize, Vec<usize>)> {
        let (coin, wave_count) = selected_run(waves)?;
        let which = wave_count.saturating_mul(BALLS_PER_WAVE).saturating_sub(1);
        let trace = Self::ball_trace(
            BOARD_ROWS,
            COIN_PROBABILITIES[coin],
            Self::experiment_variation(self.seed, coin),
            which,
        );
        Some((coin, wave_count, trace))
    }

    fn newest_wave_profile(&self, coin: usize, wave_count: usize) -> WaveProfile {
        let mut profile = [[0u8; BOARD_ROWS + 1]; BOARD_ROWS + 1];
        if wave_count == 0 {
            return profile;
        }
        let coin = coin.min(COIN_PROBABILITIES.len() - 1);
        let wave_count = wave_count.min(MAX_ROOM_POKES);
        let first_ball = (wave_count - 1) * BALLS_PER_WAVE;
        let variation = Self::experiment_variation(self.seed, coin);
        for ball in first_ball..first_ball + BALLS_PER_WAVE {
            Self::walk_ball(
                BOARD_ROWS,
                COIN_PROBABILITIES[coin],
                variation,
                ball,
                |row, rights| {
                    profile[row][rights] = profile[row][rights].saturating_add(1);
                },
            );
        }
        profile
    }

    fn board_point(
        width: usize,
        top: f64,
        bottom: f64,
        row: usize,
        rights: usize,
        rows: usize,
    ) -> (i32, i32) {
        let span = width.saturating_sub(1) as f64 * 0.82;
        let step = span / rows.max(1) as f64;
        let lateral = (2.0 * rights as f64 - row as f64) * step / 2.0;
        let x = width.saturating_sub(1) as f64 / 2.0 + lateral;
        let y = top + (bottom - top) * row as f64 / rows.max(1) as f64;
        (x.round() as i32, y.round() as i32)
    }

    fn reference_weights(coin: usize) -> [f64; BOARD_ROWS + 1] {
        let p = COIN_PROBABILITIES[coin.min(COIN_PROBABILITIES.len() - 1)];
        let q = 1.0 - p;
        let mut weights = [0.0; BOARD_ROWS + 1];
        weights[0] = q.powi(BOARD_ROWS as i32);
        for k in 0..BOARD_ROWS {
            weights[k + 1] = weights[k] * (BOARD_ROWS - k) as f64 / (k + 1) as f64 * p / q;
        }
        weights
    }

    fn draw_coin_selector(canvas: &mut dyn Surface, selected: usize) {
        let Some((width, height)) = drawing_dims(canvas) else {
            return;
        };
        let left = width.saturating_sub(1) as f64 * 0.18;
        let right = width.saturating_sub(1) as f64 * 0.82;
        let y = (height as f64 * SELECTOR_Y).round() as i32;
        let left = left.round() as i32;
        let right = right.round() as i32;
        canvas.line(left, y, right, y, '-');
        if width >= 120 && height >= 80 {
            canvas.line(left, y - 1, right, y - 1, '-');
            canvas.line(left, y + 1, right, y + 1, '-');
        }
        for (coin, label) in COIN_LABELS.iter().enumerate() {
            let fraction = coin as f64 / (COIN_PROBABILITIES.len() - 1) as f64;
            let x = (left as f64 + (right - left) as f64 * fraction).round() as i32;
            let half_tick = if coin == selected && height >= 20 {
                3
            } else {
                2
            };
            canvas.line(x, y - half_tick, x, y + half_tick, '+');
            if width >= 120 && height >= 80 {
                crate::draw_text(canvas, label, x - 5, y + 3, 1, '#');
                if coin == selected {
                    canvas.line(x - 6, y + 11, x + 6, y + 11, '#');
                }
            }
        }
    }

    fn draw_reference(canvas: &mut dyn Surface, rows: usize, coin: usize) {
        let Some((width, height)) = drawing_dims(canvas) else {
            return;
        };
        let top = height as f64 * BOARD_TOP;
        let board_bottom = height as f64 * BOARD_BOTTOM;
        let pile_top = height as f64 * PILE_TOP;
        let pile_bottom = height as f64 * PILE_BOTTOM;
        let weights = Self::reference_weights(coin);
        let max = weights.iter().copied().fold(0.0_f64, f64::max);
        let mut previous = None;
        for (bin, weight) in weights.into_iter().enumerate() {
            let (x, _) = Self::board_point(width, top, board_bottom, rows, bin, rows);
            let y = pile_bottom - weight / max * (pile_bottom - pile_top);
            if let Some((px, py)) = previous {
                canvas.line(px, py, x, y.round() as i32, ':');
            }
            previous = Some((x, y.round() as i32));
        }
    }

    fn draw_board(canvas: &mut dyn Surface, rows: usize, counts: &[u64], selected: usize) {
        let Some((width, height)) = drawing_dims(canvas) else {
            return;
        };
        Self::draw_coin_selector(canvas, selected);
        let top = height as f64 * BOARD_TOP;
        let board_bottom = height as f64 * BOARD_BOTTOM;
        let pile_top = height as f64 * PILE_TOP;
        let pile_bottom = height as f64 * PILE_BOTTOM;
        for row in 0..rows {
            for rights in 0..=row {
                let (x, y) = Self::board_point(width, top, board_bottom, row, rights, rows);
                canvas.plot(x, y, '.');
                if width >= 80 && height >= 40 {
                    canvas.plot(x - 1, y, '.');
                    canvas.plot(x + 1, y, '.');
                }
            }
        }
        Self::draw_reference(canvas, rows, selected);
        let max = counts.iter().copied().max().unwrap_or(0);
        if max == 0 {
            return;
        }
        let bin_width = (width as f64 * 0.82 / rows.max(1) as f64).max(1.0);
        for (bin, &count) in counts.iter().enumerate() {
            let (center, _) = Self::board_point(width, top, board_bottom, rows, bin, rows);
            let half = (bin_width * 0.42).max(1.0) as i32;
            let bar = ((count as f64 / max as f64) * (pile_bottom - pile_top)).round() as i32;
            for x in center - half..=center + half {
                for dy in 0..=bar {
                    canvas.plot(x, pile_bottom as i32 - dy, '*');
                }
            }
        }
    }

    fn draw_ball_trace(canvas: &mut dyn Surface, trace: &[usize], rows: usize) {
        let Some((width, height)) = drawing_dims(canvas) else {
            return;
        };
        if trace.is_empty() {
            return;
        }
        let top = height as f64 * BOARD_TOP;
        let board_bottom = height as f64 * BOARD_BOTTOM;
        let mut previous = None;
        for (row, &rights) in trace.iter().enumerate() {
            let (sx, sy) = Self::board_point(width, top, board_bottom, row, rights, rows);
            if let Some((px, py)) = previous {
                canvas.line(px, py, sx, sy, 'o');
                if width >= 80 && height >= 40 {
                    canvas.line(px - 1, py, sx - 1, sy, 'o');
                    canvas.line(px + 1, py, sx + 1, sy, 'o');
                }
            } else {
                canvas.plot(sx, sy, 'o');
            }
            previous = Some((sx, sy));
        }
        if let Some(&rights) = trace.last() {
            let (sx, _) = Self::board_point(width, top, board_bottom, rows, rights, rows);
            canvas.line(
                sx - 2,
                (height as f64 * LANDING_Y) as i32,
                sx + 2,
                (height as f64 * LANDING_Y) as i32,
                '#',
            );
        }
    }
}

fn unit(value: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        fallback
    }
}

impl Room for GaltonBoard {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "galton-board",
            title: "Galton Board",
            wing: "Chance & Order",
            blurb: "Choose a left, fair, or right-leaning coin. Each click drops 64 balls through \
                    16 peg rows; repeat it and watch chance settle into a binomial pile.",
            accent: [80, 120, 220],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, _t: f64) {
        let Some((_width, _height)) = drawing_dims(canvas) else {
            return;
        };
        Self::draw_board(canvas, BOARD_ROWS, &[], 2);
    }

    fn reveal(&self) -> &'static str {
        "The coin probability alone does not determine the next landing. With one \
         probability fixed, the number of right turns in a 16-flip landing follows \
         exactly Binomial(16, p), and repeated waves make the empirical pile estimate \
         that discrete distribution. With many rows and a coin away from either \
         extreme, a normal curve can approximate the binomial, the direction formalized \
         by the Central Limit Theorem. This board displays the finite binomial itself."
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "C bell curve",
            root: 130.81,
            tempo: 120,
            line: &[0, 2, 4, 5, 7, 5, 4, 2, 0],
            encodes: "left-right falls gathering around the center",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("AIM + CLICK: PICK COIN, DROP 64 BALLS")
    }

    fn status(&self, _t: f64) -> Option<String> {
        Some("5 COINS  DROP 64  MOVE TO BET ONE BALL".into())
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let waves = drop_waves_from_inputs(inputs);
        let bet = bet_bin_from_inputs(inputs);
        let Some((coin, wave_count, trace)) = self.newest_ball_trace(&waves) else {
            return match bet {
                Some(bin) => Some(format!("BET {bin}/{BOARD_ROWS}  CLICK DROP 64")),
                None => self.status(t),
            };
        };
        let p_right = COIN_PROBABILITIES[coin];
        let rights = trace.last().copied().unwrap_or(0);
        let balls = wave_count.saturating_mul(BALLS_PER_WAVE);
        let counts = Self::experiment_histogram(coin, wave_count, self.seed);
        let weighted: f64 = counts
            .iter()
            .enumerate()
            .map(|(bin, &count)| bin as f64 * count as f64)
            .sum();
        let mean = if balls == 0 {
            0.0
        } else {
            weighted / balls as f64
        };
        let expected = BOARD_ROWS as f64 * p_right;
        let probability = format!("{p_right:.2}");
        let probability = probability.strip_prefix('0').unwrap_or(&probability);
        // Compact status: must fit App compact footer budgets (360 wide).
        // M~E is empirical mean versus binomial expectation np.
        // Optional B{n}H/M grades a move-committed one-ball bet against the
        // highlighted last ball of the contiguous run (the prediction beat).
        let grade = bet.map(|bin| {
            if bin == rights {
                format!(" B{bin}H")
            } else {
                format!(" B{bin}M")
            }
        });
        let grade = grade.as_deref().unwrap_or("");
        if wave_count == MAX_ROOM_POKES {
            Some(format!(
                "P{probability} FULL={balls} M{mean:.1}~{expected:.1} L{rights}R{grade}"
            ))
        } else {
            Some(format!(
                "P{probability} {wave_count}x64={balls} M{mean:.1}~{expected:.1} L{rights}R{grade}"
            ))
        }
    }

    fn parameter_sound(&self, _t: f64, inputs: &[RoomInput]) -> Option<ParametricSound> {
        let waves = drop_waves_from_inputs(inputs);
        let (coin, _) = selected_run(&waves)?;
        Some(probability_voice(coin))
    }

    fn interaction_stereo(&self, inputs: &[RoomInput], sample_rate: u32) -> Option<Vec<f32>> {
        if !matches!(
            inputs.last(),
            Some(RoomInput::PointerDown { x, y, .. }) if x.is_finite() && y.is_finite()
        ) || !path_sound::supports_sample_rate(sample_rate)
        {
            return None;
        }
        let waves = drop_waves_from_inputs(inputs);
        let (coin, wave_count, trace) = self.newest_ball_trace(&waves)?;
        let profile = self.newest_wave_profile(coin, wave_count);
        let root = crate::chiptune::pitch(VOICE_ROOT_HZ * 2.0, COIN_ROOT_STEPS[coin]);
        Some(path_sound::render(
            root,
            &trace,
            &profile,
            BOARD_ROWS,
            sample_rate,
        ))
    }

    fn render_input(&self, canvas: &mut dyn Surface, t: f64, inputs: &[RoomInput]) {
        let waves = drop_waves_from_inputs(inputs);
        self.render_waves(canvas, t, &waves);
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        if pokes.is_empty() {
            self.render(canvas, t);
            return;
        }
        let waves = drop_waves(pokes);
        self.render_waves(canvas, t, &waves);
    }
}

impl GaltonBoard {
    fn render_waves(&self, canvas: &mut dyn Surface, t: f64, waves: &[DropWave]) {
        let Some((coin, wave_count, trace)) = self.newest_ball_trace(waves) else {
            self.render(canvas, t);
            return;
        };
        let counts = Self::experiment_histogram(coin, wave_count, self.seed);
        Self::draw_board(canvas, BOARD_ROWS, &counts, coin);
        Self::draw_ball_trace(canvas, &trace, BOARD_ROWS);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BALLS_PER_WAVE, BOARD_ROWS, BOARD_TOP, COIN_PROBABILITIES, COIN_ROOT_STEPS, DropWave,
        GaltonBoard, VOICE_ROOT_HZ, bet_bin_at, coin_at, drop_waves, drop_waves_from_inputs,
        path_sound, selected_run,
    };
    use crate::canvas::Canvas;
    use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
    use crate::stereo_signal_metrics;
    use crate::surface::{MAX_DIM, Surface};

    #[test]
    fn newest_wave_has_one_bounded_stereo_event_with_a_highlighted_path() {
        let room = GaltonBoard::new_with(7);
        let first = [RoomInput::PointerDown {
            x: 0.5,
            y: 0.5,
            t: 0.2,
        }];
        let samples = room
            .interaction_stereo(&first, 48_000)
            .expect("accepted wave has a peg sequence");

        assert_eq!(samples.len(), 48_000);
        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(samples.iter().all(|sample| sample.abs() <= 1.0));
        let metrics = stereo_signal_metrics(&samples);
        assert_eq!(metrics.trailing_samples, 0);
        assert_eq!(metrics.non_finite_samples, 0);
        assert_eq!(metrics.clipped_samples, 0);
        assert!((0.03..0.2).contains(&metrics.peak));
        assert!((0.003..0.06).contains(&metrics.rms));
        assert!(metrics.max_step < 0.04);
        assert!(metrics.side_to_mid_db > -60.0);
        assert_eq!(
            samples,
            room.interaction_stereo(&first, 48_000)
                .expect("same wave remains deterministic")
        );
        assert!(
            samples
                .chunks_exact(2)
                .any(|frame| (frame[0] - frame[1]).abs() > 1e-5),
            "the visible horizontal path must move across stereo"
        );

        let waves = drop_waves_from_inputs(&first);
        let (_, _, trace) = room.newest_ball_trace(&waves).expect("the rendered path");
        let rights = *trace.last().expect("landing");
        let landing = &samples[(0.4 * 48_000.0) as usize * 2..];
        let (left, right) = landing.chunks_exact(2).fold((0.0, 0.0), |energy, frame| {
            (
                energy.0 + frame[0] * frame[0],
                energy.1 + frame[1] * frame[1],
            )
        });
        match rights.cmp(&(BOARD_ROWS / 2)) {
            std::cmp::Ordering::Less => assert!(left > right),
            std::cmp::Ordering::Equal => assert!((left - right).abs() < 1e-5),
            std::cmp::Ordering::Greater => assert!(right > left),
        }
    }

    #[test]
    fn newest_wave_profile_conserves_all_64_balls_at_every_row() {
        let room = GaltonBoard::new_with(7);
        let coin = 3;
        let wave_count = 4;
        let profile = room.newest_wave_profile(coin, wave_count);

        for (row, counts) in profile.iter().enumerate() {
            assert_eq!(
                counts
                    .iter()
                    .map(|&count| usize::from(count))
                    .sum::<usize>(),
                BALLS_PER_WAVE,
                "row {row} must account for every ball"
            );
            assert!(counts[row + 1..].iter().all(|&count| count == 0));
        }

        assert_eq!(
            profile[BOARD_ROWS],
            [0, 0, 0, 0, 0, 2, 2, 6, 16, 11, 10, 7, 7, 2, 0, 1, 0]
        );
    }

    #[test]
    fn newest_wave_profile_pins_stream_range_and_highlight_identity() {
        let room = GaltonBoard::new_with(7);
        let coin = 3;
        let wave_count = 4;
        let profile = room.newest_wave_profile(coin, wave_count);
        let (_, _, highlighted) = room
            .newest_ball_trace(&[DropWave { coin }; 4])
            .expect("newest highlighted ball");

        assert_eq!(
            highlighted,
            [0, 1, 2, 3, 4, 4, 4, 5, 6, 6, 7, 7, 8, 9, 10, 11, 12]
        );

        let fingerprint =
            profile
                .iter()
                .flatten()
                .fold(0xcbf2_9ce4_8422_2325u64, |fingerprint, &count| {
                    (fingerprint ^ u64::from(count)).wrapping_mul(0x0000_0100_0000_01B3)
                });
        assert_eq!(fingerprint, 0xAEE6_F8D9_2E20_13EF);

        let mut without_highlight = profile;
        for (row, &rights) in highlighted.iter().enumerate() {
            without_highlight[row][rights] -= 1;
        }
        assert!(without_highlight.iter().all(|row| {
            row.iter().map(|&count| usize::from(count)).sum::<usize>() == BALLS_PER_WAVE - 1
        }));
    }

    #[test]
    fn all_ball_mass_changes_the_event_beyond_the_highlighted_path() {
        let room = GaltonBoard::new_with(7);
        let waves = drop_waves(&[(0.7, 0.5)]);
        let (coin, wave_count, trace) = room.newest_ball_trace(&waves).expect("newest path");
        let profile = room.newest_wave_profile(coin, wave_count);
        let empty = [[0u8; BOARD_ROWS + 1]; BOARD_ROWS + 1];
        let root = crate::chiptune::pitch(VOICE_ROOT_HZ * 2.0, COIN_ROOT_STEPS[coin]);

        let full = path_sound::render(root, &trace, &profile, BOARD_ROWS, 48_000);
        let highlighted_only = path_sound::render(root, &trace, &empty, BOARD_ROWS, 48_000);

        assert_ne!(full, highlighted_only);
        let difference = full
            .iter()
            .zip(highlighted_only)
            .map(|(full, highlighted)| (full - highlighted).abs())
            .sum::<f32>();
        assert!(difference > 1.0, "wave texture must carry audible energy");
    }

    #[test]
    fn full_wave_texture_stereo_centroid_follows_coin_bias() {
        let room = GaltonBoard::new_with(7);
        let left_profile = room.newest_wave_profile(0, 1);
        let right_profile = room.newest_wave_profile(4, 1);
        let left = path_sound::render(261.63, &[], &left_profile, BOARD_ROWS, 48_000);
        let right = path_sound::render(261.63, &[], &right_profile, BOARD_ROWS, 48_000);
        let energy = |samples: &[f32], channel: usize| {
            samples
                .chunks_exact(2)
                .map(|frame| frame[channel] * frame[channel])
                .sum::<f32>()
        };

        assert!(energy(&left, 0) > energy(&left, 1));
        assert!(energy(&right, 1) > energy(&right, 0));
    }

    #[test]
    fn same_pitch_energy_depends_on_mass_not_cell_partition() {
        let mut concentrated = [[0u8; BOARD_ROWS + 1]; BOARD_ROWS + 1];
        concentrated[8][0] = 64;
        let mut split = [[0u8; BOARD_ROWS + 1]; BOARD_ROWS + 1];
        split[8][0] = 32;
        split[8][1] = 32;

        let concentrated = path_sound::render(261.63, &[], &concentrated, BOARD_ROWS, 48_000);
        let split = path_sound::render(261.63, &[], &split, BOARD_ROWS, 48_000);
        let energy = |samples: &[f32]| samples.iter().map(|sample| sample * sample).sum::<f32>();
        let concentrated_energy = energy(&concentrated);
        let split_energy = energy(&split);

        assert!(concentrated_energy > 0.0);
        assert!(
            (concentrated_energy - split_energy).abs() / concentrated_energy < 0.001,
            "equal mass in one pitch bucket must have equal total energy"
        );
    }

    #[test]
    fn wave_event_tracks_only_an_accepted_newest_wave() {
        let room = GaltonBoard::new_with(11);
        let moved = [RoomInput::PointerMove {
            x: 0.8,
            y: 0.5,
            t: 0.2,
        }];
        assert!(room.interaction_stereo(&[], 48_000).is_none());
        assert!(room.interaction_stereo(&moved, 48_000).is_none());

        let first = RoomInput::PointerDown {
            x: 0.5,
            y: 0.5,
            t: 0.2,
        };
        let second = RoomInput::PointerDown {
            x: 0.5,
            y: 0.5,
            t: 0.4,
        };
        let one = room
            .interaction_stereo(&[first], 48_000)
            .expect("first newest path");
        let two = room
            .interaction_stereo(&[first, second], 48_000)
            .expect("second newest path");
        assert_ne!(one, two, "the next visible ball has its own path");
        assert!(
            room.interaction_stereo(&[first, moved[0]], 48_000)
                .is_none(),
            "a later bet move cannot replay the previous drop"
        );
        assert!(
            room.interaction_stereo(
                &[
                    first,
                    RoomInput::PointerUp {
                        x: 0.5,
                        y: 0.5,
                        t: 0.5,
                    },
                ],
                48_000,
            )
            .is_none(),
            "release cannot replay the previous drop"
        );
    }

    #[test]
    fn wave_event_preserves_supported_rates_and_rejects_hostile_rates() {
        let room = GaltonBoard::new();
        let input = [RoomInput::PointerDown {
            x: 0.5,
            y: 0.5,
            t: 0.2,
        }];
        for sample_rate in [8_000, 44_100, 48_000, 96_000, 192_000] {
            let samples = room
                .interaction_stereo(&input, sample_rate)
                .expect("supported native-rate sequence");
            assert_eq!(samples.len(), sample_rate as usize);
        }
        assert!(room.interaction_stereo(&input, 0).is_none());
        assert!(room.interaction_stereo(&input, 7_999).is_none());
        assert!(room.interaction_stereo(&input, 192_001).is_none());
        assert!(room.interaction_stereo(&input, u32::MAX).is_none());
    }

    #[test]
    fn full_wave_texture_is_safe_at_coin_and_rate_extremes() {
        let room = GaltonBoard::new_with(19);
        for x in [0.0, 0.5, 1.0] {
            let input = [RoomInput::PointerDown { x, y: 0.5, t: 0.2 }];
            for sample_rate in [8_000, 192_000] {
                let samples = room
                    .interaction_stereo(&input, sample_rate)
                    .expect("supported full-wave texture");
                let metrics = stereo_signal_metrics(&samples);
                assert_eq!(metrics.trailing_samples, 0);
                assert_eq!(metrics.non_finite_samples, 0);
                assert_eq!(metrics.subnormal_samples, 0);
                assert_eq!(metrics.clipped_samples, 0);
                assert!((0.03..0.25).contains(&metrics.peak));
                assert!((0.003..0.06).contains(&metrics.rms));
                assert!(metrics.left_dc.abs() < 0.002);
                assert!(metrics.right_dc.abs() < 0.002);
                assert!(metrics.max_step < 0.15);
            }
        }
    }

    fn argmax(counts: &[u64]) -> usize {
        counts
            .iter()
            .enumerate()
            .max_by_key(|&(_, c)| *c)
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    #[test]
    fn fair_coin_peaks_at_the_center() {
        let counts = GaltonBoard::experiment_histogram(2, MAX_ROOM_POKES, 0);
        assert!((argmax(&counts) as i64 - 8).abs() <= 1);
    }

    #[test]
    fn biasing_shifts_the_peak_right() {
        let left = GaltonBoard::experiment_histogram(0, MAX_ROOM_POKES, 0);
        let fair = GaltonBoard::experiment_histogram(2, MAX_ROOM_POKES, 0);
        let right = GaltonBoard::experiment_histogram(4, MAX_ROOM_POKES, 0);
        assert!(argmax(&left) < argmax(&fair));
        assert!(argmax(&right) > argmax(&fair));
    }

    #[test]
    fn total_count_is_conserved() {
        let counts = GaltonBoard::experiment_histogram(1, 3, 0);
        assert_eq!(counts.iter().sum::<u64>(), (3 * BALLS_PER_WAVE) as u64);
    }

    #[test]
    fn render_is_deterministic() {
        let room = GaltonBoard::new();
        let mut a = Canvas::new(41, 16);
        let mut b = Canvas::new(41, 16);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn opening_and_experiment_are_phase_invariant() {
        let room = GaltonBoard::new_with(9);
        let mut opening_a = Canvas::new(61, 24);
        let mut opening_b = Canvas::new(61, 24);
        room.render(&mut opening_a, 0.0);
        room.render(&mut opening_b, 0.999);
        assert_eq!(opening_a.to_text(), opening_b.to_text());

        let pokes = [(0.5, 0.2), (0.5, 0.8)];
        let mut run_a = Canvas::new(61, 24);
        let mut run_b = Canvas::new(61, 24);
        room.render_poked(&mut run_a, 0.0, &pokes);
        room.render_poked(&mut run_b, 0.999, &pokes);
        assert_eq!(run_a.to_text(), run_b.to_text());
    }

    #[test]
    fn new_with_zero_matches_default_and_nonzero_differs() {
        let r0 = GaltonBoard::new_with(0);
        let r_def = GaltonBoard::new();
        let mut a = Canvas::new(41, 16);
        let mut b = Canvas::new(41, 16);
        r0.render(&mut a, 0.0);
        r_def.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
        assert_ne!(
            GaltonBoard::experiment_histogram(2, 3, 0),
            GaltonBoard::experiment_histogram(2, 3, 42)
        );
    }

    #[test]
    fn render_produces_ink() {
        let room = GaltonBoard::new();
        let mut canvas = Canvas::new(41, 16);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 10);
    }

    #[test]
    fn every_viewport_uses_the_same_physical_row_count() {
        assert_eq!(BOARD_ROWS, 16);
        assert_eq!(GaltonBoard::experiment_histogram(2, 1, 0).len(), 17);
    }

    #[test]
    fn wide_canvas_stays_bounded_and_fills() {
        // A wide target stretches the fixed physical board and stays bounded.
        let room = GaltonBoard::new();
        let mut canvas = Canvas::new(600, 12);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 10);
    }

    #[test]
    fn zero_sized_and_extreme_inputs_do_not_panic() {
        let room = GaltonBoard::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(5, 5);
        for t in [-2.0, 0.0, 0.999, 3.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn drop_waves_preserve_order_clamp_and_filter() {
        let waves = drop_waves(&[
            (-1.0, 2.0),
            (0.25, 0.75),
            (f64::INFINITY, 0.25),
            (0.5, f64::NAN),
        ]);
        assert_eq!(waves, vec![DropWave { coin: 0 }, DropWave { coin: 1 }]);
    }

    #[test]
    fn drop_waves_use_the_newest_bounded_raw_tail() {
        let mut many = vec![(0.0, 0.0); MAX_ROOM_POKES + 3];
        many.extend(
            (0..MAX_ROOM_POKES).map(|i| (((i as f64) + 0.25) / MAX_ROOM_POKES as f64, 0.5)),
        );
        let newest = many[many.len() - MAX_ROOM_POKES..].to_vec();

        assert_eq!(drop_waves(&many), drop_waves(&newest));
        assert_eq!(drop_waves(&many).len(), MAX_ROOM_POKES);
    }

    #[test]
    fn raw_newest_tail_is_capped_before_nonfinite_filtering() {
        let mut with_invalid_tail = vec![(0.4, 0.6); MAX_ROOM_POKES];
        with_invalid_tail.extend(vec![(f64::NAN, f64::INFINITY); MAX_ROOM_POKES + 5]);

        assert!(drop_waves(&with_invalid_tail).is_empty());
    }

    #[test]
    fn nonfinite_pokes_do_not_consume_ball_identity() {
        let finite = vec![(0.25, 0.75)];
        let with_bad_points = vec![(f64::NAN, 0.4), (0.25, 0.75), (0.2, f64::INFINITY)];

        assert_eq!(drop_waves(&with_bad_points), drop_waves(&finite));
    }

    #[test]
    fn repeated_pokes_extend_the_same_deterministic_stream() {
        let waves = drop_waves(&[(0.5, 0.5), (0.5, 0.5)]);
        assert_eq!(waves, vec![DropWave { coin: 2 }, DropWave { coin: 2 }]);
        assert_eq!(selected_run(&waves), Some((2, 2)));
        let one = GaltonBoard::experiment_histogram(2, 1, 0);
        let two = GaltonBoard::experiment_histogram(2, 2, 0);
        assert_eq!(one.iter().sum::<u64>(), BALLS_PER_WAVE as u64);
        assert_eq!(two.iter().sum::<u64>(), (2 * BALLS_PER_WAVE) as u64);
        let variation = GaltonBoard::experiment_variation(0, 2);
        let mut second_wave = vec![0u64; BOARD_ROWS + 1];
        for ball in BALLS_PER_WAVE..2 * BALLS_PER_WAVE {
            let bin = GaltonBoard::landing_bin(BOARD_ROWS, COIN_PROBABILITIES[2], variation, ball);
            second_wave[bin] += 1;
        }
        let expected: Vec<_> = one
            .iter()
            .zip(second_wave)
            .map(|(&prefix, extension)| prefix + extension)
            .collect();
        assert_eq!(two, expected);
    }

    #[test]
    fn changing_coin_starts_a_fresh_contiguous_run() {
        let waves = drop_waves(&[(0.1, 0.5), (0.9, 0.5), (0.12, 0.5)]);
        let (selected, wave_count) = selected_run(&waves).expect("selected coin");
        assert_eq!(selected, 0);
        assert_eq!(wave_count, 1);
        assert_eq!(
            GaltonBoard::experiment_histogram(selected, wave_count, 0),
            GaltonBoard::experiment_histogram(0, 1, 0)
        );

        let repeated = drop_waves(&[(0.1, 0.5), (0.12, 0.5)]);
        assert_eq!(selected_run(&repeated), Some((0, 2)));
    }

    #[test]
    fn pointer_moves_and_releases_do_not_add_waves() {
        use crate::room::RoomInput;

        let room = GaltonBoard::new();
        let gesture = [
            RoomInput::PointerDown {
                x: 0.5,
                y: 0.2,
                t: 0.1,
            },
            RoomInput::PointerMove {
                x: 0.9,
                y: 0.5,
                t: 0.2,
            },
            RoomInput::PointerMove {
                x: 0.1,
                y: 0.8,
                t: 0.3,
            },
            RoomInput::PointerUp {
                x: 0.1,
                y: 0.8,
                t: 0.4,
            },
        ];
        let mut via_gesture = Canvas::new(61, 24);
        let mut via_poke = Canvas::new(61, 24);
        room.render_input(&mut via_gesture, 0.8, &gesture);
        room.render_poked(&mut via_poke, 0.8, &[(0.5, 0.2)]);
        assert_eq!(via_gesture.to_text(), via_poke.to_text());
        assert!(
            room.status_input(0.8, &gesture)
                .expect("gesture status")
                .contains("1x64=64")
        );
    }

    #[test]
    fn bounded_run_saturates_truthfully_without_rerolling() {
        let room = GaltonBoard::new();
        let full = vec![(0.5, 0.5); MAX_ROOM_POKES];
        let overfull = vec![(0.5, 0.5); MAX_ROOM_POKES + 1];
        let mut full_render = Canvas::new(61, 24);
        let mut overfull_render = Canvas::new(61, 24);
        room.render_poked(&mut full_render, 0.0, &full);
        room.render_poked(&mut overfull_render, 0.9, &overfull);
        assert_eq!(full_render.to_text(), overfull_render.to_text());

        let status = room
            .status_input(0.0, &crate::room::inputs_from_pokes(&overfull, 0.0))
            .expect("full-run status");
        assert!(status.contains("FULL=1536"));
    }

    #[test]
    fn fair_reference_uses_exact_binomial_coefficients() {
        let weights = GaltonBoard::reference_weights(2);
        let expected = [
            1.0, 16.0, 120.0, 560.0, 1820.0, 4368.0, 8008.0, 11440.0, 12870.0, 11440.0, 8008.0,
            4368.0, 1820.0, 560.0, 120.0, 16.0, 1.0,
        ];
        for (actual, coefficient) in weights.into_iter().zip(expected) {
            assert!((actual / weights[0] - coefficient).abs() < 1.0e-8);
        }
    }

    #[test]
    fn highlighted_last_ball_belongs_to_the_accumulated_histogram() {
        let coin = 3;
        let wave_count = 4;
        let histogram = GaltonBoard::experiment_histogram(coin, wave_count, 7);
        let last = wave_count * BALLS_PER_WAVE - 1;
        let variation = GaltonBoard::experiment_variation(7, coin);
        let landing =
            GaltonBoard::landing_bin(BOARD_ROWS, COIN_PROBABILITIES[coin], variation, last);
        let mut before_last = vec![0u64; BOARD_ROWS + 1];
        for ball in 0..last {
            let bin =
                GaltonBoard::landing_bin(BOARD_ROWS, COIN_PROBABILITIES[coin], variation, ball);
            before_last[bin] += 1;
        }
        before_last[landing] += 1;
        assert_eq!(histogram, before_last);
    }

    #[test]
    fn variation_changes_the_dropped_ball_trace_directly() {
        assert_ne!(
            GaltonBoard::ball_trace(21, 0.5, 0, 0),
            GaltonBoard::ball_trace(21, 0.5, 42, 0)
        );
    }

    #[test]
    fn reveal_names_the_theorem() {
        let room = GaltonBoard::new();
        let reveal = room.reveal();
        assert!(reveal.contains("Central Limit Theorem"));
        assert!(reveal.contains("exactly Binomial(16, p)"));
        assert!(reveal.contains("normal curve can approximate"));
        assert!(reveal.contains("finite binomial"));
        assert!(!room.meta().blurb.contains("every time"));
        assert!(!reveal.contains("stock market"));
        assert!(!reveal.contains("perfectly predictable"));
    }

    #[test]
    fn poked_changes_output() {
        let r0 = GaltonBoard::new_with(0);
        let mut a = Canvas::new(41, 16);
        let mut p = Canvas::new(41, 16);
        r0.render(&mut a, 0.0);
        r0.render_poked(&mut p, 0.0, &[(0.5, 0.5)]);
        assert_ne!(p.to_text(), a.to_text());
        assert!(p.ink_count() >= a.ink_count());
        assert!(
            !a.to_text().contains('*'),
            "the opening has no finished pile"
        );
        assert!(p.to_text().contains('*'), "the first wave builds a pile");
        assert!(p.to_text().contains('#'), "the dropped ball lands visibly");
    }

    #[test]
    fn dropped_ball_follows_only_physical_peg_edges() {
        let trace = GaltonBoard::ball_trace(24, 0.5, 0, 0);
        assert_eq!(trace.len(), 25);
        assert_eq!(trace[0], 0);
        for (row, pair) in trace.windows(2).enumerate() {
            assert!(pair[0] <= row);
            assert!(matches!(pair[1].saturating_sub(pair[0]), 0 | 1));
            assert!(pair[1] <= row + 1);
        }
    }

    #[test]
    fn horizontal_hand_position_controls_a_named_bias() {
        let left = drop_waves(&[(0.0, 0.9)])[0].coin;
        let right = drop_waves(&[(1.0, 0.1)])[0].coin;
        let biased_left = GaltonBoard::ball_trace(200, COIN_PROBABILITIES[left], 0, 0);
        let biased_right = GaltonBoard::ball_trace(200, COIN_PROBABILITIES[right], 0, 0);
        assert!(biased_left.last().unwrap() < biased_right.last().unwrap());

        let status = GaltonBoard::new()
            .status_input(0.0, &crate::room::inputs_from_pokes(&[(1.0, 0.2)], 0.0))
            .expect("interaction status");
        assert!(status.starts_with("P.70"));
        assert!(status.contains("1x64=64"));
        assert!(status.contains('M'));
        assert!(status.contains("~11.2")); // 16 rows * 0.70
        assert!(status.contains('L'));
        assert!(status.contains('R'));
        assert!(
            !status.contains(" B"),
            "a pure click has no move-committed bet grade"
        );
    }

    #[test]
    fn move_commits_a_one_ball_bet_and_click_grades_the_last_landing() {
        use crate::room::RoomInput;

        let room = GaltonBoard::new();
        let open = room.status(0.0).expect("open");
        assert!(open.contains("MOVE TO BET"));
        assert!(open.contains("DROP 64"));

        let bet_only = [RoomInput::PointerMove {
            x: 0.5,
            y: 0.7,
            t: 0.0,
        }];
        let pending = room.status_input(0.0, &bet_only).expect("bet status");
        assert!(pending.starts_with("BET "));
        assert!(pending.contains("CLICK DROP 64"));
        assert!(!pending.contains("1x64"));

        // Fair coin, center bet: grade the highlighted last ball of the wave.
        let coin = coin_at(0.5);
        let p_right = COIN_PROBABILITIES[coin];
        let rights = GaltonBoard::ball_trace(
            BOARD_ROWS,
            p_right,
            GaltonBoard::experiment_variation(0, coin),
            BALLS_PER_WAVE - 1,
        )
        .last()
        .copied()
        .expect("landing");
        let bet = bet_bin_at(0.5);
        let graded = [
            RoomInput::PointerMove {
                x: 0.5,
                y: 0.7,
                t: 0.0,
            },
            RoomInput::PointerDown {
                x: 0.5,
                y: 0.2,
                t: 0.1,
            },
        ];
        let status = room.status_input(0.0, &graded).expect("graded status");
        assert!(status.contains("1x64=64"));
        assert!(status.contains(&format!("L{rights}R")));
        if bet == rights {
            assert!(status.ends_with(&format!("B{bet}H")), "{status}");
        } else {
            assert!(status.ends_with(&format!("B{bet}M")), "{status}");
        }
        // Moves alone never build a pile; only downs do.
        let mut move_only = Canvas::new(41, 16);
        let mut after_drop = Canvas::new(41, 16);
        room.render_input(&mut move_only, 0.0, &bet_only);
        room.render_input(&mut after_drop, 0.0, &graded);
        assert!(!move_only.to_text().contains('*'));
        assert!(after_drop.to_text().contains('*'));
    }

    #[test]
    fn poked_ball_is_deterministic_and_safe() {
        let room = GaltonBoard::new_with(7);
        let mut a = Canvas::new(41, 16);
        let mut b = Canvas::new(41, 16);
        room.render_poked(&mut a, 0.0, &[(0.25, 0.8), (f64::NAN, f64::INFINITY)]);
        room.render_poked(&mut b, 0.0, &[(0.25, 0.8), (f64::NAN, f64::INFINITY)]);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.to_text().contains('o'));
    }

    #[test]
    fn render_poked_uses_newest_raw_tail() {
        let room = GaltonBoard::new();
        let newest = vec![(0.85, 0.85); MAX_ROOM_POKES];
        let mut all = vec![(0.1, 0.1); MAX_ROOM_POKES + 7];
        all.extend(newest.iter().copied());
        let discarded_prefix = all[..all.len() - MAX_ROOM_POKES].to_vec();
        let mut expected = Canvas::new(41, 16);
        let mut actual = Canvas::new(41, 16);
        let mut prefix_only = Canvas::new(41, 16);

        room.render_poked(&mut expected, 0.0, &newest);
        room.render_poked(&mut actual, 0.0, &all);
        room.render_poked(&mut prefix_only, 0.0, &discarded_prefix);

        assert_eq!(actual.to_text(), expected.to_text());
        assert_ne!(actual.to_text(), prefix_only.to_text());
    }

    #[test]
    fn all_invalid_pokes_match_base_render() {
        let room = GaltonBoard::new();
        let mut base = Canvas::new(41, 16);
        let mut poked = Canvas::new(41, 16);

        room.render(&mut base, f64::NAN);
        room.render_poked(
            &mut poked,
            f64::NAN,
            &[(f64::NAN, f64::INFINITY), (f64::INFINITY, 0.5)],
        );

        assert_eq!(poked.to_text(), base.to_text());
    }

    #[test]
    fn all_invalid_newest_tail_discards_older_valid_pokes() {
        let room = GaltonBoard::new();
        let mut with_valid_prefix = vec![(0.5, 0.5); MAX_ROOM_POKES];
        with_valid_prefix.extend(vec![(f64::NAN, f64::INFINITY); MAX_ROOM_POKES + 5]);
        let mut base = Canvas::new(41, 16);
        let mut poked = Canvas::new(41, 16);

        room.render(&mut base, 0.0);
        room.render_poked(&mut poked, 0.0, &with_valid_prefix);

        assert_eq!(poked.to_text(), base.to_text());
    }

    #[test]
    fn compact_interaction_preserves_the_complete_coin_selector() {
        struct RasterProbe {
            width: usize,
            height: usize,
            cells: Vec<char>,
        }

        impl RasterProbe {
            fn new(width: usize, height: usize) -> Self {
                Self {
                    width,
                    height,
                    cells: vec![' '; width * height],
                }
            }

            fn selector(&self) -> &[char] {
                &self.cells[..self.width * (self.height as f64 * BOARD_TOP) as usize]
            }
        }

        impl Surface for RasterProbe {
            fn width(&self) -> usize {
                self.width
            }

            fn height(&self) -> usize {
                self.height
            }

            fn plot(&mut self, x: i32, y: i32, ch: char) {
                if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
                    self.cells[y as usize * self.width + x as usize] = ch;
                }
            }
        }

        let room = GaltonBoard::new();
        let mut arrival = RasterProbe::new(360, 240);
        let mut interacted = RasterProbe::new(360, 240);
        room.render(&mut arrival, 0.0);
        room.render_poked(&mut interacted, 0.0, &[(0.5, 0.5); 4]);

        assert_eq!(interacted.selector(), arrival.selector());
        assert!(
            interacted
                .selector()
                .iter()
                .filter(|&&ch| ch == '#')
                .count()
                >= 30
        );
    }

    #[test]
    fn nonzero_seed_changes_poked_render() {
        let r0 = GaltonBoard::new_with(0);
        let r42 = GaltonBoard::new_with(42);
        let mut a = Canvas::new(41, 16);
        let mut b = Canvas::new(41, 16);

        r0.render_poked(&mut a, 0.0, &[(0.5, 0.5)]);
        r42.render_poked(&mut b, 0.0, &[(0.5, 0.5)]);

        assert_ne!(a.to_text(), b.to_text());
    }

    #[test]
    fn huge_custom_surface_does_not_render_unbounded_columns() {
        #[derive(Default)]
        struct HugeSurface {
            width: usize,
            height: usize,
            plots: usize,
            max_abs_coord: i32,
        }

        impl Surface for HugeSurface {
            fn width(&self) -> usize {
                self.width
            }

            fn height(&self) -> usize {
                self.height
            }

            fn plot(&mut self, x: i32, y: i32, _mark: char) {
                self.plots += 1;
                self.max_abs_coord = self.max_abs_coord.max(x.abs()).max(y.abs());
            }
        }

        let room = GaltonBoard::new();
        for (width, height) in [(usize::MAX, 12), (12, usize::MAX), (usize::MAX, usize::MAX)] {
            let mut surface = HugeSurface {
                width,
                height,
                ..HugeSurface::default()
            };
            room.render_poked(&mut surface, 0.0, &[(1.0, 1.0)]);

            assert!(surface.plots < MAX_DIM * MAX_DIM);
            assert!(surface.max_abs_coord <= MAX_DIM.saturating_sub(1) as i32);
        }
    }

    #[test]
    fn sound_uses_the_default_phrase() {
        // Galton does not override sound, so it gets the default: its own motif
        // played note for note, so the voice matches the notation listen_room
        // reports, not a generic fallback or a single held tone.
        let room = GaltonBoard::new();
        let spec = room.sound(0.0);
        let motif = room.motif().expect("galton has a motif");
        assert_eq!(spec.notes.len(), motif.line.len());
        assert!(spec.notes.len() > 1, "a phrase, not a blip");
        // The notes are staggered in time, a phrase and not a chord.
        assert!(spec.notes[1].start > spec.notes[0].start);
    }

    #[test]
    fn selected_coin_sonifies_ordered_roots_and_exact_bias_odds() {
        let room = GaltonBoard::new();
        let mut roots = Vec::new();
        let expected_ratios = [7.0 / 3.0, 3.0 / 2.0, 1.0, 3.0 / 2.0, 7.0 / 3.0];

        for (x, expected_ratio) in [0.1, 0.3, 0.5, 0.7, 0.9].into_iter().zip(expected_ratios) {
            let inputs = crate::room::inputs_from_pokes(&[(x, 0.5)], 0.4);
            let voice = room
                .parameter_sound(0.4, &inputs)
                .expect("a dropped wave has a probability voice");
            roots.push(voice.root_hz());
            assert_eq!(voice.ratio(), expected_ratio);
            assert_eq!(voice.gain(), 0.04);

            let snapshot = room.sound_input(0.4, &inputs);
            assert_eq!(snapshot.notes.len(), 2);
            assert_eq!(snapshot.notes[0].freq, voice.root_hz());
            assert_eq!(snapshot.notes[1].freq, voice.root_hz() * voice.ratio());
        }

        assert!(roots.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn no_accepted_drop_has_no_parameter_voice() {
        let room = GaltonBoard::new();
        assert!(room.parameter_sound(0.4, &[]).is_none());
        assert!(
            room.parameter_sound(
                0.4,
                &[crate::room::RoomInput::PointerMove {
                    x: 0.8,
                    y: 0.5,
                    t: 0.4,
                }],
            )
            .is_none()
        );
        assert!(
            room.parameter_sound(
                0.4,
                &[crate::room::RoomInput::PointerDown {
                    x: f64::NAN,
                    y: 0.5,
                    t: 0.4,
                }],
            )
            .is_none()
        );
    }
}

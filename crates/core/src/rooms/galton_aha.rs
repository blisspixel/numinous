//! The Galton Board engineered aha: wager where the pile will peak, then
//! meet the binomial.
//!
//! The third flagship aha, cloned from the Buffon machine's five-beat
//! anatomy (see `buffon_aha.rs`): explore freely, prime after the first
//! wave, commit a peak-bin wager (or earn the withheld beat by running the
//! experiment), summon the truth, watch the exact Binomial outline grow
//! over the pile, and consolidate on one graded sentence. The wager here
//! is a model-level commitment (where does the WHOLE pile settle), not the
//! room's one-ball bet, which grades a single stochastic landing: luck,
//! not model. The miss that meets the truth is the fertile one.

use crate::surface::Surface;

use super::galton_board::{BOARD_ROWS, COIN_PROBABILITIES};

/// Vertical start of the wager band (bottom strip of the plate), matching
/// the Buffon convention so the two flagship bands feel like one gesture.
pub const WAGER_BAND_Y: f64 = 0.88;
/// Waves before the wager invite appears (the pile must exist to bet on).
pub const MIN_WAVES_TO_PRIME: usize = 1;
/// Waves that earn the withheld beat without a wager: the experiment ran.
pub const MIN_WAVES_TO_EARN: usize = 4;
/// Morph progress that counts as done.
pub const MORPH_DONE: f64 = 1.0;
/// How many points trace the binomial outline overlay.
const OUTLINE_STEPS: usize = 96;

/// Learning-progress band for a committed peak-bin wager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuessBand {
    /// Exactly the binomial's peak bin.
    Nailed,
    /// One bin off: the fertile band.
    Close,
    /// Further out.
    Wild,
}

impl GuessBand {
    /// Compact spoken name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Nailed => "NAILED",
            Self::Close => "CLOSE",
            Self::Wild => "WILD",
        }
    }

    /// Grade a bin wager against the true peak for `coin`.
    #[must_use]
    pub fn grade(bin: usize, coin: usize) -> Self {
        let truth = peak_bin_for_coin(coin);
        let distance = bin.abs_diff(truth);
        match distance {
            0 => Self::Nailed,
            1 => Self::Close,
            _ => Self::Wild,
        }
    }
}

/// The bin where Binomial(`BOARD_ROWS`, p) peaks for the given coin.
///
/// The mode of a binomial with n trials is floor((n + 1) p) whenever
/// (n + 1) p is not an integer, and none of the five coins land on an
/// integer at n = 16, so each coin has exactly one peak. A test below pins
/// every coin's mode against the probability mass itself rather than
/// trusting the closed form.
#[must_use]
pub fn peak_bin_for_coin(coin: usize) -> usize {
    let coin = coin.min(COIN_PROBABILITIES.len() - 1);
    let p = COIN_PROBABILITIES[coin];
    (((BOARD_ROWS + 1) as f64) * p).floor() as usize
}

/// How the generation act was completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarnPath {
    /// A peak-bin wager committed (right or wrong), on a chosen coin.
    Wager {
        /// The bin the player committed to.
        bin: usize,
        /// The coin selected when the wager was committed.
        coin: usize,
        /// Band relative to that coin's true peak at commit time.
        band: GuessBand,
    },
    /// Enough waves to have run the experiment without a wager.
    Waves {
        /// How many waves earned the path.
        count: usize,
    },
}

/// Staging for the engineered aha.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AhaBeat {
    /// Free drops before the pile exists.
    Explore,
    /// At least one wave landed; invite the peak wager.
    Prime,
    /// Generation complete; the truth is withheld until summoned.
    Withheld,
    /// The exact binomial outline grows over the pile (progress 0..1).
    Morph {
        /// Morph blend, clamped to `[0, 1]`.
        progress: f64,
    },
    /// Waves continue under the settled outline.
    Confirm,
    /// Punchline available; full reveal text may open.
    Consolidated,
}

/// Pure visit state for the Galton engineered aha.
#[derive(Debug, Clone, PartialEq)]
pub struct GaltonAha {
    beat: AhaBeat,
    waves: usize,
    hover: Option<usize>,
    earn: Option<EarnPath>,
    morph_progress: f64,
}

impl Default for GaltonAha {
    fn default() -> Self {
        Self::new()
    }
}

impl GaltonAha {
    /// A fresh visit.
    #[must_use]
    pub fn new() -> Self {
        Self {
            beat: AhaBeat::Explore,
            waves: 0,
            hover: None,
            earn: None,
            morph_progress: 0.0,
        }
    }

    /// Current beat.
    #[must_use]
    pub fn beat(&self) -> AhaBeat {
        self.beat
    }

    /// Waves observed this visit.
    #[must_use]
    pub fn waves(&self) -> usize {
        self.waves
    }

    /// Hovered bin while priming.
    #[must_use]
    pub fn hover(&self) -> Option<usize> {
        self.hover
    }

    /// Earn path once generation has completed.
    #[must_use]
    pub fn earn(&self) -> Option<EarnPath> {
        self.earn
    }

    /// Morph progress in `[0, 1]`.
    #[must_use]
    pub fn morph_progress(&self) -> f64 {
        self.morph_progress
    }

    /// Generation is complete.
    #[must_use]
    pub fn earned(&self) -> bool {
        self.earn.is_some()
    }

    /// Full reveal text may open only after the morph has consolidated.
    #[must_use]
    pub fn allow_reveal_text(&self) -> bool {
        matches!(self.beat, AhaBeat::Consolidated)
    }

    /// Summon advances withheld into morph, or confirm into punchline.
    #[must_use]
    pub fn can_summon(&self) -> bool {
        matches!(self.beat, AhaBeat::Withheld | AhaBeat::Confirm)
    }

    /// Whether the visit should draw the binomial outline overlay.
    #[must_use]
    pub fn uses_outline_overlay(&self) -> bool {
        matches!(
            self.beat,
            AhaBeat::Morph { .. } | AhaBeat::Confirm | AhaBeat::Consolidated
        )
    }

    /// Note the current wave count from the room's own input grading.
    pub fn note_waves(&mut self, waves: usize) {
        self.waves = waves;
        if waves >= MIN_WAVES_TO_PRIME && matches!(self.beat, AhaBeat::Explore) {
            self.beat = AhaBeat::Prime;
        }
        if self.earn.is_none() && waves >= MIN_WAVES_TO_EARN {
            self.earn = Some(EarnPath::Waves { count: waves });
            self.hover = None;
            self.beat = AhaBeat::Withheld;
        }
    }

    /// Hover a bin on the landing row (Prime only).
    pub fn set_hover(&mut self, bin: Option<usize>) {
        if !matches!(self.beat, AhaBeat::Prime) {
            return;
        }
        self.hover = bin.map(|b| b.min(BOARD_ROWS));
    }

    /// Commit the peak wager on the selected coin. First generation act wins.
    ///
    /// Prime only, unlike Buffon's machine: pi exists before any needle is
    /// thrown, but a peak wager about a pile that does not exist yet is
    /// incoherent, so the first wave must land before the bet can.
    pub fn commit_wager(&mut self, bin: usize, coin: usize) -> bool {
        if self.earn.is_some() {
            return false;
        }
        if !matches!(self.beat, AhaBeat::Prime) {
            return false;
        }
        let bin = bin.min(BOARD_ROWS);
        let coin = coin.min(COIN_PROBABILITIES.len() - 1);
        let band = GuessBand::grade(bin, coin);
        self.earn = Some(EarnPath::Wager { bin, coin, band });
        self.hover = None;
        self.beat = AhaBeat::Withheld;
        true
    }

    /// Summon the next staged beat after generation.
    pub fn summon(&mut self) -> bool {
        match self.beat {
            AhaBeat::Withheld if self.earn.is_some() => {
                self.morph_progress = 0.0;
                self.beat = AhaBeat::Morph { progress: 0.0 };
                true
            }
            AhaBeat::Confirm => {
                self.beat = AhaBeat::Consolidated;
                true
            }
            AhaBeat::Morph { progress } if progress >= MORPH_DONE - 1e-9 => {
                self.morph_progress = 1.0;
                self.beat = AhaBeat::Confirm;
                true
            }
            _ => false,
        }
    }

    /// Face-driven morph progress. Completing the blend enters Confirm.
    pub fn set_morph_progress(&mut self, progress: f64) {
        if !matches!(self.beat, AhaBeat::Morph { .. }) {
            return;
        }
        let progress = if progress.is_finite() {
            progress.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.morph_progress = progress;
        if progress >= MORPH_DONE - 1e-9 {
            self.morph_progress = 1.0;
            self.beat = AhaBeat::Confirm;
        } else {
            self.beat = AhaBeat::Morph { progress };
        }
    }

    /// Advance morph by a non-negative delta (faces convert wall time).
    pub fn advance_morph(&mut self, delta: f64) {
        if !matches!(self.beat, AhaBeat::Morph { .. }) {
            return;
        }
        let delta = if delta.is_finite() {
            delta.max(0.0)
        } else {
            0.0
        };
        self.set_morph_progress(self.morph_progress + delta);
    }

    /// Compact status for the footer.
    #[must_use]
    pub fn status(&self, room_status: Option<&str>) -> String {
        match self.beat {
            AhaBeat::Explore => room_status.unwrap_or("CLICK: DROP 64").to_string(),
            AhaBeat::Prime => {
                let hover = self.hover.map(|b| format!(" >{b}")).unwrap_or_default();
                format!("WHERE WILL IT PEAK? BIN 0-{BOARD_ROWS}{hover}")
            }
            AhaBeat::Withheld => match self.earn {
                Some(EarnPath::Wager { bin, band, .. }) => {
                    format!("EARNED BIN {bin} {}  PRESS E", band.name())
                }
                Some(EarnPath::Waves { count }) => {
                    format!("EARNED {count} WAVES  PRESS E")
                }
                None => "EARNED  PRESS E".to_string(),
            },
            AhaBeat::Morph { progress } => {
                let pct = (progress * 100.0).round() as i32;
                format!("BINOMIAL {pct}%")
            }
            AhaBeat::Confirm => match room_status {
                Some(s) => format!("THE CURVE WAS WAITING  PRESS E  {s}"),
                None => "THE CURVE WAS WAITING  PRESS E".to_string(),
            },
            AhaBeat::Consolidated => match room_status {
                Some(s) => format!("CHANCE KEEPS SHAPE  {s}"),
                None => "CHANCE KEEPS SHAPE  E:WHY".to_string(),
            },
        }
    }

    /// Punchline once consolidated.
    #[must_use]
    pub fn punchline(&self) -> Option<&'static str> {
        matches!(self.beat, AhaBeat::Consolidated).then_some(
            "No ball knows where it is going, and the pile always knows where it will be.",
        )
    }

    /// The committed wager, its coin, and its band, once a wager exists.
    #[must_use]
    pub fn wager(&self) -> Option<(usize, usize, GuessBand)> {
        match self.earn {
            Some(EarnPath::Wager { bin, coin, band }) => Some((bin, coin, band)),
            _ => None,
        }
    }

    /// The wager graded against the binomial's peak, spoken at consolidation.
    ///
    /// The commitment must meet the truth or the wager was theater. The
    /// bands keep the language predict already speaks: a miss is fertile,
    /// never punished.
    #[must_use]
    pub fn graded(&self) -> Option<String> {
        if !matches!(self.beat, AhaBeat::Consolidated) {
            return None;
        }
        let (bin, coin, band) = self.wager()?;
        let truth = peak_bin_for_coin(coin);
        let verdict = match band {
            GuessBand::Nailed => "Nailed.",
            GuessBand::Close => "Close: the fertile band.",
            GuessBand::Wild => "A wild swing; the gap is the lesson.",
        };
        Some(format!(
            "You wagered bin {bin}; the binomial peaks at bin {truth}. {verdict}"
        ))
    }

    /// Stable beat name for playtest notes and diagnostics (not player chrome).
    #[must_use]
    pub fn beat_label(&self) -> &'static str {
        match self.beat {
            AhaBeat::Explore => "explore",
            AhaBeat::Prime => "prime",
            AhaBeat::Withheld => "withheld",
            AhaBeat::Morph { .. } => "morph",
            AhaBeat::Confirm => "confirm",
            AhaBeat::Consolidated => "consolidated",
        }
    }

    /// Compact earn path for playtest notes, or None before generation.
    #[must_use]
    pub fn earn_label(&self) -> Option<String> {
        match self.earn {
            Some(EarnPath::Wager { bin, coin, band }) => Some(format!(
                "wager:{bin}:coin{coin}:{}",
                band.name().to_ascii_lowercase()
            )),
            Some(EarnPath::Waves { count }) => Some(format!("waves:{count}")),
            None => None,
        }
    }
}

/// Exact Binomial(`BOARD_ROWS`, p) probability mass for every bin.
///
/// Computed by the Pascal recurrence rather than factorials, so nothing
/// overflows and the pegboard's own arithmetic (this row IS Pascal's row,
/// reweighted by the coin) is literally the code.
#[must_use]
pub fn binomial_mass(coin: usize) -> Vec<f64> {
    let coin = coin.min(COIN_PROBABILITIES.len() - 1);
    let p = COIN_PROBABILITIES[coin];
    let n = BOARD_ROWS;
    let mut mass = vec![0.0_f64; n + 1];
    mass[0] = 1.0;
    for _ in 0..n {
        for k in (1..=n).rev() {
            mass[k] = mass[k] * (1.0 - p) + mass[k - 1] * p;
        }
        mass[0] *= 1.0 - p;
    }
    mass
}

/// Draw the exact binomial outline over the pile band (morph overlay).
///
/// `progress` 0 is nothing; 1 is the full outline across every bin. The
/// outline sweeps outward from the peak, so the truth the wager was about
/// is the first thing the morph shows. Does not clear the pile underneath:
/// callers render the room first, then this overlay.
pub fn render_outline_overlay(canvas: &mut dyn Surface, progress: f64, coin: usize) {
    let (width, height) = canvas.draw_bounds();
    if width < 8 || height < 6 {
        return;
    }
    let progress = if progress.is_finite() {
        progress.clamp(0.0, 1.0)
    } else {
        0.0
    };
    if progress < 0.02 {
        return;
    }
    let mass = binomial_mass(coin);
    let peak_mass = mass.iter().copied().fold(f64::MIN, f64::max);
    if peak_mass <= 0.0 {
        return;
    }
    // The pile band the room draws into (see galton_board.rs constants).
    let top = height as f64 * 0.58;
    let bottom = height as f64 * 0.74;
    let peak = peak_bin_for_coin(coin) as f64;
    let reach = (BOARD_ROWS as f64).max(1.0) * progress;
    // One mark throughout: against this room's accent, the hash and the
    // at-sign collapse for a color-blind player (the dichromacy sweep
    // proved it), so the outline does not switch marks as it lands.
    let mark = '#';
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=OUTLINE_STEPS {
        let bin = BOARD_ROWS as f64 * (i as f64 / OUTLINE_STEPS as f64);
        // Interpolate the mass between whole bins for a readable curve.
        let low = (bin.floor() as usize).min(BOARD_ROWS);
        let high = (bin.ceil() as usize).min(BOARD_ROWS);
        let t = bin - low as f64;
        let m = mass[low] * (1.0 - t) + mass[high] * t;
        let x = ((bin + 0.5) / (BOARD_ROWS as f64 + 1.0) * width as f64).round() as i32;
        let y = (bottom - (bottom - top) * (m / peak_mass)).round() as i32;
        let visible = (bin - peak).abs() <= reach;
        if visible {
            if let Some((px, py)) = prev {
                canvas.line(px, py, x, y, mark);
            }
            prev = Some((x, y));
        } else {
            prev = None;
        }
    }
}

/// Draw the peak-wager band (Prime only): a bin ruler along the bottom.
pub fn render_bin_band(canvas: &mut dyn Surface, hover: Option<usize>) {
    let (width, height) = canvas.draw_bounds();
    if width < 16 || height < 6 {
        return;
    }
    let y = (height as f64 * 0.92).round() as i32;
    let y = y.clamp(1, height as i32 - 2);
    let left = (width as f64 * 0.04).round() as i32;
    let right = (width as f64 * 0.96).round() as i32;
    canvas.line(left, y, right, y, '-');
    // Tick every fourth bin so the ruler reads without clutter.
    for bin in (0..=BOARD_ROWS).step_by(4) {
        let phase = (bin as f64 + 0.5) / (BOARD_ROWS as f64 + 1.0);
        let x = left as f64 + (right - left) as f64 * phase;
        let x = x.round() as i32;
        canvas.line(x, y - 1, x, y + 1, '*');
    }
    if let Some(bin) = hover {
        let bin = bin.min(BOARD_ROWS);
        let phase = (bin as f64 + 0.5) / (BOARD_ROWS as f64 + 1.0);
        let x = left as f64 + (right - left) as f64 * phase;
        let x = x.round() as i32;
        canvas.line(x, y - 3, x, y + 1, '#');
    }
}

#[cfg(test)]
mod tests {
    use super::{AhaBeat, EarnPath, GaltonAha, GuessBand, binomial_mass, peak_bin_for_coin};
    use crate::canvas::Canvas;

    #[test]
    fn every_coin_peak_matches_the_probability_mass_itself() {
        // The closed form floor((n+1)p) must agree with the argmax of the
        // exact mass, or the wager would be graded against a formula
        // instead of the truth the pile converges to.
        for coin in 0..super::COIN_PROBABILITIES.len() {
            let mass = binomial_mass(coin);
            let argmax = mass
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(k, _)| k)
                .expect("mass is never empty");
            assert_eq!(
                peak_bin_for_coin(coin),
                argmax,
                "coin {coin}: closed form disagrees with the mass"
            );
        }
    }

    #[test]
    fn the_mass_is_a_probability_distribution() {
        for coin in 0..super::COIN_PROBABILITIES.len() {
            let total: f64 = binomial_mass(coin).iter().sum();
            assert!((total - 1.0).abs() < 1e-9, "coin {coin} sums to {total}");
        }
    }

    #[test]
    fn the_beats_stage_in_order_and_the_wager_grades() {
        let mut aha = GaltonAha::new();
        assert!(matches!(aha.beat(), AhaBeat::Explore));
        assert!(!aha.can_summon());

        // The first wave primes the wager invite.
        aha.note_waves(1);
        assert!(matches!(aha.beat(), AhaBeat::Prime));

        aha.set_hover(Some(8));
        assert_eq!(aha.hover(), Some(8));

        // No pile, no bet: a fresh machine refuses the commit outright.
        let mut unprimed = GaltonAha::new();
        assert!(!unprimed.commit_wager(8, 2), "explore cannot commit");
        assert!(unprimed.earn().is_none());

        // Committing on the fair coin: the true peak is 8.
        assert!(aha.commit_wager(8, 2));
        assert!(matches!(aha.beat(), AhaBeat::Withheld));
        let (bin, coin, band) = aha.wager().expect("wager recorded");
        assert_eq!((bin, coin), (8, 2));
        assert_eq!(band, GuessBand::Nailed);

        // A second generation act cannot overwrite the first.
        assert!(!aha.commit_wager(0, 0));

        // Summon walks morph, confirm, consolidated.
        assert!(aha.summon());
        aha.advance_morph(0.5);
        assert!(matches!(aha.beat(), AhaBeat::Morph { .. }));
        assert!(!aha.allow_reveal_text(), "mid-morph is not consolidation");
        aha.advance_morph(0.6);
        assert!(matches!(aha.beat(), AhaBeat::Confirm));
        assert!(aha.summon());
        assert!(matches!(aha.beat(), AhaBeat::Consolidated));
        assert!(aha.allow_reveal_text());

        let graded = aha.graded().expect("consolidated wager is graded");
        assert!(graded.contains("bin 8"), "{graded}");
        assert!(graded.contains("Nailed"), "{graded}");
    }

    #[test]
    fn four_waves_earn_without_a_wager_and_are_never_graded() {
        let mut aha = GaltonAha::new();
        aha.note_waves(4);
        assert!(matches!(aha.beat(), AhaBeat::Withheld));
        assert!(matches!(aha.earn(), Some(EarnPath::Waves { count: 4 })));
        assert!(aha.summon());
        aha.set_morph_progress(1.0);
        assert!(aha.summon());
        assert!(aha.graded().is_none(), "no wager, nothing to grade");
        assert_eq!(aha.earn_label().as_deref(), Some("waves:4"));
    }

    #[test]
    fn the_bands_are_exact_one_off_and_wild() {
        // Fair coin: peak 8.
        assert_eq!(GuessBand::grade(8, 2), GuessBand::Nailed);
        assert_eq!(GuessBand::grade(7, 2), GuessBand::Close);
        assert_eq!(GuessBand::grade(9, 2), GuessBand::Close);
        assert_eq!(GuessBand::grade(0, 2), GuessBand::Wild);
        // Loaded coin p=0.7: peak 11.
        assert_eq!(GuessBand::grade(11, 4), GuessBand::Nailed);
        assert_eq!(GuessBand::grade(8, 4), GuessBand::Wild);
    }

    #[test]
    fn the_outline_overlay_draws_ink_that_grows_from_the_peak() {
        let mut early = Canvas::new(72, 40);
        super::render_outline_overlay(&mut early, 0.3, 2);
        let early_ink = early.ink_count();
        assert!(early_ink > 0, "a partial outline is visible");

        let mut full = Canvas::new(72, 40);
        super::render_outline_overlay(&mut full, 1.0, 2);
        assert!(
            full.ink_count() > early_ink,
            "the outline grows with progress"
        );

        let mut none = Canvas::new(72, 40);
        super::render_outline_overlay(&mut none, 0.0, 2);
        assert_eq!(none.ink_count(), 0, "zero progress draws nothing");
    }

    #[test]
    fn hover_only_lives_in_prime_and_clamps_to_the_last_bin() {
        let mut aha = GaltonAha::new();
        aha.set_hover(Some(3));
        assert_eq!(aha.hover(), None, "explore has no bet band");
        aha.note_waves(1);
        aha.set_hover(Some(usize::MAX));
        assert_eq!(aha.hover(), Some(super::BOARD_ROWS));
    }

    use super::super::galton_board::{BOARD_ROWS, COIN_PROBABILITIES};
    const _: () = assert!(BOARD_ROWS == 16);
    const _: () = assert!(COIN_PROBABILITIES.len() == 5);
}

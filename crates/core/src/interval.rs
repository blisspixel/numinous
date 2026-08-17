//! Exact readings of the step between two notes.
//!
//! A list of frequencies is something to read. The step between two of them is
//! something a curve did, and it is the part of music that survives having no
//! ears: a perfect fifth is not 392 Hz beside 261.6 Hz, it is 3:2. This module
//! measures that step and refuses to name what it cannot find.
//!
//! Two facts are reported, and their status differs. The size in cents is a
//! measurement of the two frequencies and is always available. The small whole
//! number ratio is a *search result*: it is offered only when one is close
//! enough to say so, and it always carries how far off it is. See
//! `docs/ROSETTA.md` for why ratio is the shared language here.

/// How complicated a ratio may be and still be worth naming, as the product of
/// its two terms inside one octave (its Tenney height).
///
/// Simplicity is the whole content of the claim. Allow enough complexity and
/// every step finds some ratio, which then reports the reach of the search
/// rather than anything about the music. At this bound the vocabulary inside
/// an octave is the consonances (3:2, 4:3, 5:4, 6:5, 5:3, 8:5, 9:5, 7:5) and
/// nothing else, so a step that gets no ratio is being told something true:
/// no simple ratio explains it.
const MAX_RATIO_COMPLEXITY: u32 = 50;

/// Widest octave displacement a reported ratio will carry.
///
/// Beyond audible range; exists so the doubling cannot overflow.
const MAX_RATIO_OCTAVES: u32 = 16;

/// How far from a whole number ratio a step may sit and still be called one.
///
/// A fifth of a semitone. Wide enough to catch an equal-tempered interval
/// reaching for the just one it approximates, which is the interesting part:
/// the piano's major third really does sit fourteen cents above 5:4, and this
/// is the tolerance at which that fact can be reported rather than hidden.
const RATIO_TOLERANCE_CENTS: f64 = 20.0;

/// How far from an equal-tempered interval a step may sit and still take its
/// name.
///
/// Short of the half-semitone midpoint on purpose. A step landing in the band
/// between two names belongs to neither, and picking the nearer one there
/// would be the tool deciding what the player heard.
const NAME_TOLERANCE_CENTS: f64 = 45.0;

/// Cents in one octave.
const CENTS_PER_OCTAVE: f64 = 1200.0;

/// The twelve equal-tempered steps, by the semitone count each spans.
const STEP_NAMES: [&str; 13] = [
    "unison",
    "minor second",
    "major second",
    "minor third",
    "major third",
    "perfect fourth",
    "tritone",
    "perfect fifth",
    "minor sixth",
    "major sixth",
    "minor seventh",
    "major seventh",
    "octave",
];

/// A whole number ratio found near a measured step, with its error.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WholeRatio {
    /// Numerator of the ratio in lowest terms.
    pub numerator: u32,
    /// Denominator of the ratio in lowest terms.
    pub denominator: u32,
    /// Signed cents from the exact ratio to the measured step, rounded to a
    /// tenth. Positive means the step is wider than the whole number ratio.
    pub cents_off: f64,
}

/// Which way a step goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// The second note is higher than the first.
    Up,
    /// The second note is lower than the first.
    Down,
    /// The two notes are the same pitch.
    Level,
}

impl Direction {
    /// The stable word a face prints for this direction.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Direction::Up => "up",
            Direction::Down => "down",
            Direction::Level => "level",
        }
    }
}

/// The step between two notes, measured and where possible named.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    /// Size of the step, always positive, in cents.
    pub cents: f64,
    /// Which way the step goes.
    pub direction: Direction,
    /// The nearest whole number ratio, when one sits close enough to claim.
    pub ratio: Option<WholeRatio>,
    /// The equal-tempered name, when the step is close enough to take it.
    pub name: Option<&'static str>,
}

impl Interval {
    /// Measure the step from `from` to `to`, both in Hz.
    ///
    /// Returns `None` unless both frequencies are finite and above zero: a
    /// silent or undefined note has no interval, and inventing one would be
    /// the room talking about something that did not happen.
    #[must_use]
    pub fn between(from: f64, to: f64) -> Option<Self> {
        if !(from.is_finite() && to.is_finite()) || from <= 0.0 || to <= 0.0 {
            return None;
        }
        let signed = CENTS_PER_OCTAVE * (to / from).log2();
        let cents = signed.abs();
        let direction = if cents < f64::EPSILON {
            Direction::Level
        } else if signed > 0.0 {
            Direction::Up
        } else {
            Direction::Down
        };
        Some(Self {
            cents,
            direction,
            ratio: nearest_whole_ratio(cents),
            name: equal_tempered_name(cents),
        })
    }

    /// One short line naming what this step is, measurement first.
    ///
    /// The size is what was measured; the ratio and the name are what was
    /// found near it, and the line says which is which by carrying the error.
    #[must_use]
    pub fn describe(&self) -> String {
        let mut line = format!("{} {:.0} cents", self.direction.label(), self.cents);
        if let Some(name) = self.name {
            line.push_str(&format!(", {name}"));
        }
        if let Some(ratio) = self.ratio {
            line.push_str(&format!(
                ", {}:{} off by {:+.1}",
                ratio.numerator, ratio.denominator, ratio.cents_off
            ));
        }
        line
    }
}

/// The equal-tempered interval name within one octave, if one is near enough.
fn equal_tempered_name(cents: f64) -> Option<&'static str> {
    let octaves = (cents / CENTS_PER_OCTAVE).floor();
    let within = cents - octaves * CENTS_PER_OCTAVE;
    let semitones = (within / 100.0).round();
    let index = semitones as usize;
    if (within - semitones * 100.0).abs() > NAME_TOLERANCE_CENTS || index >= STEP_NAMES.len() {
        return None;
    }
    // Only the step inside the octave is named, so a compound interval reports
    // its size in cents rather than claiming a name it does not have.
    if octaves > 0.0 && !(index == 0 || index == STEP_NAMES.len() - 1) {
        return None;
    }
    if octaves > 0.0 && index == 0 {
        return Some("octave");
    }
    Some(STEP_NAMES[index])
}

/// The whole number ratio nearest a step, when a simple one sits close enough.
///
/// The search happens inside one octave, where simplicity means something, and
/// the octaves are put back afterwards by doubling. That is also how the step
/// is heard: an octave and a fifth is a fifth that has been moved up, and 3:1
/// is the honest name for it.
fn nearest_whole_ratio(cents: f64) -> Option<WholeRatio> {
    if !cents.is_finite() || cents < 0.0 {
        return None;
    }
    let octaves = (cents / CENTS_PER_OCTAVE).floor();
    if octaves > f64::from(MAX_RATIO_OCTAVES) {
        return None;
    }
    let octaves = octaves as u32;
    let within = cents - f64::from(octaves) * CENTS_PER_OCTAVE;

    let mut best: Option<(u32, u32, f64)> = None;
    for denominator in 1..=MAX_RATIO_COMPLEXITY {
        // Inside one octave the ratio sits in [1, 2), so the numerator is
        // bounded by the denominator and by the complexity budget together.
        for numerator in denominator..(2 * denominator).min(MAX_RATIO_COMPLEXITY) {
            if numerator * denominator > MAX_RATIO_COMPLEXITY || gcd(numerator, denominator) != 1 {
                continue;
            }
            let size = CENTS_PER_OCTAVE * (f64::from(numerator) / f64::from(denominator)).log2();
            let off = within - size;
            if best.is_none_or(|(_, _, seen): (u32, u32, f64)| off.abs() < seen.abs()) {
                best = Some((numerator, denominator, off));
            }
        }
    }

    let (numerator, denominator, off) =
        best.filter(|&(_, _, off)| off.abs() <= RATIO_TOLERANCE_CENTS)?;
    // Putting the octaves back can leave a common factor: an octave above 3:2
    // is 6:2, and the ratio a player is owed is 3:1.
    let raised = numerator.checked_shl(octaves)?;
    let common = gcd(raised, denominator);
    Some(WholeRatio {
        numerator: raised / common,
        denominator: denominator / common,
        cents_off: (off * 10.0).round() / 10.0,
    })
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

#[cfg(test)]
mod tests {
    use super::{Direction, Interval, MAX_RATIO_COMPLEXITY};

    #[test]
    fn a_perfect_fifth_reads_as_three_to_two() {
        let step = Interval::between(200.0, 300.0).expect("finite pair");
        assert_eq!(step.direction, Direction::Up);
        assert!((step.cents - 701.955).abs() < 0.01, "{}", step.cents);
        assert_eq!(step.name, Some("perfect fifth"));
        let ratio = step.ratio.expect("a fifth is 3:2");
        assert_eq!((ratio.numerator, ratio.denominator), (3, 2));
        assert!(ratio.cents_off.abs() < 0.05, "{ratio:?}");
    }

    #[test]
    fn an_octave_down_is_still_an_octave() {
        let step = Interval::between(880.0, 440.0).expect("finite pair");
        assert_eq!(step.direction, Direction::Down);
        assert!((step.cents - 1200.0).abs() < 0.01);
        assert_eq!(step.name, Some("octave"));
        let ratio = step.ratio.expect("an octave is 2:1");
        assert_eq!((ratio.numerator, ratio.denominator), (2, 1));
    }

    #[test]
    fn equal_tempered_steps_take_their_names_with_the_error_shown() {
        // A tempered fifth is not 3:2. The name is the nearest one; the ratio
        // is the nearest one; the cents say exactly how far apart they are.
        let tempered = 2f64.powf(7.0 / 12.0);
        let step = Interval::between(440.0, 440.0 * tempered).expect("finite pair");
        assert_eq!(step.name, Some("perfect fifth"));
        let ratio = step.ratio.expect("near 3:2");
        assert_eq!((ratio.numerator, ratio.denominator), (3, 2));
        assert!(
            (ratio.cents_off - (-1.955)).abs() < 0.1,
            "a tempered fifth is about two cents narrow than 3:2: {ratio:?}"
        );
    }

    #[test]
    fn a_step_between_the_names_claims_neither() {
        // A quarter tone sits equally far from two names. Naming one would be
        // the tool deciding what the player heard.
        let quarter = 2f64.powf(0.5 / 12.0);
        let step = Interval::between(440.0, 440.0 * quarter).expect("finite pair");
        assert_eq!(step.name, None, "{step:?}");
        assert!((step.cents - 50.0).abs() < 0.01);
    }

    #[test]
    fn a_dissonance_is_offered_no_ratio_to_hide_behind() {
        // A semitone is not a simple ratio, and the honest report of that is
        // silence rather than a search result. Allow enough complexity and
        // every step finds something, which tells the player nothing.
        for semitones in [1.0, 11.0] {
            let step =
                Interval::between(440.0, 440.0 * 2f64.powf(semitones / 12.0)).expect("finite pair");
            assert!(step.name.is_some(), "{step:?}");
            assert_eq!(
                step.ratio, None,
                "a {semitones}-semitone step was handed a ratio: {step:?}"
            );
        }
    }

    #[test]
    fn every_offered_ratio_is_simple_enough_to_mean_something() {
        // Sweep the audible range in fine steps: no reported ratio may be
        // more complicated than the bound, once its octaves are taken back
        // off. This is what stops the search from answering every question.
        let mut sizes = 0;
        for tenth_cents in 0..40_000 {
            let cents = f64::from(tenth_cents) / 10.0;
            let Some(step) = Interval::between(100.0, 100.0 * (cents / 1200.0).exp2()) else {
                continue;
            };
            let Some(ratio) = step.ratio else { continue };
            sizes += 1;
            let mut numerator = ratio.numerator;
            while numerator.is_multiple_of(2) && numerator / 2 >= ratio.denominator {
                numerator /= 2;
            }
            assert!(
                numerator * ratio.denominator <= MAX_RATIO_COMPLEXITY,
                "{cents} cents reported {}:{} which is not a simple ratio",
                ratio.numerator,
                ratio.denominator
            );
        }
        assert!(sizes > 100, "the sweep found almost no ratios at all");
    }

    #[test]
    fn a_tempered_third_is_reported_as_the_sharp_third_it_is() {
        // The most useful thing measurement can say about a piano: its major
        // third sits well above the ratio it is reaching for, and the size in
        // cents is what proves it rather than a claim about taste.
        let step = Interval::between(440.0, 440.0 * 2f64.powf(4.0 / 12.0)).expect("finite pair");
        assert_eq!(step.name, Some("major third"));
        let ratio = step.ratio.expect("near 5:4");
        assert_eq!((ratio.numerator, ratio.denominator), (5, 4));
        assert!(
            (ratio.cents_off - 13.7).abs() < 0.1,
            "expected about fourteen cents sharp of 5:4: {ratio:?}"
        );
    }

    #[test]
    fn identical_notes_are_level_and_are_a_unison() {
        let step = Interval::between(440.0, 440.0).expect("finite pair");
        assert_eq!(step.direction, Direction::Level);
        assert_eq!(step.cents, 0.0);
        assert_eq!(step.name, Some("unison"));
        assert_eq!(
            step.ratio.map(|r| (r.numerator, r.denominator)),
            Some((1, 1))
        );
    }

    #[test]
    fn silence_and_nonsense_have_no_interval() {
        for (from, to) in [
            (0.0, 440.0),
            (440.0, 0.0),
            (-440.0, 440.0),
            (f64::NAN, 440.0),
            (440.0, f64::INFINITY),
        ] {
            assert!(Interval::between(from, to).is_none(), "{from} to {to}");
        }
    }

    #[test]
    fn a_compound_step_reports_its_size_without_claiming_a_name() {
        // An octave and a fifth is not a fifth. The size is exact either way.
        let step = Interval::between(200.0, 600.0).expect("finite pair");
        assert!((step.cents - 1901.955).abs() < 0.01, "{}", step.cents);
        assert_eq!(step.name, None);
        assert_eq!(
            step.ratio.map(|r| (r.numerator, r.denominator)),
            Some((3, 1))
        );
    }

    #[test]
    fn the_description_leads_with_what_was_measured() {
        let line = Interval::between(200.0, 300.0)
            .expect("finite pair")
            .describe();
        assert!(line.starts_with("up 702 cents"), "{line}");
        assert!(line.contains("perfect fifth"), "{line}");
        assert!(line.contains("3:2"), "{line}");
    }
}

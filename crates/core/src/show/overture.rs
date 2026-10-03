//! The Overture: four rooms played as one piece, ending with a dial handed to
//! the player.
//!
//! Each beat is one rule repeated: jump halfway to a corner, square and add,
//! turn by one fixed angle, draw a chord from each point to its double. Three
//! of the four settle on an exact value as they end, so the piece is a set of
//! arrivals rather than a tour. The last arrival is heard as well as seen: the
//! Times Tables parameter voice sounds the ratio `K / (K - 1)`, and as the
//! dial settles on exactly 2 that ratio settles on exactly 2:1, an octave.
//!
//! This module is the score, not a player. It names rooms, exact phases, when
//! each phase lands in the full-motion piece, how long each dissolve lasts,
//! what each beat sounds, and where the piece hands over. Faces perform it:
//! MCP returns one beat per call at the caller's pace, and the App will play
//! it in time. No beat names a room's explanation, and none passes through a
//! graded target: Times Tables stays a whole lobe away from its four-lobe goal.

use super::ShowLookRole;
use crate::motifs::Motif;
use crate::room_by_id;
use crate::rooms::times_tables::TimesTables;

/// A positive rational number held exactly, in lowest terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExactRatio {
    numerator: u32,
    denominator: u32,
}

impl ExactRatio {
    /// `numerator / denominator` in lowest terms, or `None` if either is zero.
    #[must_use]
    pub const fn new(numerator: u32, denominator: u32) -> Option<Self> {
        if numerator == 0 || denominator == 0 {
            return None;
        }
        let divisor = gcd(numerator, denominator);
        Some(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    /// Numerator in lowest terms.
    #[must_use]
    pub const fn numerator(self) -> u32 {
        self.numerator
    }

    /// Denominator in lowest terms.
    #[must_use]
    pub const fn denominator(self) -> u32 {
        self.denominator
    }

    /// The nearest `f64` to the exact value.
    #[must_use]
    pub const fn value(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }
}

const fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let rest = a % b;
        a = b;
        b = rest;
    }
    a
}

/// One exact setting of the Times Tables dial on the way to the octave.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LockStep {
    multiplier: ExactRatio,
}

impl LockStep {
    const fn new(numerator: u32, denominator: u32) -> Self {
        assert!(
            numerator > denominator,
            "the dial's voice ratio is defined only above K = 1"
        );
        match ExactRatio::new(numerator, denominator) {
            Some(multiplier) => Self { multiplier },
            None => panic!("a lock step names a positive multiplier"),
        }
    }

    /// The dial's multiplier `K`, exactly.
    #[must_use]
    pub const fn multiplier(self) -> ExactRatio {
        self.multiplier
    }

    /// The parameter voice's interval, `K / (K - 1)`, exactly.
    ///
    /// With `K = n / d` this is `n / (n - d)`, so an exact dial gives an exact
    /// interval: 43/20 sounds 43:23 and 2 sounds 2:1.
    #[must_use]
    pub const fn voice_ratio(self) -> ExactRatio {
        let numerator = self.multiplier.numerator;
        match ExactRatio::new(numerator, numerator - self.multiplier.denominator) {
            Some(ratio) => ratio,
            None => panic!("the constructor keeps K above 1"),
        }
    }

    /// Whether the voice sounds exactly 2:1.
    #[must_use]
    pub const fn is_octave(self) -> bool {
        let ratio = self.voice_ratio();
        ratio.numerator == 2 && ratio.denominator == 1
    }

    /// Signed distance of the voice from an exact octave, in cents.
    ///
    /// Negative is flat. The octave itself is exactly zero, decided by the
    /// integers rather than by a logarithm.
    #[must_use]
    pub fn cents_from_octave(self) -> f64 {
        if self.is_octave() {
            return 0.0;
        }
        let ratio = self.voice_ratio();
        1200.0 * (f64::from(ratio.numerator) / (2.0 * f64::from(ratio.denominator))).log2()
    }

    /// The canonical Times Tables phase at which the untouched dial reads `K`.
    #[must_use]
    pub const fn phase(self) -> f64 {
        TimesTables::phase_for_multiplier(self.multiplier.value())
    }
}

/// The dial drifting just off 2, easing back, and locking, as exact data.
///
/// Each step's voice is a fixed number of cents flat of an octave until the
/// last, which is the octave itself: about 117, then about 42, then none.
pub const OCTAVE_LOCK: [LockStep; 3] = [
    LockStep::new(43, 20),
    LockStep::new(41, 20),
    LockStep::new(2, 1),
];

/// How a scripted phase change moves toward its key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Ease {
    /// Start and arrive at rest.
    Smooth,
    /// Constant rate, so a zoom that is exponential in phase dives at a
    /// steady pace.
    Linear,
}

impl Ease {
    fn apply(self, progress: f64) -> f64 {
        match self {
            Self::Smooth => progress * progress * (3.0 - 2.0 * progress),
            Self::Linear => progress,
        }
    }
}

/// One exact phase in a beat, and when the full-motion piece reaches it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OvertureKey {
    at_ms: u32,
    move_ms: u32,
    ease: Ease,
    phase: f64,
    beat: &'static str,
}

impl OvertureKey {
    /// Milliseconds from the start of the beat at which the phase arrives.
    #[must_use]
    pub const fn at_ms(self) -> u32 {
        self.at_ms
    }

    /// How long the move into this key lasts; the previous phase holds until
    /// it starts.
    #[must_use]
    pub const fn move_ms(self) -> u32 {
        self.move_ms
    }

    /// The shape of the move into this key.
    #[must_use]
    pub const fn ease(self) -> Ease {
        self.ease
    }

    /// Exact room phase, in `[0, 1)`.
    #[must_use]
    pub const fn phase(self) -> f64 {
        self.phase
    }

    /// Short noninterpretive cue for presenting this key.
    #[must_use]
    pub const fn beat(self) -> &'static str {
        self.beat
    }
}

/// One room's movement in the Overture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OvertureBeat {
    room_id: &'static str,
    question: &'static str,
    length_ms: u32,
    dissolve_ms: u32,
    keys: [OvertureKey; 3],
    still: usize,
    octave_lock: Option<&'static [LockStep; 3]>,
}

impl OvertureBeat {
    /// Canonical catalog room identifier. Every beat plays the canonical
    /// variation, the one its exact phases are written against.
    #[must_use]
    pub const fn room_id(&self) -> &'static str {
        self.room_id
    }

    /// Nonspoiling question that states the beat's rule plainly.
    #[must_use]
    pub const fn question(&self) -> &'static str {
        self.question
    }

    /// Length of the beat in the full-motion piece, before its dissolve.
    #[must_use]
    pub const fn length_ms(&self) -> u32 {
        self.length_ms
    }

    /// The dissolve through black into the next beat, which is also the
    /// length of the audio crossfade. Zero on the hand-off: the last room
    /// does not leave.
    #[must_use]
    pub const fn dissolve_ms(&self) -> u32 {
        self.dissolve_ms
    }

    /// The beat's keys in order: arrival, passage, curtain.
    #[must_use]
    pub const fn keys(&self) -> &[OvertureKey] {
        &self.keys
    }

    /// Index of the reduced-motion still among the keys.
    pub(super) const fn still_index(&self) -> usize {
        self.still
    }

    /// The key held under reduced motion: the beat's finished picture.
    #[must_use]
    pub const fn still(&self) -> OvertureKey {
        self.keys[self.still]
    }

    /// The exact octave lock, on the beat whose voice performs one.
    #[must_use]
    pub fn octave_lock(&self) -> Option<&'static [LockStep]> {
        self.octave_lock.map(|steps| steps.as_slice())
    }

    /// The scripted phase `at_ms` into the beat.
    ///
    /// Before a key's move begins the previous phase holds; during the move
    /// the phase eases toward the key; after the last key it holds. A pure
    /// function of beat time, so any face replays the same performance.
    #[must_use]
    pub fn phase_at(&self, at_ms: u32) -> f64 {
        let mut phase = self.keys[0].phase;
        for key in &self.keys[1..] {
            let start = key.at_ms.saturating_sub(key.move_ms);
            if at_ms <= start {
                break;
            }
            if at_ms >= key.at_ms || key.move_ms == 0 {
                phase = key.phase;
                continue;
            }
            let progress = f64::from(at_ms - start) / f64::from(key.move_ms);
            return phase + (key.phase - phase) * key.ease.apply(progress);
        }
        phase
    }

    /// The dramatic role of the key at `index`.
    pub(super) fn role(&self, index: usize) -> ShowLookRole {
        match index {
            0 => ShowLookRole::Arrival,
            index if index + 1 == self.keys.len() => ShowLookRole::Curtain,
            _ => ShowLookRole::Passage,
        }
    }
}

/// Where a score leaves the player: a room, at an exact phase, theirs to touch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShowHandoff {
    room_id: &'static str,
    phase: f64,
}

impl ShowHandoff {
    /// Canonical catalog room identifier.
    #[must_use]
    pub const fn room_id(self) -> &'static str {
        self.room_id
    }

    /// The exact phase the player receives the room at.
    #[must_use]
    pub const fn phase(self) -> f64 {
        self.phase
    }
}

/// The full Overture score.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Overture {
    id: &'static str,
    title: &'static str,
    invitation: &'static str,
    lead_in_ms: u32,
    beats: [OvertureBeat; 4],
    handoff: ShowHandoff,
}

impl Overture {
    /// Stable score identifier.
    #[must_use]
    pub const fn id(&self) -> &'static str {
        self.id
    }

    /// Player-facing title.
    #[must_use]
    pub const fn title(&self) -> &'static str {
        self.title
    }

    /// Short reason to enter without disclosing the destination.
    #[must_use]
    pub const fn invitation(&self) -> &'static str {
        self.invitation
    }

    /// The near-black opening before the first beat, while the bed fades in
    /// from silence.
    #[must_use]
    pub const fn lead_in_ms(&self) -> u32 {
        self.lead_in_ms
    }

    /// The beats in order.
    #[must_use]
    pub const fn beats(&self) -> &[OvertureBeat] {
        &self.beats
    }

    /// When beat `index` begins in the full-motion piece, or `None` past the
    /// last beat.
    #[must_use]
    pub fn beat_start_ms(&self, index: usize) -> Option<u32> {
        (index < self.beats.len()).then(|| {
            self.lead_in_ms
                + self.beats[..index]
                    .iter()
                    .map(|beat| beat.length_ms + beat.dissolve_ms)
                    .sum::<u32>()
        })
    }

    /// Length of the whole full-motion piece, through the hand-off.
    #[must_use]
    pub fn length_ms(&self) -> u32 {
        self.beat_start_ms(self.beats.len() - 1).unwrap_or(0)
            + self.beats[self.beats.len() - 1].length_ms
    }

    /// The continuous bed under every beat: the hand-off room's own motif, so
    /// the piece never changes key and resolves into that room's bed.
    #[must_use]
    pub fn bed(&self) -> Option<Motif> {
        room_by_id(self.handoff.room_id).and_then(|room| room.motif())
    }

    /// Where the piece leaves the player.
    #[must_use]
    pub const fn handoff(&self) -> ShowHandoff {
        self.handoff
    }
}

const fn key(at_ms: u32, move_ms: u32, ease: Ease, phase: f64, beat: &'static str) -> OvertureKey {
    OvertureKey {
        at_ms,
        move_ms,
        ease,
        phase,
        beat,
    }
}

/// The Overture: Chaos Game, Mandelbrot, Golden Angle, then Times Tables,
/// about eighty-four seconds in full motion.
pub const OVERTURE: Overture = Overture {
    id: "overture",
    title: "The Overture",
    invitation: "Four rooms played as one piece, each a single rule repeated, ending with a dial left in your hand.",
    lead_in_ms: 5_000,
    beats: [
        OvertureBeat {
            room_id: "chaos-game",
            question: "What does chance draw when every jump lands exactly halfway to a random corner?",
            length_ms: 17_400,
            dissolve_ms: 1_600,
            keys: [
                key(
                    0,
                    0,
                    Ease::Smooth,
                    0.5,
                    "Each jump lands a little past halfway, and the triangle comes apart into islands.",
                ),
                key(
                    6_000,
                    4_000,
                    Ease::Smooth,
                    0.25,
                    "The jump eases toward one half, and the islands draw together.",
                ),
                key(
                    11_000,
                    4_000,
                    Ease::Smooth,
                    0.0,
                    "Exactly one half: every gap closes.",
                ),
            ],
            still: 2,
            octave_lock: None,
        },
        OvertureBeat {
            room_id: "mandelbrot",
            question: "What lives at the edge of squaring a number and adding a constant, again and again?",
            length_ms: 18_400,
            dissolve_ms: 1_600,
            keys: [
                key(0, 0, Ease::Linear, 0.0, "The whole set, held still."),
                key(
                    10_700,
                    7_700,
                    Ease::Linear,
                    0.5,
                    "Falling toward the edge at a steady pace.",
                ),
                key(
                    17_630,
                    6_930,
                    Ease::Linear,
                    0.95,
                    "Deep in the valley, and the edge is still not smooth.",
                ),
            ],
            still: 0,
            octave_lock: None,
        },
        OvertureBeat {
            room_id: "golden-angle",
            question: "What happens when every seed turns by the same angle, and the angle slips?",
            length_ms: 14_000,
            dissolve_ms: 2_000,
            keys: [
                key(
                    0,
                    0,
                    Ease::Smooth,
                    0.0,
                    "Every seed turns by the golden angle.",
                ),
                key(
                    11_000,
                    2_000,
                    Ease::Smooth,
                    0.1,
                    "The angle slips by just over a degree, and the seeds fall into spiral arms.",
                ),
                key(
                    13_000,
                    1_500,
                    Ease::Smooth,
                    0.0,
                    "The angle returns and the seeds pack again.",
                ),
            ],
            still: 0,
            octave_lock: None,
        },
        OvertureBeat {
            room_id: "times-tables",
            question: "What does a circle of points draw when each one reaches for its double?",
            length_ms: 24_000,
            dissolve_ms: 0,
            keys: [
                key(
                    0,
                    0,
                    Ease::Smooth,
                    OCTAVE_LOCK[0].phase(),
                    "The dial sits just past two, and the cusp is torn.",
                ),
                key(
                    8_000,
                    4_000,
                    Ease::Smooth,
                    OCTAVE_LOCK[1].phase(),
                    "It eases back toward two.",
                ),
                key(
                    12_000,
                    3_000,
                    Ease::Smooth,
                    OCTAVE_LOCK[2].phase(),
                    "Exactly two. The dial is yours.",
                ),
            ],
            still: 2,
            octave_lock: Some(&OCTAVE_LOCK),
        },
    ],
    handoff: ShowHandoff {
        room_id: "times-tables",
        phase: OCTAVE_LOCK[2].phase(),
    },
};

#[cfg(test)]
mod tests {
    use super::{Ease, ExactRatio, OCTAVE_LOCK, OVERTURE};
    use crate::rooms::times_tables::TimesTables;
    use crate::{Canvas, Room, ShowLookRole, THRESHOLD_ROOM_ID, room_by_id, room_meta_by_id};

    #[test]
    fn exact_ratios_reduce_and_refuse_zero() {
        let ratio = ExactRatio::new(86, 46).expect("ratio");
        assert_eq!((ratio.numerator(), ratio.denominator()), (43, 23));
        assert!(ExactRatio::new(0, 3).is_none());
        assert!(ExactRatio::new(3, 0).is_none());
        assert_eq!(ExactRatio::new(4, 2).expect("ratio").value(), 2.0);
    }

    #[test]
    fn the_octave_lock_converges_from_flat_to_an_exact_two_to_one() {
        let voices = OCTAVE_LOCK
            .iter()
            .map(|step| {
                let ratio = step.voice_ratio();
                (ratio.numerator(), ratio.denominator())
            })
            .collect::<Vec<_>>();
        assert_eq!(voices, [(43, 23), (41, 21), (2, 1)]);
        let cents = OCTAVE_LOCK
            .iter()
            .map(|step| step.cents_from_octave())
            .collect::<Vec<_>>();
        assert!((cents[0] + 116.757).abs() < 0.001, "{cents:?}");
        assert!((cents[1] + 41.719).abs() < 0.001, "{cents:?}");
        assert_eq!(cents[2], 0.0);
        assert!(cents.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(OCTAVE_LOCK[2].is_octave());
        assert!(!OCTAVE_LOCK[0].is_octave() && !OCTAVE_LOCK[1].is_octave());
    }

    #[test]
    fn every_lock_step_is_what_the_room_actually_draws_and_sounds() {
        let room = TimesTables::new();
        for step in OCTAVE_LOCK {
            let k = room.live_multiplier(step.phase(), &[]);
            assert!(
                (k - step.multiplier().value()).abs() < 1e-12,
                "phase {} reads K={k}",
                step.phase()
            );
            let voice = room
                .parameter_sound(step.phase(), &[])
                .expect("Times Tables sounds its dial");
            let exact = step.voice_ratio().value();
            assert!(
                (f64::from(voice.ratio()) - exact).abs() < 1e-6,
                "voice {} against {exact}",
                voice.ratio()
            );
        }
        // At the lock the two tones are an octave apart to float precision,
        // and the room's own snapshot carries exactly those two notes.
        let lock = room.sound(OCTAVE_LOCK[2].phase());
        assert_eq!(lock.notes.len(), 2);
        assert_eq!(lock.notes[1].freq, lock.notes[0].freq * 2.0);
    }

    #[test]
    fn the_overture_never_reaches_the_four_lobe_target() {
        let room = TimesTables::new();
        let beat = &OVERTURE.beats()[3];
        for at_ms in (0..=beat.length_ms()).step_by(100) {
            let k = room.live_multiplier(beat.phase_at(at_ms), &[]);
            assert!(k < 2.2, "at {at_ms} ms the dial reads {k}");
        }
        assert!(!room.goal_met(OVERTURE.handoff().phase(), &[]));
    }

    #[test]
    fn every_beat_plays_a_canonical_room_with_ordered_keys_and_a_question() {
        assert_eq!(OVERTURE.id(), "overture");
        assert_eq!(OVERTURE.title(), "The Overture");
        let rooms = OVERTURE
            .beats()
            .iter()
            .map(|beat| beat.room_id())
            .collect::<Vec<_>>();
        assert_eq!(
            rooms,
            ["chaos-game", "mandelbrot", "golden-angle", "times-tables"]
        );
        for beat in OVERTURE.beats() {
            let metadata = room_meta_by_id(beat.room_id()).expect("catalog room");
            assert_eq!(metadata.id, beat.room_id());
            assert!(beat.question().ends_with('?'));
            assert_eq!(beat.keys().len(), 3);
            let mut previous_arrival = 0;
            for key in beat.keys() {
                assert!((0.0..1.0).contains(&key.phase()), "{}", beat.room_id());
                assert!(key.at_ms() <= beat.length_ms());
                let move_start = key.at_ms().checked_sub(key.move_ms());
                assert!(move_start.is_some_and(|start| start >= previous_arrival));
                assert!(!key.beat().is_empty());
                previous_arrival = key.at_ms();
            }
            assert_eq!(beat.role(0), ShowLookRole::Arrival);
            assert_eq!(beat.role(1), ShowLookRole::Passage);
            assert_eq!(beat.role(2), ShowLookRole::Curtain);
            let room = room_by_id(beat.room_id()).expect("room");
            for key in beat.keys() {
                let mut canvas = Canvas::new(72, 32);
                room.render(&mut canvas, key.phase());
                assert!(canvas.ink_count() > 0, "{} blank", beat.room_id());
            }
        }
        let last = OVERTURE.beats().last().expect("beats");
        assert_eq!(last.dissolve_ms(), 0);
        assert_eq!(OVERTURE.handoff().room_id(), last.room_id());
        assert_eq!(OVERTURE.handoff().phase(), last.still().phase());
        assert_eq!(OVERTURE.handoff().room_id(), THRESHOLD_ROOM_ID);
        assert!(last.octave_lock().is_some());
        assert!(
            OVERTURE.beats()[..3]
                .iter()
                .all(|beat| beat.octave_lock().is_none())
        );
    }

    #[test]
    fn three_of_four_beats_settle_on_an_exact_value_in_their_still() {
        let chaos = &OVERTURE.beats()[0];
        assert_eq!(chaos.still().phase(), 0.0, "jump exactly one half");
        let golden = &OVERTURE.beats()[2];
        assert_eq!(golden.still().phase(), 0.0, "exactly the golden angle");
        assert_eq!(golden.keys()[2].phase(), golden.keys()[0].phase());
        let times = &OVERTURE.beats()[3];
        assert!(times.octave_lock().expect("lock")[2].is_octave());
        let status = room_by_id("chaos-game")
            .expect("room")
            .status(chaos.still().phase())
            .expect("status");
        assert!(status.contains("JUMP 0.50"), "{status}");
    }

    #[test]
    fn the_score_times_add_up_to_about_eighty_four_seconds() {
        assert_eq!(OVERTURE.beat_start_ms(0), Some(5_000));
        assert_eq!(OVERTURE.beat_start_ms(1), Some(24_000));
        assert_eq!(OVERTURE.beat_start_ms(2), Some(44_000));
        assert_eq!(OVERTURE.beat_start_ms(3), Some(60_000));
        assert_eq!(OVERTURE.beat_start_ms(4), None);
        assert_eq!(OVERTURE.length_ms(), 84_000);
        let bed = OVERTURE.bed().expect("the hand-off room has a motif");
        assert_eq!(bed.tempo, 96);
    }

    #[test]
    fn scripted_phase_holds_then_eases_into_each_key() {
        let golden = &OVERTURE.beats()[2];
        assert_eq!(golden.phase_at(0), 0.0);
        assert_eq!(golden.phase_at(8_999), 0.0, "holds before the slip begins");
        let slipping = golden.phase_at(10_000);
        assert!(slipping > 0.0 && slipping < 0.1, "{slipping}");
        assert_eq!(golden.phase_at(11_000), 0.1);
        assert_eq!(golden.phase_at(13_000), 0.0);
        assert_eq!(golden.phase_at(u32::MAX), 0.0);
        let dive = &OVERTURE.beats()[1];
        assert_eq!(dive.phase_at(3_000), 0.0);
        let early = dive.phase_at(3_000 + 770);
        let later = dive.phase_at(3_000 + 1_540);
        assert!(
            (later - 2.0 * early).abs() < 1e-12,
            "a linear dive keeps a constant rate"
        );
        assert_eq!(dive.phase_at(17_630), 0.95);
        assert_eq!(Ease::Smooth.apply(0.5), 0.5);
    }
}

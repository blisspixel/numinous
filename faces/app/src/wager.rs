//! The universal readout wager: call the number before you look.
//!
//! The engineered ahas stage a wager for two flagship rooms, hand-built
//! beat by beat. This is the other half of the same idea, the one that
//! reaches every room instead: `predict` already poses a deterministic
//! question for any room with a moving numeric readout, grades a guess in
//! the same non-punitive bands, and 356 of the catalog's 358 rooms have a
//! status line to pose from. That engine shipped wired to one face. Here
//! it is on the App's own hands.
//!
//! The gesture is the flagships' gesture, generalized: a band along the
//! bottom of the plate, aimed by hand or by arrow key, committed once. The
//! keyboard route is not a courtesy; it is the only hand verb inside a
//! room a keyboard player has, which is a boundary `numinous access`
//! states plainly and this narrows by one.

use numinous_core::{Band, Prediction, PredictionGrade, Room, Surface};

/// Vertical start of the wager band, matching the flagship convention so
/// every wager in the product is aimed with one gesture.
pub(crate) const WAGER_BAND_Y: f64 = 0.88;
/// How far one arrow-key press moves the aim, as a fraction of the span.
/// Fifty steps crosses the whole readout, fine enough to land the Nailed
/// band (a tenth of the span) without a marathon.
const KEY_STEP: f64 = 0.02;

/// A posed readout wager on the current room, while the player aims it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RoomWager {
    prediction: Prediction,
    /// Where the aim sits along the span, in `[0, 1]`.
    aim: f64,
    graded: Option<PredictionGrade>,
}

impl RoomWager {
    /// Pose the room's prediction, or `None` for a room with no moving
    /// numeric readout to call.
    pub(crate) fn pose(room: &dyn Room, seed: u64) -> Option<Self> {
        let prediction = numinous_core::pose_prediction(room, seed)?;
        Some(Self {
            prediction,
            // Open at the middle of the span: a starting point that favors
            // no answer, unlike an endpoint.
            aim: 0.5,
            graded: None,
        })
    }

    /// The value the aim currently names.
    pub(crate) fn aimed_value(&self) -> f64 {
        let (lo, hi) = self.prediction.span;
        lo + (hi - lo) * self.aim
    }

    /// The grade, once the wager has been committed.
    ///
    /// The run path reads the grade through [`Self::status`] and
    /// [`Self::verdict`]; this observer exists for the tests that prove a
    /// call is committed exactly once.
    #[cfg(test)]
    pub(crate) fn graded(&self) -> Option<&PredictionGrade> {
        self.graded.as_ref()
    }

    /// Whether the wager is still open to aiming.
    pub(crate) fn open(&self) -> bool {
        self.graded.is_none()
    }

    /// Aim from a normalized pointer x, ignoring a non-finite hand.
    pub(crate) fn aim_at(&mut self, x: f64) {
        if self.graded.is_some() || !x.is_finite() {
            return;
        }
        self.aim = x.clamp(0.0, 1.0);
    }

    /// Nudge the aim by whole steps, the keyboard's route to the same band.
    pub(crate) fn nudge(&mut self, steps: i32) {
        if self.graded.is_some() {
            return;
        }
        self.aim = (self.aim + f64::from(steps) * KEY_STEP).clamp(0.0, 1.0);
    }

    /// Commit the call and meet the truth. Returns the grade, or `None` if
    /// the wager was already committed or the room's readout vanished.
    pub(crate) fn commit(&mut self, room: &dyn Room) -> Option<&PredictionGrade> {
        if self.graded.is_some() {
            return None;
        }
        let guess = self.aimed_value();
        let grade = numinous_core::grade_prediction(room, &self.prediction, guess)?;
        self.graded = Some(grade);
        self.graded.as_ref()
    }

    /// The footer line: the invite while aiming, the answer once called.
    ///
    /// Non-punitive in the language `predict` already speaks, and it names
    /// the truth either way, because a wager that never meets the truth was
    /// theater.
    pub(crate) fn status(&self) -> String {
        match &self.graded {
            None => format!(
                "CALL {} AT T {:.3}: {}  ARROWS OR DRAG  ENTER: CALL",
                self.prediction.label.to_uppercase(),
                self.prediction.phase,
                fmt_value(self.aimed_value()),
            ),
            Some(grade) => format!(
                "CALLED {}  TRUTH {}  {}",
                fmt_value(grade.guess),
                fmt_value(grade.actual),
                grade.band.name(),
            ),
        }
    }

    /// One graded sentence, spoken once the truth has arrived.
    pub(crate) fn verdict(&self) -> Option<String> {
        let grade = self.graded.as_ref()?;
        let verdict = match grade.band {
            Band::Nailed => "Nailed.",
            Band::Close => "Close: the fertile band.",
            Band::Wild => "A wild swing; the gap is the lesson.",
        };
        Some(format!(
            "You called {} for {}; it reads {}. {verdict}",
            fmt_value(grade.guess),
            self.prediction.label.to_uppercase(),
            fmt_value(grade.actual),
        ))
    }

    /// Draw the wager band: the span, the aim, and the truth once called.
    pub(crate) fn draw(&self, canvas: &mut dyn Surface) {
        let (width, height) = canvas.draw_bounds();
        if width < 16 || height < 6 {
            return;
        }
        let y = (height as f64 * 0.92).round() as i32;
        let y = y.clamp(1, height as i32 - 2);
        let left = (width as f64 * 0.06).round() as i32;
        let right = (width as f64 * 0.94).round() as i32;
        canvas.line(left, y, right, y, '-');
        let at = |unit: f64| {
            let unit = unit.clamp(0.0, 1.0);
            (left as f64 + f64::from(right - left) * unit).round() as i32
        };
        // Quarter ticks give the span a readable ruler.
        for step in 0..=4 {
            let x = at(f64::from(step) / 4.0);
            canvas.line(x, y - 1, x, y + 1, '*');
        }
        let aim_x = at(self.aim);
        canvas.line(aim_x, y - 3, aim_x, y + 1, '#');
        if let Some(grade) = &self.graded {
            let (lo, hi) = self.prediction.span;
            let unit = if (hi - lo).abs() < 1e-12 {
                0.5
            } else {
                (grade.actual - lo) / (hi - lo)
            };
            // The truth reads apart from the aim by shape and side, never
            // by hue: it hangs below the rule where the aim rises above
            // it, and it paints the plain accent, because the magenta
            // mark collapses against warm accents for a color-blind
            // player (the App's own dichromacy scan proved it).
            let truth_x = at(unit);
            canvas.line(truth_x, y + 1, truth_x, y + 3, 'V');
            canvas.line(truth_x - 1, y + 1, truth_x + 1, y + 1, 'V');
        }
    }
}

/// Readable to three places without trailing noise, the same shape the
/// room's own status lines use.
fn fmt_value(value: f64) -> String {
    if !value.is_finite() {
        return "?".to_string();
    }
    let text = format!("{value:.3}");
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() || trimmed == "-" {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::RoomWager;
    use numinous_core::{Band, Canvas};

    fn room(id: &str) -> Box<dyn numinous_core::Room> {
        numinous_core::room_by_id(id).expect("catalog room")
    }

    #[test]
    fn a_readout_room_poses_and_grades_a_call() {
        let room = room("lorenz");
        let mut wager = RoomWager::pose(room.as_ref(), 7).expect("lorenz has a readout");
        assert!(wager.open());
        assert!(wager.graded().is_none());

        // The aim opens at the middle of the span and moves both ways.
        let middle = wager.aimed_value();
        wager.nudge(5);
        assert!(wager.aimed_value() > middle);
        wager.nudge(-5);
        assert!((wager.aimed_value() - middle).abs() < 1e-9);

        wager.aim_at(1.0);
        let high = wager.aimed_value();
        wager.aim_at(0.0);
        assert!(wager.aimed_value() < high, "the band spans the readout");

        let grade = wager.commit(room.as_ref()).expect("the truth exists");
        let (guess, actual) = (grade.guess, grade.actual);
        assert!(!wager.open(), "a call is committed once");
        assert!(wager.commit(room.as_ref()).is_none(), "no second call");

        let verdict = wager.verdict().expect("a committed call is answered");
        assert!(verdict.contains("You called"), "{verdict}");
        assert!(
            verdict.contains("Nailed") || verdict.contains("fertile") || verdict.contains("gap"),
            "the verdict speaks a band: {verdict}"
        );
        // The truth is named whichever way it went.
        assert!(wager.status().contains("TRUTH"), "{}", wager.status());
        assert!(guess.is_finite() && actual.is_finite());
    }

    #[test]
    fn aiming_the_truth_lands_the_nailed_band() {
        // Prove the grading is real: aim where the room actually reads and
        // the band must be Nailed, not merely "some band was spoken".
        let room = room("lorenz");
        let mut wager = RoomWager::pose(room.as_ref(), 11).expect("readout");
        let truth = {
            let mut probe = RoomWager::pose(room.as_ref(), 11).expect("readout");
            probe.commit(room.as_ref()).expect("truth").actual
        };
        let (lo, hi) = (wager.prediction.span.0, wager.prediction.span.1);
        let unit = ((truth - lo) / (hi - lo)).clamp(0.0, 1.0);
        wager.aim_at(unit);
        let grade = wager.commit(room.as_ref()).expect("graded");
        assert_eq!(grade.band, Band::Nailed, "aiming the truth must nail it");
    }

    #[test]
    fn a_committed_call_stops_moving() {
        let room = room("lorenz");
        let mut wager = RoomWager::pose(room.as_ref(), 3).expect("readout");
        wager.commit(room.as_ref()).expect("graded");
        let settled = wager.aimed_value();
        wager.nudge(9);
        wager.aim_at(0.0);
        assert!(
            (wager.aimed_value() - settled).abs() < 1e-12,
            "the aim freezes once the truth is out"
        );
    }

    #[test]
    fn the_band_draws_the_span_the_aim_and_the_truth() {
        let room = room("lorenz");
        let mut wager = RoomWager::pose(room.as_ref(), 5).expect("readout");
        let mut open = Canvas::new(72, 30);
        wager.draw(&mut open);
        let open_ink = open.ink_count();
        assert!(open_ink > 0, "the band is visible while aiming");

        wager.commit(room.as_ref()).expect("graded");
        let mut called = Canvas::new(72, 30);
        wager.draw(&mut called);
        assert!(called.ink_count() > open_ink, "the truth adds its own mark");

        // A plate too small for a readable band draws nothing rather than
        // scribbling over the room.
        let mut tiny = Canvas::new(8, 4);
        wager.draw(&mut tiny);
        assert_eq!(tiny.ink_count(), 0);
    }

    #[test]
    fn a_hostile_hand_cannot_move_the_aim_off_the_span() {
        let room = room("lorenz");
        let mut wager = RoomWager::pose(room.as_ref(), 2).expect("readout");
        let (lo, hi) = (wager.prediction.span.0, wager.prediction.span.1);
        wager.aim_at(f64::NAN);
        assert!(wager.aimed_value().is_finite());
        wager.aim_at(9.0e9);
        assert!(wager.aimed_value() <= hi.max(lo) + 1e-9);
        wager.nudge(i32::MAX);
        assert!(wager.aimed_value() <= hi.max(lo) + 1e-9);
        wager.nudge(i32::MIN);
        assert!(wager.aimed_value() >= lo.min(hi) - 1e-9);
    }

    #[test]
    fn an_ambient_room_with_no_readout_poses_nothing() {
        // Not every room has a number to call; those must refuse rather
        // than invent one.
        let refused = numinous_core::all_rooms()
            .into_iter()
            .find(|room| RoomWager::pose(room.as_ref(), 1).is_none());
        if let Some(room) = refused {
            assert!(
                numinous_core::pose_prediction(room.as_ref(), 1).is_none(),
                "the refusal comes from the shared engine, not from here"
            );
        }
    }
}

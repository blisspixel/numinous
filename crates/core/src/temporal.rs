//! Exact phase pairs and windows for deterministic temporal comparison.

/// Most looks one dwell may carry.
///
/// Staying is not sampling: a player who wants a hundred frames wants an
/// animation, which this face does not pretend to give. A small window keeps
/// the act honest and the render budget bounded.
pub const MAX_DWELL_LOOKS: usize = 8;

/// Fewest looks that can hold still across anything.
pub const MIN_DWELL_LOOKS: usize = 2;

/// Several explicit normalized phases: one player staying in one room.
///
/// A window says only which room states to compare. Like [`TemporalPair`] it
/// asserts no elapsed duration, frame rate, or interpolated path, and repeated
/// phases are valid because looking twice at the same moment is a real thing to
/// do and honestly answers that nothing moved.
#[derive(Debug, Clone, PartialEq)]
pub struct DwellWindow {
    phases: Vec<f64>,
}

impl DwellWindow {
    /// Validates two to [`MAX_DWELL_LOOKS`] finite phases in `[0.0, 1.0)`.
    #[must_use]
    pub fn new(phases: Vec<f64>) -> Option<Self> {
        ((MIN_DWELL_LOOKS..=MAX_DWELL_LOOKS).contains(&phases.len())
            && phases
                .iter()
                .all(|phase| phase.is_finite() && (0.0..1.0).contains(phase)))
        .then_some(Self { phases })
    }

    /// The ordered phases, as given.
    #[must_use]
    pub fn phases(&self) -> &[f64] {
        &self.phases
    }

    /// How many looks this window holds.
    #[must_use]
    pub fn looks(&self) -> usize {
        self.phases.len()
    }
}

/// Two explicit normalized phases, ordered from observation to destination.
///
/// A pair says only which two room states to compare. It does not assert an
/// elapsed duration, frame rate, interpolated path, or wall-clock ordering.
/// Equal and decreasing pairs are valid because a caller may need to verify a
/// fixed point or compare across the room's normalized loop boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TemporalPair {
    from_t: f64,
    to_t: f64,
}

impl TemporalPair {
    /// Validates two finite phases in `[0.0, 1.0)`.
    #[must_use]
    pub fn new(from_t: f64, to_t: f64) -> Option<Self> {
        (from_t.is_finite()
            && to_t.is_finite()
            && (0.0..1.0).contains(&from_t)
            && (0.0..1.0).contains(&to_t))
        .then_some(Self { from_t, to_t })
    }

    /// The first observation phase.
    #[must_use]
    pub const fn from_t(self) -> f64 {
        self.from_t
    }

    /// The destination phase.
    #[must_use]
    pub const fn to_t(self) -> f64 {
        self.to_t
    }
}

#[cfg(test)]
mod tests {
    use super::{DwellWindow, MAX_DWELL_LOOKS, TemporalPair};

    /// The mathematical core reads no wall clock, so a turn can be taken at any
    /// interval at all.
    ///
    /// This has been true since the beginning and has never been an invariant,
    /// which means it could regress in a single commit and nobody would notice
    /// until a participant on a slow clock arrived and found the room had moved
    /// under them. A room's state is a function of its phase and its touches;
    /// phases are exact and carry no duration; input history is bounded by
    /// count and never by age. Nothing here decays and nothing times out, so a
    /// visit resumed after a second and a visit resumed after a decade meet the
    /// same room.
    ///
    /// File locking is the one exception and it is not about play: stale-lock
    /// recovery and lock-token uniqueness genuinely need the host clock, and
    /// they touch no room, no phase, and no grade.
    #[test]
    fn the_core_of_the_world_does_not_read_a_clock() {
        // Spelled in halves so this scan does not find its own needles and so
        // the file that owns the rule is still covered by it.
        let clocks = [
            format!("{}{}", "System", "Time"),
            format!("{}{}", "Instant", "::now"),
            format!("{}{}", "UNIX_", "EPOCH"),
        ];
        /// Where a clock is allowed, and why.
        const LOCKING: &str = "persistence.rs";

        let core = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut found = Vec::new();
        let mut walk = vec![core.clone()];
        while let Some(directory) = walk.pop() {
            let Ok(entries) = std::fs::read_dir(&directory) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk.push(path);
                    continue;
                }
                if path.extension().is_none_or(|kind| kind != "rs") {
                    continue;
                }
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                if name == LOCKING {
                    continue;
                }
                let Ok(source) = std::fs::read_to_string(&path) else {
                    continue;
                };
                for line in source.lines() {
                    // The doc comment above says the word, so read code only.
                    let code = line.trim_start();
                    if code.starts_with("//") || code.starts_with("///") {
                        continue;
                    }
                    if clocks.iter().any(|clock| code.contains(clock.as_str())) {
                        found.push(format!("{}: {}", name, code.trim()));
                    }
                }
            }
        }
        assert!(
            found.is_empty(),
            "the core reads a wall clock outside {LOCKING}, so a room can now \
             move under a participant who answers slowly: {}",
            found.join("; ")
        );
    }

    #[test]
    fn a_window_accepts_two_through_the_maximum_looks() {
        for looks in 2..=MAX_DWELL_LOOKS {
            let phases: Vec<f64> = (0..looks).map(|look| look as f64 / 16.0).collect();
            let window = DwellWindow::new(phases.clone()).expect("valid window");
            assert_eq!(window.looks(), looks);
            assert_eq!(window.phases(), phases.as_slice());
        }
    }

    #[test]
    fn a_window_refuses_a_single_look_or_a_crowd() {
        assert!(DwellWindow::new(vec![]).is_none());
        assert!(DwellWindow::new(vec![0.5]).is_none());
        let crowd: Vec<f64> = (0..=MAX_DWELL_LOOKS)
            .map(|look| look as f64 / 32.0)
            .collect();
        assert!(DwellWindow::new(crowd).is_none());
    }

    #[test]
    fn a_window_refuses_phases_outside_the_loop() {
        for phases in [
            vec![0.2, f64::NAN],
            vec![0.2, f64::INFINITY],
            vec![-f64::EPSILON, 0.2],
            vec![0.2, 1.0],
        ] {
            assert!(DwellWindow::new(phases).is_none());
        }
    }

    #[test]
    fn a_window_keeps_repeated_phases_because_looking_twice_is_real() {
        let window = DwellWindow::new(vec![0.4, 0.4, 0.4]).expect("valid window");
        assert_eq!(window.looks(), 3);
    }

    #[test]
    fn accepts_forward_equal_and_decreasing_exact_phases() {
        for (from_t, to_t) in [(0.2, 0.35), (0.5, 0.5), (0.95, 0.05)] {
            let pair = TemporalPair::new(from_t, to_t).expect("valid pair");
            assert_eq!(pair.from_t(), from_t);
            assert_eq!(pair.to_t(), to_t);
        }
    }

    #[test]
    fn rejects_nonfinite_and_out_of_range_phases() {
        for (from_t, to_t) in [
            (f64::NAN, 0.0),
            (0.0, f64::INFINITY),
            (-f64::EPSILON, 0.0),
            (0.0, -f64::EPSILON),
            (1.0, 0.0),
            (0.0, 1.0),
        ] {
            assert!(TemporalPair::new(from_t, to_t).is_none());
        }
    }
}

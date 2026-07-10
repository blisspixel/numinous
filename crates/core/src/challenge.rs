//! The challenge: a posed, seeded, graded touch goal for an interactive room.
//!
//! Ruling 13 of the July 2026 review (`docs/REVIEW.md`): challenge specs are
//! metrics, not binary. A challenge poses a target box on the frame and asks
//! the player to make the room's math answer inside it. The grade reports
//! distances and fractions a mind can descend like a gradient; `passed` is a
//! convenience summary, never the only signal. Everything is deterministic:
//! the same room, seed, and frame pose the same goal, and the same attempt
//! earns the same grade, so challenges are replayable and comparable across
//! minds, exactly like the daily games.

use crate::canvas::Canvas;
use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room};
use crate::surface::Surface;

/// A posed touch goal: change at least `min_cells` cells inside `target`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Challenge {
    /// The room this challenge is posed for.
    pub room: String,
    /// The seed the target was drawn from; the same seed reposes the same goal.
    pub seed: u64,
    /// Frame width in cells; attempts are graded on exactly this frame.
    pub width: usize,
    /// Frame height in cells.
    pub height: usize,
    /// Inclusive target box as (x0, y0, x1, y1).
    pub target: (usize, usize, usize, usize),
    /// The response threshold: changed cells inside the box needed to pass.
    pub min_cells: usize,
    /// The goal, spoken plainly.
    pub goal: String,
}

/// A graded attempt. Metrics first; `passed` is the summary, not the signal.
#[derive(Debug, Clone, PartialEq)]
pub struct ChallengeGrade {
    /// Changed cells that landed inside the target box.
    pub cells_in_target: usize,
    /// All changed cells, anywhere on the frame.
    pub cells_changed: usize,
    /// Progress toward the threshold, `cells_in_target / min_cells`, capped at 1.
    pub threshold_fraction: f64,
    /// Distance from the centroid of all changed cells to the target center,
    /// in cells; the frame diagonal when nothing changed at all.
    pub center_distance: f64,
    /// Whether the threshold was met.
    pub passed: bool,
    /// A graded 0-100 score: mostly threshold progress, partly proximity.
    pub score: u32,
}

/// A posed parameter goal: sweep the room until its own readout lands on a
/// target number. Ruling 13's deeper half: "find the stall angle to one
/// decimal" style, where the metric is the phenomenon's own parameter. The
/// readout is taken from [`Room::status`], the same line the player sees, so
/// the goal and the instrument can never disagree.
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterGoal {
    /// The room this goal is posed for.
    pub room: String,
    /// The seed the target was drawn from.
    pub seed: u64,
    /// Which number in the status line is the readout: the first one that
    /// moves across the sweep. Lissajous reads "X:Y = 3:2.00" where the 3 is
    /// a constant tuning, so position matters, not just presence.
    pub index: usize,
    /// The readout's label, straight from the status line (e.g. "TILT").
    pub label: String,
    /// The value to land on.
    pub target: f64,
    /// How close counts as landed.
    pub tolerance: f64,
    /// The full observed range of the readout across the sweep.
    pub span: (f64, f64),
    /// The goal, spoken plainly.
    pub goal: String,
}

/// A graded parameter attempt. Metrics first, as always.
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterGrade {
    /// The readout at the attempted phase.
    pub value: f64,
    /// Distance from the target.
    pub distance: f64,
    /// Whether the value landed within tolerance.
    pub within: bool,
    /// A graded 0-100 score: closeness across the observed span.
    pub score: u32,
}

/// How many phases the sweep is sampled at while posing a parameter goal.
const PARAMETER_SAMPLES: usize = 64;

/// Every number in a status line, as (byte offset of its first character,
/// parsed value), left to right. A sign counts as part of a number only when
/// a digit follows it, so "X:Y = 3:2.00" yields the 3 and the 2.00 and a
/// hyphen in prose never starts a phantom number.
fn status_numbers(status: &str) -> Vec<(usize, f64)> {
    let bytes = status.as_bytes();
    let mut numbers = Vec::new();
    let mut start = 0;
    while start < bytes.len() {
        let c = bytes[start] as char;
        if c.is_ascii_digit()
            || ((c == '-' || c == '+')
                && bytes
                    .get(start + 1)
                    .is_some_and(|next| (*next as char).is_ascii_digit()))
        {
            let mut end = start + 1;
            while end < bytes.len() && ((bytes[end] as char).is_ascii_digit() || bytes[end] == b'.')
            {
                end += 1;
            }
            if let Ok(value) = status[start..end].parse() {
                numbers.push((start, value));
            }
            start = end;
        } else {
            start += 1;
        }
    }
    numbers
}

/// The readout's label: the status text before the number at byte offset
/// `cut`, trimmed of separators. Falls back to "READOUT" for label-less
/// lines.
fn status_label(status: &str, cut: usize) -> String {
    let label = status[..cut]
        .trim()
        .trim_end_matches(['=', ':', ' '])
        .trim();
    if label.is_empty() {
        "READOUT".to_string()
    } else {
        label.to_string()
    }
}

/// A goal number with three significant digits, so tiny tolerances read at
/// their true scale instead of vanishing into "0.000".
fn fmt_sig(x: f64) -> String {
    let decimals = sig_decimals(x);
    format!("{x:.decimals$}")
}

/// A tolerance for prose, rounded DOWN at its last displayed digit: the
/// spoken bound may be tighter than the grader enforces, never looser, so
/// text can never call an attempt inside when the grade says otherwise.
fn fmt_tolerance(x: f64) -> String {
    let decimals = sig_decimals(x);
    let scale = 10f64.powi(decimals as i32);
    let floored = (x * scale).floor() / scale;
    format!("{floored:.decimals$}")
}

/// How many decimals give roughly three significant digits for `x`.
fn sig_decimals(x: f64) -> usize {
    if x == 0.0 || !x.is_finite() {
        return 3;
    }
    let magnitude = x.abs().log10().floor() as i32;
    (2 - magnitude).max(0) as usize
}

/// The readout's spoken name at a chosen column: the label immediately before
/// it, unless that prefix swallowed an earlier number (Lissajous's "X:Y = 3:"
/// before the moving second component), in which case fall back to the line's
/// name, the text before its first number. Keeps the goal string clean
/// ("X:Y", not "X:Y = 3") without hardcoding any room.
fn readout_label(status: &str, numbers: &[(usize, f64)], index: usize) -> String {
    let precise = status_label(status, numbers[index].0);
    if precise.chars().any(|c| c.is_ascii_digit()) {
        status_label(status, numbers[0].0)
    } else {
        precise
    }
}

/// Pose the deterministic parameter goal for a room and seed, or `None` for
/// rooms without a numeric readout or whose readout never moves. The readout
/// is the first column that is present and label-stable across every sample
/// and that varies across the sweep (constant numbers are labels or tunings,
/// not readouts). The target is one of the sweep's own sampled values, so
/// every posed goal is reachable by construction: some phase lands on it.
#[must_use]
pub fn pose_parameter_goal(room: &dyn Room, seed: u64) -> Option<ParameterGoal> {
    let mut statuses = Vec::with_capacity(PARAMETER_SAMPLES);
    for i in 0..PARAMETER_SAMPLES {
        let t = i as f64 / PARAMETER_SAMPLES as f64;
        let status = room.status(t)?;
        let numbers = status_numbers(&status);
        statuses.push((status, numbers));
    }
    // Read only the leading columns present in every sample. Times Tables'
    // status carries a trailing note whose own number comes and goes
    // ("CLOSED: 5 LOBES" vs "OPEN, WANDERING"), so the total count is not
    // stable; the K column in front of it is, and that is the readout.
    let min_columns = statuses.iter().map(|(_, n)| n.len()).min().unwrap_or(0);
    let (index, lo, hi) = (0..min_columns).find_map(|index| {
        // Alignment guard: column `index` must carry the same label in every
        // sample, so it is provably the same readout across the sweep.
        let name = readout_label(&statuses[0].0, &statuses[0].1, index);
        let aligned = statuses
            .iter()
            .all(|(s, n)| readout_label(s, n, index) == name);
        if !aligned {
            return None;
        }
        let column = statuses.iter().map(|(_, n)| n[index].1);
        let lo = column.clone().fold(f64::INFINITY, f64::min);
        let hi = column.fold(f64::NEG_INFINITY, f64::max);
        let moving = lo.is_finite() && hi.is_finite() && hi - lo >= 1e-9;
        moving.then_some((index, lo, hi))
    })?;
    let meta = room.meta();
    let mut rng = SplitMix64::new(seed ^ fnv1a(meta.id.as_bytes()) ^ 0x5041_5241);
    let target = statuses[rng.below(PARAMETER_SAMPLES as u64) as usize].1[index].1;
    let tolerance = (hi - lo) / 40.0;
    let label = readout_label(&statuses[0].0, &statuses[0].1, index);
    let goal = format!(
        "SWEEP {} UNTIL {label} LANDS WITHIN {} OF {}",
        meta.title.to_uppercase(),
        fmt_tolerance(tolerance),
        fmt_sig(target),
    );
    Some(ParameterGoal {
        room: meta.id.to_string(),
        seed,
        index,
        label,
        target,
        tolerance,
        span: (lo, hi),
        goal,
    })
}

/// Grade a parameter attempt at phase `t`: read the room's own readout and
/// measure the distance. Returns `None` only if the room's status vanished
/// or its readout column went missing, which no catalog room does.
#[must_use]
pub fn grade_parameter(room: &dyn Room, goal: &ParameterGoal, t: f64) -> Option<ParameterGrade> {
    let status = room.status(t)?;
    let value = status_numbers(&status).get(goal.index)?.1;
    let distance = (value - goal.target).abs();
    let span = (goal.span.1 - goal.span.0).max(1e-9);
    let score = (100.0 * (1.0 - (distance / span).min(1.0))).round() as u32;
    Some(ParameterGrade {
        value,
        distance,
        within: distance <= goal.tolerance,
        score,
    })
}

/// How many seeded probe hands posing may try before giving up.
const PROBE_ATTEMPTS: usize = 8;

/// The phases each probe hand is tried at: some rooms answer the hand only
/// once their animation reaches it (Goldbach's comet draws the selected
/// witness later in the sweep), so posing must look at more than phase zero.
const PROBE_PHASES: [f64; 3] = [0.0, 0.5, 0.9];

/// Pose the deterministic challenge for a room, seed, and frame size.
///
/// Returns `None` for rooms without a touch verb: a room that does not answer
/// hands cannot fairly be asked to. Winnability is guaranteed by
/// construction, not assumed: posing probes the room with a seeded hand,
/// centers the target box on the measured response, and sets the threshold
/// at or below what that witness hand actually changed inside the box. Rooms
/// answer in very different places (Voronoi answers at the hand; Goldbach
/// answers along the comet), so a box placed blind can be provably
/// impossible; a box placed on evidence never is.
#[must_use]
pub fn pose_challenge(
    room: &dyn Room,
    seed: u64,
    width: usize,
    height: usize,
) -> Option<Challenge> {
    room.verb()?;
    let frame = Canvas::new(width, height);
    let (width, height) = (frame.width(), frame.height());
    if width < 8 || height < 8 {
        return None;
    }
    let meta = room.meta();
    let box_w = (width / 4).max(4);
    let box_h = (height / 4).max(4);
    let cap = (box_w * box_h / 12).max(2);
    for (attempt, phase) in
        (0..PROBE_ATTEMPTS).flat_map(|a| PROBE_PHASES.iter().map(move |&p| (a, p)))
    {
        let probe = probe_hand(meta.id, seed, attempt);
        let changed = probed_change(room, width, height, &probe, phase);
        if changed.is_empty() {
            continue;
        }
        // Place the box where the measured response is densest, so even a
        // multi-modal answer (markers scattered along a comet) yields a
        // target the witness hand provably reaches.
        let (x0, y0, witness_in_box) = densest_box(&changed, width, height, box_w, box_h);
        let target = (x0, y0, x0 + box_w - 1, y0 + box_h - 1);
        if witness_in_box < 2 {
            continue;
        }
        let min_cells = witness_in_box.min(cap);
        let goal = format!(
            "TOUCH {} SO AT LEAST {min_cells} CELLS CHANGE INSIDE ({},{})..({},{}) ON A {width}x{height} FRAME",
            meta.title.to_uppercase(),
            target.0,
            target.1,
            target.2,
            target.3,
        );
        return Some(Challenge {
            room: meta.id.to_string(),
            seed,
            width,
            height,
            target,
            min_cells,
            goal,
        });
    }
    None
}

/// The seeded probe hand for a pose attempt: three clustered points kept off
/// the extreme edges, so rooms whose per-poke answer is small still show a
/// measurable response.
fn probe_hand(id: &str, seed: u64, attempt: usize) -> Vec<(f64, f64)> {
    let mut rng =
        SplitMix64::new(seed ^ fnv1a(id.as_bytes()) ^ (attempt as u64).wrapping_mul(0x9e37));
    let x = 0.15 + 0.6 * rng.next_f64();
    let y = 0.15 + 0.6 * rng.next_f64();
    vec![(x, y), (x + 0.08, y), (x, y + 0.08)]
}

/// The box position holding the most changed cells, found exactly with a 2D
/// prefix sum over the frame (tiny: the standard frame is 72x32). Ties break
/// toward the top-left so posing stays deterministic. Returns (x0, y0, count).
fn densest_box(
    changed: &[(usize, usize)],
    width: usize,
    height: usize,
    box_w: usize,
    box_h: usize,
) -> (usize, usize, usize) {
    // prefix[y][x] counts changed cells in the rectangle [0, x) x [0, y).
    let mut prefix = vec![vec![0usize; width + 1]; height + 1];
    for &(x, y) in changed {
        prefix[y + 1][x + 1] += 1;
    }
    for y in 0..height {
        for x in 0..width {
            prefix[y + 1][x + 1] += prefix[y][x + 1] + prefix[y + 1][x] - prefix[y][x];
        }
    }
    let mut best = (0, 0, 0);
    for y0 in 0..=height - box_h {
        for x0 in 0..=width - box_w {
            let (x1, y1) = (x0 + box_w, y0 + box_h);
            let count = prefix[y1][x1] + prefix[y0][x0] - prefix[y0][x1] - prefix[y1][x0];
            if count > best.2 {
                best = (x0, y0, count);
            }
        }
    }
    best
}

/// The cells a probe hand changes at the given phase on the given frame.
fn probed_change(
    room: &dyn Room,
    width: usize,
    height: usize,
    probe: &[(f64, f64)],
    phase: f64,
) -> Vec<(usize, usize)> {
    let mut base = Canvas::new(width, height);
    room.render(&mut base, phase);
    let mut poked = Canvas::new(width, height);
    room.render_poked(&mut poked, phase, probe);
    let mut changed = Vec::new();
    for y in 0..height {
        for x in 0..width {
            if base.cell(x, y) != poked.cell(x, y) {
                changed.push((x, y));
            }
        }
    }
    changed
}

/// Grade an attempt: render the room bare and with the hand points at the
/// challenge's own frame size and phase `t`, then measure the answer.
///
/// Hand history is bounded to [`MAX_ROOM_POKES`] newest-last, matching every
/// other face path, so a grade can never be bought with an unbounded hand.
#[must_use]
pub fn grade_challenge(
    room: &dyn Room,
    challenge: &Challenge,
    t: f64,
    pokes: &[(f64, f64)],
) -> ChallengeGrade {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    let pokes = &pokes[start..];
    let mut base = Canvas::new(challenge.width, challenge.height);
    room.render(&mut base, t);
    let mut poked = Canvas::new(challenge.width, challenge.height);
    room.render_poked(&mut poked, t, pokes);

    let (x0, y0, x1, y1) = challenge.target;
    let mut cells_in_target = 0usize;
    let mut cells_changed = 0usize;
    let (mut sum_x, mut sum_y) = (0.0f64, 0.0f64);
    for y in 0..challenge.height {
        for x in 0..challenge.width {
            if base.cell(x, y) == poked.cell(x, y) {
                continue;
            }
            cells_changed += 1;
            sum_x += x as f64;
            sum_y += y as f64;
            if (x0..=x1).contains(&x) && (y0..=y1).contains(&y) {
                cells_in_target += 1;
            }
        }
    }

    let diagonal =
        ((challenge.width * challenge.width + challenge.height * challenge.height) as f64).sqrt();
    let center_distance = if cells_changed == 0 {
        diagonal
    } else {
        let cx = sum_x / cells_changed as f64;
        let cy = sum_y / cells_changed as f64;
        let tx = (x0 + x1) as f64 / 2.0;
        let ty = (y0 + y1) as f64 / 2.0;
        ((cx - tx).powi(2) + (cy - ty).powi(2)).sqrt()
    };
    let threshold_fraction = (cells_in_target as f64 / challenge.min_cells as f64).min(1.0);
    let proximity = (1.0 - center_distance / diagonal).max(0.0);
    let score = (100.0 * (0.7 * threshold_fraction + 0.3 * proximity)).round() as u32;
    ChallengeGrade {
        cells_in_target,
        cells_changed,
        threshold_fraction,
        center_distance,
        passed: cells_in_target >= challenge.min_cells,
        score,
    }
}

/// FNV-1a over bytes: a tiny, stable hash to mix a room id into a seed.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::{grade_challenge, pose_challenge};
    use crate::registry::room_by_id;

    fn interactive_room() -> Box<dyn crate::room::Room> {
        let room = room_by_id("voronoi").expect("voronoi exists");
        assert!(room.verb().is_some(), "voronoi answers the hand");
        room
    }

    #[test]
    fn posing_is_deterministic_and_inside_the_frame() {
        let room = interactive_room();
        let a = pose_challenge(room.as_ref(), 7, 72, 32).expect("interactive rooms pose");
        let b = pose_challenge(room.as_ref(), 7, 72, 32).expect("same seed, same goal");
        assert_eq!(a, b);
        let (x0, y0, x1, y1) = a.target;
        assert!(x0 <= x1 && x1 < a.width);
        assert!(y0 <= y1 && y1 < a.height);
        assert!(a.min_cells >= 2);
        assert!(a.goal.contains("CELLS CHANGE"));
    }

    #[test]
    fn different_seeds_move_the_target() {
        let room = interactive_room();
        let targets: Vec<_> = (0..8)
            .map(|seed| {
                pose_challenge(room.as_ref(), seed, 72, 32)
                    .expect("poses")
                    .target
            })
            .collect();
        assert!(
            targets.windows(2).any(|w| w[0] != w[1]),
            "eight seeds should not all pose the same box"
        );
    }

    #[test]
    fn quiet_rooms_and_tiny_frames_pose_nothing() {
        // Derive a verbless room from the registry so this test cannot go
        // vacuous if a hardcoded room later gains a verb.
        if let Some(quiet) = crate::registry::all_rooms()
            .into_iter()
            .find(|room| room.verb().is_none())
        {
            assert!(pose_challenge(quiet.as_ref(), 1, 72, 32).is_none());
        }
        let room = interactive_room();
        assert!(pose_challenge(room.as_ref(), 1, 4, 4).is_none());
    }

    #[test]
    fn a_well_aimed_hand_can_actually_pass() {
        let room = interactive_room();
        let challenge = pose_challenge(room.as_ref(), 7, 72, 32).expect("poses");
        let (x0, y0, x1, y1) = challenge.target;
        // Spread five wells across the target box; Voronoi border
        // renegotiation changes far more cells than the threshold asks for.
        let to_norm = |x: usize, y: usize| {
            (
                x as f64 / (challenge.width - 1) as f64,
                y as f64 / (challenge.height - 1) as f64,
            )
        };
        let pokes = vec![
            to_norm((x0 + x1) / 2, (y0 + y1) / 2),
            to_norm(x0 + 1, y0 + 1),
            to_norm(x1 - 1, y0 + 1),
            to_norm(x0 + 1, y1 - 1),
            to_norm(x1 - 1, y1 - 1),
        ];
        let grade = grade_challenge(room.as_ref(), &challenge, 0.0, &pokes);
        assert!(
            grade.passed,
            "a hand spread across the target must clear the threshold: {grade:?}"
        );
        assert!(grade.cells_in_target >= challenge.min_cells);
        assert!((grade.threshold_fraction - 1.0).abs() < f64::EPSILON);
        assert!(grade.score > 70, "a pass scores high: {}", grade.score);
    }

    #[test]
    fn a_touch_inside_the_target_outgrades_an_empty_hand() {
        let room = interactive_room();
        let challenge = pose_challenge(room.as_ref(), 7, 72, 32).expect("poses");
        let (x0, y0, x1, y1) = challenge.target;
        let cx = (x0 + x1) as f64 / 2.0 / (challenge.width - 1) as f64;
        let cy = (y0 + y1) as f64 / 2.0 / (challenge.height - 1) as f64;
        let touched = grade_challenge(room.as_ref(), &challenge, 0.0, &[(cx, cy)]);
        let empty = grade_challenge(room.as_ref(), &challenge, 0.0, &[]);
        assert_eq!(empty.cells_changed, 0);
        assert_eq!(empty.score, 0);
        assert!(!empty.passed);
        assert!(
            touched.cells_changed > 0,
            "a dropped well changes the frame"
        );
        assert!(touched.score > empty.score);
        assert!(touched.center_distance < empty.center_distance);
    }

    #[test]
    fn grading_is_deterministic_and_bounds_the_hand() {
        let room = interactive_room();
        let challenge = pose_challenge(room.as_ref(), 3, 72, 32).expect("poses");
        let a = grade_challenge(room.as_ref(), &challenge, 0.25, &[(0.5, 0.5)]);
        let b = grade_challenge(room.as_ref(), &challenge, 0.25, &[(0.5, 0.5)]);
        assert_eq!(a, b);
        // An oversized hand history grades exactly like its newest bounded tail.
        let mut flood: Vec<(f64, f64)> = (0..200).map(|i| (i as f64 / 200.0, 0.1)).collect();
        flood.push((0.5, 0.5));
        let bounded_tail = &flood[flood.len() - crate::room::MAX_ROOM_POKES..];
        let flooded = grade_challenge(room.as_ref(), &challenge, 0.25, &flood);
        let tail = grade_challenge(room.as_ref(), &challenge, 0.25, bounded_tail);
        assert_eq!(flooded, tail);
    }

    #[test]
    fn the_metrics_stay_graded_not_binary() {
        let room = interactive_room();
        let challenge = pose_challenge(room.as_ref(), 7, 72, 32).expect("poses");
        // A touch far from the target still earns proximity-graded feedback.
        let (x0, _, x1, _) = challenge.target;
        let far_x = if (x0 + x1) / 2 < challenge.width / 2 {
            0.95
        } else {
            0.05
        };
        let far = grade_challenge(room.as_ref(), &challenge, 0.0, &[(far_x, 0.5)]);
        assert!(far.cells_changed > 0);
        assert!(far.center_distance > 0.0);
        assert!(far.score < 100, "distance must cost score");
    }

    #[test]
    fn status_numbers_and_labels_parse_from_real_readouts() {
        let values = |s: &str| {
            super::status_numbers(s)
                .iter()
                .map(|n| n.1)
                .collect::<Vec<_>>()
        };
        assert_eq!(values("TILT = +0.33"), vec![0.33]);
        assert_eq!(values("DETUNE -0.041"), vec![-0.041]);
        assert_eq!(values("X:Y = 3:2.00"), vec![3.0, 2.0]);
        assert_eq!(values("no numbers here"), Vec::<f64>::new());
        // A hyphen in prose starts no phantom number, so the label cut and
        // the value parser can never disagree about where a number begins.
        assert_eq!(values("E-FIELD 0.5"), vec![0.5]);
        let label =
            |s: &str, index: usize| super::status_label(s, super::status_numbers(s)[index].0);
        assert_eq!(label("TILT = +0.33", 0), "TILT");
        assert_eq!(label("HEIGHT = SLOPE = 1.85", 0), "HEIGHT = SLOPE");
        assert_eq!(label("X:Y = 3:2.00", 1), "X:Y = 3");
        assert_eq!(label("E-FIELD 0.5", 0), "E-FIELD");
        assert_eq!(super::status_label("1.85", 0), "READOUT");
    }

    #[test]
    fn spoken_tolerances_are_never_looser_than_the_graded_bound() {
        for &tolerance in &[0.00148, 0.0015, 0.31, 12.5, 300.0] {
            let spoken: f64 = super::fmt_tolerance(tolerance).parse().expect("parses");
            assert!(
                spoken <= tolerance,
                "prose must bound from below: {spoken} vs {tolerance}"
            );
            assert!(spoken > 0.0, "and never vanish to zero: {tolerance}");
        }
        assert_eq!(super::fmt_sig(0.00148), "0.00148");
        assert_eq!(super::fmt_sig(12.5), "12.5");
    }

    #[test]
    fn lissajous_poses_on_its_moving_ratio_not_the_constant_tuning() {
        // "X:Y = 3:2.00": the 3 is a fixed tuning, the 2.00 sweeps. The goal
        // must target the number that moves, or the room could never pose.
        let room = crate::registry::room_by_id("lissajous").expect("room");
        let goal = super::pose_parameter_goal(room.as_ref(), 5).expect("the moving column poses");
        assert_eq!(goal.index, 1, "the constant first column is skipped");
        assert!(goal.span.0 < goal.span.1);
        // The spoken label names the line, not the swallowed tuning number:
        // "X:Y", never "X:Y = 3".
        assert_eq!(goal.label, "X:Y", "goal reads clean: {}", goal.goal);
        assert!(!goal.label.chars().any(|c| c.is_ascii_digit()));
    }

    #[test]
    fn times_tables_poses_on_its_moving_k_despite_a_wandering_note() {
        // Times Tables' status is "K = {k}   {note}" where the note's own
        // number comes and goes ("CLOSED: 5 LOBES" vs "OPEN, WANDERING").
        // The whole-line count is unstable, but the leading K column is the
        // readout and it sweeps; the room must pose on it, not decline.
        let room = crate::registry::room_by_id("times-tables").expect("room");
        let goal =
            super::pose_parameter_goal(room.as_ref(), 1).expect("K is a moving, aligned readout");
        assert_eq!(goal.index, 0);
        assert_eq!(goal.label, "K");
        assert!(goal.span.0 < goal.span.1, "K sweeps: {:?}", goal.span);
        let landed = (0..super::PARAMETER_SAMPLES).any(|i| {
            let t = i as f64 / super::PARAMETER_SAMPLES as f64;
            super::grade_parameter(room.as_ref(), &goal, t).is_some_and(|g| g.within)
        });
        assert!(landed, "reachable at a sampled phase");
    }

    #[test]
    fn parameter_goals_pose_deterministically_and_are_reachable() {
        let room = crate::registry::room_by_id("slope-rider").expect("room");
        let a = super::pose_parameter_goal(room.as_ref(), 7).expect("slope-rider has a readout");
        let b = super::pose_parameter_goal(room.as_ref(), 7).expect("same seed, same goal");
        assert_eq!(a, b);
        assert!(a.tolerance > 0.0);
        assert!(a.span.0 < a.span.1);
        assert!(
            a.goal.contains("TILT"),
            "the goal names the readout: {}",
            a.goal
        );
        // Reachable by construction: the target is one of the sweep's own
        // sampled values, so some sampled phase lands within tolerance.
        let landed = (0..super::PARAMETER_SAMPLES).any(|i| {
            let t = i as f64 / super::PARAMETER_SAMPLES as f64;
            super::grade_parameter(room.as_ref(), &a, t).is_some_and(|g| g.within)
        });
        assert!(landed, "a sampled phase lands on the target");
    }

    #[test]
    fn parameter_grades_are_metrics_not_binary() {
        let room = crate::registry::room_by_id("harmonograph").expect("room");
        let goal = super::pose_parameter_goal(room.as_ref(), 3).expect("poses");
        // Every attempt earns a graded distance, even far misses.
        let mut scores = Vec::new();
        for i in 0..8 {
            let t = i as f64 / 8.0;
            let grade = super::grade_parameter(room.as_ref(), &goal, t).expect("grades");
            assert!(grade.distance.is_finite());
            assert!(grade.score <= 100);
            scores.push(grade.score);
        }
        assert!(
            scores.iter().any(|&s| s != scores[0]),
            "different phases earn different scores: a gradient to climb"
        );
    }

    #[test]
    fn rooms_without_a_status_line_pose_no_parameter_goal() {
        // A room with no status line cannot pose.
        for room in crate::registry::all_rooms() {
            if room.status(0.3).is_none() {
                assert!(
                    super::pose_parameter_goal(room.as_ref(), 1).is_none(),
                    "{} has no readout to target",
                    room.meta().id
                );
            }
        }
    }

    #[test]
    fn every_catalog_room_with_a_verb_poses_a_winnable_challenge() {
        for room in crate::registry::all_rooms() {
            let posed = pose_challenge(room.as_ref(), 42, 72, 32);
            assert_eq!(
                posed.is_some(),
                room.verb().is_some(),
                "{} must pose exactly when it answers the hand",
                room.meta().id
            );
            // Winnability is constructed, so it must also be provable: one of
            // the seeded probe hands the pose examined clears its threshold.
            if let Some(challenge) = posed {
                let id = room.meta().id;
                let won = (0..super::PROBE_ATTEMPTS).any(|attempt| {
                    let probe = super::probe_hand(id, 42, attempt);
                    super::PROBE_PHASES.iter().any(|&phase| {
                        grade_challenge(room.as_ref(), &challenge, phase, &probe).passed
                    })
                });
                assert!(won, "{id} posed a challenge its own witness cannot win");
            }
        }
    }
}

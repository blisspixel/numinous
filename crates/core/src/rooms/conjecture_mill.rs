//! The Conjecture Mill: typed guesses enter, exact counterexamples leave.
//!
//! The room performs a complete deterministic search over primitive rational
//! quadratic polynomials. Tests can refute a candidate, but only exact
//! coefficient equality stamps an identity as proved. DRAG: STEER THE SEARCH.

use crate::font;
use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const COEFF_MIN: i32 = -4;
const COEFF_COUNT: usize = 9;
const DENOMINATOR_COUNT: usize = 4;
const GRAMMAR_SIZE: usize = COEFF_COUNT * COEFF_COUNT * COEFF_COUNT * DENOMINATOR_COUNT;
const SAMPLE_COUNT: usize = 24;
const MIN_RAW_TRIALS: usize = 24;
const TARGET_COUNT: usize = 6;
const RECENT_CANDIDATES: usize = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Polynomial {
    quadratic: i32,
    linear: i32,
    constant: i32,
    denominator: i32,
}

impl Polynomial {
    fn numerator(self, n: i64) -> i64 {
        i64::from(self.quadratic) * n * n + i64::from(self.linear) * n + i64::from(self.constant)
    }

    fn equivalent(self, other: Self) -> bool {
        let left = i64::from(other.denominator);
        let right = i64::from(self.denominator);
        i64::from(self.quadratic) * left == i64::from(other.quadratic) * right
            && i64::from(self.linear) * left == i64::from(other.linear) * right
            && i64::from(self.constant) * left == i64::from(other.constant) * right
    }

    fn primitive(self) -> bool {
        gcd(
            gcd(
                gcd(self.quadratic.unsigned_abs(), self.linear.unsigned_abs()),
                self.constant.unsigned_abs(),
            ),
            self.denominator as u32,
        ) == 1
    }

    fn complexity(self) -> u32 {
        self.quadratic.unsigned_abs()
            + self.linear.unsigned_abs()
            + self.constant.unsigned_abs()
            + self.denominator as u32
    }

    fn formula(self) -> String {
        format!(
            "({}N^2{:+}N{:+})/{}",
            self.quadratic, self.linear, self.constant, self.denominator
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct Target {
    name: &'static str,
    polynomial: Polynomial,
}

impl Target {
    fn value(self, n: i64) -> i64 {
        self.polynomial.numerator(n) / i64::from(self.polynomial.denominator)
    }
}

const TARGETS: [Target; TARGET_COUNT] = [
    Target {
        name: "TRIANGULAR",
        polynomial: Polynomial {
            quadratic: 1,
            linear: 1,
            constant: 0,
            denominator: 2,
        },
    },
    Target {
        name: "SUM FIRST N ODDS",
        polynomial: Polynomial {
            quadratic: 1,
            linear: 0,
            constant: 0,
            denominator: 1,
        },
    },
    Target {
        name: "PRONIC",
        polynomial: Polynomial {
            quadratic: 1,
            linear: 1,
            constant: 0,
            denominator: 1,
        },
    },
    Target {
        name: "PENTAGONAL",
        polynomial: Polynomial {
            quadratic: 3,
            linear: -1,
            constant: 0,
            denominator: 2,
        },
    },
    Target {
        name: "CENTERED HEX",
        polynomial: Polynomial {
            quadratic: 3,
            linear: -3,
            constant: 1,
            denominator: 1,
        },
    },
    Target {
        name: "ODD NUMBERS",
        polynomial: Polynomial {
            quadratic: 0,
            linear: 2,
            constant: -1,
            denominator: 1,
        },
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Counterexample {
    n: i64,
    got_numerator: i64,
    got_denominator: i64,
    wanted: i64,
}

#[derive(Clone, Copy, Debug)]
struct Evaluation {
    matches: usize,
    counterexample: Option<Counterexample>,
}

#[derive(Clone, Copy, Debug)]
struct Candidate {
    polynomial: Polynomial,
    evaluation: Evaluation,
}

#[derive(Clone, Copy, Debug)]
struct Steering {
    target_index: usize,
    order_seed: u64,
    temperament: usize,
}

#[derive(Clone, Copy, Debug)]
struct SearchState {
    target: Target,
    current: Candidate,
    best: Candidate,
    tested: usize,
    raw_budget: usize,
    proof: Option<Candidate>,
    recent: [Candidate; RECENT_CANDIDATES],
    recent_len: usize,
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

fn phase_unit(t: f64) -> f64 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn polynomial_at(mut index: usize) -> Polynomial {
    let denominator = index % DENOMINATOR_COUNT + 1;
    index /= DENOMINATOR_COUNT;
    let constant = index % COEFF_COUNT;
    index /= COEFF_COUNT;
    let linear = index % COEFF_COUNT;
    index /= COEFF_COUNT;
    let quadratic = index % COEFF_COUNT;
    Polynomial {
        quadratic: quadratic as i32 + COEFF_MIN,
        linear: linear as i32 + COEFF_MIN,
        constant: constant as i32 + COEFF_MIN,
        denominator: denominator as i32,
    }
}

fn polynomial_index(polynomial: Polynomial) -> usize {
    let encode = |coefficient: i32| (coefficient - COEFF_MIN) as usize;
    (((encode(polynomial.quadratic) * COEFF_COUNT + encode(polynomial.linear)) * COEFF_COUNT
        + encode(polynomial.constant))
        * DENOMINATOR_COUNT)
        + polynomial.denominator as usize
        - 1
}

fn finite_pokes(pokes: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .copied()
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .map(|(x, y)| (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)))
        .collect()
}

fn steering(seed: u64, pokes: &[(f64, f64)]) -> Steering {
    let hands = finite_pokes(pokes);
    let mut entropy = seed ^ 0xC0A7_EC70_4E11_0001;
    for (index, &(x, y)) in hands.iter().enumerate() {
        let qx = (x * 65_535.0).round() as u64;
        let qy = (y * 65_535.0).round() as u64;
        entropy ^= qx.rotate_left((index % 61) as u32);
        entropy = entropy.wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(17) ^ qy;
    }
    let mut rng = SplitMix64::new(entropy);
    let order_seed = rng.next_u64();
    let target_index = hands
        .last()
        .map(|(x, _)| ((*x * TARGET_COUNT as f64) as usize).min(TARGET_COUNT - 1))
        .unwrap_or_else(|| rng.below(TARGET_COUNT as u64) as usize);
    let temperament = hands
        .last()
        .map(|(_, y)| ((*y * 3.0) as usize).min(2))
        .unwrap_or_else(|| rng.below(3) as usize);
    Steering {
        target_index,
        order_seed,
        temperament,
    }
}

fn permutation_step(seed: u64) -> usize {
    let mut step = (seed % (GRAMMAR_SIZE - 1) as u64) as usize + 1;
    while gcd(step as u32, GRAMMAR_SIZE as u32) != 1 {
        step = step % (GRAMMAR_SIZE - 1) + 1;
    }
    step
}

fn order(target: Target, seed: u64) -> (usize, usize, usize) {
    let step = permutation_step(seed);
    let proof_raw_position =
        GRAMMAR_SIZE * 3 / 5 + (seed.rotate_right(23) % (GRAMMAR_SIZE * 3 / 10) as u64) as usize;
    let target_index = polynomial_index(target.polynomial);
    let displacement = proof_raw_position.wrapping_mul(step) % GRAMMAR_SIZE;
    let offset = (target_index + GRAMMAR_SIZE - displacement) % GRAMMAR_SIZE;
    (offset, step, proof_raw_position)
}

fn evaluate(target: Target, polynomial: Polynomial) -> Evaluation {
    let mut matches = 0;
    let mut counterexample = None;
    for n in 1..=SAMPLE_COUNT as i64 {
        let wanted = target.value(n);
        let got_numerator = polynomial.numerator(n);
        let got_denominator = i64::from(polynomial.denominator);
        if got_numerator == wanted * got_denominator {
            matches += 1;
        } else if counterexample.is_none() {
            counterexample = Some(Counterexample {
                n,
                got_numerator,
                got_denominator,
                wanted,
            });
        }
    }
    Evaluation {
        matches,
        counterexample,
    }
}

fn better(candidate: Candidate, incumbent: Candidate) -> bool {
    candidate.evaluation.matches > incumbent.evaluation.matches
        || (candidate.evaluation.matches == incumbent.evaluation.matches
            && candidate.polynomial.complexity() < incumbent.polynomial.complexity())
}

fn search(target: Target, order_seed: u64, raw_budget: usize) -> SearchState {
    let (offset, step, _) = order(target, order_seed);
    let raw_budget = raw_budget.clamp(DENOMINATOR_COUNT, GRAMMAR_SIZE);
    // The denominator is the fastest-changing grammar coordinate. A valid
    // permutation step is odd, so its first four positions include denominator
    // 1 and therefore one primitive candidate. Seed BEST from that admitted
    // candidate, never from a raw formula the search would skip.
    let first_admitted_position = if step % DENOMINATOR_COUNT == 1 {
        (DENOMINATOR_COUNT - offset % DENOMINATOR_COUNT) % DENOMINATOR_COUNT
    } else {
        offset % DENOMINATOR_COUNT
    };
    let first_admitted = polynomial_at((offset + first_admitted_position * step) % GRAMMAR_SIZE);
    let admitted = Candidate {
        polynomial: first_admitted,
        evaluation: evaluate(target, first_admitted),
    };
    let mut current = admitted;
    let mut best = admitted;
    let mut tested = 0;
    let mut proof = None;
    let mut recent = [admitted; RECENT_CANDIDATES];
    let mut recent_len = 0;
    for raw_position in 0..raw_budget {
        let index = (offset + raw_position * step) % GRAMMAR_SIZE;
        let polynomial = polynomial_at(index);
        if !polynomial.primitive() {
            continue;
        }
        let candidate = Candidate {
            polynomial,
            evaluation: evaluate(target, polynomial),
        };
        tested += 1;
        current = candidate;
        recent.rotate_right(1);
        recent[0] = candidate;
        recent_len = (recent_len + 1).min(RECENT_CANDIDATES);
        if better(candidate, best) {
            best = candidate;
        }
        if polynomial.equivalent(target.polynomial) {
            proof = Some(candidate);
            best = candidate;
            break;
        }
    }
    SearchState {
        target,
        current,
        best,
        tested,
        raw_budget,
        proof,
        recent,
        recent_len,
    }
}

fn state(seed: u64, t: f64, pokes: &[(f64, f64)]) -> (Steering, SearchState) {
    let steering = steering(seed, pokes);
    let progress = phase_unit(t);
    let raw_budget =
        MIN_RAW_TRIALS + (progress * (GRAMMAR_SIZE - MIN_RAW_TRIALS) as f64).floor() as usize;
    (
        steering,
        search(
            TARGETS[steering.target_index],
            steering.order_seed,
            raw_budget,
        ),
    )
}

fn observed_values(target: Target) -> String {
    (1..=6)
        .map(|n| target.value(n).to_string())
        .collect::<Vec<_>>()
        .join("  ")
}

fn draw_label(surface: &mut dyn Surface, text: &str, x: i32, y: i32, scale: i32, mark: char) {
    if surface.safe_char_aspect() < 0.75 {
        for (column, character) in text.chars().enumerate() {
            surface.plot(x + column as i32, y, character);
        }
    } else {
        font::draw_text(surface, text, x, y, scale, mark);
    }
}

fn draw_frame(surface: &mut dyn Surface, x0: i32, y0: i32, x1: i32, y1: i32) {
    surface.line(x0, y0, x1, y0, '.');
    surface.line(x1, y0, x1, y1, '.');
    surface.line(x1, y1, x0, y1, '.');
    surface.line(x0, y1, x0, y0, '.');
}

fn draw_blackboard(surface: &mut dyn Surface, steering: Steering, search: SearchState) {
    let (width, height) = surface.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let raster = surface.safe_char_aspect() >= 0.75;
    let scale = if !raster || width < 520 {
        1
    } else if width < 900 {
        2
    } else {
        3
    };
    let margin = if raster { 12 * scale } else { 1 };
    let line = if raster {
        10 * scale
    } else if height < 22 {
        1
    } else {
        2
    };
    let x1 = width.saturating_sub(margin.max(1) as usize + 1) as i32;
    let y1 = height.saturating_sub(margin.max(1) as usize + 1) as i32;
    draw_frame(surface, margin / 2, margin / 2, x1, y1);

    let mut y = margin;
    draw_label(surface, "CONJECTURE MILL", margin, y, scale, '#');
    y += line;
    draw_label(
        surface,
        &format!("OBSERVED: {}", search.target.name),
        margin,
        y,
        scale,
        '+',
    );
    y += line;
    draw_label(
        surface,
        &observed_values(search.target),
        margin,
        y,
        scale,
        '*',
    );
    y += line + if raster { scale * 2 } else { 0 };

    let shown = search.proof.unwrap_or(search.current);
    draw_label(surface, "CHALK:", margin, y, scale, '+');
    y += line;
    draw_label(
        surface,
        &shown.polynomial.formula(),
        margin,
        y,
        scale,
        if search.proof.is_some() { '#' } else { '*' },
    );
    y += line;

    if search.proof.is_some() {
        draw_label(surface, "PROVED: COEFFICIENTS MATCH", margin, y, scale, '#');
    } else if let Some(witness) = shown.evaluation.counterexample {
        let result = if witness.got_denominator == 1 {
            witness.got_numerator.to_string()
        } else {
            format!("{}/{}", witness.got_numerator, witness.got_denominator)
        };
        draw_label(
            surface,
            &format!("X N={} GOT {} WANT {}", witness.n, result, witness.wanted),
            margin,
            y,
            scale,
            'x',
        );
    }
    y += line;
    draw_label(
        surface,
        &format!(
            "BEST {}/{}  {}",
            search.best.evaluation.matches,
            SAMPLE_COUNT,
            search.best.polynomial.formula()
        ),
        margin,
        y,
        scale,
        '+',
    );

    y += line;
    draw_label(
        surface,
        &format!(
            "TESTED {}  REFUTED {}",
            search.tested,
            search
                .tested
                .saturating_sub(usize::from(search.proof.is_some()))
        ),
        margin,
        y,
        scale,
        '+',
    );
    y += line;
    draw_label(
        surface,
        &format!(
            "ORDER {:08X}  {}",
            steering.order_seed as u32,
            ["CAREFUL", "BALANCED", "WILD"][steering.temperament]
        ),
        margin,
        y,
        scale,
        '+',
    );
    y += line;
    draw_label(surface, "REJECTED LEDGER", margin, y, scale, '.');
    y += line;
    let gauge_y = (y1 - margin.max(1)).max(0);
    for candidate in search.recent[..search.recent_len].iter().rev() {
        if candidate.polynomial.equivalent(search.target.polynomial) {
            continue;
        }
        if y + line >= gauge_y {
            break;
        }
        let witness_n = candidate
            .evaluation
            .counterexample
            .map_or(0, |witness| witness.n);
        let entry = format!("X {}  AT N={witness_n}", candidate.polynomial.formula());
        draw_label(surface, &entry, margin, y, scale, 'x');
        y += line;
    }

    let gauge_left = margin;
    let gauge_right = x1 - margin.max(1);
    if gauge_right > gauge_left {
        surface.line(gauge_left, gauge_y, gauge_right, gauge_y, '.');
        let span = gauge_right - gauge_left;
        let filled = search.raw_budget * span as usize / GRAMMAR_SIZE;
        surface.line(
            gauge_left,
            gauge_y,
            gauge_left + filled as i32,
            gauge_y,
            if search.proof.is_some() { '#' } else { '+' },
        );
    }

    if raster && width >= 300 {
        let tree_x = x1 - 36 * scale;
        let tree_top = margin + line;
        for branch in 0..5 {
            let branch_y = tree_top + branch * line;
            surface.line(tree_x, tree_top, tree_x + 18 * scale, branch_y, '+');
            surface.plot(
                tree_x + 18 * scale,
                branch_y,
                if branch == 3 { 'x' } else { 'o' },
            );
        }
    }
}

/// A deterministic laboratory for conjecture, counterexample, and proof.
#[derive(Debug, Default)]
pub struct ConjectureMill {
    seed: u64,
}

impl ConjectureMill {
    /// Create the default replay.
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create a replayable search order from a visit seed.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }
}

impl Room for ConjectureMill {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "conjecture-mill",
            title: "The Conjecture Mill",
            wing: "Number & Pattern",
            blurb: "Typed formulas crawl across a blackboard. Exact counterexamples erase the \
                    bad; coefficient proof stamps the survivor. Time runs a complete finite \
                    search. DRAG: STEER THE SEARCH.",
            accent: [120, 220, 170],
        }
    }

    fn render(&self, surface: &mut dyn Surface, t: f64) {
        let (steering, search) = state(self.seed, t, &[]);
        draw_blackboard(surface, steering, search);
    }

    fn render_poked(&self, surface: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        if hands.is_empty() {
            self.render(surface, t);
            return;
        }
        let (steering, search) = state(self.seed, t, &hands);
        draw_blackboard(surface, steering, search);
    }

    fn postcard_t(&self) -> f64 {
        0.95
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "chalk and counterexample",
            root: 164.81,
            tempo: 108,
            line: &[0, 2, -1, 5, 3, 7, 4, 12],
            encodes: "a guess climbing until one counterexample cuts it short",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: STEER THE SEARCH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (_, search) = state(self.seed, t, &[]);
        Some(if search.proof.is_some() {
            format!("PROVED {}  TRIALS {}", search.target.name, search.tested)
        } else {
            format!(
                "TRIALS {}  BEST {}/{}  DRAG:STEER",
                search.tested, search.best.evaluation.matches, SAMPLE_COUNT
            )
        })
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (steering, search) = state(self.seed, t, &hands);
        let temperament = ["CAREFUL", "BALANCED", "WILD"][steering.temperament];
        Some(if search.proof.is_some() {
            format!("HAND PROVED {}  {}", search.target.name, temperament)
        } else {
            format!(
                "HAND {}  BEST {}/{}  {}",
                search.target.name, search.best.evaluation.matches, SAMPLE_COUNT, temperament
            )
        })
    }

    fn reveal(&self) -> &'static str {
        "Infinite random typing can eventually contain any finite sentence, but mathematics \
         needs a language and a judge. This mill enumerates typed quadratic formulas, erases \
         each bad guess with an exact counterexample, and stamps PROVED only when rational \
         coefficients match for every integer. Your hand changes the search order, never the \
         truth. Real discovery still needs new ideas, proof, and human scrutiny."
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ConjectureMill, GRAMMAR_SIZE, SAMPLE_COUNT, TARGETS, evaluate, finite_pokes, order,
        polynomial_at, polynomial_index, search, state, steering,
    };
    use crate::canvas::Canvas;
    use crate::room::{MAX_ROOM_POKES, Room, RoomInput};

    #[test]
    fn polynomial_index_round_trips_the_complete_grammar() {
        for index in 0..GRAMMAR_SIZE {
            assert_eq!(polynomial_index(polynomial_at(index)), index);
        }
    }

    #[test]
    fn every_order_is_a_complete_permutation_with_a_late_proof() {
        for (target_index, target) in TARGETS.into_iter().enumerate() {
            for seed in [0, 1, 42, u64::MAX] {
                let (offset, step, proof_position) = order(target, seed);
                let mut seen = vec![false; GRAMMAR_SIZE];
                for position in 0..GRAMMAR_SIZE {
                    let index = (offset + position * step) % GRAMMAR_SIZE;
                    assert!(!seen[index]);
                    seen[index] = true;
                }
                assert!(seen.into_iter().all(|visited| visited));
                assert_eq!(
                    polynomial_at((offset + proof_position * step) % GRAMMAR_SIZE),
                    target.polynomial,
                    "target {target_index}, seed {seed}"
                );
                assert!(proof_position >= GRAMMAR_SIZE * 3 / 5);
                assert!(proof_position < GRAMMAR_SIZE * 9 / 10);
            }
        }
    }

    #[test]
    fn complete_search_proves_every_target_by_coefficients() {
        for target in TARGETS {
            for seed in [0, 7, 99, u64::MAX] {
                let result = search(target, seed, GRAMMAR_SIZE);
                let proof = result
                    .proof
                    .expect("the complete grammar reaches the target");
                assert!(proof.polynomial.equivalent(target.polynomial));
                assert_eq!(proof.evaluation.matches, SAMPLE_COUNT);
                assert!(proof.evaluation.counterexample.is_none());
            }
        }
    }

    #[test]
    fn every_nonproof_candidate_has_an_exact_counterexample() {
        for target in TARGETS {
            for index in 0..GRAMMAR_SIZE {
                let polynomial = polynomial_at(index);
                if !polynomial.primitive() || polynomial.equivalent(target.polynomial) {
                    continue;
                }
                let result = evaluate(target, polynomial);
                let witness = result
                    .counterexample
                    .expect("distinct quadratics disagree within 24 samples");
                assert_ne!(
                    witness.got_numerator,
                    witness.wanted * witness.got_denominator
                );
                assert_eq!(target.value(witness.n), witness.wanted);
            }
        }
    }

    #[test]
    fn tests_alone_never_set_the_proof_flag() {
        let target = TARGETS[0];
        let (_, _, proof_position) = order(target, 11);
        let before = search(target, 11, proof_position);
        assert!(before.proof.is_none());
        assert!(before.current.evaluation.counterexample.is_some());
        let at = search(target, 11, proof_position + 1);
        assert!(at.proof.is_some());
    }

    #[test]
    fn displayed_candidates_always_belong_to_the_primitive_grammar() {
        for seed in 0..=2_048 {
            let (_, result) = state(seed, 0.0, &[]);
            assert!(result.current.polynomial.primitive(), "seed {seed}");
            assert!(result.best.polynomial.primitive(), "seed {seed}");
            assert_eq!(
                result.best.evaluation.matches == SAMPLE_COUNT,
                result.proof.is_some(),
                "seed {seed} cannot display an unstamped identity"
            );
        }
        for seed in [24, 980] {
            let (_, result) = state(seed, 0.0, &[]);
            assert!(result.current.polynomial.primitive());
            assert!(result.best.polynomial.primitive());
        }
    }

    #[test]
    fn a_hand_changes_the_lab_and_search_order_without_changing_truth() {
        let left = steering(0, &[(0.05, 0.1), (0.1, 0.2)]);
        let right = steering(0, &[(0.9, 0.8)]);
        assert_ne!(left.target_index, right.target_index);
        assert_ne!(left.order_seed, right.order_seed);
        let left_result = search(TARGETS[left.target_index], left.order_seed, GRAMMAR_SIZE);
        let right_result = search(TARGETS[right.target_index], right.order_seed, GRAMMAR_SIZE);
        assert!(
            left_result
                .proof
                .unwrap()
                .polynomial
                .equivalent(left_result.target.polynomial)
        );
        assert!(
            right_result
                .proof
                .unwrap()
                .polynomial
                .equivalent(right_result.target.polynomial)
        );
    }

    #[test]
    fn finite_pokes_keep_only_the_newest_valid_bounded_points() {
        let mut pokes = vec![(f64::NAN, 0.5), (0.5, f64::INFINITY)];
        pokes.extend((0..MAX_ROOM_POKES + 5).map(|index| {
            let value = index as f64 / MAX_ROOM_POKES as f64;
            (value, 1.0 - value)
        }));
        let finite = finite_pokes(&pokes);
        assert_eq!(finite.len(), MAX_ROOM_POKES);
        assert!(finite.iter().all(|(x, y)| {
            x.is_finite() && y.is_finite() && (0.0..=1.0).contains(x) && (0.0..=1.0).contains(y)
        }));
    }

    #[test]
    fn room_status_is_bounded_and_hand_specific() {
        let room = ConjectureMill::new();
        let ambient = room.status(0.25).unwrap();
        let hand = room
            .status_input(
                0.25,
                &[
                    RoomInput::PointerDown {
                        x: 0.2,
                        y: 0.2,
                        t: 0.1,
                    },
                    RoomInput::PointerMove {
                        x: 0.8,
                        y: 0.8,
                        t: 0.2,
                    },
                ],
            )
            .unwrap();
        assert_ne!(ambient, hand);
        assert!(ambient.chars().count() <= 56);
        assert!(hand.chars().count() <= 56);
        assert!(ambient.contains("DRAG") || ambient.contains("PROVED"));
        assert!(hand.contains("HAND"));
    }

    #[test]
    fn ascii_blackboard_is_readable_at_default_and_compact_sizes() {
        let room = ConjectureMill::new();
        for (width, height) in [(80, 24), (40, 20)] {
            let mut canvas = Canvas::new(width, height);
            room.render(&mut canvas, 0.45);
            let text = canvas.to_text();
            assert!(text.contains("CONJECTURE"));
            assert!(text.contains("OBSERVED"));
            assert!(text.contains("CHALK"));
            assert!(text.contains("ORDER"));
            assert!(canvas.ink_count() > width);
            let frame_bottom = height - 2;
            assert!(
                (frame_bottom + 1..height)
                    .all(|y| (0..width).all(|x| canvas.cell(x, y) == Some(' ')))
            );
        }
    }

    #[test]
    fn render_is_deterministic_and_hand_and_variation_change_it() {
        let render = |room: &ConjectureMill, pokes: &[(f64, f64)]| {
            let mut canvas = Canvas::new(80, 24);
            room.render_poked(&mut canvas, 0.55, pokes);
            canvas.to_text()
        };
        let room = ConjectureMill::new();
        let ambient = render(&room, &[]);
        assert_eq!(ambient, render(&room, &[]));
        assert_ne!(ambient, render(&room, &[(0.85, 0.75)]));
        assert_ne!(ambient, render(&ConjectureMill::new_with(42), &[]));
    }

    #[test]
    fn hostile_phase_is_total_and_full_phase_reaches_proof() {
        let room = ConjectureMill::new();
        for phase in [f64::NAN, f64::NEG_INFINITY, -1.0, 2.0, f64::INFINITY] {
            let mut canvas = Canvas::new(40, 20);
            room.render(&mut canvas, phase);
            assert!(canvas.ink_count() > 0);
        }
        let (_, result) = state(0, 1.0, &[]);
        assert!(result.proof.is_some());
    }

    #[test]
    fn reveal_separates_testing_from_proof() {
        let reveal = ConjectureMill::new().reveal();
        assert!(reveal.contains("counterexample"));
        assert!(reveal.contains("PROVED"));
        assert!(reveal.contains("never the truth"));
    }
}

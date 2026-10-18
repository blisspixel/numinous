//! Attention as Soft Light: one query, four fixed keys, and a key you place.
//!
//! One head, with no learned projections and no values matrix. Each key is
//! scored by the query dotted with that key, divided by the square root of
//! the key dimension, then softened by temperature. That score form is the
//! one in Vaswani et al., Attention Is All You Need (2017). The picture is
//! the weights. A key farther from the query can still take the light when
//! it points the same way.
//! See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_INPUTS, Room, RoomInput, inputs_from_pokes};
use crate::sound::{Note, SoundSpec};
use crate::surface::Surface;

/// Query vector. Fixed, unprojected.
const QUERY: (f64, f64) = (1.0, 0.0);
/// Key dimension. The scale is `sqrt(d_k)`.
const KEY_DIMENSION: f64 = 2.0;
/// Cold end of the dial. Near one winner.
const T_MIN: f64 = 0.08;
/// Hot end of the dial. Near a uniform chord.
const T_MAX: f64 = 4.0;
/// Right-hand strip. `y` is the dial, 0 cold and 1 hot.
const DIAL_X: f64 = 0.90;
/// Bottom band, and only left of the dial. `x` names a key.
const WAGER_Y: f64 = 0.92;
/// The field lives in `x` of `[0, FIELD_X]`. Past that is the dial.
const FIELD_X: f64 = 0.88;
/// Half-width of the key plane, in key units.
const FIELD_EDGE: f64 = 1.8;
/// How far one `+` or `-` moves the dial when no hand is on it.
const DIAL_STEPS: f64 = 20.0;
/// Partial gain per unit of weight. The weights sum to 1, so the chord
/// sums to this, under the mixer's limiter knee.
const GAIN_OF_WEIGHT: f32 = 0.2;
const CHORD_SECONDS: f32 = 1.5;
const PARTIALS: [f32; 5] = [220.0, 277.18, 329.63, 392.0, 493.88];

const FIXED_KEYS: [(f64, f64); 4] = [
    (0.40, 0.00),
    (0.80, 0.90),
    (0.10, 0.20),
    (-0.20, 0.10),
];
/// The fixture decoy: longer than the nearest key, and more aligned with the query.
#[cfg(test)]
const DECOY_POINT: (f64, f64) = (1.60, 1.20);

struct Scene {
    temperature: f64,
    count: usize,
    keys: [(f64, f64); 5],
    weights: [f64; 5],
    /// Argmax among the four fixed keys, read from the full softmax.
    fixed_winner: usize,
    winner: usize,
    nearest: usize,
    wager: Option<usize>,
}

fn phase_unit(t: f64) -> f64 {
    if t.is_finite() { t.clamp(0.0, 1.0) } else { 0.0 }
}

fn temperature_of(unit: f64) -> f64 {
    T_MIN + unit.clamp(0.0, 1.0) * (T_MAX - T_MIN)
}

fn field_point(x: f64, y: f64) -> (f64, f64) {
    let width = FIELD_EDGE * 2.0;
    (
        (x / FIELD_X) * width - FIELD_EDGE,
        FIELD_EDGE - y * width,
    )
}

fn step_unit(unit: f64, ch: char) -> Option<f64> {
    let step = 1.0 / DIAL_STEPS;
    match ch {
        '+' | '=' => Some((unit + step).clamp(0.0, 1.0)),
        '-' | '_' => Some((unit - step).clamp(0.0, 1.0)),
        _ => None,
    }
}

fn dot(key: (f64, f64)) -> f64 {
    QUERY.0 * key.0 + QUERY.1 * key.1
}

fn score_of(key: (f64, f64)) -> f64 {
    dot(key) / KEY_DIMENSION.sqrt()
}

fn softmax(scores: &[f64], temperature: f64) -> [f64; 5] {
    let mut weights = [0.0; 5];
    let inv = 1.0 / temperature.max(T_MIN);
    let mut max = f64::NEG_INFINITY;
    let mut shifted = [0.0; 5];
    for (index, score) in scores.iter().enumerate() {
        shifted[index] = score * inv;
        max = max.max(shifted[index]);
    }
    let mut sum = 0.0;
    for (index, shift) in shifted.iter().enumerate().take(scores.len()) {
        weights[index] = (shift - max).exp();
        sum += weights[index];
    }
    let sum = sum.max(1e-12);
    for weight in &mut weights {
        *weight /= sum;
    }
    weights
}

fn argmax(weights: &[f64]) -> usize {
    let mut best = 0;
    for (index, weight) in weights.iter().enumerate().skip(1) {
        if *weight > weights[best] {
            best = index;
        }
    }
    best
}

fn distance(key: (f64, f64)) -> f64 {
    (QUERY.0 - key.0).hypot(QUERY.1 - key.1)
}

fn nearest_index(keys: &[(f64, f64)], count: usize) -> usize {
    let mut best = 0;
    let mut best_distance = distance(keys[0]);
    for (index, key) in keys.iter().enumerate().take(count).skip(1) {
        let here = distance(*key);
        if here < best_distance {
            best = index;
            best_distance = here;
        }
    }
    best
}

#[cfg(test)]
fn weight_row(keys: &[(f64, f64)], temperature: f64) -> Vec<f64> {
    let scores: Vec<f64> = keys.iter().copied().map(score_of).collect();
    let weights = softmax(&scores, temperature);
    weights[..keys.len()].to_vec()
}

fn entropy(weights: &[f64]) -> f64 {
    let mut total = 0.0;
    for weight in weights {
        if *weight > 1e-12 {
            total -= weight * weight.ln();
        }
    }
    total
}

fn wager_index(x: f64, count: usize) -> usize {
    let scaled = (x.clamp(0.0, FIELD_X) / FIELD_X) * count as f64;
    (scaled.floor() as usize).min(count.saturating_sub(1))
}

fn scene_at(t: f64, inputs: &[RoomInput]) -> Scene {
    let phase = phase_unit(t);
    let start = inputs.len().saturating_sub(MAX_ROOM_INPUTS);
    let mut dial = None;
    let mut decoy = None;
    let mut wager_x = None;
    for input in &inputs[start..] {
        match *input {
            RoomInput::PointerDown { x, y, .. } | RoomInput::PointerMove { x, y, .. } => {
                if !x.is_finite() || !y.is_finite() {
                    continue;
                }
                let (x, y) = (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0));
                if x >= DIAL_X {
                    dial = Some(y);
                } else if y >= WAGER_Y {
                    wager_x = Some(x);
                } else {
                    decoy = Some(field_point(x, y));
                }
            }
            RoomInput::Key { ch } => {
                if let Some(next) = step_unit(dial.unwrap_or(phase), ch) {
                    dial = Some(next);
                }
            }
            _ => {}
        }
    }
    let mut keys = [(0.0, 0.0); 5];
    keys[..4].copy_from_slice(&FIXED_KEYS);
    let count = if let Some(point) = decoy {
        keys[4] = point;
        5
    } else {
        4
    };
    let mut scores = [0.0; 5];
    for (score, key) in scores.iter_mut().zip(keys).take(count) {
        *score = score_of(key);
    }
    let temperature = temperature_of(dial.unwrap_or(phase));
    let weights = softmax(&scores[..count], temperature);
    let fixed_winner = argmax(&weights[..4]);
    let winner = argmax(&weights[..count]);
    Scene {
        temperature,
        count,
        keys,
        weights,
        fixed_winner,
        winner,
        nearest: nearest_index(&keys, count),
        wager: wager_x.map(|x| wager_index(x, count)),
    }
}

fn nearest_name(index: usize) -> String {
    if index == 4 {
        "D".to_string()
    } else {
        format!("K{index}")
    }
}

fn readout(scene: &Scene) -> String {
    let mut line = format!("T={:.2}", scene.temperature);
    if scene.count == 5 {
        line.push_str(&format!(" decoy={:.2}", scene.weights[4]));
    }
    line.push_str(&format!(
        " was={:.2} H={:.2} near={}",
        scene.weights[scene.fixed_winner],
        entropy(&scene.weights[..scene.count]),
        nearest_name(scene.nearest)
    ));
    if let Some(slot) = scene.wager {
        let grade = if slot == scene.winner { "yes" } else { "no" };
        line.push_str(&format!(" call={slot} {grade}"));
    } else {
        // First contact has to name a hand. The call, when present, already does.
        line.push_str(" DIAL");
    }
    line
}

/// The graded call for this hand, or nothing when the top band was not touched.
///
/// The token is `attention:<index>:right` or `attention:<index>:wrong`.
/// Index 0..3 are the fixed keys. Index 4 is the placed key, and it exists
/// only after one has been placed. The grade is the argmax of the weights.
#[must_use]
pub fn wager_token(t: f64, inputs: &[RoomInput]) -> Option<String> {
    let scene = scene_at(t, inputs);
    let slot = scene.wager?;
    let grade = if slot == scene.winner { "right" } else { "wrong" };
    Some(format!("attention:{slot}:{grade}"))
}

fn weight_mark(weight: f64) -> char {
    if weight > 0.5 {
        '#'
    } else if weight > 0.2 {
        '*'
    } else if weight > 0.05 {
        '+'
    } else {
        '.'
    }
}

fn to_px(point: (f64, f64), width: usize, height: usize) -> (i32, i32) {
    let span = FIELD_EDGE * 2.0;
    let nx = ((point.0 + FIELD_EDGE) / span).clamp(0.0, 1.0);
    let ny = ((FIELD_EDGE - point.1) / span).clamp(0.0, 1.0);
    (
        (nx * width.saturating_sub(1) as f64).round() as i32,
        (ny * height.saturating_sub(1) as f64).round() as i32,
    )
}

fn draw(canvas: &mut dyn Surface, scene: &Scene) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let query = to_px(QUERY, width, height);
    for (index, (key, weight)) in scene
        .keys
        .iter()
        .zip(scene.weights)
        .take(scene.count)
        .enumerate()
    {
        let point = to_px(*key, width, height);
        let mark = weight_mark(weight);
        canvas.line(query.0, query.1, point.0, point.1, mark);
        canvas.plot(point.0, point.1, mark);
        let x = ((0.12 + index as f64 * 0.14) * width.saturating_sub(1) as f64).round() as i32;
        let bar = (weight * height as f64 * 0.35).max(1.0);
        let y1 = height.saturating_sub(1) as i32;
        let y0 = (y1 as f64 - bar).round() as i32;
        canvas.line(x, y1, x, y0, '=');
    }
    canvas.plot(query.0, query.1, 'Q');
}

fn chord(scene: &Scene) -> SoundSpec {
    let notes = scene.weights[..scene.count]
        .iter()
        .enumerate()
        .filter(|(_, weight)| **weight > 1e-6)
        .map(|(index, weight)| Note {
            freq: PARTIALS[index],
            start: 0.0,
            dur: CHORD_SECONDS,
            amp: *weight as f32 * GAIN_OF_WEIGHT,
        })
        .collect();
    SoundSpec {
        duration: CHORD_SECONDS,
        notes,
    }
}

/// Attention room. The keys do not move with the variation seed.
#[derive(Debug, Default)]
pub struct Attention;

impl Attention {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Create the room. The seed is accepted and ignored: the keys are fixed.
    #[must_use]
    pub fn new_with(_seed: u64) -> Self {
        Self
    }
}

impl Room for Attention {
    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, &scene_at(t, &[]));
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let inputs = inputs_from_pokes(pokes, t);
        draw(canvas, &scene_at(t, &inputs));
    }

    fn render_input(&self, canvas: &mut dyn Surface, t: f64, inputs: &[RoomInput]) {
        draw(canvas, &scene_at(t, inputs));
    }

    fn postcard_t(&self) -> f64 {
        0.0
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "soft light",
            root: 329.63,
            tempo: 104,
            line: &[0, 4, 7, 11, 7, 4, 0, 11],
            encodes: "one query sharing its light as the dial warms",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TEMPERATURE, OR PLACE A DECOY")
    }

    fn status(&self, t: f64) -> Option<String> {
        Some(readout(&scene_at(t, &[])))
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        Some(readout(&scene_at(t, inputs)))
    }

    fn sound(&self, t: f64) -> SoundSpec {
        chord(&scene_at(t, &[]))
    }

    fn sound_input(&self, t: f64, inputs: &[RoomInput]) -> SoundSpec {
        chord(&scene_at(t, inputs))
    }

    fn reveal(&self) -> &'static str {
        "One head, unprojected, with no values matrix. Each key is scored by \
         the query dotted with the key, divided by sqrt(2), then softened by \
         temperature (Vaswani et al. 2017). A key farther from the query can \
         outscore a nearer one when it points the same way. The weights are \
         that softmax."
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Attention, DECOY_POINT, FIXED_KEYS, GAIN_OF_WEIGHT, T_MAX, T_MIN, chord, distance, entropy,
        scene_at, score_of, weight_row,
    };
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput, inputs_from_pokes};

    const LOW: f64 = 0.15;
    const HIGH: f64 = 4.0;
    const LOW4: [f64; 4] = [0.1267001723, 0.8350081873, 0.0308029321, 0.0074887083];
    const HIGH4: [f64; 4] = [0.2550379794, 0.2737247805, 0.2418649383, 0.2293723018];
    const LOW5: [f64; 5] = [
        0.0033997484,
        0.0224057924,
        0.0008265357,
        0.0002009447,
        0.9731669788,
    ];
    const HIGH5: [f64; 5] = [
        0.1939000686,
        0.2081072547,
        0.1838848796,
        0.1743869881,
        0.2397208090,
    ];
    const SCORES4: [f64; 4] = [0.2828427125, 0.5656854249, 0.0707106781, -0.1414213562];

    fn assert_close(got: &[f64], want: &[f64]) {
        assert_eq!(got.len(), want.len());
        for (index, (got, want)) in got.iter().zip(want).enumerate() {
            assert!(
                (got - want).abs() < 1e-9,
                "index {index}: got {got} want {want}"
            );
        }
    }

    fn dial_poke(temperature: f64) -> (f64, f64) {
        (0.95, (temperature - T_MIN) / (T_MAX - T_MIN))
    }

    fn decoy_poke() -> (f64, f64) {
        (
            (DECOY_POINT.0 + 1.8) / 3.6 * 0.88,
            (1.8 - DECOY_POINT.1) / 3.6,
        )
    }

    fn five_keys() -> Vec<(f64, f64)> {
        let mut keys = FIXED_KEYS.to_vec();
        keys.push(DECOY_POINT);
        keys
    }

    #[test]
    fn fixed_key_scores_match_the_hand_computation() {
        let scores: Vec<f64> = FIXED_KEYS.iter().copied().map(score_of).collect();
        assert_close(&scores, &SCORES4);
    }

    #[test]
    fn farther_longer_decoy_wins_at_low_temperature() {
        let keys = five_keys();
        assert!(distance(DECOY_POINT) > distance(FIXED_KEYS[0]));
        let weights = weight_row(&keys, LOW);
        assert_close(&weights, &LOW5);
        assert!(weights[4] > 0.9);
        assert!(weights[4] > weights[0]);
        let scene_keys = keys;
        let nearest = scene_keys
            .iter()
            .enumerate()
            .min_by(|left, right| {
                distance(*left.1)
                    .partial_cmp(&distance(*right.1))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(index, _)| index);
        assert_eq!(nearest, Some(0));
    }

    #[test]
    fn weights_approach_uniform_as_temperature_rises() {
        let keys = five_keys();
        let cold = weight_row(&keys, LOW);
        let hot = weight_row(&keys, HIGH);
        assert_close(&hot, &HIGH5);
        let max = hot.iter().copied().fold(0.0, f64::max);
        let min = hot.iter().copied().fold(f64::MAX, f64::min);
        assert!(max < 0.28, "{max}");
        assert!(min > 0.15, "{min}");
        assert!((max - min - 0.065333821).abs() < 1e-8);
        let mut previous = entropy(&cold);
        for temperature in [0.5, 1.0, 2.0, HIGH] {
            let here = entropy(&weight_row(&keys, temperature));
            assert!(here + 1e-12 >= previous, "{here} after {previous}");
            previous = here;
        }
    }

    #[test]
    fn four_key_rows_match_the_hand_computation() {
        assert_close(&weight_row(&FIXED_KEYS, LOW), &LOW4);
        assert_close(&weight_row(&FIXED_KEYS, HIGH), &HIGH4);
        let sum: f64 = weight_row(&FIXED_KEYS, LOW).iter().sum();
        assert!((sum - 1.0).abs() < 1e-9);
    }

    #[test]
    fn the_poke_lands_on_the_fixture_and_the_dial_overrides_phase() {
        let pokes = [dial_poke(LOW), decoy_poke()];
        let inputs = inputs_from_pokes(&pokes, 0.99);
        let scene = scene_at(0.99, &inputs);
        assert!((scene.temperature - LOW).abs() < 1e-12);
        assert_eq!(scene.count, 5);
        assert_close(&scene.weights[..5], &LOW5);
        assert_eq!(scene.nearest, 0);
        assert_eq!(scene.fixed_winner, 1);
        assert_eq!(scene.winner, 4);
        let bare = scene_at(0.0, &[]);
        assert!((bare.temperature - T_MIN).abs() < 1e-12);
        assert_eq!(bare.count, 4);
    }

    #[test]
    fn the_same_points_agree_across_a_poke_list_and_a_trail() {
        let poke = decoy_poke();
        let dial = dial_poke(LOW);
        let from_pokes = inputs_from_pokes(&[dial, poke], 0.2);
        let trail = [
            RoomInput::PointerDown {
                x: 0.2,
                y: 0.2,
                t: 0.2,
            },
            RoomInput::PointerMove {
                x: poke.0,
                y: poke.1,
                t: 0.2,
            },
            RoomInput::PointerMove {
                x: dial.0,
                y: dial.1,
                t: 0.2,
            },
        ];
        let left = scene_at(0.2, &from_pokes);
        let right = scene_at(0.2, &trail);
        assert_close(&left.weights[..5], &right.weights[..5]);
        assert!((left.temperature - right.temperature).abs() < 1e-12);
    }

    #[test]
    fn a_key_steps_the_dial_from_the_phase() {
        let stepped = scene_at(0.0, &[RoomInput::Key { ch: '+' }]);
        let step = (T_MAX - T_MIN) / 20.0;
        assert!((stepped.temperature - (T_MIN + step)).abs() < 1e-12);
        let held = scene_at(0.0, &[RoomInput::Key { ch: '-' }]);
        assert!((held.temperature - T_MIN).abs() < 1e-12);
    }

    #[test]
    fn status_names_the_readings_and_not_the_joining_sentence() {
        let room = Attention::new();
        for phase in [0.0, 0.25, 0.5, 0.75, 0.99] {
            let line = room.status(phase).expect("status");
            assert!(line.chars().count() <= 56, "{line}");
            assert!(line.contains("T=") && line.contains("was=") && line.contains("H="));
            assert!(line.contains("near="), "{line}");
            assert!(!line.contains("decoy="), "{line}");
            assert_plain(&line);
        }
        let inputs = inputs_from_pokes(&[dial_poke(LOW), decoy_poke(), (0.75, 0.95)], 0.0);
        let line = room.status_input(0.0, &inputs).expect("poked");
        assert!(line.starts_with("T=0.15 decoy=0.97 was=0.02 "), "{line}");
        assert!(line.contains("near=K0"), "{line}");
        assert!(line.contains("call=4 yes"), "{line}");
        assert!(line.chars().count() <= 56, "{} {}", line.chars().count(), line);
        assert_plain(&line);
        let reveal = room.reveal();
        assert!(reveal.contains("Vaswani"));
        assert!(reveal.contains("values matrix"));
        assert!(!line.to_ascii_lowercase().contains("vaswani"));
    }

    fn assert_plain(line: &str) {
        for word in ["longer", "nearer", "because", "dot", "softmax", "query"] {
            let hit = line
                .split(|ch: char| !ch.is_ascii_alphabetic())
                .any(|part| part.eq_ignore_ascii_case(word));
            assert!(!hit, "{line} says {word}");
        }
    }

    #[test]
    fn a_wager_token_is_the_exact_argmax() {
        let inputs = inputs_from_pokes(&[dial_poke(LOW), decoy_poke(), (0.75, 0.95)], 0.4);
        assert_eq!(
            super::wager_token(0.4, &inputs).as_deref(),
            Some("attention:4:right")
        );
        let miss = inputs_from_pokes(&[dial_poke(LOW), decoy_poke(), (0.1, 0.95)], 0.4);
        assert_eq!(
            super::wager_token(0.4, &miss).as_deref(),
            Some("attention:0:wrong")
        );
        assert!(super::wager_token(0.4, &inputs_from_pokes(&[decoy_poke()], 0.4)).is_none());
        let before = inputs_from_pokes(&[dial_poke(LOW), (0.75, 0.95)], 0.0);
        assert_eq!(
            super::wager_token(0.0, &before).as_deref(),
            Some("attention:3:wrong")
        );
    }

    #[test]
    fn partial_gains_equal_the_weights_and_stay_quiet() {
        let inputs = inputs_from_pokes(&[dial_poke(LOW), decoy_poke()], 0.0);
        let scene = scene_at(0.0, &inputs);
        let sound = chord(&scene);
        assert_eq!(sound.notes.len(), 5);
        for (index, note) in sound.notes.iter().enumerate() {
            let weight = scene.weights[index];
            assert!(weight > 1e-6);
            assert!((note.amp - weight as f32 * GAIN_OF_WEIGHT).abs() < 1e-5);
        }
        let samples = sound.render(8_000);
        let peak = samples.iter().copied().map(f32::abs).fold(0.0_f32, f32::max);
        assert!(peak <= 0.5, "{peak}");
        let room = Attention::new();
        assert_eq!(
            room.sound(0.2).notes.len(),
            room.sound_input(0.2, &[]).notes.len()
        );
    }

    #[test]
    fn the_seed_does_not_move_the_keys() {
        assert_eq!(
            Attention::new().status(0.2),
            Attention::new_with(99).status(0.2)
        );
    }

    #[test]
    fn render_ink_and_motif() {
        let mut canvas = Canvas::new(40, 28);
        Attention::new().render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 15);
        let mut poked = Canvas::new(40, 28);
        Attention::new().render_poked(&mut poked, 0.0, &[dial_poke(LOW), decoy_poke()]);
        assert_ne!(canvas.to_text(), poked.to_text());
        assert!(Attention::new().motif().unwrap().line.len() >= 6);
        assert!((Attention::new().postcard_t()).abs() < 1e-12);
    }

    #[test]
    fn the_citation_names_the_paper_and_leaves_the_other_rooms() {
        let line = crate::citations::for_room("attention");
        assert!(line.contains("Vaswani"));
        assert!(line.contains("1706.03762"));
        assert!(!crate::citations::for_room("learning-clock").contains("Vaswani"));
        assert!(!crate::citations::for_room("gradient-valley").contains("Vaswani"));
    }
}

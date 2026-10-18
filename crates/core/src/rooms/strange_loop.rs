//! Strange Loop: a descent that returns to the frame it left.
//!
//! The straight picture repeats under a scale of 256. The bent picture is the
//! conformal map from B. de Smit and H. W. Lenstra Jr., "The Mathematical
//! Structure of Escher's Print Gallery", Notices of the AMS 50(4), 2003,
//! pp. 446-451. Page 446 states that an idealized print contains a copy of
//! itself, turned clockwise by 157.6255960832... degrees and scaled down by
//! a factor of about 22.58. Zooming by that factor lands on the same frame.
//! The room draws its own frame, not the print. It makes no claim about
//! consciousness. See `docs/DIGITAL_MINDS.md` and `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_INPUTS, Room, RoomInput, inputs_from_pokes};
use crate::surface::Surface;

/// Straight repetition. Four expansions by 4, and `4^4 = 256` (the paper's grid).
const DROSTE_SCALE: f64 = 256.0;
/// Right-hand strip. `y` below the midline is the straight repetition.
const BEND_X: f64 = 0.90;
/// Bottom band. `x` names deeper, back, or turned.
const WAGER_Y: f64 = 0.92;
/// Page 446's clockwise angle, in degrees. The period test locks this numeral.
#[cfg(test)]
const PAPER_TURN_DEGREES: f64 = 157.6255960832;
/// The closed form's scale factor. The period test locks this numeral.
#[cfg(test)]
const PAPER_ZOOM: f64 = 22.58368452861849;

/// One query of the picture. `t` is the zoom along one bent period.
struct Scene {
    t: f64,
    bend: bool,
    wager: Option<usize>,
}

fn phase_turns(t: f64) -> f64 {
    if t.is_finite() && t > 0.0 { t } else { 0.0 }
}

fn cdiv(numer: (f64, f64), denom: (f64, f64)) -> (f64, f64) {
    let n = denom.0 * denom.0 + denom.1 * denom.1;
    (
        (numer.0 * denom.0 + numer.1 * denom.1) / n,
        (numer.1 * denom.0 - numer.0 * denom.1) / n,
    )
}

fn cabs(value: (f64, f64)) -> f64 {
    value.0.hypot(value.1)
}

fn cexp(value: (f64, f64)) -> (f64, f64) {
    let radius = value.0.exp();
    (radius * value.1.cos(), radius * value.1.sin())
}

/// `beta = 2 pi i / (2 pi i - ln(1/256))`.
fn beta() -> (f64, f64) {
    let period = DROSTE_SCALE.ln();
    let two_pi = std::f64::consts::TAU;
    cdiv((0.0, two_pi), (period, two_pi))
}

/// The multiplier of the inner copy: `(1/256)^beta`.
fn inner_copy() -> (f64, f64) {
    let period = DROSTE_SCALE.ln();
    cexp((-period * beta().0, -period * beta().1))
}

/// Clockwise degrees of one bent period. Page 446's 157.6255960832....
fn turn_degrees() -> f64 {
    let copy = inner_copy();
    -copy.1.atan2(copy.0).to_degrees()
}

/// How many times smaller that inner copy is. The paper's factor of about 22.58.
fn zoom_factor() -> f64 {
    1.0 / cabs(inner_copy())
}

fn seed_angle(seed: u64) -> f64 {
    (seed % 8) as f64 * std::f64::consts::TAU / 48.0
}

fn scene_at(t: f64, inputs: &[RoomInput]) -> Scene {
    let phase = phase_turns(t);
    let start = inputs.len().saturating_sub(MAX_ROOM_INPUTS);
    let mut zoom = None;
    let mut bend = None;
    let mut wager_x = None;
    for input in &inputs[start..] {
        match *input {
            RoomInput::PointerDown { x, y, .. } | RoomInput::PointerMove { x, y, .. } => {
                if !x.is_finite() || !y.is_finite() {
                    continue;
                }
                let (x, y) = (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0));
                if y >= WAGER_Y {
                    wager_x = Some(x);
                } else if x >= BEND_X {
                    bend = Some(y >= 0.5);
                } else {
                    zoom = Some(y);
                }
            }
            _ => {}
        }
    }
    let unit = zoom.unwrap_or(phase);
    let slot = wager_x.map(|x| {
        let scaled = (x.clamp(0.0, BEND_X) / BEND_X) * 3.0;
        (scaled.floor() as usize).min(2)
    });
    Scene {
        t: unit,
        bend: bend.unwrap_or(true),
        wager: slot,
    }
}

/// Log-polar sample after the zoom. The bent branch divides by `beta` first,
/// so one period subtracts exactly `ln(256)` and the frame repeats.
fn sample_log(dx: f64, dy: f64, scene: &Scene, origin: f64) -> Option<(f64, f64)> {
    let radius = dx.hypot(dy);
    if radius < 1e-8 {
        return None;
    }
    let rho = radius.ln();
    let theta = dy.atan2(dx) + origin;
    let (rho, theta) = if scene.bend {
        cdiv((rho, theta), beta())
    } else {
        (rho - scene.t * zoom_factor().ln(), theta)
    };
    let rho = if scene.bend {
        rho - scene.t * DROSTE_SCALE.ln()
    } else {
        rho
    };
    Some((rho, theta))
}

fn mark_of(dx: f64, dy: f64, scene: &Scene, origin: f64) -> char {
    let Some((rho, theta)) = sample_log(dx, dy, scene, origin) else {
        return ' ';
    };
    let period = DROSTE_SCALE.ln();
    let u = (rho / period).rem_euclid(1.0);
    let v = (theta / std::f64::consts::TAU).rem_euclid(1.0);
    let frame = u < 0.07 || u > 0.93 || v < 0.07 || v > 0.93;
    let spine = (u - 0.38).abs() < 0.018;
    if frame {
        '#'
    } else if spine {
        '.'
    } else {
        ' '
    }
}

fn draw(canvas: &mut dyn Surface, scene: &Scene, origin: f64) {
    let width = canvas.width();
    let height = canvas.height();
    if width == 0 || height == 0 {
        return;
    }
    let aspect = canvas.char_aspect();
    let aspect = if aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        1.0
    };
    let cx = (width.saturating_sub(1)) as f64 / 2.0;
    let cy = (height.saturating_sub(1)) as f64 / 2.0;
    let sx = (width as f64 * 0.45).max(1.0);
    let sy = (height as f64 * 0.45).max(1.0);
    for y in 0..height {
        for x in 0..width {
            let dx = (x as f64 - cx) / sx;
            let dy = (y as f64 - cy) / sy * aspect;
            let mark = mark_of(dx, dy, scene, origin);
            if mark != ' ' {
                canvas.plot(x as i32, y as i32, mark);
            }
        }
    }
}

fn correct_slot(bend: bool) -> usize {
    if bend { 2 } else { 0 }
}

fn slot_name(slot: usize) -> &'static str {
    match slot {
        0 => "deeper",
        1 => "back",
        _ => "turned",
    }
}

fn readout(scene: &Scene) -> String {
    let scale = zoom_factor().powf(scene.t);
    let turn = if scene.bend {
        turn_degrees() * scene.t
    } else {
        0.0
    };
    let bend = if scene.bend { 1 } else { 0 };
    let mut line = format!("zoom={scale:.2} turn={turn:.1} bend={bend}");
    if let Some(slot) = scene.wager {
        let grade = if slot == correct_slot(scene.bend) {
            "yes"
        } else {
            "no"
        };
        line.push_str(&format!(" call={} {grade}", slot_name(slot)));
    } else {
        line.push_str(" ZOOM");
    }
    line
}

/// The graded call, or nothing when the player has not named an ending.
#[must_use]
pub fn wager_token(t: f64, inputs: &[RoomInput]) -> Option<String> {
    let scene = scene_at(t, inputs);
    let slot = scene.wager?;
    let grade = if slot == correct_slot(scene.bend) {
        "right"
    } else {
        "wrong"
    };
    Some(format!("strange-loop:{slot}:{grade}"))
}

/// Strange Loop room. The variation seed turns the whole frame a little.
#[derive(Debug, Default)]
pub struct StrangeLoop {
    seed: u64,
}

impl StrangeLoop {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create the room with a fixed turn of the frame.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }
}

impl Room for StrangeLoop {
    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, &scene_at(t, &[]), seed_angle(self.seed));
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let inputs = inputs_from_pokes(pokes, t);
        draw(canvas, &scene_at(t, &inputs), seed_angle(self.seed));
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "a descent that returns",
            root: 207.65,
            tempo: 88,
            line: &[0, 4, 7, 12, 7, 4, 0, -5, 0],
            encodes: "one zoom that lands on the frame it left",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: ZOOM")
    }

    fn status(&self, t: f64) -> Option<String> {
        Some(readout(&scene_at(t, &[])))
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        Some(readout(&scene_at(t, inputs)))
    }

    fn reveal(&self) -> &'static str {
        "Zoom by the factor on page 446 of de Smit and Lenstra, Notices of the \
         AMS 50(4), 2003, and the frame you left is the frame you meet, turned \
         clockwise by 157.6255960832 degrees. The straight repetition is a \
         scale of 256 with no turn, and that same zoom does not close. Quine \
         keeps a pattern that contains its own description."
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PAPER_TURN_DEGREES, PAPER_ZOOM, StrangeLoop, scene_at, turn_degrees, wager_token,
        zoom_factor,
    };
    use crate::canvas::Canvas;
    use crate::room::{Room, inputs_from_pokes};

    #[test]
    fn the_period_matches_page_446() {
        assert!((turn_degrees() - PAPER_TURN_DEGREES).abs() < 1e-9);
        assert!((zoom_factor() - PAPER_ZOOM).abs() < 1e-9);
    }

    #[test]
    fn one_bent_period_returns_the_same_frame() {
        let room = StrangeLoop::new();
        let mut start = Canvas::new(48, 32);
        let mut period = Canvas::new(48, 32);
        room.render(&mut start, 0.0);
        room.render(&mut period, 1.0);
        assert_eq!(start.to_text(), period.to_text());
        assert!(start.ink_count() > 10);
    }

    #[test]
    fn the_same_zoom_on_the_straight_picture_does_not_close() {
        let room = StrangeLoop::new();
        let pokes = [(0.95, 0.2)];
        let mut start = Canvas::new(48, 32);
        let mut later = Canvas::new(48, 32);
        room.render_poked(&mut start, 0.0, &pokes);
        room.render_poked(&mut later, 1.0, &pokes);
        assert_ne!(start.to_text(), later.to_text());
    }

    #[test]
    fn a_mid_zoom_is_not_the_opening_frame() {
        let room = StrangeLoop::new();
        let mut start = Canvas::new(48, 32);
        let mut mid = Canvas::new(48, 32);
        room.render(&mut start, 0.0);
        room.render(&mut mid, 0.35);
        assert_ne!(start.to_text(), mid.to_text());
    }

    #[test]
    fn status_names_the_readings_and_the_call() {
        let open = StrangeLoop::new().status(0.0).unwrap();
        assert!(open.contains("ZOOM"), "{open}");
        assert!(open.contains("bend=1"), "{open}");
        assert!(!open.split(|ch: char| !ch.is_ascii_alphabetic()).any(|word| {
            matches!(word, "because" | "returns" | "copy" | "deeper")
        }));
        let bent = scene_at(1.0, &[]);
        let line = super::readout(&bent);
        assert!(line.starts_with("zoom=22.58 turn=157.6"), "{line}");
        let inputs = inputs_from_pokes(&[(0.75, 0.95)], 0.2);
        let called = StrangeLoop::new().status_input(0.2, &inputs).unwrap();
        assert!(called.contains("call=turned yes"), "{called}");
        assert_eq!(
            wager_token(0.2, &inputs).as_deref(),
            Some("strange-loop:2:right")
        );
        let straight = inputs_from_pokes(&[(0.95, 0.2), (0.15, 0.95)], 0.4);
        assert_eq!(
            wager_token(0.4, &straight).as_deref(),
            Some("strange-loop:0:right")
        );
    }

    #[test]
    fn the_reveal_names_the_paper_and_leaves_quine_its_own_sentence() {
        let reveal = StrangeLoop::new().reveal();
        assert!(reveal.contains("de Smit"));
        assert!(reveal.contains("157.6255960832"));
        assert!(reveal.contains("256"));
        let quine = crate::rooms::quine::Quine::new().reveal();
        assert!(quine.contains("prints its own source"));
        assert!(!reveal.contains("prints its own source"));
        assert!(!quine.contains("de Smit"));
    }

    #[test]
    fn the_seed_turns_the_frame_and_the_period_still_closes() {
        let room = StrangeLoop::new_with(3);
        let mut start = Canvas::new(36, 24);
        let mut period = Canvas::new(36, 24);
        let mut bare = Canvas::new(36, 24);
        room.render(&mut start, 0.2);
        room.render(&mut period, 1.2);
        StrangeLoop::new().render(&mut bare, 0.2);
        assert_eq!(start.to_text(), period.to_text());
        assert_ne!(start.to_text(), bare.to_text());
    }

    #[test]
    fn non_finite_phase_stays_at_the_opening() {
        let room = StrangeLoop::new();
        let mut finite = Canvas::new(24, 16);
        let mut messy = Canvas::new(24, 16);
        room.render(&mut finite, 0.0);
        room.render(&mut messy, f64::NAN);
        assert_eq!(finite.to_text(), messy.to_text());
    }
}

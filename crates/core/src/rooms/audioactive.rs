//! Audioactive Decay: look-and-say shatters into Conway's constant.
//!
//! Read a digit string aloud by runs: 1 becomes 11, 21 becomes 1211, 1211
//! becomes 111221. Conway proved the process splits into 92 "atoms" and the
//! length grows by a fixed constant lambda ≈ 1.303577 each step. CLICK: SPEAK
//! THE NEXT LINE. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

/// Seed sequences (digit strings as ASCII).
const SEEDS: &[&str] = &["1", "3", "22", "13112221"];
/// Max generations to display.
const MAX_GEN: usize = 12;
/// Max characters drawn on the plate.
const MAX_DRAW: usize = 120;
const VARIATION_SALT: u64 = 0xA0D1_0A5E_5EED_0001;

/// Conway's constant (growth rate of look-and-say length).
const LAMBDA: f64 = 1.303_577_269;

fn phase_unit(t: f64) -> f64 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn finite_pokes(pokes: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .copied()
        .filter(|&(x, y)| x.is_finite() && y.is_finite())
        .map(|(x, y)| (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)))
        .collect()
}

fn look_and_say(s: &str) -> String {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(s.len() * 2);
    let mut i = 0;
    while i < bytes.len() {
        let d = bytes[i];
        let mut j = i + 1;
        while j < bytes.len() && bytes[j] == d {
            j += 1;
        }
        let count = j - i;
        // Count is almost always a single digit in look-and-say from single digits.
        out.push(char::from(b'0' + (count as u8).min(9)));
        out.push(char::from(d));
        i = j;
    }
    out
}

fn look_line(seed_s: &str, n: usize) -> String {
    let mut s = seed_s.to_string();
    for _ in 0..n {
        s = look_and_say(&s);
        if s.len() > 50_000 {
            break;
        }
    }
    s
}

fn seed_string(seed: u64) -> &'static str {
    if seed == 0 {
        SEEDS[0]
    } else {
        SEEDS[((seed ^ VARIATION_SALT) as usize) % SEEDS.len()]
    }
}

fn ambient_gen(t: f64) -> usize {
    (phase_unit(t) * MAX_GEN as f64).round() as usize
}

fn draw_digits(canvas: &mut dyn Surface, s: &str, step: usize) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let digits: Vec<char> = s.chars().take(MAX_DRAW).collect();
    let top = height as f64 * 0.16;
    let baseline = height as f64 * 0.82;
    let band = (baseline - top).max(1.0);
    let mut previous_peak = None;
    for (index, ch) in digits.iter().copied().enumerate() {
        let px = ((index + 1) as f64 * width as f64 / (digits.len() + 1) as f64).round() as i32;
        let level = match ch {
            '1' => 0.38,
            '2' => 0.62,
            '3' => 0.86,
            _ => 1.0,
        };
        let py = (baseline - band * level).round() as i32;
        let mark = match ch {
            '1' => '.',
            '2' => ':',
            '3' => '+',
            _ => '#',
        };
        canvas.line(px, baseline.round() as i32, px, py, mark);
        if let Some((last_x, last_y)) = previous_peak {
            canvas.line(last_x, last_y, px, py, mark);
        }
        previous_peak = Some((px, py));
    }
    // Generation tick marks along the bottom.
    let gy = height.saturating_sub(1) as i32;
    for g in 0..=step.min(20) {
        let x = ((g as f64 / 20.0) * width.saturating_sub(1) as f64).round() as i32;
        canvas.plot(x, gy, if g == step { '#' } else { '.' });
    }
}

/// Audioactive Decay room.
#[derive(Debug, Default)]
pub struct Audioactive {
    seed: u64,
}

impl Audioactive {
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
}

impl Room for Audioactive {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "audioactive",
            title: "Audioactive Decay",
            wing: "Number & Pattern",
            blurb: "Speak a digit string by runs and it mutates: look-and-say. Length grows by \
                    Conway's constant; the sequence shatters into 92 atoms. t advances generations; \
                    CLICK: SPEAK THE NEXT LINE.",
            accent: [180, 100, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let step = ambient_gen(t);
        let s = look_line(seed_string(self.seed), step);
        draw_digits(canvas, &s, step);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "look and say",
            root: 207.65,
            tempo: 120,
            line: &[0, 2, 4, 5, 7, 5, 4, 0],
            encodes: "run lengths speaking the next digit line",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: SPEAK THE NEXT LINE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let step = ambient_gen(t);
        let s = look_line(seed_string(self.seed), step);
        let prev = if step == 0 {
            seed_string(self.seed).len()
        } else {
            look_line(seed_string(self.seed), step - 1).len()
        };
        let ratio = if prev == 0 {
            0.0
        } else {
            s.len() as f64 / prev as f64
        };
        Some(format!("G{step}  L{}  x{ratio:.3}  CLICK:SPEAK", s.len()))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        if hands.is_empty() {
            self.render(canvas, t);
            return;
        }
        let step = (ambient_gen(t) + hands.len()).min(MAX_GEN + 8);
        let s = look_line(seed_string(self.seed), step);
        draw_digits(canvas, &s, step);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let step = (ambient_gen(t) + hands.len()).min(MAX_GEN + 8);
        let s = look_line(seed_string(self.seed), step);
        let prev = look_line(seed_string(self.seed), step.saturating_sub(1)).len();
        let ratio = if prev == 0 {
            0.0
        } else {
            s.len() as f64 / prev as f64
        };
        let dlam = ratio - LAMBDA;
        Some(format!(
            "SPEAK G{step}  L{}  x{ratio:.3}  dL={dlam:+.3}",
            s.len()
        ))
    }

    fn reveal(&self) -> &'static str {
        "Look-and-say is pure reading: count runs of digits. Conway proved the \
         infinite process splits into 92 irreducible atoms and that length grows \
         by a fixed algebraic number lambda ≈ 1.303577269 each generation: \
         Conway's constant."
    }
}

#[cfg(test)]
mod tests {
    use super::{Audioactive, LAMBDA, look_and_say};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn classic_look_and_say_steps() {
        assert_eq!(look_and_say("1"), "11");
        assert_eq!(look_and_say("11"), "21");
        assert_eq!(look_and_say("21"), "1211");
        assert_eq!(look_and_say("1211"), "111221");
        assert_eq!(look_and_say("111221"), "312211");
    }

    #[test]
    fn length_grows() {
        let mut s = "1".to_string();
        let mut prev = s.len();
        for _ in 0..8 {
            s = look_and_say(&s);
            assert!(s.len() >= prev);
            prev = s.len();
        }
    }

    #[test]
    fn first_contact_status_invites_speak() {
        let room = Audioactive::new();
        let open = room.status(0.0).expect("open");
        assert!(open.contains("CLICK") || open.contains("SPEAK"), "{open}");
        assert!(open.chars().count() <= 56, "{open}");
    }

    #[test]
    fn speak_changes_status() {
        let room = Audioactive::new();
        let open = room.status(0.0).expect("open");
        let input = [RoomInput::PointerDown {
            x: 0.5,
            y: 0.5,
            t: 0.0,
        }];
        let after = room.status_input(0.0, &input).expect("speak");
        assert_ne!(after, open);
        assert!(after.contains("SPEAK") || after.contains("G"), "{after}");
        assert!(after.chars().count() <= 56, "{after}");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = Audioactive::new();
        let mut a = Canvas::new(48, 24);
        let mut b = Canvas::new(48, 24);
        room.render(&mut a, 0.6);
        room.render(&mut b, 0.6);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 5);
    }

    #[test]
    fn click_advances_generation_picture() {
        let room = Audioactive::new();
        let mut base = Canvas::new(40, 20);
        let mut poked = Canvas::new(40, 20);
        room.render(&mut base, 0.0);
        room.render_poked(&mut poked, 0.0, &[(0.5, 0.5)]);
        let changed = (0..20)
            .flat_map(|y| (0..40).map(move |x| (x, y)))
            .filter(|&(x, y)| base.cell(x, y) != poked.cell(x, y))
            .count();
        assert!(changed >= 15, "spoken line changed only {changed} cells");
    }

    #[test]
    fn variation_changes_seed_string_path() {
        let mut a = Canvas::new(40, 20);
        let mut b = Canvas::new(40, 20);
        Audioactive::new_with(0).render(&mut a, 0.4);
        Audioactive::new_with(2).render(&mut b, 0.4);
        assert_ne!(a.to_text(), b.to_text());
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Audioactive::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, f64::NAN, f64::INFINITY] {
            room.render(&mut canvas, t);
            room.render_poked(&mut canvas, t, &[(f64::NAN, f64::INFINITY)]);
        }
    }

    #[test]
    fn reveal_names_conway_or_look_and_say() {
        let text = Audioactive::new().reveal().to_ascii_lowercase();
        assert!(text.contains("conway") || text.contains("look") || text.contains("lambda"));
        let _ = LAMBDA;
    }

    #[test]
    fn motif_is_playable() {
        let motif = Audioactive::new().motif().expect("motif");
        assert!(motif.line.len() >= 6);
        assert!(motif.pattern().seconds() > 0.0);
    }
}

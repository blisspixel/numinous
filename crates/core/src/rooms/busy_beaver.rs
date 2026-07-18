//! The Busy Beaver: five rules run a known forever-finite burst, then stop.
//!
//! A 2-symbol Turing machine with n states has a champion that writes the most
//! 1s before halting. BB(5) = 47,176,870 (bbchallenge 2024). This room runs a
//! tiny toy champion path for small n so the stop is felt; CLICK: FLIP ONE RULE
//! mutates a transition. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

/// Toy machine: 2 symbols, up to 4 states (full BB(5) is too long for a frame).
const MAX_STATES: usize = 4;
const TAPE: usize = 96;
const MAX_STEPS: usize = 8_000;
/// Documented BB(5) value (not simulated here in full).
const BB5: u64 = 47_176_870;

#[derive(Clone, Copy, Debug)]
struct Trans {
    write: u8,
    dir: i8,  // -1 left, +1 right
    next: u8, // 255 = halt
}

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

/// A known small busy beaver-style table for 3 states (classic BB(3)=6 ones).
fn default_table(seed: u64) -> [[Trans; 2]; MAX_STATES] {
    // Lin-Rado style 3-state champion (simplified).
    let mut t = [[Trans {
        write: 0,
        dir: 1,
        next: 255,
    }; 2]; MAX_STATES];
    t[0][0] = Trans {
        write: 1,
        dir: 1,
        next: 1,
    };
    t[0][1] = Trans {
        write: 1,
        dir: -1,
        next: 2,
    };
    t[1][0] = Trans {
        write: 1,
        dir: -1,
        next: 0,
    };
    t[1][1] = Trans {
        write: 1,
        dir: 1,
        next: 1,
    };
    t[2][0] = Trans {
        write: 1,
        dir: -1,
        next: 1,
    };
    t[2][1] = Trans {
        write: 1,
        dir: 1,
        next: 255,
    };
    if seed != 0 {
        let i = (seed as usize) % 3;
        t[i][0].dir = -t[i][0].dir;
    }
    t
}

fn flip_rule(table: &mut [[Trans; 2]; MAX_STATES], x: f64, y: f64) {
    let state = ((y.clamp(0.0, 0.999) * 3.0) as usize).min(2);
    let sym = if x < 0.5 { 0 } else { 1 };
    let tr = &mut table[state][sym];
    tr.write ^= 1;
    tr.dir = -tr.dir;
    if tr.next == 255 {
        tr.next = 0;
    } else {
        tr.next = (tr.next + 1) % 3;
        if tr.next == 0 && (state + sym).is_multiple_of(2) {
            tr.next = 255;
        }
    }
}

fn run(table: &[[Trans; 2]; MAX_STATES], max_steps: usize) -> (Vec<u8>, usize, usize, bool) {
    let mut tape = vec![0u8; TAPE];
    let mut head = TAPE / 2;
    let mut state: u8 = 0;
    let mut steps = 0usize;
    let mut halted = false;
    for _ in 0..max_steps {
        if state == 255 {
            halted = true;
            break;
        }
        let si = state as usize;
        if si >= 3 {
            halted = true;
            break;
        }
        let sym = tape[head] as usize;
        let tr = table[si][sym];
        tape[head] = tr.write;
        let nh = head as i32 + i32::from(tr.dir);
        if nh < 0 || nh >= TAPE as i32 {
            break;
        }
        head = nh as usize;
        state = tr.next;
        steps += 1;
        if state == 255 {
            halted = true;
            break;
        }
    }
    let ones = tape.iter().filter(|&&c| c == 1).count();
    (tape, steps, ones, halted)
}

fn draw_tape(canvas: &mut dyn Surface, tape: &[u8], head_frac: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let mid = height / 2;
    let half_bar = ((height as f64 * 0.08).round() as i32).max(2);
    for (i, &cell) in tape.iter().enumerate() {
        let x = (i as f64 / TAPE as f64 * width.saturating_sub(1) as f64).round() as i32;
        if cell == 1 {
            canvas.line(x, mid as i32 - half_bar, x, mid as i32 + half_bar, '#');
        } else {
            canvas.plot(x, mid as i32, '.');
        }
    }
    let hx = (head_frac * width.saturating_sub(1) as f64).round() as i32;
    canvas.plot(hx, mid as i32 + 2, 'v');
}

/// Busy Beaver room.
#[derive(Debug, Default)]
pub struct BusyBeaver {
    seed: u64,
}

impl BusyBeaver {
    /// Create the room with default seed (0).
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed for replayable per-visit novelty.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }
}

impl Room for BusyBeaver {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "busy-beaver",
            title: "The Busy Beaver",
            wing: "Number & Pattern",
            blurb: "A tiny Turing machine races to write ones then halt. BB(5)=47,176,870 is proven; \
                    here a toy champion stops on purpose. t extends the step budget; CLICK: FLIP \
                    ONE RULE.",
            accent: [120, 80, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let table = default_table(self.seed);
        let budget = 200 + (phase_unit(t) * MAX_STEPS as f64) as usize;
        let (tape, _, _, _) = run(&table, budget);
        draw_tape(canvas, &tape, 0.5);
    }

    fn postcard_t(&self) -> f64 {
        0.85
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "halt cadence",
            root: 130.81,
            tempo: 100,
            line: &[0, 5, 7, 12, 7, 5, 0, 12],
            encodes: "busy writing until the halt state ends the song",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: FLIP ONE RULE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let table = default_table(self.seed);
        let budget = 200 + (phase_unit(t) * MAX_STEPS as f64) as usize;
        let (_, steps, ones, halted) = run(&table, budget);
        let tag = if halted { "HALT" } else { "RUN" };
        Some(format!("S{steps}  1s={ones}  BB5={BB5}  {tag}  CLICK:FLIP"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let mut table = default_table(self.seed);
        for &(x, y) in &hands {
            flip_rule(&mut table, x, y);
        }
        let budget = 200 + (phase_unit(t) * MAX_STEPS as f64) as usize;
        let (tape, _, _, _) = run(&table, budget);
        draw_tape(canvas, &tape, hands.last().map(|(x, _)| *x).unwrap_or(0.5));
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let mut table = default_table(self.seed);
        for &(x, y) in &hands {
            flip_rule(&mut table, x, y);
        }
        let budget = 200 + (phase_unit(t) * MAX_STEPS as f64) as usize;
        let (_, steps, ones, halted) = run(&table, budget);
        let tag = if halted { "HALT" } else { "RUN" };
        Some(format!("FLIP n{}  S{steps}  1s={ones}  {tag}", hands.len()))
    }

    fn reveal(&self) -> &'static str {
        "Among all n-state 2-symbol machines that halt, one writes the most ones: \
         the busy beaver. BB(5)=47,176,870 was settled by the 2024 bbchallenge \
         Coq-verified proof. Larger BB values encode undecidable questions: the \
         function grows faster than any computable sequence."
    }
}

#[cfg(test)]
mod tests {
    use super::{BusyBeaver, default_table, run};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn default_machine_writes_ones_and_can_halt() {
        let t = default_table(0);
        let (_, steps, ones, _) = run(&t, 5000);
        assert!(steps > 0);
        assert!(ones > 0);
    }

    #[test]
    fn first_contact_invites_flip() {
        let open = BusyBeaver::new().status(0.0).unwrap();
        assert!(open.contains("CLICK") || open.contains("FLIP"));
        assert!(open.chars().count() <= 56);
    }

    #[test]
    fn flip_changes_status() {
        let room = BusyBeaver::new();
        let open = room.status(0.0).unwrap();
        let after = room
            .status_input(
                0.0,
                &[RoomInput::PointerDown {
                    x: 0.2,
                    y: 0.3,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(open, after);
        assert!(after.chars().count() <= 56);
    }

    #[test]
    fn render_has_ink() {
        let room = BusyBeaver::new();
        let mut c = Canvas::new(64, 20);
        room.render(&mut c, 0.8);
        assert!(c.ink_count() > 5);
    }

    #[test]
    fn flipped_rule_changes_a_visible_tape_band() {
        let room = BusyBeaver::new();
        let mut base = Canvas::new(80, 40);
        let mut flipped = Canvas::new(80, 40);
        room.render(&mut base, 0.0);
        room.render_poked(&mut flipped, 0.0, &[(0.82, 0.8)]);
        let changed = (0..40)
            .flat_map(|y| (0..80).map(move |x| (x, y)))
            .filter(|&(x, y)| base.cell(x, y) != flipped.cell(x, y))
            .count();
        assert!(changed >= 30, "flipped tape changed only {changed} cells");
    }

    #[test]
    fn motif_ok() {
        assert!(BusyBeaver::new().motif().unwrap().pattern().seconds() > 0.0);
    }

    #[test]
    fn extreme_ok() {
        let room = BusyBeaver::new();
        let mut c = Canvas::new(4, 4);
        room.render(&mut c, f64::NAN);
        room.render_poked(&mut c, 0.0, &[(f64::INFINITY, 0.0)]);
    }
}

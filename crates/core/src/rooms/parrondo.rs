//! Parrondo's Trap: two losing games, scheduled as ABB, win.
//!
//! Game A is a slightly losing coin. Game B is capital-dependent and losing
//! in isolation. The periodic ABB schedule changes the residue distribution
//! that meets B's bad coin and yields positive drift. See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
use crate::surface::Surface;

const SEED: u64 = 0x0A44_0AD0_0000_0001;
/// Number of turns in the room's comparison experiment.
pub const DEMONSTRATION_STEPS: usize = 120;
const GAME_A_WIN: f64 = 0.495;
const GAME_B_TRAP_WIN: f64 = 0.095;
const GAME_B_OTHER_WIN: f64 = 0.745;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// One of the three policies offered by Parrondo's Trap.
pub enum Policy {
    /// Play the slightly losing game A every turn.
    OnlyA,
    /// Play the capital-dependent losing game B every turn.
    OnlyB,
    /// Repeat A, B, B.
    CycleAbb,
}

impl Policy {
    /// Compact label shared by room chrome and agent evidence.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::OnlyA => "A",
            Self::OnlyB => "B",
            Self::CycleAbb => "ABB",
        }
    }

    /// Map an App number key onto a policy call.
    #[must_use]
    pub fn from_key_digit(digit: u8) -> Option<Self> {
        match digit {
            1 => Some(Self::OnlyA),
            2 => Some(Self::OnlyB),
            3 => Some(Self::CycleAbb),
            _ => None,
        }
    }

    /// Map a normalized hand position onto one of the three policies.
    #[must_use]
    pub fn from_unit_x(x: f64) -> Self {
        let x = if x.is_finite() {
            x.clamp(0.0, 1.0)
        } else {
            0.5
        };
        if x < 1.0 / 3.0 {
            Self::OnlyA
        } else if x < 2.0 / 3.0 {
            Self::OnlyB
        } else {
            Self::CycleAbb
        }
    }

    fn uses_a(self, turn: usize) -> bool {
        match self {
            Self::OnlyA => true,
            Self::OnlyB => false,
            Self::CycleAbb => turn.is_multiple_of(3),
        }
    }
}

fn policy(t: f64, hand: Option<(f64, f64)>) -> Policy {
    let u = if let Some((x, _)) = hand {
        x
    } else {
        phase_unit(t)
    };
    Policy::from_unit_x(u)
}

fn play(policy: Policy, seed: u64, steps: usize) -> Vec<i32> {
    let mut rng = SplitMix64::new(SEED ^ seed ^ (steps as u64));
    let mut cap = 0i32;
    let mut path = Vec::with_capacity(steps + 1);
    path.push(cap);
    for i in 0..steps {
        let p = if policy.uses_a(i) {
            GAME_A_WIN
        } else if cap % 3 == 0 {
            GAME_B_TRAP_WIN
        } else {
            GAME_B_OTHER_WIN
        };
        if rng.next_f64() < p {
            cap += 1;
        } else {
            cap -= 1;
        }
        path.push(cap);
    }
    path
}

/// Exact expected capital after every turn under a policy.
///
/// The state is the probability of occupying each capital residue modulo
/// three. Tracking it makes the room's truth deterministic and avoids grading
/// a player against one fortunate random walk.
#[must_use]
pub fn expected_path(policy: Policy, steps: usize) -> Vec<f64> {
    let mut residues = [1.0, 0.0, 0.0];
    let mut capital = 0.0;
    let mut path = Vec::with_capacity(steps.saturating_add(1));
    path.push(capital);
    for turn in 0..steps {
        let mut next = [0.0; 3];
        for (residue, mass) in residues.into_iter().enumerate() {
            let win = if policy.uses_a(turn) {
                GAME_A_WIN
            } else if residue == 0 {
                GAME_B_TRAP_WIN
            } else {
                GAME_B_OTHER_WIN
            };
            capital += mass * (2.0 * win - 1.0);
            next[(residue + 1) % 3] += mass * win;
            next[(residue + 2) % 3] += mass * (1.0 - win);
        }
        residues = next;
        path.push(capital);
    }
    path
}

/// Exact expected capital at the end of the room's comparison experiment.
#[must_use]
pub fn expected_end(policy: Policy) -> f64 {
    *expected_path(policy, DEMONSTRATION_STEPS)
        .last()
        .unwrap_or(&0.0)
}

fn draw(canvas: &mut dyn Surface, path: &[i32]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || path.len() < 2 {
        return;
    }
    let min = path.iter().copied().min().unwrap_or(0);
    let max = path.iter().copied().max().unwrap_or(1);
    let span = (max - min).max(1) as f64;
    let mut prev: Option<(i32, i32)> = None;
    for (i, &c) in path.iter().enumerate() {
        let x =
            (i as f64 / (path.len() - 1) as f64 * width.saturating_sub(1) as f64).round() as i32;
        let u = (c - min) as f64 / span;
        let y = ((1.0 - u) * height.saturating_sub(1) as f64).round() as i32;
        if let Some(o) = prev {
            canvas.line(o.0, o.1, x, y, if c >= 0 { '#' } else { '*' });
        }
        prev = Some((x, y));
    }
    // Zero line.
    let zy = ((1.0 - (0 - min) as f64 / span) * height.saturating_sub(1) as f64).round() as i32;
    canvas.line(0, zy, width.saturating_sub(1) as i32, zy, '.');
}

/// Parrondo room.
#[derive(Debug, Default)]
pub struct Parrondo {
    seed: u64,
}

impl Parrondo {
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

impl Room for Parrondo {

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let pol = policy(t, None);
        let path = play(pol, self.seed, DEMONSTRATION_STEPS);
        draw(canvas, &path);
    }

    fn postcard_t(&self) -> f64 {
        0.75
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "parrondo",
            root: 207.65,
            tempo: 116,
            line: &[0, 3, 5, 7, 12, 7, 5, 3],
            encodes: "two losing games scheduled into a winner",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TOGGLE THE RULE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let pol = policy(t, None);
        let path = play(pol, self.seed, DEMONSTRATION_STEPS);
        let end = *path.last().unwrap_or(&0);
        Some(format!("rule={}  cap={end}  DRAG:RULE", pol.name()))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let pol = policy(t, hands.last().copied());
        let path = play(pol, self.seed ^ hands.len() as u64, DEMONSTRATION_STEPS);
        draw(canvas, &path);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let pol = policy(t, hands.last().copied());
        let path = play(pol, self.seed ^ hands.len() as u64, DEMONSTRATION_STEPS);
        let end = *path.last().unwrap_or(&0);
        Some(format!("{}  end={end}", pol.name()))
    }

    fn reveal(&self) -> &'static str {
        "Parrondo's paradox: games A and B each lose in expectation, but the \
         periodic schedule ABB wins. A changes which capital residues meet B, \
         so B's bad coin is used less often than when B runs alone."
    }
}

#[cfg(test)]
mod tests {
    use super::{DEMONSTRATION_STEPS, Parrondo, Policy, expected_end, expected_path};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Parrondo::new().status(0.2).unwrap();
        assert!(s.contains("DRAG") || s.contains("RULE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn toggle_changes() {
        let r = Parrondo::new();
        let o = r.status(0.1).unwrap();
        let a = r
            .status_input(
                0.1,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn exact_expectation_proves_two_losers_and_the_abb_winner() {
        assert!(expected_end(Policy::OnlyA) < 0.0);
        assert!(expected_end(Policy::OnlyB) < 0.0);
        assert!(expected_end(Policy::CycleAbb) > 7.0);
        assert_eq!(
            expected_path(Policy::CycleAbb, DEMONSTRATION_STEPS).len(),
            DEMONSTRATION_STEPS + 1
        );
    }

    #[test]
    fn policy_mapping_is_bounded_and_names_the_schedule() {
        assert_eq!(Policy::from_key_digit(3), Some(Policy::CycleAbb));
        assert_eq!(Policy::from_key_digit(4), None);
        assert_eq!(Policy::from_unit_x(f64::NAN), Policy::OnlyB);
        assert_eq!(Policy::from_unit_x(f64::INFINITY), Policy::OnlyB);
        assert_eq!(Policy::CycleAbb.name(), "ABB");
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Parrondo::new().render(&mut c, 0.8);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(Parrondo::new().motif().unwrap().line.len() >= 6);
    }
}

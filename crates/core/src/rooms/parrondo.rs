//! Parrondo's Trap: two losing games, alternating, win.
//!
//! Game A: slight losing coin. Game B: capital-dependent bias. Alternating
//! yields positive drift. TOGGLE: THE RULE. See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const SEED: u64 = 0x0A44_0AD0_0000_0001;
const STEPS: usize = 120;

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

#[derive(Clone, Copy)]
enum Policy {
    OnlyA,
    OnlyB,
    Alternate,
}

fn policy(t: f64, hand: Option<(f64, f64)>) -> Policy {
    let u = if let Some((x, _)) = hand {
        x
    } else {
        phase_unit(t)
    };
    if u < 0.33 {
        Policy::OnlyA
    } else if u < 0.66 {
        Policy::OnlyB
    } else {
        Policy::Alternate
    }
}

fn play(policy: Policy, seed: u64, steps: usize) -> Vec<i32> {
    let mut rng = SplitMix64::new(SEED ^ seed ^ (steps as u64));
    let mut cap = 0i32;
    let mut path = Vec::with_capacity(steps + 1);
    path.push(cap);
    let eps = 0.005;
    for i in 0..steps {
        let use_a = match policy {
            Policy::OnlyA => true,
            Policy::OnlyB => false,
            Policy::Alternate => i % 2 == 0,
        };
        let p = if use_a {
            0.5 - eps
        } else if cap % 3 == 0 {
            0.1 - eps
        } else {
            0.75 - eps
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
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "parrondo",
            title: "Parrondo's Trap",
            wing: "Number & Pattern",
            blurb: "Two losing games, played in alternation, can win. t and DRAG: TOGGLE THE RULE \
                    between A, B, and ABAB.",
            accent: [180, 100, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let pol = policy(t, None);
        let path = play(pol, self.seed, STEPS);
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
            encodes: "two losers alternating into a winner",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TOGGLE THE RULE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let pol = policy(t, None);
        let path = play(pol, self.seed, STEPS);
        let end = *path.last().unwrap_or(&0);
        let name = match pol {
            Policy::OnlyA => "A",
            Policy::OnlyB => "B",
            Policy::Alternate => "ABAB",
        };
        Some(format!("rule={name}  cap={end}  DRAG:RULE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let pol = policy(t, hands.last().copied());
        let path = play(pol, self.seed ^ hands.len() as u64, STEPS);
        draw(canvas, &path);
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.line(px - 2, py, px + 2, py, 'o');
                canvas.line(px, py - 2, px, py + 2, 'o');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let pol = policy(t, hands.last().copied());
        let path = play(pol, self.seed ^ hands.len() as u64, STEPS);
        let end = *path.last().unwrap_or(&0);
        let name = match pol {
            Policy::OnlyA => "ONLY A",
            Policy::OnlyB => "ONLY B",
            Policy::Alternate => "ALTERNATE",
        };
        Some(format!("{name}  end={end}"))
    }

    fn reveal(&self) -> &'static str {
        "Parrondo's paradox: two games with negative expected value can combine \
         into a positive-expectation strategy. Alternation breaks the capital \
         trap that makes game B lose on its own."
    }
}

#[cfg(test)]
mod tests {
    use super::{Parrondo, Policy, play};
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
    fn alternate_tends_higher_than_a() {
        // Stochastic; use long run mean over seeds.
        let mut sum_a = 0i64;
        let mut sum_ab = 0i64;
        for seed in 0..20u64 {
            sum_a += i64::from(*play(Policy::OnlyA, seed, 200).last().unwrap_or(&0));
            sum_ab += i64::from(*play(Policy::Alternate, seed, 200).last().unwrap_or(&0));
        }
        assert!(sum_ab > sum_a);
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

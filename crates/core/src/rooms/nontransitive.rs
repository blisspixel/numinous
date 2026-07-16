//! Nontransitive Dice: A beats B, B beats C, C beats A.
//!
//! Three dice with carefully chosen faces; pairwise win rates cycle.
//! ROLL: THE TRIO. See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const SEED: u64 = 0xD1CE_710A_0000_0001;

/// Efron's nontransitive set (simplified 4-face for display).
const A: [u8; 6] = [4, 4, 4, 4, 0, 0];
const B: [u8; 6] = [3, 3, 3, 3, 3, 3];
const C: [u8; 6] = [6, 6, 2, 2, 2, 2];
// Miwin-style: A beats B, B beats C, C beats A with positive margin.

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

fn win_rate(left: &[u8; 6], right: &[u8; 6]) -> f64 {
    let mut wins = 0u32;
    let mut total = 0u32;
    for &a in left {
        for &b in right {
            total += 1;
            if a > b {
                wins += 1;
            }
        }
    }
    wins as f64 / total as f64
}

fn roll(faces: &[u8; 6], rng: &mut SplitMix64) -> u8 {
    faces[(rng.next_u64() as usize) % faces.len()]
}

fn simulate(seed: u64, rounds: usize) -> (f64, f64, f64) {
    let mut rng = SplitMix64::new(SEED ^ seed);
    let mut ab = 0u32;
    let mut bc = 0u32;
    let mut ca = 0u32;
    for _ in 0..rounds {
        let a = roll(&A, &mut rng);
        let b = roll(&B, &mut rng);
        let c = roll(&C, &mut rng);
        if a > b {
            ab += 1;
        }
        if b > c {
            bc += 1;
        }
        if c > a {
            ca += 1;
        }
    }
    let n = rounds as f64;
    (ab as f64 / n, bc as f64 / n, ca as f64 / n)
}

fn draw(canvas: &mut dyn Surface, ab: f64, bc: f64, ca: f64, show: Option<(u8, u8, u8)>) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Three nodes in a triangle with directed edges labeled by win rate.
    let pts = [(0.50, 0.18, 'A'), (0.18, 0.78, 'B'), (0.82, 0.78, 'C')];
    let to_px = |x: f64, y: f64| -> (i32, i32) {
        (
            (x * width.saturating_sub(1) as f64).round() as i32,
            (y * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    let pa = to_px(pts[0].0, pts[0].1);
    let pb = to_px(pts[1].0, pts[1].1);
    let pc = to_px(pts[2].0, pts[2].1);
    canvas.line(pa.0, pa.1, pb.0, pb.1, if ab > 0.5 { '#' } else { '.' });
    canvas.line(pb.0, pb.1, pc.0, pc.1, if bc > 0.5 { '#' } else { '.' });
    canvas.line(pc.0, pc.1, pa.0, pa.1, if ca > 0.5 { '#' } else { '.' });
    for (x, y, ch) in pts {
        let p = to_px(x, y);
        canvas.plot(p.0, p.1, ch);
    }
    // Bars for exact pairwise rates.
    let bars = [(ab, 0.25), (bc, 0.5), (ca, 0.75)];
    for (rate, yf) in bars {
        let y = (yf * height as f64).round() as i32;
        let x1 = (0.15 * width as f64).round() as i32;
        let x2 = ((0.15 + rate * 0.7) * width as f64).round() as i32;
        canvas.line(x1, y, x2, y, '=');
    }
    if let Some((a, b, c)) = show {
        let y = (0.92 * height as f64).round() as i32;
        let label = format!("roll {a}/{b}/{c}");
        for (i, ch) in label.chars().enumerate() {
            let x = ((0.2 + i as f64 * 0.03) * width as f64).round() as i32;
            canvas.plot(x, y, ch);
        }
    }
}

/// Nontransitive Dice room.
#[derive(Debug, Default)]
pub struct Nontransitive {
    seed: u64,
}

impl Nontransitive {
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

impl Room for Nontransitive {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "nontransitive",
            title: "Nontransitive Dice",
            wing: "Number & Pattern",
            blurb: "A beats B, B beats C, C beats A: ranking collapses. t runs trials; CLICK: ROLL \
                    THE TRIO.",
            accent: [200, 140, 60],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let rounds = 200 + (phase_unit(t) * 800.0) as usize;
        let (ab, bc, ca) = simulate(self.seed, rounds);
        draw(canvas, ab, bc, ca, None);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "cycle dice",
            root: 174.61,
            tempo: 120,
            line: &[0, 5, 9, 5, 0, 7, 12, 5],
            encodes: "three win rates that refuse a total order",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: ROLL THE TRIO")
    }

    fn status(&self, t: f64) -> Option<String> {
        let ab = win_rate(&A, &B);
        let bc = win_rate(&B, &C);
        let ca = win_rate(&C, &A);
        let _ = t;
        Some(format!(
            "A>B={:.0}% B>C={:.0}% C>A={:.0}%  CLICK",
            ab * 100.0,
            bc * 100.0,
            ca * 100.0
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let rounds = 300 + hands.len() * 100 + (phase_unit(t) * 400.0) as usize;
        let (ab, bc, ca) = simulate(self.seed ^ hands.len() as u64, rounds);
        let mut rng = SplitMix64::new(SEED ^ self.seed ^ (hands.len() as u64).wrapping_mul(17));
        let show = (roll(&A, &mut rng), roll(&B, &mut rng), roll(&C, &mut rng));
        draw(canvas, ab, bc, ca, Some(show));
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.line(px - 2, py, px + 2, py, '+');
                canvas.line(px, py - 2, px, py + 2, '+');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let rounds = 400 + hands.len() * 120;
        let (ab, bc, ca) = simulate(self.seed ^ hands.len() as u64, rounds);
        Some(format!(
            "ROLL n={rounds}  {:.0}/{:.0}/{:.0}",
            ab * 100.0,
            bc * 100.0,
            ca * 100.0
        ))
    }

    fn reveal(&self) -> &'static str {
        "Nontransitive dice: pairwise majority can cycle. A beats B more than \
         half the time, B beats C, C beats A. Fairness is not a total order; \
         ranking collapses under comparison."
    }
}

#[cfg(test)]
mod tests {
    use super::{A, B, C, Nontransitive, win_rate};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Nontransitive::new().status(0.3).unwrap();
        assert!(s.contains("ROLL") || s.contains("A>B"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn roll_changes() {
        let r = Nontransitive::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.5,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn cycle_exists() {
        assert!(win_rate(&A, &B) > 0.5);
        assert!(win_rate(&B, &C) > 0.5);
        assert!(win_rate(&C, &A) > 0.5);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Nontransitive::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(Nontransitive::new().motif().unwrap().line.len() >= 6);
    }
}

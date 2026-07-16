//! Birthday paradox: collision probability among n random days.
//!
//! DRAG: TUNE N. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

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

fn n_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 { 0.0 } else { (seed % 10) as f64 };
    if let Some((x, _)) = hand {
        2.0 + x * 80.0 + s
    } else {
        5.0 + phase_unit(t) * 70.0 + s
    }
}

fn p_collision(n: u32, d: u32) -> f64 {
    if n <= 1 {
        return 0.0;
    }
    if n as u64 > d as u64 {
        return 1.0;
    }
    let mut prod = 1.0_f64;
    for k in 1..n {
        prod *= 1.0 - (k as f64) / (d as f64);
        if prod <= 0.0 {
            return 1.0;
        }
    }
    (1.0 - prod).clamp(0.0, 1.0)
}

fn draw(canvas: &mut dyn Surface, n_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let d = 365u32 + if seed == 0 { 0 } else { (seed % 5) as u32 * 10 };
    let n_mark = n_f.round().clamp(2.0, 100.0) as u32;
    let mut prev: Option<(i32, i32)> = None;
    let max_n = 80u32;
    for n in 1..=max_n {
        let p = p_collision(n, d);
        let x = ((n as f64 / max_n as f64) * width.saturating_sub(1) as f64).round() as i32;
        let y = ((1.0 - p) * height.saturating_sub(1) as f64 * 0.9 + height as f64 * 0.05).round()
            as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, x, y, '#');
        }
        prev = Some((x, y));
    }
    // 0.5 line and current n mark
    let y_half =
        ((1.0 - 0.5) * height.saturating_sub(1) as f64 * 0.9 + height as f64 * 0.05).round() as i32;
    canvas.line(0, y_half, width.saturating_sub(1) as i32, y_half, '.');
    let xm = ((n_mark as f64 / max_n as f64) * width.saturating_sub(1) as f64).round() as i32;
    canvas.line(xm, 0, xm, height.saturating_sub(1) as i32, '|');
}

/// Birthday paradox room.
#[derive(Debug, Default)]
pub struct Birthday {
    seed: u64,
}

impl Birthday {
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

impl Room for Birthday {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "birthday",
            title: "Birthday Paradox",
            wing: "Chance & Order",
            blurb: "Shared birthday odds grow faster than intuition. t and DRAG: TUNE N.",
            accent: [160, 60, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, n_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "birthday",
            root: 23.12,
            tempo: 90,
            line: &[0, 4, 7, 12, 9, 4, 0, 7],
            encodes: "birthday paradox: 23 people already pass 50 percent collision",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE N")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = n_param(t, None, self.seed).round() as u32;
        let p = p_collision(n, 365);
        let vs = if n >= 23 { ">=50%" } else { "<50%" };
        Some(format!("n={n}  p={p:.2}  {vs}  DRAG:N"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = n_param(t, hands.last().copied(), self.seed);
        draw(canvas, n, self.seed ^ hands.len() as u64);
        if let Some(&(x, y)) = hands.last() {
            let (bw, bh) = canvas.draw_bounds();
            if bw > 0 && bh > 0 {
                let px = (x * bw.saturating_sub(1) as f64).round() as i32;
                let py = (y * bh.saturating_sub(1) as f64).round() as i32;
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
        let n = n_param(t, hands.last().copied(), self.seed).round() as u32;
        let p = p_collision(n, 365);
        let pairs = n.saturating_sub(1) as u64 * n as u64 / 2;
        let flag = if p >= 0.5 { "over half" } else { "under half" };
        Some(format!("n={n}  p={p:.3}  {pairs} pairs  {flag}"))
    }

    fn reveal(&self) -> &'static str {
        "Among 23 people the chance two share a birthday already exceeds one half. \
         Pairwise comparisons grow like n^2, so collisions appear long before the \
         number of days: the birthday paradox is quadratic counting in disguise."
    }
}

#[cfg(test)]
mod tests {
    use super::{Birthday, p_collision};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn twenty_three_near_half() {
        let p = p_collision(23, 365);
        assert!(p > 0.5 && p < 0.55);
    }

    #[test]
    fn action_names_pairs_and_threshold() {
        let s = Birthday::new()
            .status_input(
                0.4,
                &[RoomInput::PointerDown {
                    x: 0.3,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert!(s.contains("pairs") || s.contains("half"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn status_invites() {
        let s = Birthday::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("p="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = Birthday::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
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
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Birthday::new().render(&mut c, 0.35);
        assert!(c.ink_count() > 0);
    }
}

//! Kolakoski sequence: self-describing run lengths of 1s and 2s.
//!
//! DRAG: TUNE LENGTH. See `docs/ROOMS.md`.

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

fn length(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 { 0.0 } else { (seed % 20) as f64 };
    if let Some((x, _)) = hand {
        20.0 + x * 100.0 + s
    } else {
        30.0 + phase_unit(t) * 80.0 + s
    }
}

/// Generate the first n terms of the Kolakoski sequence over {1,2}.
fn kolakoski(n: usize) -> Vec<u8> {
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![1];
    }
    if n == 2 {
        return vec![1, 2];
    }
    // Seed the unique prefix 1,2,2 then extend by run lengths.
    let mut a: Vec<u8> = Vec::with_capacity(n);
    a.push(1);
    a.push(2);
    a.push(2);
    let mut i = 2usize;
    while a.len() < n {
        let run = a[i] as usize;
        let next = if a.last() == Some(&1) { 2u8 } else { 1u8 };
        for _ in 0..run {
            if a.len() >= n {
                break;
            }
            a.push(next);
        }
        i += 1;
    }
    a.truncate(n);
    a
}

fn draw(canvas: &mut dyn Surface, n_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = n_f.round().clamp(8.0, 160.0) as usize;
    let seq = kolakoski(n);
    let y1 = (height as f64 * 0.35).round() as i32;
    let y2 = (height as f64 * 0.65).round() as i32;
    let mut prev: Option<(i32, i32)> = None;
    for (i, &v) in seq.iter().enumerate() {
        let x = ((i as f64 / n.saturating_sub(1).max(1) as f64) * width.saturating_sub(1) as f64)
            .round() as i32;
        let y = if v == 1 { y1 } else { y2 };
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, x, y, '#');
        }
        canvas.line(x, y - 1, x, y + 1, if v == 1 { '+' } else { '*' });
        prev = Some((x, y));
    }
    let _ = seed;
}

/// Kolakoski room.
#[derive(Debug, Default)]
pub struct Kolakoski {
    seed: u64,
}

impl Kolakoski {
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

impl Room for Kolakoski {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "kolakoski",
            title: "Kolakoski Sequence",
            wing: "Number & Pattern",
            blurb: "Self-describing runs of 1 and 2. t and DRAG: TUNE LENGTH.",
            accent: [80, 100, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, length(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "kolakoski",
            root: 51.91,
            tempo: 86,
            line: &[0, 2, 0, 5, 0, 7, 0, 12],
            encodes: "Kolakoski: run lengths of 1s and 2s describe the sequence itself",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE LENGTH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = length(t, None, self.seed).round();
        Some(format!("n={n:.0}  self  DRAG:LEN"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = length(t, hands.last().copied(), self.seed);
        draw(canvas, n, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = length(t, hands.last().copied(), self.seed).round() as usize;
        let seq = kolakoski(n);
        let ones = seq.iter().filter(|&&v| v == 1).count();
        let dens = ones as f64 / n.max(1) as f64;
        Some(format!("n={n}  dens1={dens:.2}  kol"))
    }

    fn reveal(&self) -> &'static str {
        "The Kolakoski sequence is made of 1s and 2s whose run lengths are the \
         sequence itself: 1,2,2,1,1,2,1,2,2,1,... It is a classic open-ended \
         combinatorial object; even the density of 1s is not fully settled."
    }
}

#[cfg(test)]
mod tests {
    use super::{Kolakoski, kolakoski};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn sequence_starts_correctly() {
        assert_eq!(kolakoski(10), vec![1, 2, 2, 1, 1, 2, 1, 2, 2, 1]);
    }

    #[test]
    fn status_invites() {
        let s = Kolakoski::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("self"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn length_changes() {
        let r = Kolakoski::new();
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
        Kolakoski::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

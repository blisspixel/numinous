//! Josephus circle: every k-th person is removed until one remains.
//!
//! Classic counting-out on a circle. DRAG: SET N AND K. See `docs/ROOMS.md`.

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

fn nk(t: f64, hand: Option<(f64, f64)>) -> (usize, usize) {
    if let Some((x, y)) = hand {
        let n = (8 + (x * 40.0) as usize).clamp(8, 48);
        let k = (2 + (y * 10.0) as usize).clamp(2, 12);
        (n, k)
    } else {
        let u = phase_unit(t);
        let n = (12 + (u * 24.0) as usize).clamp(8, 40);
        let k = (2 + ((1.0 - u) * 5.0) as usize).clamp(2, 8);
        (n, k)
    }
}

/// Elimination order (indices 0..n-1); last entry is the survivor.
fn josephus_order(n: usize, k: usize) -> Vec<usize> {
    let mut alive: Vec<usize> = (0..n).collect();
    let mut order = Vec::with_capacity(n);
    let mut idx = 0usize;
    while !alive.is_empty() {
        idx = (idx + k - 1) % alive.len();
        order.push(alive.remove(idx));
        if alive.is_empty() {
            break;
        }
        idx %= alive.len();
    }
    order
}

fn draw(canvas: &mut dyn Surface, n: usize, k: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || n == 0 {
        return;
    }
    let order = josephus_order(n, k);
    let cx = width as f64 * 0.5;
    let cy = height as f64 * 0.5;
    let r = (width.min(height) as f64 * 0.38).max(4.0);
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 20) as f64 * 0.05
    };
    // Positions on the circle.
    let mut pos = Vec::with_capacity(n);
    for i in 0..n {
        let a = rot - std::f64::consts::FRAC_PI_2 + std::f64::consts::TAU * i as f64 / n as f64;
        let x = cx + r * a.cos();
        let y = cy + r * a.sin();
        pos.push((x, y));
    }
    // Draw elimination chords in order, fading.
    let mut prev: Option<(i32, i32)> = None;
    for (step, &who) in order.iter().enumerate() {
        let (x, y) = pos[who];
        let px = x.round() as i32;
        let py = y.round() as i32;
        let ch = if step + 1 == n {
            '@'
        } else if step < n / 3 {
            '.'
        } else if step < 2 * n / 3 {
            '*'
        } else {
            '#'
        };
        canvas.plot(px, py, ch);
        if let Some(o) = prev {
            canvas.line(o.0, o.1, px, py, if step + 1 == n { '+' } else { '.' });
        }
        prev = Some((px, py));
    }
    // Circle outline sample.
    for i in 0..64 {
        let a = std::f64::consts::TAU * i as f64 / 64.0;
        let px = (cx + r * a.cos()).round() as i32;
        let py = (cy + r * a.sin()).round() as i32;
        canvas.plot(px, py, ':');
    }
}

/// Josephus problem room.
#[derive(Debug, Default)]
pub struct Josephus {
    seed: u64,
}

impl Josephus {
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

impl Room for Josephus {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "josephus",
            title: "Josephus Circle",
            wing: "Number & Pattern",
            blurb: "Every k-th seat is removed until one remains. t and DRAG: SET N AND K.",
            accent: [160, 40, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (n, k) = nk(t, None);
        draw(canvas, n, k, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "josephus",
            root: 110.0,
            tempo: 132,
            line: &[0, 7, 0, 7, 12, 0, 7, 12],
            encodes: "count k, remove, walk the ring until one",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET N AND K")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (n, k) = nk(t, None);
        let survivor = josephus_order(n, k).last().copied().unwrap_or(0) + 1;
        Some(format!("n={n} k={k}  win={survivor}  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (n, k) = nk(t, hands.last().copied());
        draw(canvas, n, k, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (n, k) = nk(t, hands.last().copied());
        let order = josephus_order(n, k);
        let survivor = order.last().copied().unwrap_or(0) + 1;
        // For k=2, f(n)=2l+1 where n=2^m+l.
        let closed = if k == 2 && n > 0 {
            let mut pow = 1usize;
            while pow * 2 <= n {
                pow *= 2;
            }
            2 * (n - pow) + 1
        } else {
            survivor
        };
        Some(format!("n={n} k={k}  f={survivor}  k2={closed}"))
    }

    fn reveal(&self) -> &'static str {
        "Josephus counts every k-th person around a circle until one remains. \
         For k=2 the survivor index has a closed form from the binary rotation \
         of n. The general case is recursive: f(1,k)=0, f(n,k)=(f(n-1,k)+k) mod n."
    }
}

#[cfg(test)]
mod tests {
    use super::{Josephus, josephus_order};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Josephus::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("n="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn nk_changes() {
        let r = Josephus::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.8,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn k2_power_of_two() {
        // For n=8, k=2, survivor is position 1 (0-based index 0).
        assert_eq!(*josephus_order(8, 2).last().unwrap(), 0);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Josephus::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Josephus::new().motif().unwrap().line.len() >= 6);
    }
}

//! 2D Ising model: spins on a lattice, heat-bath style Monte Carlo.
//!
//! DRAG: TUNE TEMPERATURE. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const STEPS: usize = 2_000;

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

fn temp(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    // Tc ~ 2.269 for square 2D Ising (J=1, kB=1)
    if let Some((x, _)) = hand {
        0.5 + x * 4.0 + s
    } else {
        1.0 + phase_unit(t) * 2.5 + s
    }
}

fn draw(canvas: &mut dyn Surface, temperature: f64, seed: u64) -> f64 {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return 0.0;
    }
    let w = width.min(48);
    let h = height.min(24);
    let n = w * h;
    let mut spins = vec![1i8; n];
    let mut state = seed ^ 0x1515_1515_1515_1515;
    let mut next_u = || {
        state = state
            .wrapping_mul(0x5851_f42d_4c95_7f2d)
            .wrapping_add(0x1405_7b7e_f767_814f);
        (state >> 33) as f64 / (u32::MAX as f64)
    };
    for s in &mut spins {
        *s = if next_u() < 0.5 { 1 } else { -1 };
    }
    let beta = 1.0 / temperature.max(0.05);
    for _ in 0..STEPS {
        let i = (next_u() * n as f64) as usize % n;
        let x = i % w;
        let y = i / w;
        let mut nn = 0i32;
        for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = (x as i32 + dx).rem_euclid(w as i32) as usize;
            let ny = (y as i32 + dy).rem_euclid(h as i32) as usize;
            nn += i32::from(spins[ny * w + nx]);
        }
        let s = i32::from(spins[i]);
        let d_e = 2 * s * nn;
        if d_e <= 0 || next_u() < (-f64::from(d_e) * beta).exp() {
            spins[i] = -spins[i];
        }
    }
    let mag: f64 = spins.iter().map(|&s| f64::from(s)).sum::<f64>() / n as f64;
    for y in 0..height {
        for x in 0..width {
            let gx = (x * w / width.max(1)).min(w - 1);
            let gy = (y * h / height.max(1)).min(h - 1);
            let s = spins[gy * w + gx];
            let ch = if s > 0 { '#' } else { '.' };
            canvas.plot(x as i32, y as i32, ch);
        }
    }
    mag.abs()
}

/// Ising model room.
#[derive(Debug, Default)]
pub struct Ising {
    seed: u64,
}

impl Ising {
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

impl Room for Ising {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "ising",
            title: "Ising Lattice",
            wing: "Chance & Order",
            blurb: "Spins freeze or melt across a critical heat. t and DRAG: TUNE TEMPERATURE.",
            accent: [180, 40, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let _ = draw(canvas, temp(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "ising",
            root: 138.59,
            tempo: 70,
            line: &[0, 0, 7, 7, 12, 12, 7, 0],
            encodes: "neighbor spins agree until heat wins",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE TEMPERATURE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let te = temp(t, None, self.seed);
        Some(format!("T={te:.2}  ising  DRAG:TEMP"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let te = temp(t, hands.last().copied(), self.seed);
        let _ = draw(canvas, te, self.seed ^ hands.len() as u64);
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
        let te = temp(t, hands.last().copied(), self.seed);
        let tc = 2.0 * (1.0_f64 + 2.0_f64.sqrt()).ln(); // Onsager Tc ~ 2.269
        Some(format!("T={te:.3}  Tc~{tc:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The 2D Ising model is spins that prefer neighbors of the same sign. \
         Onsager solved its free energy: below Tc ~ 2.269 a spontaneous \
         magnetization appears; above it the lattice melts into disorder."
    }
}

#[cfg(test)]
mod tests {
    use super::Ising;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Ising::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TEMP"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn temp_changes() {
        let r = Ising::new();
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
        Ising::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 0);
    }
}

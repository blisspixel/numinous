//! Kuramoto oscillators: phase sync on a ring of coupled clocks.
//!
//! DRAG: TUNE COUPLING. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const N: usize = 48;
const STEPS: usize = 200;
const DT: f64 = 0.05;

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

fn coupling(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        x * 4.0 + s
    } else {
        phase_unit(t) * 3.0 + s
    }
}

fn order_param(phases: &[f64]) -> f64 {
    let mut sx = 0.0;
    let mut sy = 0.0;
    let n = phases.len() as f64;
    for &th in phases {
        sx += th.cos();
        sy += th.sin();
    }
    (sx * sx + sy * sy).sqrt() / n
}

fn draw(canvas: &mut dyn Surface, k: f64, seed: u64) -> f64 {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return 0.0;
    }
    let mut rng_state = seed ^ 0x9e37_79b9_7f4a_7c15;
    let mut next_u = || {
        rng_state = rng_state
            .wrapping_mul(0x5851_f42d_4c95_7f2d)
            .wrapping_add(0x1405_7b7e_f767_814f);
        (rng_state >> 33) as f64 / (u32::MAX as f64)
    };
    let mut phases: Vec<f64> = (0..N).map(|_| next_u() * std::f64::consts::TAU).collect();
    let omegas: Vec<f64> = (0..N)
        .map(|i| {
            let base = -1.0 + 2.0 * (i as f64 / (N - 1) as f64);
            base + (next_u() - 0.5) * 0.2
        })
        .collect();
    for _ in 0..STEPS {
        let mut next = phases.clone();
        for i in 0..N {
            let mut sum = 0.0;
            for j in 0..N {
                sum += (phases[j] - phases[i]).sin();
            }
            next[i] = phases[i] + DT * (omegas[i] + (k / N as f64) * sum);
        }
        phases = next;
    }
    let r = order_param(&phases);
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let rad = (width.min(height) as f64) * 0.4;
    // Unit circle guide (dense stroke).
    let mut prev_c: Option<(i32, i32)> = None;
    for i in 0..=96 {
        let th = std::f64::consts::TAU * (i as f64 / 96.0);
        let px = (cx + rad * th.cos()).round() as i32;
        let py = (cy - rad * th.sin()).round() as i32;
        if let Some(o) = prev_c {
            canvas.line(o.0, o.1, px, py, '.');
        }
        prev_c = Some((px, py));
    }
    for &th in &phases {
        let px = (cx + rad * th.cos()).round() as i32;
        let py = (cy - rad * th.sin()).round() as i32;
        let ch = if r > 0.6 { '#' } else { '*' };
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx * dx + dy * dy <= 2 {
                    canvas.plot(px + dx, py + dy, ch);
                }
            }
        }
    }
    // Order vector (mean phase).
    let mut sx = 0.0;
    let mut sy = 0.0;
    for &th in &phases {
        sx += th.cos();
        sy += th.sin();
    }
    sx /= N as f64;
    sy /= N as f64;
    let ox = (cx + rad * sx).round() as i32;
    let oy = (cy - rad * sy).round() as i32;
    canvas.line(cx.round() as i32, cy.round() as i32, ox, oy, '+');
    canvas.line(cx.round() as i32, cy.round() as i32 + 1, ox, oy + 1, '*');
    r
}

/// Kuramoto sync room.
#[derive(Debug, Default)]
pub struct Kuramoto {
    seed: u64,
}

impl Kuramoto {
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

impl Room for Kuramoto {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "kuramoto",
            title: "Kuramoto Sync",
            wing: "Motion & Dynamics",
            blurb: "Coupled phase clocks find a shared beat. t and DRAG: TUNE COUPLING.",
            accent: [40, 160, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let _ = draw(canvas, coupling(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.7
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "kuramoto",
            root: 311.13,
            tempo: 84,
            line: &[0, 5, 0, 5, 7, 12, 7, 5],
            encodes: "phase oscillators locking into one order parameter",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE COUPLING")
    }

    fn status(&self, t: f64) -> Option<String> {
        let k = coupling(t, None, self.seed);
        Some(format!("K={k:.2}  kuramoto  DRAG:K"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let k = coupling(t, hands.last().copied(), self.seed);
        let _ = draw(canvas, k, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let k = coupling(t, hands.last().copied(), self.seed);
        // Recompute r for status by a cheap second run on tiny canvas path
        // Use analytic-ish proxy: stronger K raises expected order
        let r_est = (1.0 - (-k * 0.4).exp()).clamp(0.0, 1.0);
        Some(format!("K={k:.3}  R~{r_est:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Kuramoto's model is the textbook of sync: oscillators with private \
         natural frequencies couple through sine of phase difference. Above a \
         critical K they lock; the order parameter R rises from zero to one."
    }
}

#[cfg(test)]
mod tests {
    use super::Kuramoto;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Kuramoto::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("K"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn k_changes() {
        let r = Kuramoto::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.95,
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
        Kuramoto::new().render(&mut c, 0.7);
        assert!(c.ink_count() > 0);
    }
}

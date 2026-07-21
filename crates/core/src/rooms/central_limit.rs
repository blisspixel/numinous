//! Central limit theorem: sample means converge to a Gaussian bell.
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
    let s = if seed == 0 { 0.0 } else { (seed % 5) as f64 };
    if let Some((x, _)) = hand {
        1.0 + x * 40.0 + s
    } else {
        2.0 + phase_unit(t) * 30.0 + s
    }
}

fn next_u01(state: &mut u64) -> f64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    ((*state >> 33) as f64) / ((1u64 << 31) as f64)
}

fn draw(canvas: &mut dyn Surface, n_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n = n_f.round().clamp(1.0, 50.0) as usize;
    let samples = 2000usize;
    let bins = width.clamp(16, 64);
    let mut hist = vec![0u32; bins];
    let mut state = if seed == 0 {
        0x1234_5678_9abc_def0
    } else {
        seed
    };
    // Sample means of n uniform[0,1] draws, standardized roughly.
    let mu = 0.5;
    let var = 1.0 / 12.0;
    let sigma = (var / n as f64).sqrt();
    for _ in 0..samples {
        let mut s = 0.0;
        for _ in 0..n {
            s += next_u01(&mut state);
        }
        let mean = s / n as f64;
        let z = (mean - mu) / sigma.max(1e-9);
        // map z in [-3,3] to bins
        let u = ((z + 3.0) / 6.0).clamp(0.0, 0.999);
        let b = (u * bins as f64).floor() as usize;
        hist[b] += 1;
    }
    let max_c = *hist.iter().max().unwrap_or(&1).max(&1) as f64;
    let base = height.saturating_sub(2) as i32;
    for (i, &c) in hist.iter().enumerate() {
        let x = ((i as f64 / bins as f64) * width.saturating_sub(1) as f64).round() as i32;
        let h = ((c as f64 / max_c) * height.saturating_sub(2) as f64 * 0.9).round() as i32;
        canvas.line(x, base, x, base - h, '#');
    }
    // Gaussian overlay
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let u = col as f64 / width.saturating_sub(1).max(1) as f64;
        let z = -3.0 + 6.0 * u;
        let g = (-0.5 * z * z).exp();
        let y = base - (g * height.saturating_sub(2) as f64 * 0.9).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, y, '.');
        }
        prev = Some((col as i32, y));
    }
}

/// Central limit theorem room.
#[derive(Debug, Default)]
pub struct CentralLimit {
    seed: u64,
}

impl CentralLimit {
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

impl Room for CentralLimit {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "central-limit",
            title: "Central Limit",
            wing: "Chance & Order",
            blurb: "Means of uniforms become a bell as n grows. t and DRAG: TUNE N.",
            accent: [60, 100, 60],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, n_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "central-limit",
            root: 14.57,
            tempo: 94,
            line: &[0, 4, 7, 12, 7, 4, 0, 7],
            encodes: "CLT: standardized sample means converge in law to N(0,1)",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE N")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = n_param(t, None, self.seed).round().max(1.0);
        // Uniform[0,1] mean 0.5, var 1/12; SE of mean = 1/sqrt(12 n)
        let se = (1.0 / (12.0 * n)).sqrt();
        Some(format!("n={n:.0}  SE~{se:.3}  DRAG:N"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = n_param(t, hands.last().copied(), self.seed);
        let seed = self.seed
            ^ hands
                .last()
                .map(|&(x, y)| ((x * 1e6) as u64) ^ ((y * 1e6) as u64))
                .unwrap_or(0)
            ^ hands.len() as u64;
        draw(canvas, n, seed);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = n_param(t, hands.last().copied(), self.seed)
            .round()
            .max(1.0);
        let se = (1.0 / (12.0 * n)).sqrt();
        let se1 = (1.0_f64 / 12.0).sqrt();
        let shrink = se1 / se;
        Some(format!("n={n:.0}  SE={se:.3}  ~{shrink:.1}x tighter"))
    }

    fn reveal(&self) -> &'static str {
        "The central limit theorem says averages of many independent variables, \
         properly scaled, look Gaussian no matter the parent law (with mild \
         conditions). That is why measurement noise so often draws a bell."
    }
}

#[cfg(test)]
mod tests {
    use super::CentralLimit;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = CentralLimit::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("mean"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = CentralLimit::new();
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
        CentralLimit::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 0);
    }
}

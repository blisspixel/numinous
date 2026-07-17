//! Mackey-Glass delayed feedback: classic infinite-dimensional chaos toy.
//!
//! DRAG: TUNE DELAY. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const HIST: usize = 200;
const STEPS: usize = 1_200;
const DT: f64 = 0.1;

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

fn delay_steps(t: f64, hand: Option<(f64, f64)>, seed: u64) -> usize {
    let s = if seed == 0 { 0 } else { (seed % 5) as usize };
    if let Some((x, _)) = hand {
        (10 + (x * 40.0) as usize + s).clamp(8, 60)
    } else {
        (15 + (phase_unit(t) * 30.0) as usize + s).clamp(8, 55)
    }
}

fn draw(canvas: &mut dyn Surface, tau: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let beta = 0.2;
    let gamma = 0.1;
    let n = 10.0;
    let mut buf = vec![
        0.9 + if seed == 0 {
            0.0
        } else {
            (seed % 7) as f64 * 0.01
        };
        HIST.max(tau + 2)
    ];
    let mut idx = 0usize;
    let mut series = Vec::with_capacity(STEPS);
    for _ in 0..STEPS {
        let x = buf[idx];
        let x_tau = buf[(idx + buf.len() - tau) % buf.len()];
        let dx = beta * x_tau / (1.0 + x_tau.powf(n)) - gamma * x;
        let nx = (x + dx * DT).max(0.0);
        idx = (idx + 1) % buf.len();
        buf[idx] = nx;
        series.push(nx);
    }
    let min_v = series.iter().cloned().fold(f64::MAX, f64::min);
    let max_v = series.iter().cloned().fold(f64::MIN, f64::max);
    let dv = (max_v - min_v).max(1e-6);
    // time series
    let mut prev: Option<(i32, i32)> = None;
    for (i, &v) in series.iter().enumerate() {
        let u = i as f64 / (series.len() - 1).max(1) as f64;
        let px = (u * width.saturating_sub(1) as f64).round() as i32;
        let py = ((1.0 - (v - min_v) / dv) * height.saturating_sub(1) as f64).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '*');
        }
        prev = Some((px, py));
    }
    // delay embedding (x(t), x(t-tau))
    for i in tau..series.len() {
        let x = series[i];
        let y = series[i - tau];
        let u = ((x - min_v) / dv).clamp(0.0, 1.0);
        let v = ((y - min_v) / dv).clamp(0.0, 1.0);
        let ix = ((0.55 + 0.4 * u) * width.saturating_sub(1) as f64).round() as i32;
        let iy = ((1.0 - v) * height.saturating_sub(1) as f64 * 0.9 + height as f64 * 0.05).round()
            as i32;
        canvas.plot(ix, iy, '#');
    }
}

/// Mackey-Glass room.
#[derive(Debug, Default)]
pub struct MackeyGlass {
    seed: u64,
}

impl MackeyGlass {
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

impl Room for MackeyGlass {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "mackey-glass",
            title: "Mackey-Glass",
            wing: "Motion & Dynamics",
            blurb: "Delayed feedback births a strange attractor. t and DRAG: TUNE DELAY.",
            accent: [40, 140, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, delay_steps(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "mackey glass",
            root: 61.74,
            tempo: 64,
            line: &[0, 3, 5, 8, 12, 8, 5, 3],
            encodes: "blood-cell delay equation as infinite-dimensional chaos",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE DELAY")
    }

    fn status(&self, t: f64) -> Option<String> {
        let tau = delay_steps(t, None, self.seed);
        Some(format!("tau={tau}  MG  DRAG:DELAY"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let tau = delay_steps(t, hands.last().copied(), self.seed);
        draw(canvas, tau, self.seed ^ hands.len() as u64);
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
        let tau = delay_steps(t, hands.last().copied(), self.seed);
        // Delay-embedding dimension hint: tau steps of history.
        Some(format!("tau={tau}  delay emb"))
    }

    fn reveal(&self) -> &'static str {
        "The Mackey-Glass equation models blood cell production with a delay. \
         That delay turns a simple ODE into an infinite-dimensional system that \
         can oscillate or wander on a strange attractor."
    }
}

#[cfg(test)]
mod tests {
    use super::MackeyGlass;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = MackeyGlass::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("DELAY") || s.contains("tau"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn delay_changes() {
        let r = MackeyGlass::new();
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
        MackeyGlass::new().render(&mut c, 0.6);
        assert!(c.ink_count() > 0);
    }
}

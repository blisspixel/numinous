//! Heat kernel: Gaussian fundamental solution of the heat equation.
//!
//! DRAG: TUNE TIME. See `docs/ROOMS.md`.

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

fn time_p(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.05 + x * 1.5 + s
    } else {
        0.08 + phase_unit(t) * 1.2 + s
    }
}

fn draw(canvas: &mut dyn Surface, tau: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let tau = tau.clamp(0.04, 2.0);
    // 1D heat kernel K(x,t) = (4 pi t)^{-1/2} exp(-x^2/(4t))
    let amp = 1.0 / (4.0 * std::f64::consts::PI * tau).sqrt();
    let y0 = height as f64 * 0.9;
    let y_scale = height as f64 * 0.75 / amp.max(0.1);
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let x = -3.0 + 6.0 * (col as f64) / width.saturating_sub(1).max(1) as f64;
        let k = amp * (-x * x / (4.0 * tau)).exp();
        let py = (y0 - k * y_scale).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
        }
        prev = Some((col as i32, py));
    }
    // sigma tick: width ~ sqrt(2 tau)
    let sig = (2.0 * tau).sqrt();
    let px = (((sig + 3.0) / 6.0) * width.saturating_sub(1) as f64).round() as i32;
    canvas.line(
        px,
        height.saturating_sub(3) as i32,
        px,
        height.saturating_sub(1) as i32,
        '|',
    );
    let px2 = (((-sig + 3.0) / 6.0) * width.saturating_sub(1) as f64).round() as i32;
    canvas.line(
        px2,
        height.saturating_sub(3) as i32,
        px2,
        height.saturating_sub(1) as i32,
        '|',
    );
    let _ = seed;
}

/// Heat kernel room.
#[derive(Debug, Default)]
pub struct HeatKernel {
    seed: u64,
}

impl HeatKernel {
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

impl Room for HeatKernel {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "heat-kernel",
            title: "Heat Kernel",
            wing: "Change",
            blurb: "Gaussian spreads as sqrt(t). t and DRAG: TUNE TIME.",
            accent: [200, 80, 30],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, time_p(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.4
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "heat-kernel",
            root: 123.47,
            tempo: 94,
            line: &[0, 4, 7, 9, 12, 7, 4, 0],
            encodes: "heat kernel: Gaussian width grows like square root of time",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE TIME")
    }

    fn status(&self, t: f64) -> Option<String> {
        let tau = time_p(t, None, self.seed);
        let sig = (2.0 * tau).sqrt();
        Some(format!("t={tau:.2}  s={sig:.2}  DRAG:T"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let tau = time_p(t, hands.last().copied(), self.seed);
        draw(canvas, tau, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let tau = time_p(t, hands.last().copied(), self.seed);
        // Heat kernel peak density (4 pi t)^-1/2 in 1D; width sqrt(2t).
        let sig = (2.0 * tau).sqrt();
        let peak = if tau > 1e-9 {
            (4.0 * std::f64::consts::PI * tau).sqrt().recip()
        } else {
            0.0
        };
        Some(format!("t={tau:.2}  s={sig:.2}  peak={peak:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The fundamental solution of the heat equation is a Gaussian whose \
         variance grows like time. An instantaneous hot spot spreads with width \
         proportional to sqrt(t) while the peak falls as 1/sqrt(t)."
    }
}

#[cfg(test)]
mod tests {
    use super::HeatKernel;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = HeatKernel::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("s="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn time_changes() {
        let r = HeatKernel::new();
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
        HeatKernel::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 0);
    }
}

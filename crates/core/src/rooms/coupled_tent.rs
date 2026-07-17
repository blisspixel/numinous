//! Coupled tent maps: two tents with weak coupling (toy synchronization).
//!
//! DRAG: TUNE COUPLING. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const STEPS: usize = 300;

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
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        x * 0.5 + s
    } else {
        phase_unit(t) * 0.35 + s
    }
}

fn tent(x: f64) -> f64 {
    2.0 * x.min(1.0 - x)
}

fn draw(canvas: &mut dyn Surface, eps: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let mut x = if seed == 0 {
        0.2
    } else {
        0.1 + (seed % 30) as f64 * 0.01
    };
    let mut y = 0.7;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..STEPS {
        let nx = (1.0 - eps) * tent(x) + eps * tent(y);
        let ny = (1.0 - eps) * tent(y) + eps * tent(x);
        x = nx.clamp(0.0, 1.0);
        y = ny.clamp(0.0, 1.0);
        let px = (x * width.saturating_sub(1) as f64).round() as i32;
        let py = ((1.0 - y) * height.saturating_sub(1) as f64).round() as i32;
        if let Some(o) = prev {
            canvas.line(o.0, o.1, px, py, if i + 40 > STEPS { '#' } else { '*' });
        }
        prev = Some((px, py));
    }
    // Sync diagonal
    canvas.line(
        0,
        height.saturating_sub(1) as i32,
        width.saturating_sub(1) as i32,
        0,
        '.',
    );
}

/// Coupled tent maps room.
#[derive(Debug, Default)]
pub struct CoupledTent {
    seed: u64,
}

impl CoupledTent {
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

impl Room for CoupledTent {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "coupled-tent",
            title: "Coupled Tents",
            wing: "Motion & Dynamics",
            blurb: "Two tent maps with coupling: sync or independent chaos. t and DRAG: TUNE \
                    COUPLING.",
            accent: [40, 160, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, coupling(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "coupled tent",
            root: 196.0,
            tempo: 112,
            line: &[0, 5, 0, 7, 0, 12, 5, 7],
            encodes: "two expanding maps learning to lock",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE COUPLING")
    }

    fn status(&self, t: f64) -> Option<String> {
        let e = coupling(t, None, self.seed);
        Some(format!("eps={e:.2}  couple  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let e = coupling(t, hands.last().copied(), self.seed);
        draw(canvas, e, self.seed ^ hands.len() as u64);
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
        let e = coupling(t, hands.last().copied(), self.seed);
        let mut x = if self.seed == 0 {
            0.2
        } else {
            0.1 + (self.seed % 30) as f64 * 0.01
        };
        let mut y = 0.7_f64;
        // Burn-in then mean |x-y| as a sync residual.
        for _ in 0..80 {
            let nx = (1.0 - e) * tent(x) + e * tent(y);
            let ny = (1.0 - e) * tent(y) + e * tent(x);
            x = nx.clamp(0.0, 1.0);
            y = ny.clamp(0.0, 1.0);
        }
        let mut sum = 0.0_f64;
        let mut n = 0usize;
        for _ in 0..200 {
            let nx = (1.0 - e) * tent(x) + e * tent(y);
            let ny = (1.0 - e) * tent(y) + e * tent(x);
            x = nx.clamp(0.0, 1.0);
            y = ny.clamp(0.0, 1.0);
            sum += (x - y).abs();
            n += 1;
        }
        let mean_d = if n > 0 { sum / n as f64 } else { 0.0 };
        let label = if mean_d < 0.05 {
            "sync"
        } else if mean_d < 0.2 {
            "near"
        } else {
            "free"
        };
        Some(format!("eps={e:.2}  |x-y|={mean_d:.3}  {label}"))
    }

    fn reveal(&self) -> &'static str {
        "Coupled chaotic maps can synchronize when the coupling is strong enough. \
         Two tent maps share a diagonal of perfect lock; below the threshold they \
         wander independently in the square."
    }
}

#[cfg(test)]
mod tests {
    use super::CoupledTent;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = CoupledTent::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = CoupledTent::new();
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
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        CoupledTent::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(CoupledTent::new().motif().unwrap().line.len() >= 6);
    }
}

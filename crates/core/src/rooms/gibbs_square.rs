//! Gibbs phenomenon on a partial-sum square wave (distinct from fourier-square room).
//!
//! DRAG: SET HARMONICS. See `docs/ROOMS.md`.

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

fn harmonics(t: f64, hand: Option<(f64, f64)>) -> usize {
    if let Some((x, _)) = hand {
        (1 + (x * 24.0) as usize).clamp(1, 28)
    } else {
        (2 + (phase_unit(t) * 18.0) as usize).clamp(1, 24)
    }
}

fn square_partial(x: f64, n: usize) -> f64 {
    // (4/pi) sum_{k odd} sin(k pi x) / k
    let mut s = 0.0;
    let mut k = 1usize;
    let mut count = 0usize;
    while count < n {
        s += (k as f64 * std::f64::consts::PI * x).sin() / k as f64;
        k += 2;
        count += 1;
    }
    4.0 * s / std::f64::consts::PI
}

fn draw(canvas: &mut dyn Surface, n: usize, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cy = (height.saturating_sub(1) / 2) as f64;
    let amp = height as f64 * 0.38;
    let j = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.01
    };
    // ideal square outline
    for col in 0..width {
        let x = col as f64 / width.saturating_sub(1).max(1) as f64 + j;
        let ideal = if (x.fract() * 2.0).floor() as i32 % 2 == 0 {
            1.0
        } else {
            -1.0
        };
        let py = (cy - amp * ideal).round() as i32;
        canvas.plot(col as i32, py, '.');
    }
    // partial sum
    let mut prev: Option<(i32, i32)> = None;
    for col in 0..width {
        let x = col as f64 / width.saturating_sub(1).max(1) as f64 + j;
        let y = square_partial(x.fract(), n).clamp(-1.5, 1.5);
        let py = (cy - amp * y / 1.2).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, col as i32, py, '#');
        }
        prev = Some((col as i32, py));
    }
}

/// Gibbs square-wave room.
#[derive(Debug, Default)]
pub struct GibbsSquare {
    seed: u64,
}

impl GibbsSquare {
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

impl Room for GibbsSquare {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "gibbs-square",
            title: "Gibbs Square",
            wing: "Waves & Sound",
            blurb: "Fourier square partials overshoot at jumps. t and DRAG: SET HARMONICS.",
            accent: [200, 80, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, harmonics(t, None), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "gibbs square",
            root: 392.0,
            tempo: 88,
            line: &[0, 7, 0, 7, 12, 7, 0, 12],
            encodes: "odd harmonics that never kill the edge overshoot",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET HARMONICS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = harmonics(t, None);
        Some(format!("n={n}  gibbs  DRAG:N"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = harmonics(t, hands.last().copied());
        draw(canvas, n, self.seed ^ hands.len() as u64);
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
        let n = harmonics(t, hands.last().copied());
        // Gibbs overshoot ~ 9% of jump
        Some(format!("N={n}  overshoot~9%"))
    }

    fn reveal(&self) -> &'static str {
        "A square wave's Fourier partial sums overshoot near every jump by a \
         fixed fraction of the discontinuity. Adding terms sharpens the edge \
         but never kills the Gibbs horns."
    }
}

#[cfg(test)]
mod tests {
    use super::GibbsSquare;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = GibbsSquare::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("gibbs"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn n_changes() {
        let r = GibbsSquare::new();
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
        GibbsSquare::new().render(&mut c, 0.6);
        assert!(c.ink_count() > 0);
    }
}

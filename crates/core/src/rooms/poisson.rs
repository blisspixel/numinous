//! Poisson process: exponential waits, count staircase N(t).
//!
//! DRAG: TUNE RATE. See `docs/ROOMS.md`.

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

fn rate(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.3
    };
    if let Some((x, _)) = hand {
        0.5 + x * 8.0 + s
    } else {
        1.0 + phase_unit(t) * 6.0 + s
    }
}

/// Deterministic LCG for replayable interarrival draws from seed.
fn next_u01(state: &mut u64) -> f64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
    ((*state >> 33) as f64) / ((1u64 << 31) as f64)
}

fn draw(canvas: &mut dyn Surface, lam: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let lam = lam.clamp(0.5, 12.0);
    let mut state = if seed == 0 {
        0x9e37_79b9_7f4a_7c15
    } else {
        seed ^ 0xdead_beef_cafe_babe
    };
    // Unit time horizon; N(t) staircase (double stroke so large plates read).
    let mut t = 0.0_f64;
    let mut n = 0u32;
    let mut prev_x = 0i32;
    let mut prev_y = height.saturating_sub(2) as i32;
    let t_max = 1.0;
    let n_scale = height as f64 * 0.85 / (lam * t_max * 2.2).max(4.0);
    while t < t_max {
        let u = next_u01(&mut state).clamp(1e-12, 1.0 - 1e-12);
        let wait = -u.ln() / lam;
        t += wait;
        if t > t_max {
            break;
        }
        n += 1;
        let x = ((t / t_max) * width.saturating_sub(1) as f64).round() as i32;
        let y = (height as f64 - 2.0 - n as f64 * n_scale)
            .round()
            .clamp(1.0, height.saturating_sub(2) as f64) as i32;
        // Horizontal then vertical step, doubled for density.
        canvas.line(prev_x, prev_y, x, prev_y, '#');
        canvas.line(prev_x, prev_y - 1, x, prev_y - 1, '*');
        canvas.line(x, prev_y, x, y, '*');
        canvas.line(x + 1, prev_y, x + 1, y, '.');
        // Event blot at the jump.
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx * dx + dy * dy <= 2 {
                    canvas.plot(x + dx, y + dy, 'o');
                }
            }
        }
        prev_x = x;
        prev_y = y;
    }
    canvas.line(prev_x, prev_y, width.saturating_sub(1) as i32, prev_y, '#');
    canvas.line(
        prev_x,
        prev_y - 1,
        width.saturating_sub(1) as i32,
        prev_y - 1,
        '*',
    );
    // Rate meter.
    let meter_y = height.saturating_sub(1) as i32;
    let filled = ((lam / 12.0).clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32;
    canvas.line(0, meter_y, width.saturating_sub(1) as i32, meter_y, '-');
    if filled > 0 {
        canvas.line(0, meter_y, filled, meter_y, '=');
    }
}

/// Poisson process room.
#[derive(Debug, Default)]
pub struct Poisson {
    seed: u64,
}

impl Poisson {
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

impl Room for Poisson {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "poisson",
            title: "Poisson Process",
            wing: "Chance & Order",
            blurb: "Exponential waits make a count staircase. t and DRAG: TUNE RATE.",
            accent: [40, 120, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, rate(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "poisson",
            root: 25.96,
            tempo: 82,
            line: &[0, 5, 0, 7, 0, 12, 0, 5],
            encodes: "Poisson process: independent exponential waits, N(t) staircase",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE RATE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let lam = rate(t, None, self.seed);
        Some(format!("lam={lam:.2}  N(t)  DRAG:RATE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let lam = rate(t, hands.last().copied(), self.seed);
        draw(canvas, lam, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let lam = rate(t, hands.last().copied(), self.seed).clamp(0.3, 12.0);
        // Poisson process: E[N(1)] = lam, mean interarrival 1/lam.
        let mean_wait = 1.0 / lam;
        // Match render_poked seed variation so realized n agrees with the draw.
        let seed = self.seed ^ hands.len() as u64;
        let mut state = if seed == 0 {
            0x9e37_79b9_7f4a_7c15
        } else {
            seed ^ 0xdead_beef_cafe_babe
        };
        let mut t_acc = 0.0_f64;
        let mut n = 0u32;
        while t_acc < 1.0 {
            let u = next_u01(&mut state).clamp(1e-12, 1.0 - 1e-12);
            t_acc += -u.ln() / lam;
            if t_acc > 1.0 {
                break;
            }
            n += 1;
        }
        Some(format!(
            "lam={lam:.1}  E[N]={lam:.1}  n={n}  wait={mean_wait:.2}"
        ))
    }

    fn reveal(&self) -> &'static str {
        "A Poisson process fires with independent exponential waiting times. The \
         count N(t) is a random staircase whose mean and variance both equal \
         lambda t; it models rare events, arrivals, and radioactive clicks."
    }
}

#[cfg(test)]
mod tests {
    use super::Poisson;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Poisson::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("N(t)"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn rate_changes() {
        let r = Poisson::new();
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
        Poisson::new().render(&mut c, 0.45);
        assert!(c.ink_count() > 0);
    }
}

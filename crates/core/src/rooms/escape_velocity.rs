//! Escape velocity: v = sqrt(2GM/r) vs circular orbit speed.
//!
//! DRAG: TUNE R. See `docs/ROOMS.md`.

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

fn radius(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.2 + x * 2.5 + s
    } else {
        0.4 + phase_unit(t) * 2.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, r: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let r = r.clamp(0.15, 3.0);
    // Plot v_esc(r) and v_circ(r) = v_esc/sqrt(2)
    let mut prev_e: Option<(i32, i32)> = None;
    let mut prev_c: Option<(i32, i32)> = None;
    let r_max = 3.0;
    let v_max = (2.0 / 0.15_f64).sqrt(); // scale
    for col in 0..width {
        let rr = 0.15 + (r_max - 0.15) * (col as f64) / width.saturating_sub(1).max(1) as f64;
        let ve = (2.0 / rr).sqrt();
        let vc = ve / std::f64::consts::SQRT_2;
        let ye = ((1.0 - ve / v_max) * height.saturating_sub(1) as f64 * 0.9 + height as f64 * 0.05)
            .round() as i32;
        let yc = ((1.0 - vc / v_max) * height.saturating_sub(1) as f64 * 0.9 + height as f64 * 0.05)
            .round() as i32;
        if let Some((ox, oy)) = prev_e {
            canvas.line(ox, oy, col as i32, ye, '#');
        }
        if let Some((ox, oy)) = prev_c {
            canvas.line(ox, oy, col as i32, yc, '.');
        }
        prev_e = Some((col as i32, ye));
        prev_c = Some((col as i32, yc));
    }
    let xu = (((r - 0.15) / (r_max - 0.15)).clamp(0.0, 1.0) * width.saturating_sub(1) as f64)
        .round() as i32;
    canvas.line(xu, 0, xu, height.saturating_sub(1) as i32, '|');
    let _ = seed;
}

/// Escape velocity room.
#[derive(Debug, Default)]
pub struct EscapeVelocity {
    seed: u64,
}

impl EscapeVelocity {
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

impl Room for EscapeVelocity {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "escape-velocity",
            title: "Escape Velocity",
            wing: "Motion & Dynamics",
            blurb: "v_esc = sqrt(2GM/r); circular is slower by sqrt(2). t and DRAG: TUNE R.",
            accent: [50, 50, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, radius(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "escape-velocity",
            root: 7.72,
            tempo: 88,
            line: &[0, 5, 7, 10, 12, 10, 7, 5],
            encodes: "escape speed is circular times root two from energy zero",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE R")
    }

    fn status(&self, t: f64) -> Option<String> {
        let r = radius(t, None, self.seed);
        let ve = (2.0 / r).sqrt();
        Some(format!("r={r:.2}  ve={ve:.2}  DRAG:R"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let r = radius(t, hands.last().copied(), self.seed);
        draw(canvas, r, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let r = radius(t, hands.last().copied(), self.seed);
        let ve = (2.0 / r).sqrt();
        let circ = (1.0 / r).sqrt(); // circular orbit speed scale
        let ratio = ve / circ.max(1e-9);
        Some(format!("ve={ve:.2}  vcirc={circ:.2}  ve/vc={ratio:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Escape velocity is the speed that makes total energy zero: v = sqrt(2GM/r). \
         A circular orbit is slower by sqrt(2). Rockets need continuous thrust \
         because atmospheres and staging make pure ballistic escape rare."
    }
}

#[cfg(test)]
mod tests {
    use super::EscapeVelocity;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = EscapeVelocity::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("ve="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn r_changes() {
        let r = EscapeVelocity::new();
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
        EscapeVelocity::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

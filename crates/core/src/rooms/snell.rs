//! Snell's law: ray bending at an interface (n1 sin i = n2 sin r).
//!
//! DRAG: TUNE INCIDENCE. See `docs/ROOMS.md`.

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

fn incidence(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.03
    };
    if let Some((x, _)) = hand {
        x * 1.2 + s // radians, up to ~70 deg
    } else {
        0.2 + phase_unit(t) * 0.9 + s
    }
}

fn draw(canvas: &mut dyn Surface, i_ang: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let n1 = 1.0;
    let n2 = 1.5
        + if seed == 0 {
            0.0
        } else {
            (seed % 4) as f64 * 0.05
        };
    let mid_y = height as f64 * 0.5;
    let mid_x = width as f64 * 0.5;
    // interface
    canvas.line(
        0,
        mid_y.round() as i32,
        width.saturating_sub(1) as i32,
        mid_y.round() as i32,
        '-',
    );
    // normal
    canvas.line(
        mid_x.round() as i32,
        0,
        mid_x.round() as i32,
        height.saturating_sub(1) as i32,
        '|',
    );
    let i = i_ang.clamp(0.05, 1.35);
    let sin_r = (n1 / n2) * i.sin();
    let tir = sin_r > 1.0;
    // incident ray from top
    let len = height as f64 * 0.4;
    let ix0 = mid_x - len * i.sin();
    let iy0 = mid_y - len * i.cos();
    canvas.line(
        ix0.round() as i32,
        iy0.round() as i32,
        mid_x.round() as i32,
        mid_y.round() as i32,
        '#',
    );
    if tir {
        // reflect
        let rx1 = mid_x + len * i.sin();
        let ry1 = mid_y - len * i.cos();
        canvas.line(
            mid_x.round() as i32,
            mid_y.round() as i32,
            rx1.round() as i32,
            ry1.round() as i32,
            '*',
        );
    } else {
        let r = sin_r.asin();
        let rx1 = mid_x + len * r.sin();
        let ry1 = mid_y + len * r.cos();
        canvas.line(
            mid_x.round() as i32,
            mid_y.round() as i32,
            rx1.round() as i32,
            ry1.round() as i32,
            '#',
        );
    }
}

/// Snell's law room.
#[derive(Debug, Default)]
pub struct Snell {
    seed: u64,
}

impl Snell {
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

impl Room for Snell {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "snell",
            title: "Snell's Law",
            wing: "Waves & Sound",
            blurb: "Rays bend at an interface; total reflection past critical. t and DRAG: TUNE \
                    INCIDENCE.",
            accent: [40, 140, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, incidence(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "snell",
            root: 659.25,
            tempo: 86,
            line: &[0, 5, 9, 12, 9, 5, 0, 7],
            encodes: "n1 sin i equals n2 sin r until total reflection",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE INCIDENCE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let i = incidence(t, None, self.seed);
        Some(format!("i={i:.2}  snell  DRAG:I"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let i = incidence(t, hands.last().copied(), self.seed);
        draw(canvas, i, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let i = incidence(t, hands.last().copied(), self.seed);
        let n2 = 1.5;
        let crit = (1.0_f64 / n2).asin();
        if i > crit {
            Some(format!("I={i:.3}  TIR"))
        } else {
            let r = ((1.0 / n2) * i.sin()).asin();
            Some(format!("I={i:.3}  r={r:.2}"))
        }
    }

    fn reveal(&self) -> &'static str {
        "Snell's law is n1 sin i = n2 sin r. Light bends toward the normal when \
         entering denser media. Past a critical angle from dense to rare, total \
         internal reflection takes over."
    }
}

#[cfg(test)]
mod tests {
    use super::Snell;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Snell::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("snell"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn incidence_changes() {
        let r = Snell::new();
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
        Snell::new().render(&mut c, 0.45);
        assert!(c.ink_count() > 0);
    }
}

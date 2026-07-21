//! Kaplan-Yorke map: a classic 2D map with a fractal attractor dimension.
//!
//! x' = 2x mod 1; y' = lambda y + cos(4 pi x). DRAG: TUNE LAMBDA.
//! See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const ITERS: usize = 8_000;

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

fn lambda(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    let l = if let Some((x, _)) = hand {
        0.1 + x * 0.8 + s
    } else {
        0.2 + phase_unit(t) * 0.5 + s
    };
    l.clamp(0.05, 0.95)
}

fn draw(canvas: &mut dyn Surface, lam: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Fixed view keeps ink dense even when the attractor is thin.
    let mut x: f64 = std::f64::consts::FRAC_1_PI;
    let mut y: f64 = 0.0;
    for i in 0..ITERS {
        y = lam * y + (4.0 * std::f64::consts::PI * x).cos();
        x = (2.0 * x).rem_euclid(1.0);
        if !x.is_finite() || !y.is_finite() {
            break;
        }
        let u = x.clamp(0.0, 1.0);
        let v = ((y + 2.5) / 5.0).clamp(0.0, 1.0);
        let ix = (u * width.saturating_sub(1) as f64).round() as i32;
        let iy = ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32;
        canvas.plot(ix, iy, if i % 6 == 0 { '#' } else { '*' });
    }
}

/// Kaplan-Yorke map room.
#[derive(Debug, Default)]
pub struct KaplanYorke {
    seed: u64,
}

impl KaplanYorke {
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

impl Room for KaplanYorke {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "kaplan-yorke",
            title: "Kaplan-Yorke Map",
            wing: "Motion & Dynamics",
            blurb: "Doubling in x, damped drive in y: fractal attractor. t and DRAG: TUNE LAMBDA.",
            accent: [80, 160, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, lambda(t, None, self.seed));
    }

    fn postcard_t(&self) -> f64 {
        0.45
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "kaplan yorke",
            root: 233.08,
            tempo: 110,
            line: &[0, 7, 2, 9, 4, 11, 6, 14],
            encodes: "expanding x drive filtered into a fractal y",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE LAMBDA")
    }

    fn status(&self, t: f64) -> Option<String> {
        let l = lambda(t, None, self.seed);
        Some(format!("lam={l:.2}  KY  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let l = lambda(t, hands.last().copied(), self.seed);
        draw(canvas, l);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let l = lambda(t, hands.last().copied(), self.seed);
        // Kaplan-Yorke map: x' = 2x mod 1, y' = lambda y + cos(4 pi x).
        // Lyapunovs: ln2 and ln|lambda|; dim = 1 + ln2/|ln lambda| for |l|<1.
        let dim = if l.abs() > 1e-9 && l.abs() < 1.0 {
            1.0 + std::f64::consts::LN_2 / (-l.abs().ln())
        } else {
            1.0
        };
        Some(format!("lam={l:.2}  Dky={dim:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The Kaplan-Yorke map couples the expanding doubling map to a damped \
         driven coordinate. For 0 < lambda < 1 the attractor has a simple \
         dimension formula D = 1 + ln 2 / |ln lambda|."
    }
}

#[cfg(test)]
mod tests {
    use super::KaplanYorke;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = KaplanYorke::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = KaplanYorke::new();
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
        KaplanYorke::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(KaplanYorke::new().motif().unwrap().line.len() >= 6);
    }
}

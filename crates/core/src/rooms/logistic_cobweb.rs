//! Logistic Cobweb: iteration as a staircase under the parabola.
//!
//! x -> r x (1-x) drawn as cobweb against y=x. DRAG: SET R. See `docs/ROOMS.md`.

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

fn r_param(t: f64, hand: Option<(f64, f64)>) -> f64 {
    if let Some((x, _)) = hand {
        2.5 + x * 1.5
    } else {
        2.8 + phase_unit(t) * 1.1
    }
}

fn f(x: f64, r: f64) -> f64 {
    r * x * (1.0 - x)
}

fn cobweb(r: f64, x0: f64, steps: usize) -> Vec<(f64, f64)> {
    let mut path = Vec::new();
    let mut x = x0.clamp(0.01, 0.99);
    for _ in 0..steps {
        let y = f(x, r);
        path.push((x, y));
        path.push((y, y));
        x = y;
        if !x.is_finite() {
            break;
        }
    }
    path
}

fn draw(canvas: &mut dyn Surface, r: f64, path: &[(f64, f64)]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let to_px = |x: f64, y: f64| -> (i32, i32) {
        (
            (x.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32,
            ((1.0 - y.clamp(0.0, 1.0)) * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    // y=x
    canvas.line(
        to_px(0.0, 0.0).0,
        to_px(0.0, 0.0).1,
        to_px(1.0, 1.0).0,
        to_px(1.0, 1.0).1,
        '.',
    );
    // parabola
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=64 {
        let x = i as f64 / 64.0;
        let y = f(x, r).clamp(0.0, 1.2).min(1.0);
        let p = to_px(x, y);
        if let Some(o) = prev {
            canvas.line(o.0, o.1, p.0, p.1, '#');
        }
        prev = Some(p);
    }
    // cobweb
    prev = None;
    for &(x, y) in path {
        let p = to_px(x, y.clamp(0.0, 1.0));
        if let Some(o) = prev {
            canvas.line(o.0, o.1, p.0, p.1, '*');
        }
        prev = Some(p);
    }
}

/// Logistic cobweb room.
#[derive(Debug, Default)]
pub struct LogisticCobweb {
    seed: u64,
}

impl LogisticCobweb {
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

impl Room for LogisticCobweb {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "logistic-cobweb",
            title: "The Cobweb",
            wing: "Motion & Dynamics",
            blurb: "Logistic map as cobweb: climb the parabola, slide to y=x. t and DRAG: SET R \
                    through fixed point, period doubling, chaos.",
            accent: [255, 140, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let r = r_param(t, None);
        let x0 = 0.2
            + if self.seed == 0 {
                0.0
            } else {
                (self.seed % 5) as f64 * 0.05
            };
        let path = cobweb(r, x0, 30);
        draw(canvas, r, &path);
    }

    fn postcard_t(&self) -> f64 {
        0.7
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "cobweb",
            root: 146.83,
            tempo: 112,
            line: &[0, 3, 7, 10, 12, 10, 7, 3],
            encodes: "iteration climbing a parabola into chaos",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SET R")
    }

    fn status(&self, t: f64) -> Option<String> {
        let r = r_param(t, None);
        let path = cobweb(r, 0.3, 40);
        let last = path.last().map(|p| p.1).unwrap_or(0.0);
        Some(format!("r={r:.2}  x={last:.3}  DRAG:R"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let r = r_param(t, hands.last().copied());
        let x0 = hands
            .last()
            .map(|&(_, y)| y.clamp(0.05, 0.95))
            .unwrap_or(0.3);
        let path = cobweb(r, x0, 35);
        draw(canvas, r, &path);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let r = r_param(t, hands.last().copied());
        let x0 = hands.last().map(|&(_, y)| y).unwrap_or(0.3);
        let path = cobweb(r, x0, 40);
        let last = path.last().map(|p| p.1).unwrap_or(0.0);
        let regime = if r < 3.0 {
            "FIXED"
        } else if r < 3.57 {
            "CYCLE"
        } else {
            "CHAOS"
        };
        Some(format!("R={r:.2}  x={last:.3}  {regime}"))
    }

    fn reveal(&self) -> &'static str {
        "The logistic map x -> r x (1-x) is a parabola of destiny. Cobweb \
         diagrams make iteration visible: fixed points, period doubling, and \
         chaos as r climbs past about 3.57."
    }
}

#[cfg(test)]
mod tests {
    use super::{LogisticCobweb, cobweb, f};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = LogisticCobweb::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("r="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn set_r_changes() {
        let r = LogisticCobweb::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.95,
                    y: 0.4,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn fixed_point_stable_low_r() {
        let r = 2.5;
        let fp = 1.0 - 1.0 / r;
        assert!((f(fp, r) - fp).abs() < 1e-9);
        let path = cobweb(r, 0.2, 50);
        let last = path.last().unwrap().1;
        assert!((last - fp).abs() < 0.05);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        LogisticCobweb::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(LogisticCobweb::new().motif().unwrap().line.len() >= 6);
    }
}

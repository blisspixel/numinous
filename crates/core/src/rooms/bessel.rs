//! Bessel J0: cylindrical wave zeros as radial rings.
//!
//! DRAG: TUNE SCALE. See `docs/ROOMS.md`.

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

fn scale_p(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.04
    };
    if let Some((x, _)) = hand {
        0.5 + x * 2.0 + s
    } else {
        0.8 + phase_unit(t) * 1.5 + s
    }
}

/// Toy J0 via series for small, asymptotic cos for large.
fn j0(x: f64) -> f64 {
    let ax = x.abs();
    if ax < 8.0 {
        let z = x * x;
        let mut t = 1.0;
        let mut s = 1.0;
        for k in 1..12 {
            t *= -z / (4.0 * (k as f64) * (k as f64));
            s += t;
        }
        s
    } else {
        let amp = (2.0 / (std::f64::consts::PI * ax)).sqrt();
        amp * (ax - std::f64::consts::FRAC_PI_4).cos()
    }
}

fn draw(canvas: &mut dyn Surface, sc: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let sc = sc.clamp(0.4, 3.0);
    let max_r = (width.min(height) as f64) * 0.48;
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    for ring in 0..40 {
        let rr = (ring as f64 + 0.5) / 40.0 * max_r;
        let x = (rr / max_r) * 15.0 / sc;
        let v = j0(x).abs();
        if v < 0.05 {
            continue;
        }
        let ch = if v > 0.6 {
            '#'
        } else if v > 0.3 {
            '*'
        } else if v > 0.12 {
            '+'
        } else {
            '.'
        };
        let steps = ((rr * 2.0 * std::f64::consts::PI).ceil() as i32).clamp(12, 80);
        let mut prev: Option<(i32, i32)> = None;
        for i in 0..=steps {
            let th = 2.0 * std::f64::consts::PI * (i as f64 / steps as f64) + rot;
            let px = (cx + rr * th.cos()).round() as i32;
            let py = (cy - rr * th.sin() * 0.55).round() as i32;
            if let Some((ox, oy)) = prev {
                canvas.line(ox, oy, px, py, ch);
            }
            prev = Some((px, py));
        }
    }
}

/// Bessel room.
#[derive(Debug, Default)]
pub struct Bessel {
    seed: u64,
}

impl Bessel {
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

impl Room for Bessel {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "bessel",
            title: "Bessel J0",
            wing: "Waves & Sound",
            blurb: "Cylindrical wave zeros as rings. t and DRAG: TUNE SCALE.",
            accent: [50, 90, 150],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, scale_p(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "bessel",
            root: 146.83,
            tempo: 88,
            line: &[0, 5, 7, 10, 12, 10, 7, 5],
            encodes: "J0 cylindrical waves: nodal rings at Bessel zeros",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE SCALE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let s = scale_p(t, None, self.seed);
        Some(format!("s={s:.2}  J0  DRAG:SCALE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let s = scale_p(t, hands.last().copied(), self.seed);
        draw(canvas, s, self.seed ^ hands.len() as u64);
        if let Some(&(x, y)) = hands.last() {
            let (bw, bh) = canvas.draw_bounds();
            if bw > 0 && bh > 0 {
                let px = (x * bw.saturating_sub(1) as f64).round() as i32;
                let py = (y * bh.saturating_sub(1) as f64).round() as i32;
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
        let s = scale_p(t, hands.last().copied(), self.seed).clamp(0.4, 3.0);
        // Radial argument spans [0, 15/s]; count J0 sign changes (nodal rings).
        let x_max = 15.0 / s;
        let mut zeros = 0u32;
        let mut prev = j0(1e-6);
        for i in 1..=240 {
            let x = x_max * (i as f64) / 240.0;
            let v = j0(x);
            if prev * v <= 0.0 {
                zeros += 1;
            }
            prev = v;
        }
        Some(format!("s={s:.2}  ~{zeros} rings  j0_1~2.40"))
    }

    fn reveal(&self) -> &'static str {
        "Bessel functions solve radial waves on a disk. J0 is the circularly \
         symmetric mode: its zeros mark quiet rings of a vibrating drum and the \
         nodal circles of cylindrical waves."
    }
}

#[cfg(test)]
mod tests {
    use super::Bessel;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Bessel::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("J0"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn scale_changes() {
        let r = Bessel::new();
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
        Bessel::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

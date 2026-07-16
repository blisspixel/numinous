//! Airy disk: circular-aperture diffraction rings.
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

fn scale_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.03
    };
    if let Some((x, _)) = hand {
        0.6 + x * 2.0 + s
    } else {
        0.9 + phase_unit(t) * 1.5 + s
    }
}

/// Toy jinc intensity stand-in for Airy rings (zeros near multiples of pi).
fn airy_i(x: f64) -> f64 {
    if x.abs() < 1e-6 {
        return 1.0;
    }
    let s = x.sin() / x;
    (s * s * (1.0 + 0.15 * (x / 4.0).cos().powi(2))).clamp(0.0, 1.0)
}

fn draw(canvas: &mut dyn Surface, sc: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let sc = sc.clamp(0.5, 3.0);
    let max_r = (width.min(height) as f64) * 0.48;
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 6) as f64 * 0.02
    };
    // Sample radial rings and draw isophotes.
    for ring in 0..48 {
        let rr = (ring as f64 + 0.5) / 48.0 * max_r;
        let x = (rr / max_r) * 12.0 / sc;
        let inten = airy_i(x);
        if inten < 0.04 {
            continue;
        }
        let ch = if inten > 0.55 {
            '#'
        } else if inten > 0.25 {
            '*'
        } else if inten > 0.1 {
            '+'
        } else {
            '.'
        };
        let steps = ((rr * 2.0 * std::f64::consts::PI).ceil() as i32).clamp(12, 96);
        let mut prev: Option<(i32, i32)> = None;
        for i in 0..=steps {
            let th = 2.0 * std::f64::consts::PI * (i as f64 / steps as f64) + rot;
            let px = (cx + rr * th.cos()).round() as i32;
            let py = (cy - rr * th.sin() * 0.55).round() as i32; // char aspect-ish
            if let Some((ox, oy)) = prev {
                if (px - ox).abs() + (py - oy).abs() > 0 {
                    canvas.line(ox, oy, px, py, ch);
                }
            }
            prev = Some((px, py));
        }
    }
    // Center peak.
    canvas.line(cx as i32 - 1, cy as i32, cx as i32 + 1, cy as i32, '#');
    canvas.line(cx as i32, cy as i32 - 1, cx as i32, cy as i32 + 1, '#');
}

/// Airy disk room.
#[derive(Debug, Default)]
pub struct Airy {
    seed: u64,
}

impl Airy {
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

impl Room for Airy {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "airy",
            title: "Airy Disk",
            wing: "Waves & Sound",
            blurb: "Circular aperture diffraction rings. t and DRAG: TUNE SCALE.",
            accent: [200, 180, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, scale_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "airy",
            root: 523.25,
            tempo: 96,
            line: &[0, 7, 12, 7, 0, 5, 12, 5],
            encodes: "point source through a circular stop becomes Airy rings",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE SCALE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let s = scale_param(t, None, self.seed);
        Some(format!("s={s:.2}  rings  DRAG:SCALE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let s = scale_param(t, hands.last().copied(), self.seed);
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
        let s = scale_param(t, hands.last().copied(), self.seed);
        Some(format!("S={s:.3}  airy"))
    }

    fn reveal(&self) -> &'static str {
        "Light through a circular aperture forms an Airy pattern: a bright core \
         and concentric dark rings where the Bessel function J1 vanishes. Telescope \
         resolution is set by the radius of that first dark ring."
    }
}

#[cfg(test)]
mod tests {
    use super::Airy;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Airy::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("rings"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn scale_changes() {
        let r = Airy::new();
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
        Airy::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

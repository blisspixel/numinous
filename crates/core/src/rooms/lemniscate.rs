//! Bernoulli lemniscate: figure-eight algebraic curve (infinity symbol).
//!
//! Ambient phase walks a pen along both lobes. DRAG: TUNE SCALE.
//! See `docs/ROOMS.md`.

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

fn scale(_t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.55 + x * 0.5 + s
    } else {
        // Ambient scale holds a full infinity mark; motion lives in the pen.
        0.85 + s * 0.5
    }
}

fn point(th: f64, rad: f64, rot: f64) -> (f64, f64) {
    let s = th.sin();
    let c = th.cos();
    let den = 1.0 + s * s;
    let x = rad * c / den;
    let y = rad * s * c / den;
    let xr = x * rot.cos() - y * rot.sin();
    let yr = x * rot.sin() + y * rot.cos();
    (xr, yr)
}

fn draw(canvas: &mut dyn Surface, a: f64, show: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let rad = (width.min(height) as f64) * 0.48 * a.clamp(0.5, 1.15);
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 9) as f64 * 0.04
    };
    let show = show.clamp(0.0, 1.0);
    // Parametric Bernoulli lemniscate (both lobes in one sweep).
    let steps = 520;
    let drawn = ((show * steps as f64).round() as usize).min(steps);
    // Soft ghost of the full infinity mark.
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = std::f64::consts::TAU * (i as f64 / steps as f64);
        let (xr, yr) = point(th, rad, rot);
        let px = (cx + xr).round() as i32;
        let py = (cy - yr * 0.85).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // Bright path so far.
    prev = None;
    for i in 0..=drawn {
        let th = std::f64::consts::TAU * (i as f64 / steps as f64);
        let (xr, yr) = point(th, rad, rot);
        let px = (cx + xr).round() as i32;
        let py = (cy - yr * 0.85).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
            canvas.line(ox, oy + 1, px, py + 1, '*');
        }
        prev = Some((px, py));
    }
    let pen_th = show * std::f64::consts::TAU;
    let (xr, yr) = point(pen_th, rad, rot);
    let pen_x = (cx + xr).round() as i32;
    let pen_y = (cy - yr * 0.85).round() as i32;
    for dy in -2..=2 {
        for dx in -2..=2 {
            if dx * dx + dy * dy <= 5 {
                canvas.plot(pen_x + dx, pen_y + dy, 'o');
            }
        }
    }
}

/// Lemniscate room.
#[derive(Debug, Default)]
pub struct Lemniscate {
    seed: u64,
}

impl Lemniscate {
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

impl Room for Lemniscate {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "lemniscate",
            title: "Lemniscate",
            wing: "Shape & Space",
            blurb: "Bernoulli infinity draws both lobes. Watch the pen; DRAG: TUNE SCALE.",
            accent: [160, 40, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, scale(t, None, self.seed), phase_unit(t), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "lemniscate",
            root: 415.3,
            tempo: 102,
            line: &[0, 7, 12, 7, 0, 7, 12, 7],
            encodes: "r squared equals two a squared cos two theta",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE SCALE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = scale(t, None, self.seed);
        let p = (phase_unit(t) * 100.0).round() as i32;
        Some(format!("a={a:.2}  draw={p}%  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = scale(t, hands.last().copied(), self.seed);
        let show = hands
            .last()
            .map(|&(_, y)| y)
            .unwrap_or_else(|| phase_unit(t));
        draw(canvas, a, show, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let a = scale(t, hands.last().copied(), self.seed);
        // Bernoulli lemniscate r^2 = 2 a^2 cos(2theta): area a^2, half-width a sqrt(2).
        let area = a * a;
        let half_w = a * std::f64::consts::SQRT_2;
        Some(format!("a={a:.2}  area={area:.2}  half={half_w:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The lemniscate of Bernoulli is the locus of product of distances to \
         two foci equal to the square of half the interfocal distance. In polar \
         form r^2 = 2 a^2 cos(2 theta): the algebraic infinity sign."
    }
}

#[cfg(test)]
mod tests {
    use super::Lemniscate;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Lemniscate::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("draw"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn scale_changes() {
        let r = Lemniscate::new();
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
    fn ambient_pen_moves_the_plate() {
        let r = Lemniscate::new();
        let mut a = Canvas::new(80, 48);
        let mut b = Canvas::new(80, 48);
        r.render(&mut a, 0.15);
        r.render(&mut b, 0.75);
        assert_ne!(a.to_text(), b.to_text(), "pen must walk the infinity mark");
        assert!(a.ink_count() > 40);
        assert!(b.ink_count() > 40);
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Lemniscate::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

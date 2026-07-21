//! Prism dispersion: refractive index falls with wavelength (toy Cauchy).
//!
//! DRAG: TUNE ANGLE. See `docs/ROOMS.md`.

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

fn apex(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.03
    };
    if let Some((x, _)) = hand {
        0.3 + x * 1.0 + s
    } else {
        0.4 + phase_unit(t) * 0.8 + s
    }
}

fn n_of_lambda(lam: f64) -> f64 {
    // Cauchy: n = A + B/lam^2
    1.5 + 0.02 / (lam * lam + 0.05)
}

fn draw(canvas: &mut dyn Surface, alpha: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let alpha = alpha.clamp(0.25, 1.4);
    // Prism outline
    let apex_x = (width as f64 * 0.35).round() as i32;
    let base_y = (height as f64 * 0.75).round() as i32;
    let tip_y = (height as f64 * 0.25).round() as i32;
    let left = (width as f64 * 0.15).round() as i32;
    let right = (width as f64 * 0.55).round() as i32;
    canvas.line(left, base_y, apex_x, tip_y, '#');
    canvas.line(apex_x, tip_y, right, base_y, '#');
    canvas.line(left, base_y, right, base_y, '#');
    // White ray in
    let in_y = (height as f64 * 0.45).round() as i32;
    canvas.line(0, in_y, apex_x - 2, in_y, '+');
    // Dispersed rays out: different n -> different bend
    let i_ang = 0.5 + alpha * 0.2;
    for (k, lam) in [0.4, 0.5, 0.6, 0.7, 0.8].iter().enumerate() {
        let n = n_of_lambda(*lam);
        let sin_r = (i_ang.sin() / n).clamp(-1.0, 1.0);
        let r = sin_r.asin();
        let out = alpha - r;
        let dx = (width as f64 * 0.4 * out.cos()).round() as i32;
        let dy = (height as f64 * 0.35 * out.sin() * (0.5 + 0.15 * k as f64)).round() as i32;
        let ch = match k {
            0 => 'v',
            1 => 'b',
            2 => 'g',
            3 => 'y',
            _ => 'r',
        };
        canvas.line(apex_x, in_y, apex_x + dx, in_y + dy, ch);
    }
    let _ = seed;
}

/// Prism dispersion room.
#[derive(Debug, Default)]
pub struct SnellPrism {
    seed: u64,
}

impl SnellPrism {
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

impl Room for SnellPrism {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "snell-prism",
            title: "Prism Dispersion",
            wing: "Waves & Sound",
            blurb: "n(lambda) splits white light in a prism. t and DRAG: TUNE ANGLE.",
            accent: [140, 40, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, apex(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "snell-prism",
            root: 6.88,
            tempo: 96,
            line: &[0, 2, 5, 7, 9, 12, 7, 2],
            encodes: "prism: Cauchy n falls with wavelength, violet bends more",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE ANGLE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let a = apex(t, None, self.seed);
        Some(format!("a={a:.2}  disp  DRAG:ANG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let a = apex(t, hands.last().copied(), self.seed);
        draw(canvas, a, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let a = apex(t, hands.last().copied(), self.seed).clamp(0.25, 1.4);
        let i_ang = 0.5 + a * 0.2;
        let mut outs = [0.0_f64; 5];
        for (k, lam) in [0.4, 0.5, 0.6, 0.7, 0.8].iter().enumerate() {
            let n = n_of_lambda(*lam);
            let sin_r = (i_ang.sin() / n).clamp(-1.0, 1.0);
            let r = sin_r.asin();
            outs[k] = a - r;
        }
        let spread = (outs[4] - outs[0]).abs();
        Some(format!(
            "a={a:.2}  spread={spread:.2}  nV={:.2}",
            n_of_lambda(0.4)
        ))
    }

    fn reveal(&self) -> &'static str {
        "A prism disperses light because the refractive index depends on \
         wavelength. Violet sees a larger n, bends more at each face, and leaves \
         the far side lower than red: Newton's spectrum from Snell's law."
    }
}

#[cfg(test)]
mod tests {
    use super::SnellPrism;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = SnellPrism::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("disp"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn angle_changes() {
        let r = SnellPrism::new();
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
        SnellPrism::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

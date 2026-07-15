//! Spherical Harmonics / Hydrogen: the singing sphere and the shape of the atom.
//!
//! Real spherical harmonics Y_lm lobed patterns (toy sampling on a sphere
//! projection). DRAG: RAISE l AND m. See `docs/ROOMS.md`.

use std::f64::consts::PI;

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

/// Associated Legendre toys for low l,m (hardcoded for determinism and speed).
fn ylm(l: u32, m: i32, theta: f64, phi: f64) -> f64 {
    let ct = theta.cos();
    let st = theta.sin();
    let m = m.clamp(-(l as i32), l as i32);
    let base = match (l, m.abs()) {
        (0, 0) => 0.5 * (1.0 / PI).sqrt(),
        (1, 0) => (0.75 / PI).sqrt() * ct,
        (1, 1) => -(0.375 / PI).sqrt() * st,
        (2, 0) => (1.25 / PI).sqrt() * 0.5 * (3.0 * ct * ct - 1.0),
        (2, 1) => -(1.875 / PI).sqrt() * st * ct,
        (2, 2) => (0.46875 / PI).sqrt() * st * st,
        (3, 0) => (1.75 / PI).sqrt() * 0.5 * (5.0 * ct * ct * ct - 3.0 * ct),
        (3, 1) => -(1.3125 / PI).sqrt() * st * (5.0 * ct * ct - 1.0) * 0.5,
        (3, 2) => (3.28125 / PI).sqrt() * st * st * ct * 0.5,
        (3, 3) => -(0.546875 / PI).sqrt() * st * st * st,
        _ => {
            // Fallback: product of sins for higher.
            st.powi(m.abs()) * ct.powi((l as i32 - m.abs()).max(0))
        }
    };
    if m == 0 {
        base
    } else if m > 0 {
        base * (m as f64 * phi).cos() * std::f64::consts::SQRT_2
    } else {
        base * ((-m) as f64 * phi).sin() * std::f64::consts::SQRT_2
    }
}

fn quantum(t: f64, hand: Option<(f64, f64)>, seed: u64) -> (u32, i32) {
    let (l, m) = if let Some((x, y)) = hand {
        let l = (x * 3.99).floor() as u32;
        let m_max = l as i32;
        let m = ((y * 2.0 - 1.0) * (m_max as f64 + 0.01)).round() as i32;
        (l.min(3), m.clamp(-m_max, m_max))
    } else {
        let l = (phase_unit(t) * 3.0).round() as u32;
        let m = if seed == 0 {
            0
        } else {
            (seed % (l as u64 + 1)) as i32
        };
        (l.min(3), m)
    };
    (l, m)
}

fn draw(canvas: &mut dyn Surface, l: u32, m: i32, phase: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Orthographic sphere: sample plate as disk.
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let r = width.min(height) as f64 * 0.42;
    for py in 0..height {
        for px in 0..width {
            let dx = (px as f64 - cx) / r;
            let dy = (py as f64 - cy) / r;
            let rho2 = dx * dx + dy * dy;
            if rho2 > 1.0 {
                continue;
            }
            let z = (1.0 - rho2).sqrt();
            // Rotate slightly with phase.
            let x = dx * phase.cos() - z * phase.sin();
            let zz = dx * phase.sin() + z * phase.cos();
            let y = -dy;
            // Standard polar: theta from +z.
            let theta = zz.clamp(-1.0, 1.0).acos();
            let phi = y.atan2(x);
            let val = ylm(l, m, theta, phi);
            if val.abs() < 0.08 {
                continue;
            }
            let ch = if val > 0.25 {
                '#'
            } else if val > 0.08 {
                '*'
            } else if val < -0.25 {
                'o'
            } else {
                '+'
            };
            canvas.plot(px as i32, py as i32, ch);
        }
    }
    // Outline.
    let steps = 64;
    let mut prev: Option<(i32, i32)> = None;
    for s in 0..=steps {
        let a = std::f64::consts::TAU * s as f64 / steps as f64;
        let x = (cx + r * a.cos()).round() as i32;
        let y = (cy + r * a.sin()).round() as i32;
        if let Some(o) = prev {
            canvas.line(o.0, o.1, x, y, '.');
        }
        prev = Some((x, y));
    }
}

/// Spherical harmonics / hydrogen room.
#[derive(Debug, Default)]
pub struct Harmonics {
    seed: u64,
}

impl Harmonics {
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

impl Room for Harmonics {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "harmonics",
            title: "The Singing Sphere",
            wing: "Waves & Sound",
            blurb: "Real spherical harmonics Y_lm: the lobes of atomic orbitals and of a ringing \
                    sphere. t lifts l; DRAG: RAISE l AND m.",
            accent: [100, 160, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (l, m) = quantum(t, None, self.seed);
        draw(canvas, l, m, phase_unit(t) * 1.2);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "ylm lobe",
            root: 329.63,
            tempo: 85,
            line: &[0, 4, 8, 11, 16, 11, 8, 4],
            encodes: "spherical harmonics: the atom's and the bell's shared shape",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: RAISE l AND m")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (l, m) = quantum(t, None, self.seed);
        Some(format!("Y_l={l} m={m}  DRAG:l,m"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (l, m) = quantum(t, hands.last().copied(), self.seed);
        draw(canvas, l, m, phase_unit(t));
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
        let (l, m) = quantum(t, hands.last().copied(), self.seed);
        let name = match (l, m) {
            (0, 0) => "s",
            (1, 0) => "pz",
            (1, _) => "px/py",
            (2, 0) => "dz2",
            (2, _) => "d",
            (3, _) => "f",
            _ => "Y",
        };
        Some(format!("Y({l},{m})  {name}  orbital"))
    }

    fn reveal(&self) -> &'static str {
        "Spherical harmonics are the angular shapes of free vibration on a \
         sphere and of the hydrogen wavefunction. The same lobes are orbitals, \
         drum modes, and the cosmic microwave background's multipoles."
    }
}

#[cfg(test)]
mod tests {
    use super::{Harmonics, ylm};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Harmonics::new().status(0.4).unwrap();
        assert!(s.contains("DRAG") || s.contains("Y_"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn quantum_changes() {
        let r = Harmonics::new();
        let o = r.status(0.1).unwrap();
        let a = r
            .status_input(
                0.1,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.2,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn y00_positive() {
        assert!(ylm(0, 0, 0.0, 0.0) > 0.0);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(36, 28);
        Harmonics::new().render(&mut c, 0.6);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(Harmonics::new().motif().unwrap().line.len() >= 6);
    }
}

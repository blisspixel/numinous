//! Spherical harmonic nodal lines: Y_lm on the sphere.
//!
//! DRAG: TUNE L. See `docs/ROOMS.md`.

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

fn degree_l(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 { 0.0 } else { (seed % 3) as f64 };
    if let Some((x, _)) = hand {
        1.0 + x * 5.0 + s * 0.1
    } else {
        1.0 + phase_unit(t) * 4.5 + s * 0.1
    }
}

/// Associated Legendre-ish real spherical harmonic stand-in.
fn y_lm(l: usize, m: usize, theta: f64, phi: f64) -> f64 {
    let c = theta.cos();
    // P_l^m rough via (1-c^2)^{m/2} * P_l style
    let p = match (l, m) {
        (0, _) => 1.0,
        (1, 0) => c,
        (1, _) => theta.sin(),
        (2, 0) => 1.5 * c * c - 0.5,
        (2, 1) => -3.0 * c * theta.sin(),
        (2, _) => 3.0 * theta.sin().powi(2),
        (3, 0) => 2.5 * c * c * c - 1.5 * c,
        (3, 1) => -1.5 * (5.0 * c * c - 1.0) * theta.sin(),
        (3, 2) => 15.0 * c * theta.sin().powi(2),
        (3, _) => -15.0 * theta.sin().powi(3),
        _ => {
            // higher: Legendre recurrence shadow
            let mut p0 = 1.0;
            let mut p1 = c;
            for n in 1..l {
                let p2 = ((2.0 * n as f64 + 1.0) * c * p1 - n as f64 * p0) / (n as f64 + 1.0);
                p0 = p1;
                p1 = p2;
            }
            p1 * theta.sin().powi(m.min(2) as i32)
        }
    };
    let mf = m as f64;
    p * (mf * phi).cos()
}

fn draw(canvas: &mut dyn Surface, l_f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let r = (width.min(height) as f64) * 0.42;
    let l = l_f.round().clamp(1.0, 6.0) as usize;
    let m_base = l / 2;
    let m = if seed == 0 {
        m_base
    } else {
        (m_base + (seed % (l as u64 + 1)) as usize) % (l + 1)
    };
    // Sphere outline.
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=48 {
        let th = 2.0 * std::f64::consts::PI * (i as f64 / 48.0);
        let px = (cx + r * th.cos()).round() as i32;
        let py = (cy - r * th.sin() * 0.55).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // Sample nodal lines where Y_lm ~ 0 and positive lobes.
    for ti in 0..36 {
        let theta = std::f64::consts::PI * (ti as f64 + 0.5) / 36.0;
        for pi in 0..48 {
            let phi = 2.0 * std::f64::consts::PI * (pi as f64 / 48.0);
            let y = y_lm(l, m, theta, phi);
            if y.abs() < 0.12 {
                // nodal
                let x = theta.sin() * phi.cos();
                let yy = theta.sin() * phi.sin();
                let z = theta.cos();
                if z < -0.05 {
                    continue;
                }
                let px = (cx + r * x).round() as i32;
                let py = (cy - r * yy * 0.55).round() as i32;
                canvas.line(px, py, px, py, '+');
            } else if y > 0.35 {
                let x = theta.sin() * phi.cos();
                let yy = theta.sin() * phi.sin();
                let z = theta.cos();
                if z < -0.1 {
                    continue;
                }
                let px = (cx + r * x).round() as i32;
                let py = (cy - r * yy * 0.55).round() as i32;
                canvas.line(px, py, px, py, '#');
            }
        }
    }
}

/// Spherical harmonic room.
#[derive(Debug, Default)]
pub struct SphericalHarmonic {
    seed: u64,
}

impl SphericalHarmonic {
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

impl Room for SphericalHarmonic {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "spherical-harmonic",
            title: "Spherical Harmonic",
            wing: "Waves & Sound",
            blurb: "Y_lm nodal lines on the sphere. t and DRAG: TUNE L.",
            accent: [40, 100, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, degree_l(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "spherical-harmonic",
            root: 58.27,
            tempo: 84,
            line: &[0, 3, 7, 10, 12, 7, 3, 0],
            encodes: "spherical harmonics: angular atoms of Laplace on the sphere",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE L")
    }

    fn status(&self, t: f64) -> Option<String> {
        let l = degree_l(t, None, self.seed).round();
        Some(format!("l={l:.0}  Ylm  DRAG:L"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let l = degree_l(t, hands.last().copied(), self.seed);
        draw(canvas, l, self.seed ^ hands.len() as u64);
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
        let l = degree_l(t, hands.last().copied(), self.seed).round();
        Some(format!("L={l:.0}  Ylm"))
    }

    fn reveal(&self) -> &'static str {
        "Spherical harmonics Y_lm are the angular eigenfunctions of the Laplacian \
         on the sphere. Atomic orbitals, CMB multipoles, and gravitational fields \
         expand in this basis; zeros of Y_lm draw nodal lines on the globe."
    }
}

#[cfg(test)]
mod tests {
    use super::SphericalHarmonic;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = SphericalHarmonic::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("Ylm"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn l_changes() {
        let r = SphericalHarmonic::new();
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
        SphericalHarmonic::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}

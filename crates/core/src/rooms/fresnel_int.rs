//! Fresnel integrals: C(t) and S(t) form the clothoid/Euler spiral.
//!
//! DRAG: TUNE T. See `docs/ROOMS.md`.

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

fn tmax(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 4) as f64 * 0.2
    };
    if let Some((x, _)) = hand {
        1.2 + x * 4.0 + s
    } else {
        1.8 + phase_unit(t) * 3.2 + s
    }
    .clamp(1.2, 6.0)
}

fn draw(canvas: &mut dyn Surface, tmax: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let sc = (width.min(height) as f64) * 0.5;
    let steps = 720;
    let mut prev: Option<(i32, i32)> = None;
    let mut c = 0.0;
    let mut s = 0.0;
    let dt = tmax / steps as f64;
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.1
    };
    for i in 0..=steps {
        let u = i as f64 * dt;
        // incremental Fresnel: dC=cos(pi/2 u^2) du, dS=sin(pi/2 u^2) du
        if i > 0 {
            let mid = u - 0.5 * dt;
            let ang = std::f64::consts::FRAC_PI_2 * mid * mid;
            c += ang.cos() * dt;
            s += ang.sin() * dt;
        }
        let x = c * rot.cos() - s * rot.sin();
        let y = c * rot.sin() + s * rot.cos();
        let px = (cx + x * sc).round() as i32;
        let py = (cy - y * sc).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
            canvas.line(ox, oy + 1, px, py + 1, '*');
        }
        prev = Some((px, py));
    }
    // Asymptote attractor mark (filled blot, not a reticle).
    let ax = (cx + 0.5 * sc).round() as i32;
    let ay = (cy - 0.5 * sc).round() as i32;
    for dy in -1..=1 {
        for dx in -1..=1 {
            canvas.plot(ax + dx, ay + dy, 'o');
        }
    }
}

/// Fresnel integrals room.
#[derive(Debug, Default)]
pub struct FresnelInt {
    seed: u64,
}

impl FresnelInt {
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

impl Room for FresnelInt {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "fresnel-int",
            title: "Fresnel Integrals",
            wing: "Analysis",
            blurb: "C(t), S(t) clothoid spiral to (1/2,1/2). t and DRAG: TUNE T.",
            accent: [80, 90, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, tmax(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "fresnel-int",
            root: 311.13,
            tempo: 78,
            line: &[0, 2, 5, 9, 12, 9, 5, 2],
            encodes: "Fresnel C,S: Euler spiral curvature linear in arc length",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE T")
    }

    fn status(&self, t: f64) -> Option<String> {
        let tm = tmax(t, None, self.seed);
        Some(format!("t={tm:.2}  fresnel  DRAG:T"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let tm = tmax(t, hands.last().copied(), self.seed);
        draw(canvas, tm, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let tm = tmax(t, hands.last().copied(), self.seed);
        // Integrate Fresnel C,S to tmax; asymptote is (1/2, 1/2).
        let steps = 120usize;
        let dt = tm / steps as f64;
        let mut c = 0.0_f64;
        let mut s = 0.0_f64;
        for i in 1..=steps {
            let u = i as f64 * dt;
            let mid = u - 0.5 * dt;
            let ang = std::f64::consts::FRAC_PI_2 * mid * mid;
            c += ang.cos() * dt;
            s += ang.sin() * dt;
        }
        let err = (c - 0.5).hypot(s - 0.5);
        Some(format!("T={tm:.2}  C={c:.2} S={s:.2}  d={err:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Fresnel integrals C(t)=int cos(pi/2 u^2) du and S(t)=int sin(pi/2 u^2) du \
         parametrize the clothoid (Euler spiral): curvature grows linearly with \
         arc length. As t grows they spiral into the point (1/2, 1/2)."
    }
}

#[cfg(test)]
mod tests {
    use super::FresnelInt;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = FresnelInt::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("fresnel"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn t_changes() {
        let r = FresnelInt::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.95,
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
        FresnelInt::new().render(&mut c, 0.6);
        assert!(c.ink_count() > 0);
    }
}

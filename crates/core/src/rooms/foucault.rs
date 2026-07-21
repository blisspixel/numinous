//! Foucault pendulum: Earth's rotation precesses the swing plane.
//!
//! DRAG: TUNE LATITUDE. See `docs/ROOMS.md`.

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

/// Latitude in radians, 0 at equator to pi/2 at pole.
fn latitude(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        x * std::f64::consts::FRAC_PI_2 + s * 0.1
    } else {
        (0.2 + phase_unit(t) * 0.7) * std::f64::consts::FRAC_PI_2 + s * 0.1
    }
    .clamp(0.0, std::f64::consts::FRAC_PI_2)
}

fn draw(canvas: &mut dyn Surface, lat: f64, t: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let amp = (width.min(height) as f64) * 0.4;
    // Precession rate ~ Omega * sin(lat). Animate a few swings under precession.
    let omega = 0.35
        + if seed == 0 {
            0.0
        } else {
            (seed % 4) as f64 * 0.03
        };
    let precess = omega * lat.sin() * (2.0 + phase_unit(t) * 8.0);
    let swing_n = 28;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=swing_n * 12 {
        let u = i as f64 / (swing_n * 12) as f64;
        let swing = (u * swing_n as f64 * std::f64::consts::PI).sin();
        let ang = precess * u;
        let x = amp * swing * ang.cos();
        let y = amp * swing * ang.sin() * 0.55;
        let px = (cx + x).round() as i32;
        let py = (cy - y).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, if i % 3 == 0 { '#' } else { '.' });
        }
        prev = Some((px, py));
    }
    // Pivot.
    canvas.line(cx as i32 - 1, cy as i32, cx as i32 + 1, cy as i32, 'o');
    canvas.line(cx as i32, cy as i32 - 1, cx as i32, cy as i32 + 1, 'o');
}

/// Foucault pendulum room.
#[derive(Debug, Default)]
pub struct Foucault {
    seed: u64,
}

impl Foucault {
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

impl Room for Foucault {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "foucault",
            title: "Foucault Pendulum",
            wing: "Motion & Dynamics",
            blurb: "Swing plane precesses with sin(latitude). t and DRAG: TUNE LATITUDE.",
            accent: [50, 90, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, latitude(t, None, self.seed), t, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.65
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "foucault",
            root: 415.3,
            tempo: 78,
            line: &[0, 7, 5, 12, 7, 0, 5, 12],
            encodes: "earth rotation turns the swing plane by Omega sin lat",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE LATITUDE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let lat = latitude(t, None, self.seed);
        let deg = lat.to_degrees();
        Some(format!("lat={deg:.0}deg  DRAG:LAT"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let lat = latitude(t, hands.last().copied(), self.seed);
        draw(canvas, lat, t, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let lat = latitude(t, hands.last().copied(), self.seed);
        let deg = lat.to_degrees();
        // Precession period: T_earth / sin(lat); sidereal day ~ 0.997 d.
        let sin_l = lat.sin().abs().max(1e-6);
        let hours = 23.934 / sin_l;
        Some(format!("lat={deg:.0}  T~{hours:.1}h  precess"))
    }

    fn reveal(&self) -> &'static str {
        "Foucault's 1851 pendulum proved Earth rotates: the swing plane turns \
         relative to the floor at Omega sin(latitude). At the pole a full turn \
         takes one sidereal day; at the equator the plane holds still."
    }
}

#[cfg(test)]
mod tests {
    use super::Foucault;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Foucault::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("lat"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn lat_changes() {
        let r = Foucault::new();
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
        Foucault::new().render(&mut c, 0.65);
        assert!(c.ink_count() > 0);
    }
}

//! Kepler's second law: equal areas in equal times on an ellipse.
//!
//! DRAG: TUNE ECC. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

/// Largest eccentricity exposed by the room's hand control.
pub const MAX_ECCENTRICITY: f64 = 0.9;
const ORBIT_SAMPLES: usize = 120;
const AREA_SECTORS: usize = 6;

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

fn ecc(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.03
    };
    if let Some((x, _)) = hand {
        (x * 0.85 + s).clamp(0.0, MAX_ECCENTRICITY)
    } else {
        (phase_unit(t) * 0.75 + s).clamp(0.0, MAX_ECCENTRICITY)
    }
}

/// Eccentricity rendered for this exact replayable input sequence.
#[must_use]
pub fn eccentricity_for_inputs(t: f64, inputs: &[RoomInput], seed: u64) -> f64 {
    let hand = inputs.iter().rev().find_map(|input| match *input {
        RoomInput::PointerDown { x, y, .. }
        | RoomInput::PointerMove { x, y, .. }
        | RoomInput::PointerUp { x, y, .. }
            if x.is_finite() && y.is_finite() =>
        {
            Some((x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)))
        }
        _ => None,
    });
    ecc(t, hand, seed)
}

/// Solve Kepler's equation `M = E - e sin(E)` for eccentric anomaly.
///
/// The room admits only `0 <= e <= 0.9`, where bounded Newton iteration is
/// stable from `M` and gives substantially more precision than a pixel needs.
fn eccentric_anomaly(mean_anomaly: f64, eccentricity: f64) -> f64 {
    let mean = mean_anomaly.clamp(0.0, std::f64::consts::TAU);
    let e = eccentricity.clamp(0.0, MAX_ECCENTRICITY);
    let mut anomaly = mean;
    for _ in 0..12 {
        let residual = anomaly - e * anomaly.sin() - mean;
        let derivative = 1.0 - e * anomaly.cos();
        anomaly -= residual / derivative;
    }
    anomaly
}

#[derive(Clone, Copy)]
pub(super) struct OrbitGeometry {
    pub(super) cx: f64,
    pub(super) cy: f64,
    pub(super) a: f64,
    pub(super) b: f64,
    pub(super) focus_x: f64,
}

pub(super) fn orbit_geometry(width: usize, height: usize, e: f64) -> OrbitGeometry {
    let e = e.clamp(0.0, MAX_ECCENTRICITY);
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let a = (width.min(height) as f64) * 0.4;
    let b = a * (1.0 - e * e).sqrt();
    OrbitGeometry {
        cx,
        cy,
        a,
        b,
        focus_x: cx - a * e,
    }
}

pub(super) fn point_at_mean(geometry: OrbitGeometry, e: f64, mean: f64) -> (i32, i32) {
    let anomaly = eccentric_anomaly(mean, e);
    (
        (geometry.cx + geometry.a * anomaly.cos()).round() as i32,
        (geometry.cy - geometry.b * anomaly.sin() * 0.55).round() as i32,
    )
}

fn draw(canvas: &mut dyn Surface, e: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let e = e.clamp(0.0, MAX_ECCENTRICITY);
    let geometry = orbit_geometry(width, height, e);
    // ellipse
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=ORBIT_SAMPLES {
        let th = std::f64::consts::TAU * (i as f64 / ORBIT_SAMPLES as f64);
        let px = (geometry.cx + geometry.a * th.cos()).round() as i32;
        let py = (geometry.cy - geometry.b * th.sin() * 0.55).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
        }
        prev = Some((px, py));
    }
    // sun at focus
    let fxi = geometry.focus_x.round() as i32;
    let fyi = geometry.cy.round() as i32;
    canvas.line(fxi - 1, fyi, fxi + 1, fyi, 'o');
    canvas.line(fxi, fyi - 1, fxi, fyi + 1, 'o');
    // Equal mean-anomaly intervals are equal time intervals. Solving Kepler's
    // equation places their boundaries at exactly equal swept-area phases.
    let n_sec = AREA_SECTORS + if seed == 0 { 0 } else { (seed % 2) as usize };
    for s in 0..n_sec {
        let m1 = std::f64::consts::TAU * (s as f64) / n_sec as f64;
        let m2 = std::f64::consts::TAU * ((s + 1) as f64) / n_sec as f64;
        let (x1, y1) = point_at_mean(geometry, e, m1);
        let (x2, y2) = point_at_mean(geometry, e, m2);
        canvas.line(fxi, fyi, x1, y1, '.');
        canvas.line(fxi, fyi, x2, y2, '.');
        // chord of sector
        canvas.line(x1, y1, x2, y2, if s % 2 == 0 { '*' } else { '+' });
    }
}

/// Kepler equal-area room.
#[derive(Debug, Default)]
pub struct KeplerLaws {
    seed: u64,
}

impl KeplerLaws {
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

impl Room for KeplerLaws {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "kepler-laws",
            title: "Kepler Areas",
            wing: "Motion & Dynamics",
            blurb: "Equal areas in equal times on an ellipse.",
            accent: [100, 70, 30],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, ecc(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "kepler-laws",
            root: 8.18,
            tempo: 80,
            line: &[0, 3, 7, 12, 7, 3, 0, 12],
            encodes: "Kepler II: radius to the sun sweeps equal areas per time",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE ECC")
    }

    fn status(&self, t: f64) -> Option<String> {
        let e = ecc(t, None, self.seed);
        Some(format!("e={e:.2}  areas  DRAG:ECC"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let e = ecc(t, hands.last().copied(), self.seed);
        draw(canvas, e, self.seed ^ hands.len() as u64);
    }

    fn render_input(&self, canvas: &mut dyn Surface, t: f64, inputs: &[RoomInput]) {
        let e = eccentricity_for_inputs(t, inputs, self.seed);
        draw(canvas, e, self.seed ^ inputs.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let hand_exists = inputs.iter().any(|input| match *input {
            RoomInput::PointerDown { x, y, .. }
            | RoomInput::PointerMove { x, y, .. }
            | RoomInput::PointerUp { x, y, .. } => x.is_finite() && y.is_finite(),
            _ => false,
        });
        if !hand_exists {
            return self.status(t);
        }
        let e = eccentricity_for_inputs(t, inputs, self.seed);
        // peri/aphelion distance ratio (1-e)/(1+e) for a=1
        let ra_rp = if e < 0.99 {
            (1.0 + e) / (1.0 - e)
        } else {
            f64::INFINITY
        };
        if ra_rp.is_finite() {
            Some(format!("e={e:.3}  ra/rp={ra_rp:.2}  areas"))
        } else {
            Some(format!("e={e:.3}  parabolic"))
        }
    }

    fn reveal(&self) -> &'static str {
        "Kepler's second law: the line from the sun to a planet sweeps equal areas \
         in equal times. Near perihelion the planet races; near aphelion it crawls. \
         Angular momentum conservation is the modern reason."
    }
}

#[cfg(test)]
mod tests {
    use super::{KeplerLaws, eccentric_anomaly, eccentricity_for_inputs};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = KeplerLaws::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("areas"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn ecc_changes() {
        let r = KeplerLaws::new();
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
        KeplerLaws::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }

    #[test]
    fn equal_time_boundaries_have_equal_swept_area_phases() {
        let eccentricity = 0.82;
        for index in 0..=12 {
            let mean = std::f64::consts::TAU * index as f64 / 12.0;
            let anomaly = eccentric_anomaly(mean, eccentricity);
            let recovered = anomaly - eccentricity * anomaly.sin();
            assert!(
                (recovered - mean).abs() < 1.0e-12,
                "{index}: {recovered} != {mean}"
            );
        }
    }

    #[test]
    fn exact_replay_input_owns_the_eccentricity() {
        let inputs = [
            RoomInput::PointerDown {
                x: 0.2,
                y: 0.4,
                t: 0.1,
            },
            RoomInput::PointerUp {
                x: 0.8,
                y: 0.4,
                t: 0.2,
            },
        ];
        let e = eccentricity_for_inputs(0.0, &inputs, 0);
        assert!((e - 0.68).abs() < 1.0e-12, "{e}");

        let room = KeplerLaws::new();
        let mut replayed = Canvas::new(48, 24);
        room.render_input(&mut replayed, 0.0, &inputs);
        let mut exact = Canvas::new(48, 24);
        room.render_poked(&mut exact, 0.0, &[(0.2, 0.4), (0.8, 0.4)]);
        assert_eq!(replayed.to_text(), exact.to_text());
    }
}

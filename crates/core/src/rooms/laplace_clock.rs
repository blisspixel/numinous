//! Laplace's Clockwork: Io-Europa-Ganymede 1:2:4 and the forbidden triple.
//!
//! Three moons on resonant orbits. When mean motions lock 1:2:4, a triple
//! conjunction is avoided: Laplace resonance. DRAG: DETUNE A MOON. See
//! `docs/ROOMS.md`.

use std::f64::consts::TAU;

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

/// Mean motions; locked n1:n2:n3 = 4:2:1 roughly for Io:Europa:Ganymede periods inverted.
fn motions(detune: f64) -> (f64, f64, f64) {
    // Periods T ~ 1,2,4 => n = 2pi/T ~ 4,2,1 in relative units.
    let n1 = 4.0;
    let n2 = 2.0 * (1.0 + detune);
    let n3 = 1.0;
    (n1, n2, n3)
}

fn positions(t: f64, detune: f64, seed: u64) -> [(f64, f64); 3] {
    let (n1, n2, n3) = motions(detune);
    let phase = phase_unit(t) * TAU * 2.0
        + if seed == 0 {
            0.0
        } else {
            (seed % 5) as f64 * 0.2
        };
    // Laplace angle: lambda1 - 3 lambda2 + 2 lambda3 ~ 180 deg when locked.
    let th1 = n1 * phase;
    let th2 = n2 * phase;
    let th3 = n3 * phase;
    let r1 = 0.22;
    let r2 = 0.34;
    let r3 = 0.48;
    [
        (0.5 + r1 * th1.cos(), 0.5 + r1 * th1.sin()),
        (0.5 + r2 * th2.cos(), 0.5 + r2 * th2.sin()),
        (0.5 + r3 * th3.cos(), 0.5 + r3 * th3.sin()),
    ]
}

fn laplace_angle(t: f64, detune: f64) -> f64 {
    let (n1, n2, n3) = motions(detune);
    let phase = phase_unit(t) * TAU * 2.0;
    let l1 = n1 * phase;
    let l2 = n2 * phase;
    let l3 = n3 * phase;
    (l1 - 3.0 * l2 + 2.0 * l3).rem_euclid(TAU)
}

fn draw(canvas: &mut dyn Surface, moons: &[(f64, f64); 3]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let to_px = |x: f64, y: f64| -> (i32, i32) {
        (
            (x.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32,
            (y.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    let j = to_px(0.5, 0.5);
    for d in -2..=2 {
        canvas.plot(j.0 + d, j.1, '#');
        canvas.plot(j.0, j.1 + d, '#');
    }
    for r in [0.22, 0.34, 0.48] {
        let mut prev = to_px(0.5 + r, 0.5);
        for i in 1..=64 {
            let a = TAU * i as f64 / 64.0;
            let p = to_px(0.5 + r * a.cos(), 0.5 + r * a.sin());
            canvas.line(prev.0, prev.1, p.0, p.1, '.');
            prev = p;
        }
    }
    for (i, m) in moons.iter().enumerate() {
        let p = to_px(m.0, m.1);
        let ch = ['*', '+', 'o'][i];
        canvas.plot(p.0, p.1, ch);
        canvas.plot(p.0 + 1, p.1, ch);
    }
}

fn draw_detune_gauge(canvas: &mut dyn Surface, detune: f64) {
    if detune.abs() <= f64::EPSILON {
        return;
    }
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let center = width as f64 * 0.5;
    let reach = width as f64 * 0.22;
    let end = center + (detune / 0.2).clamp(-1.0, 1.0) * reach;
    let y = height as f64 * 0.88;
    let tick = (height as f64 * 0.035).clamp(2.0, 12.0);
    canvas.line(
        center.round() as i32,
        y.round() as i32,
        end.round() as i32,
        y.round() as i32,
        '!',
    );
    canvas.line(
        end.round() as i32,
        (y - tick).round() as i32,
        end.round() as i32,
        (y + tick).round() as i32,
        '!',
    );
}

/// Laplace's Clockwork room.
#[derive(Debug, Default)]
pub struct LaplaceClock {
    seed: u64,
}

impl LaplaceClock {
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

impl Room for LaplaceClock {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "laplace-clock",
            title: "Laplace's Clockwork",
            wing: "Motion & Dynamics",
            blurb: "Io, Europa, Ganymede lock 1:2:4; the Laplace angle avoids the triple \
                    conjunction. t turns the clock; DRAG: DETUNE A MOON and watch lock break.",
            accent: [220, 180, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let moons = positions(t, 0.0, self.seed);
        draw(canvas, &moons);
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "1:2:4 lock",
            root: 174.61,
            tempo: 120,
            line: &[0, 4, 7, 12, 7, 4, 0, 12],
            encodes: "three clocks locked so a triple meet never lands",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: DETUNE A MOON")
    }

    fn status(&self, t: f64) -> Option<String> {
        let ang = laplace_angle(t, 0.0).to_degrees();
        Some(format!("L={ang:.0}deg  1:2:4  DRAG:DETUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let detune = hands.last().map(|(x, _)| (*x - 0.5) * 0.4).unwrap_or(0.0);
        let moons = positions(t, detune, self.seed);
        draw(canvas, &moons);
        draw_detune_gauge(canvas, detune);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let detune = (hands.last().unwrap().0 - 0.5) * 0.4;
        let ang = laplace_angle(t, detune).to_degrees();
        let lock = if detune.abs() < 0.03 { "LOCK" } else { "DRIFT" };
        Some(format!("DETUNE {detune:+.2}  L={ang:.0}  {lock}"))
    }

    fn reveal(&self) -> &'static str {
        "Io, Europa, and Ganymede keep mean motions near 4:2:1. The Laplace \
         angle lambda_I - 3 lambda_E + 2 lambda_G librates about 180 degrees, so \
         a triple conjunction is forbidden: orbital clockwork that protects the \
         resonance."
    }
}

#[cfg(test)]
mod tests {
    use super::LaplaceClock;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = LaplaceClock::new().status(0.0).unwrap();
        assert!(s.contains("DRAG") || s.contains("DETUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn detune_changes() {
        let r = LaplaceClock::new();
        let o = r.status(0.0).unwrap();
        let a = r
            .status_input(
                0.0,
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
    fn detune_draws_a_visible_drift_gauge() {
        let room = LaplaceClock::new();
        let mut base = Canvas::new(80, 40);
        let mut detuned = Canvas::new(80, 40);
        room.render(&mut base, 0.4);
        room.render_poked(&mut detuned, 0.4, &[(0.98, 0.5)]);
        let changed = (0..40)
            .flat_map(|y| (0..80).map(move |x| (x, y)))
            .filter(|&(x, y)| base.cell(x, y) != detuned.cell(x, y))
            .count();
        assert!(changed >= 8, "detune gauge changed only {changed} cells");
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        LaplaceClock::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(LaplaceClock::new().motif().unwrap().line.len() >= 6);
    }
}

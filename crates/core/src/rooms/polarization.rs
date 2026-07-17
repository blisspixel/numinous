//! Malus's law: intensity through two polarizers, I = I0 cos^2 theta.
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

fn angle(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        x * std::f64::consts::PI + s
    } else {
        phase_unit(t) * std::f64::consts::PI + s
    }
}

fn malus_intensity(theta: f64) -> f64 {
    if theta.is_finite() {
        theta.cos().powi(2).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn polarization_grade(intensity: f64) -> &'static str {
    if intensity >= 0.9 {
        "OPEN"
    } else if intensity <= 0.01 {
        "DARK"
    } else {
        "DIM"
    }
}

fn next_unit(state: &mut u64) -> f64 {
    *state = state.wrapping_mul(0x5851_f42d_4c95_7f2d).wrapping_add(1);
    ((*state >> 32) as u32) as f64 / (u32::MAX as f64 + 1.0)
}

fn draw(canvas: &mut dyn Surface, theta: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let intensity = malus_intensity(theta);
    // field of dots with density ~ intensity
    let mut state = seed ^ 0xc01a_b15e_c0de;
    for y in 0..height {
        for x in 0..width {
            if next_unit(&mut state) < intensity * 0.85 {
                canvas.plot(x as i32, y as i32, if intensity > 0.5 { '#' } else { '*' });
            }
        }
    }
    // two polarizer bars
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let len = width.min(height) as f64 * 0.35;
    // fixed vertical
    canvas.line(
        cx.round() as i32,
        (cy - len).round() as i32,
        cx.round() as i32,
        (cy + len).round() as i32,
        '|',
    );
    // rotatable
    let dx = len * theta.sin();
    let dy = len * theta.cos();
    canvas.line(
        (cx - dx).round() as i32,
        (cy - dy).round() as i32,
        (cx + dx).round() as i32,
        (cy + dy).round() as i32,
        '/',
    );
}

/// Polarization / Malus room.
#[derive(Debug, Default)]
pub struct Polarization {
    seed: u64,
}

impl Polarization {
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

impl Room for Polarization {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "polarization",
            title: "Polarization",
            wing: "Waves & Sound",
            blurb: "Malus: intensity falls as cos squared of angle. t and DRAG: TUNE ANGLE.",
            accent: [180, 40, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, angle(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "polarization",
            root: 698.46,
            tempo: 70,
            line: &[0, 0, 5, 7, 12, 7, 5, 0],
            encodes: "crossed polarizers darken as cos squared",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE ANGLE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let th = angle(t, None, self.seed);
        let intensity = malus_intensity(th);
        Some(format!("I={:.0}%  pol  DRAG:ANG", intensity * 100.0))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let th = angle(t, hands.last().copied(), self.seed);
        draw(canvas, th, self.seed);
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
        let th = angle(t, hands.last().copied(), self.seed);
        let intensity = malus_intensity(th);
        let grade = polarization_grade(intensity);
        Some(format!(
            "{grade} I={:.1}%  angle={:.1}deg",
            intensity * 100.0,
            th.to_degrees()
        ))
    }

    fn reveal(&self) -> &'static str {
        "Malus's law says transmitted intensity through two polarizers is \
         I0 cos^2 theta. Crossed filters go dark; aligned ones stay bright. \
         Light is a transverse wave with a preferred plane."
    }
}

#[cfg(test)]
mod tests {
    use super::{Polarization, malus_intensity, next_unit, polarization_grade};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Polarization::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("pol") || s.contains("ANG"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn angle_changes() {
        let r = Polarization::new();
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
    fn malus_law_hits_aligned_and_crossed_limits() {
        let aligned = malus_intensity(0.0);
        let crossed = malus_intensity(std::f64::consts::FRAC_PI_2);
        let half = malus_intensity(std::f64::consts::FRAC_PI_4);
        assert!((aligned - 1.0).abs() < 1e-12);
        assert!(crossed < 1e-12);
        assert!((half - 0.5).abs() < 1e-12);
        assert_eq!(polarization_grade(aligned), "OPEN");
        assert_eq!(polarization_grade(crossed), "DARK");
        assert_eq!(polarization_grade(half), "DIM");
    }

    #[test]
    fn density_sampler_covers_the_unit_interval() {
        let mut state = 0xc01a_b15e_c0de;
        let samples: Vec<_> = (0..4096).map(|_| next_unit(&mut state)).collect();
        let min = samples.iter().copied().fold(f64::INFINITY, f64::min);
        let max = samples.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!(min < 0.05, "min={min}");
        assert!(max > 0.95, "max={max}");
        assert!((mean - 0.5).abs() < 0.03, "mean={mean}");
    }

    #[test]
    fn duplicate_hand_history_does_not_resample_the_field() {
        let room = Polarization::new();
        let hand = (0.4, 0.5);
        let mut single = Canvas::new(48, 24);
        let mut duplicate = Canvas::new(48, 24);
        room.render_poked(&mut single, 0.3, &[hand]);
        room.render_poked(&mut duplicate, 0.3, &[hand, hand]);
        assert_eq!(single.to_text(), duplicate.to_text());
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Polarization::new().render(&mut c, 0.35);
        assert!(c.ink_count() > 0);
    }
}

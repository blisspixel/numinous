//! Oregonator: three-variable oscillatory model of the Belousov-Zhabotinsky reaction.
//!
//! DRAG: TUNE F. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const STEPS: usize = 2_000;
const DT: f64 = 0.01;

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

fn f_param(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        0.5 + x * 2.0 + s
    } else {
        0.8 + phase_unit(t) * 1.2 + s
    }
}

fn draw(canvas: &mut dyn Surface, f: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let eps = 0.04;
    let q = 0.0008;
    let mut x = 0.5
        + if seed == 0 {
            0.0
        } else {
            (seed % 5) as f64 * 0.02
        };
    let mut y = 0.5;
    let mut z = 0.5;
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_z = f64::MAX;
    let mut max_z = f64::MIN;
    let mut pts = Vec::with_capacity(STEPS);
    for _ in 0..200 {
        let dx = (x * (1.0 - x) + f * (q - x) * z / (q + x)) / eps;
        let dy = x - y;
        let dz = y - z;
        x = (x + dx * DT).max(1e-6);
        y = (y + dy * DT).max(1e-6);
        z = (z + dz * DT).max(1e-6);
    }
    for _ in 0..STEPS {
        let dx = (x * (1.0 - x) + f * (q - x) * z / (q + x)) / eps;
        let dy = x - y;
        let dz = y - z;
        x = (x + dx * DT).max(1e-6);
        y = (y + dy * DT).max(1e-6);
        z = (z + dz * DT).max(1e-6);
        if !x.is_finite() || !z.is_finite() {
            break;
        }
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_z = min_z.min(z);
        max_z = max_z.max(z);
        pts.push((x, z));
    }
    let dx = (max_x - min_x).max(1e-6);
    let dz = (max_z - min_z).max(1e-6);
    for (i, &(px, pz)) in pts.iter().enumerate() {
        let u = ((px - min_x) / dx).clamp(0.0, 1.0);
        let v = ((pz - min_z) / dz).clamp(0.0, 1.0);
        let ix = (u * width.saturating_sub(1) as f64).round() as i32;
        let iy = ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32;
        canvas.plot(ix, iy, if i % 19 == 0 { '#' } else { '*' });
    }
}

/// Oregonator room.
#[derive(Debug, Default)]
pub struct Oregonator {
    seed: u64,
}

impl Oregonator {
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

impl Room for Oregonator {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "oregonator",
            title: "Oregonator",
            wing: "Motion & Dynamics",
            blurb: "BZ chemical clock reduced to three variables. t and DRAG: TUNE F.",
            accent: [200, 40, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, f_param(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "oregonator",
            root: 587.33,
            tempo: 106,
            line: &[0, 4, 7, 11, 14, 11, 7, 4],
            encodes: "chemical oscillation reduced to three ODEs",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE F")
    }

    fn status(&self, t: f64) -> Option<String> {
        let f = f_param(t, None, self.seed);
        Some(format!("f={f:.2}  BZ  DRAG:F"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let f = f_param(t, hands.last().copied(), self.seed);
        draw(canvas, f, self.seed ^ hands.len() as u64);
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
        let f = f_param(t, hands.last().copied(), self.seed);
        Some(format!("F={f:.3}  oregonator"))
    }

    fn reveal(&self) -> &'static str {
        "The Oregonator is Field and Noyes's three-variable reduction of the \
         Belousov-Zhabotinsky reaction. Stoichiometric factor f steers the \
         chemical clock between steady and oscillatory regimes."
    }
}

#[cfg(test)]
mod tests {
    use super::Oregonator;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Oregonator::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("F"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn f_changes() {
        let r = Oregonator::new();
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
        Oregonator::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

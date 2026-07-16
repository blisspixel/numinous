//! Sierpinski Carpet: remove the middle ninth forever.
//!
//! Start with a square; punch the center; recurse on the eight remaining.
//! DRAG: DEEPEN THE CUT. See `docs/ROOMS.md`.

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

fn depth(t: f64, hand: Option<(f64, f64)>) -> u32 {
    if let Some((x, _)) = hand {
        (1 + (x * 4.0) as u32).clamp(1, 5)
    } else {
        (1 + (phase_unit(t) * 3.0) as u32).clamp(1, 4)
    }
}

/// True if (u,v) in unit square survives `d` carpet removals.
fn in_carpet(mut u: f64, mut v: f64, d: u32) -> bool {
    for _ in 0..d {
        let ix = (u * 3.0).floor() as i32;
        let iy = (v * 3.0).floor() as i32;
        if ix == 1 && iy == 1 {
            return false;
        }
        u = (u * 3.0).rem_euclid(1.0);
        v = (v * 3.0).rem_euclid(1.0);
    }
    true
}

fn draw(canvas: &mut dyn Surface, d: u32, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let margin = 0.08;
    let _ = seed;
    for py in 0..height {
        for px in 0..width {
            let u = px as f64 / width.saturating_sub(1).max(1) as f64;
            let v = py as f64 / height.saturating_sub(1).max(1) as f64;
            if u < margin || u > 1.0 - margin || v < margin || v > 1.0 - margin {
                continue;
            }
            let uu = (u - margin) / (1.0 - 2.0 * margin);
            let vv = (v - margin) / (1.0 - 2.0 * margin);
            if in_carpet(uu, vv, d) {
                canvas.plot(px as i32, py as i32, if d >= 3 { '#' } else { '*' });
            }
        }
    }
}

/// Sierpinski Carpet room.
#[derive(Debug, Default)]
pub struct SierpinskiCarpet {
    seed: u64,
}

impl SierpinskiCarpet {
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

impl Room for SierpinskiCarpet {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "sierpinski-carpet",
            title: "The Carpet",
            wing: "Fractals",
            blurb: "Sierpinski carpet: punch the middle ninth, forever. Area vanishes; dimension \
                    stays between 1 and 2. t and DRAG: DEEPEN THE CUT.",
            accent: [200, 100, 140],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let d = depth(t, None)
            + if self.seed == 0 {
                0
            } else {
                (self.seed % 2) as u32
            };
        draw(canvas, d.min(5), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "carpet",
            root: 130.81,
            tempo: 88,
            line: &[0, 0, 5, 7, 12, 7, 5, 0],
            encodes: "middle ninths punched until area is zero",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: DEEPEN THE CUT")
    }

    fn status(&self, t: f64) -> Option<String> {
        let d = depth(t, None);
        // Remaining area fraction (8/9)^d
        let area = (8.0_f64 / 9.0).powi(d as i32);
        Some(format!("depth={d}  area~{area:.3}  DRAG:CUT"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let d = depth(t, hands.last().copied());
        draw(canvas, d, self.seed);
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
        let d = depth(t, hands.last().copied());
        let area = (8.0_f64 / 9.0).powi(d as i32);
        // Hausdorff dim log 8 / log 3
        let dim = 8.0_f64.ln() / 3.0_f64.ln();
        Some(format!("CUT d={d}  area={area:.3}  dim={dim:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "The Sierpinski carpet removes the open middle ninth of each square, \
         forever. Remaining area goes to zero while Hausdorff dimension is \
         log(8)/log(3), a fractal fabric between line and plane."
    }
}

#[cfg(test)]
mod tests {
    use super::{SierpinskiCarpet, in_carpet};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = SierpinskiCarpet::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("CUT"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn cut_changes() {
        let r = SierpinskiCarpet::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
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
    fn center_removed() {
        assert!(!in_carpet(0.5, 0.5, 1));
        assert!(in_carpet(0.1, 0.1, 1));
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(36, 28);
        SierpinskiCarpet::new().render(&mut c, 0.4);
        assert!(c.ink_count() > 50);
    }

    #[test]
    fn motif_ok() {
        assert!(SierpinskiCarpet::new().motif().unwrap().line.len() >= 6);
    }
}

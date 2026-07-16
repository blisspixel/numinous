//! The Lens: Einstein rings from a mass you never see.
//!
//! A dark mass on the plate bends light from a background source into arcs
//! and rings (thin-lens gravitational lensing toy). DRAG: MOVE THE DARK MASS.
//! See `docs/ROOMS.md`.

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

fn einstein_radius(mass: f64) -> f64 {
    (0.08 * mass).sqrt().clamp(0.05, 0.35)
}

fn draw(canvas: &mut dyn Surface, lens: (f64, f64), source: (f64, f64), mass: f64) {
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
    let re = einstein_radius(mass);
    // Dark mass: invisible except as a faint cross of absence.
    let lp = to_px(lens.0, lens.1);
    canvas.plot(lp.0, lp.1, '+');
    // Source true position (behind).
    let sp = to_px(source.0, source.1);
    canvas.plot(sp.0, sp.1, 'o');
    // Image ring/arcs: points on Einstein ring around lens, weighted by alignment.
    let dx = source.0 - lens.0;
    let dy = source.1 - lens.1;
    let align = 1.0 - (dx.hypot(dy) / 0.4).clamp(0.0, 1.0);
    let n = 64;
    for i in 0..n {
        let a = TAU * i as f64 / n as f64;
        // Offset ring toward source side for arcs when misaligned.
        let ox = lens.0 + re * a.cos() + dx * 0.15 * (1.0 - align);
        let oy = lens.1 + re * a.sin() + dy * 0.15 * (1.0 - align);
        let p = to_px(ox, oy);
        let ch = if align > 0.7 { '#' } else { '*' };
        canvas.plot(p.0, p.1, ch);
    }
}

/// The Lens room.
#[derive(Debug, Default)]
pub struct TheLens {
    seed: u64,
}

impl TheLens {
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

impl Room for TheLens {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "the-lens",
            title: "The Lens",
            wing: "Shape & Space",
            blurb: "A mass you never see bends background light into Einstein rings and arcs. t \
                    grows the mass; DRAG: MOVE THE DARK MASS.",
            accent: [160, 140, 200],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let lens = (
            0.5 + if self.seed == 0 {
                0.0
            } else {
                ((self.seed % 5) as f64 - 2.0) * 0.02
            },
            0.5,
        );
        let source = (0.5, 0.5 - phase_unit(t) * 0.05);
        let mass = 0.5 + phase_unit(t);
        draw(canvas, lens, source, mass);
    }

    fn postcard_t(&self) -> f64 {
        0.2
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "einstein ring",
            root: 185.0,
            tempo: 96,
            line: &[0, 5, 7, 12, 7, 5, 0, 7],
            encodes: "invisible mass drawing a ring of light",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: MOVE THE DARK MASS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let mass = 0.5 + phase_unit(t);
        let re = einstein_radius(mass);
        Some(format!("M={mass:.2}  Re={re:.2}  DRAG:MASS"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let lens = hands.last().copied().unwrap_or((0.5, 0.5));
        let source = (0.5, 0.48);
        let mass = 0.5 + phase_unit(t);
        draw(canvas, lens, source, mass);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (x, y) = *hands.last().unwrap();
        let mass = 0.5 + phase_unit(t);
        let re = einstein_radius(mass);
        let off = (x - 0.5).hypot(y - 0.5);
        let shape = if off < 0.05 { "RING" } else { "ARCS" };
        Some(format!(
            "MASS@{:.0}%{:.0}%  Re={re:.2}  {shape}",
            x * 100.0,
            y * 100.0
        ))
    }

    fn reveal(&self) -> &'static str {
        "Mass bends null geodesics. When a compact mass sits almost on the line \
         of sight to a source, the images form Einstein rings and arcs: light from \
         a body you may never see, drawn by a mass you also may not."
    }
}

#[cfg(test)]
mod tests {
    use super::TheLens;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = TheLens::new().status(0.0).unwrap();
        assert!(s.contains("DRAG") || s.contains("MASS"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn mass_changes() {
        let r = TheLens::new();
        let o = r.status(0.0).unwrap();
        let a = r
            .status_input(
                0.0,
                &[RoomInput::PointerDown {
                    x: 0.6,
                    y: 0.4,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        TheLens::new().render(&mut c, 0.3);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(TheLens::new().motif().unwrap().line.len() >= 6);
    }
}

//! Whitney umbrella: the classic cross-cap singularity surface.
//!
//! DRAG: TUNE SLICE. See `docs/ROOMS.md`.

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

fn slice(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.05
    };
    if let Some((x, _)) = hand {
        -1.2 + x * 2.4 + s
    } else {
        -1.0 + phase_unit(t) * 2.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, u0: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let sc = (width.min(height) as f64) * 0.28;
    // Whitney umbrella: x = u v, y = u, z = v^2
    let u_steps = 24;
    let v_steps = 40;
    let u_mid = u0.clamp(-1.4, 1.4);
    for ui in 0..=u_steps {
        let u = -1.5 + 3.0 * (ui as f64 / u_steps as f64);
        let mut prev: Option<(i32, i32)> = None;
        for vi in 0..=v_steps {
            let v = -1.4 + 2.8 * (vi as f64 / v_steps as f64);
            let x = u * v;
            let y = u;
            let z = v * v;
            // Prefer curves near the selected u slice.
            let w = 1.0 - (u - u_mid).abs() * 0.6;
            if w < 0.2 {
                prev = None;
                continue;
            }
            let tilt = if seed == 0 {
                0.0
            } else {
                (seed % 4) as f64 * 0.05
            };
            let xr = x * tilt.cos() - y * tilt.sin();
            let yr = x * tilt.sin() + y * tilt.cos();
            let px = (cx + xr * sc).round() as i32;
            let py = (cy - (yr * 0.5 + z * 0.35) * sc).round() as i32;
            if let Some((ox, oy)) = prev {
                let ch = if (u - u_mid).abs() < 0.12 { '#' } else { '.' };
                canvas.line(ox, oy, px, py, ch);
            }
            prev = Some((px, py));
        }
    }
    // Handle line (self-intersection): x=0, y=0, z>=0 half-line in projection.
    canvas.line(
        cx as i32,
        cy as i32,
        cx as i32,
        (cy - 1.2 * sc * 0.35).round() as i32,
        '|',
    );
}

/// Whitney umbrella room.
#[derive(Debug, Default)]
pub struct WhitneyUmbrella {
    seed: u64,
}

impl WhitneyUmbrella {
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

impl Room for WhitneyUmbrella {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "whitney-umbrella",
            title: "Whitney Umbrella",
            wing: "Shape & Space",
            blurb: "Cross-cap singularity x=uv, y=u, z=v^2. t and DRAG: TUNE SLICE.",
            accent: [90, 70, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, slice(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "whitney-umbrella",
            root: 65.41,
            tempo: 76,
            line: &[0, 2, 5, 7, 12, 7, 5, 2],
            encodes: "Whitney umbrella: stable cross-cap singularity of surfaces",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE SLICE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let u = slice(t, None, self.seed);
        Some(format!("u={u:.2}  cross  DRAG:SLC"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let u = slice(t, hands.last().copied(), self.seed);
        draw(canvas, u, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let u = slice(t, hands.last().copied(), self.seed);
        // Whitney: x^2 = y^2 z; at fixed u the section half-width is |u|.
        let half = u.abs();
        Some(format!("u={u:.2}  half={half:.2}  x^2=y^2z"))
    }

    fn reveal(&self) -> &'static str {
        "The Whitney umbrella is the surface x = u v, y = u, z = v^2. It is the \
         local model of a cross-cap: a self-intersecting handle with a pinch line \
         that is the archetype of stable singularities of surfaces in 3-space."
    }
}

#[cfg(test)]
mod tests {
    use super::WhitneyUmbrella;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = WhitneyUmbrella::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("cross"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn slice_changes() {
        let r = WhitneyUmbrella::new();
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
        WhitneyUmbrella::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 0);
    }
}

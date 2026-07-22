//! Fermat (parabolic) spiral: r^2 = a^2 theta.
//!
//! Ambient phase unfurls both equal-area arms. DRAG: TUNE TURNS.
//! See `docs/ROOMS.md`.

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

fn turns(_t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 4) as f64 * 0.2
    };
    if let Some((x, _)) = hand {
        1.0 + x * 6.0 + s
    } else {
        // Ambient turns hold a readable pair of arms; motion lives in unfurl.
        4.0 + s
    }
}

fn draw(canvas: &mut dyn Surface, n_turns: f64, unfurl: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let cx = (width.saturating_sub(1) / 2) as f64;
    let cy = (height.saturating_sub(1) / 2) as f64;
    let max_th = n_turns.clamp(0.5, 10.0) * std::f64::consts::TAU;
    let a = (width.min(height) as f64) * 0.42 / max_th.sqrt().max(1.0);
    let rot = if seed == 0 {
        0.0
    } else {
        (seed % 9) as f64 * 0.05
    };
    let unfurl = unfurl.clamp(0.0, 1.0);
    // Unfurl grows both arms from the center outward.
    let active = 0.12 + 0.88 * unfurl;
    let steps = 500;
    let drawn = ((active * steps as f64).round() as usize).min(steps);
    // Soft ghost of the full double arm.
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let th = max_th * (i as f64 / steps as f64);
        let r = a * th.sqrt();
        let ang = th + rot;
        let px = (cx + r * ang.cos()).round() as i32;
        let py = (cy - r * ang.sin()).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    prev = None;
    for i in 0..=steps {
        let th = max_th * (i as f64 / steps as f64);
        let r = a * th.sqrt();
        let ang = th + rot + std::f64::consts::PI;
        let px = (cx + r * ang.cos()).round() as i32;
        let py = (cy - r * ang.sin()).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '.');
        }
        prev = Some((px, py));
    }
    // Bright arms so far.
    prev = None;
    let mut tip_a = (cx.round() as i32, cy.round() as i32);
    for i in 0..=drawn {
        let th = max_th * (i as f64 / steps as f64);
        let r = a * th.sqrt();
        let ang = th + rot;
        let px = (cx + r * ang.cos()).round() as i32;
        let py = (cy - r * ang.sin()).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
            canvas.line(ox, oy + 1, px, py + 1, '*');
        }
        tip_a = (px, py);
        prev = Some((px, py));
    }
    prev = None;
    let mut tip_b = (cx.round() as i32, cy.round() as i32);
    for i in 0..=drawn {
        let th = max_th * (i as f64 / steps as f64);
        let r = a * th.sqrt();
        let ang = th + rot + std::f64::consts::PI;
        let px = (cx + r * ang.cos()).round() as i32;
        let py = (cy - r * ang.sin()).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, '#');
            canvas.line(ox, oy + 1, px, py + 1, '*');
        }
        tip_b = (px, py);
        prev = Some((px, py));
    }
    for tip in [tip_a, tip_b] {
        for dy in -2..=2 {
            for dx in -2..=2 {
                if dx * dx + dy * dy <= 5 {
                    canvas.plot(tip.0 + dx, tip.1 + dy, 'o');
                }
            }
        }
    }
}

/// Fermat spiral room.
#[derive(Debug, Default)]
pub struct FermatSpiral {
    seed: u64,
}

impl FermatSpiral {
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

impl Room for FermatSpiral {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "fermat-spiral",
            title: "Fermat Spiral",
            wing: "Shape & Space",
            blurb: "Equal-area arms unfurl together. Watch the tips; DRAG: TUNE TURNS.",
            accent: [200, 160, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, turns(t, None, self.seed), phase_unit(t), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "fermat spiral",
            root: 554.37,
            tempo: 92,
            line: &[0, 2, 5, 9, 12, 9, 5, 2],
            encodes: "equal-area arms of a square-root spiral",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE TURNS")
    }

    fn status(&self, t: f64) -> Option<String> {
        let n = turns(t, None, self.seed);
        let p = (phase_unit(t) * 100.0).round() as i32;
        Some(format!("turns={n:.1}  unfurl={p}%  DRAG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let n = turns(t, hands.last().copied(), self.seed);
        let show = hands
            .last()
            .map(|&(_, y)| y)
            .unwrap_or_else(|| phase_unit(t));
        draw(canvas, n, show, self.seed ^ hands.len() as u64);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let n = turns(t, hands.last().copied(), self.seed);
        // Fermat: r^2 = a^2 theta; outer r grows as sqrt(turns).
        let theta = n * std::f64::consts::TAU;
        let r_out = theta.max(0.0).sqrt();
        Some(format!("turns={n:.2}  r~{r_out:.2}  r^2~th"))
    }

    fn reveal(&self) -> &'static str {
        "Fermat's spiral satisfies r^2 = a^2 theta. Adjacent arms enclose equal \
         areas, so it appears in phyllotaxis models and in the packing of \
         florets: nature's equal-share curve."
    }
}

#[cfg(test)]
mod tests {
    use super::FermatSpiral;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = FermatSpiral::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("unfurl"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn turns_change() {
        let r = FermatSpiral::new();
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
    fn ambient_unfurl_moves_the_plate() {
        let r = FermatSpiral::new();
        let mut a = Canvas::new(80, 48);
        let mut b = Canvas::new(80, 48);
        r.render(&mut a, 0.15);
        r.render(&mut b, 0.75);
        assert_ne!(a.to_text(), b.to_text(), "arms must unfurl");
        assert!(a.ink_count() > 40);
        assert!(b.ink_count() > 40);
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        FermatSpiral::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}

//! The Stretch: everyone is the center; redshift an octave down.
//!
//! Click a galaxy: Hubble flow makes every other galaxy recede from that
//! choice. Recession velocity v = H0 * d; redshift z grows with distance.
//! CLICK any galaxy. See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const N: usize = 64;
const SEED: u64 = 0x57E7_5EED_0000_0001;

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

fn galaxies(seed: u64) -> Vec<(f64, f64)> {
    let mut rng = SplitMix64::new(SEED ^ seed);
    (0..N).map(|_| (rng.next_f64(), rng.next_f64())).collect()
}

fn nearest(gs: &[(f64, f64)], p: (f64, f64)) -> usize {
    gs.iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let da = (a.0 - p.0).hypot(a.1 - p.1);
            let db = (b.0 - p.0).hypot(b.1 - p.1);
            da.total_cmp(&db)
        })
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn draw(canvas: &mut dyn Surface, gs: &[(f64, f64)], center: usize, h0: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let (cx, cy) = gs[center];
    for (i, &(x, y)) in gs.iter().enumerate() {
        let px = (x * width.saturating_sub(1) as f64).round() as i32;
        let py = (y * height.saturating_sub(1) as f64).round() as i32;
        if i == center {
            for dy in -2..=2 {
                for dx in -2..=2 {
                    if dx * dx + dy * dy <= 5 {
                        canvas.plot(px + dx, py + dy, '#');
                    }
                }
            }
            continue;
        }
        let d = (x - cx).hypot(y - cy);
        let z = h0 * d;
        let ch = if z > 0.45 {
            '.'
        } else if z > 0.22 {
            '*'
        } else {
            '#'
        };
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx * dx + dy * dy <= 2 {
                    canvas.plot(px + dx, py + dy, ch);
                }
            }
        }
        // Recession ray scales with redshift so the stretch is visible.
        if d > 1e-3 {
            let ux = (x - cx) / d;
            let uy = (y - cy) / d;
            let len = 0.06 + 0.1 * z.min(1.2);
            let ex = x + ux * len;
            let ey = y + uy * len;
            let qx = (ex * width.saturating_sub(1) as f64).round() as i32;
            let qy = (ey * height.saturating_sub(1) as f64).round() as i32;
            canvas.line(px, py, qx, qy, '.');
        }
    }
}

/// The Stretch room.
#[derive(Debug, Default)]
pub struct TheStretch {
    seed: u64,
}

impl TheStretch {
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

impl Room for TheStretch {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "the-stretch",
            title: "The Stretch",
            wing: "Shape & Space",
            blurb: "Click any galaxy: everyone is the center. Hubble flow makes the rest recede; \
                    redshift grows with distance. t sets H0; CLICK a galaxy to stand there.",
            accent: [180, 100, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let gs = galaxies(self.seed);
        let h0 = 0.5 + phase_unit(t);
        // Ambient stands at the plate center's nearest galaxy, not always index 0,
        // so a corner click is unlikely to re-select the same home.
        let home = nearest(&gs, (0.5, 0.5));
        draw(canvas, &gs, home, h0);
    }

    fn postcard_t(&self) -> f64 {
        0.5
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "hubble flow",
            root: 146.83,
            tempo: 88,
            line: &[0, 5, 7, 12, 12, 7, 5, 0],
            encodes: "recession stretching pitch down with distance",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: PICK A GALAXY")
    }

    fn status(&self, t: f64) -> Option<String> {
        let h0 = 0.5 + phase_unit(t);
        let gs = galaxies(self.seed);
        let home = nearest(&gs, (0.5, 0.5));
        Some(format!("H0={h0:.2}  CENTER {home}  CLICK:GALAXY"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let gs = galaxies(self.seed);
        // Hand y also steers H0 so a poke always moves the plate even if the
        // nearest galaxy matches ambient home.
        let (c, h0) = if let Some(&(x, y)) = hands.last() {
            (nearest(&gs, (x, y)), 0.35 + y * 1.4)
        } else {
            (nearest(&gs, (0.5, 0.5)), 0.5 + phase_unit(t))
        };
        draw(canvas, &gs, c, h0);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let gs = galaxies(self.seed);
        let &(hx, hy) = hands.last().unwrap();
        let h0 = 0.35 + hy * 1.4;
        let c = nearest(&gs, (hx, hy));
        let (cx, cy) = gs[c];
        let mut max_z: f64 = 0.0;
        for (i, &(x, y)) in gs.iter().enumerate() {
            if i == c {
                continue;
            }
            max_z = max_z.max(h0 * (x - cx).hypot(y - cy));
        }
        Some(format!("HERE g{c}  H0={h0:.2}  zmax={max_z:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "In expanding space there is no privileged center: stand on any galaxy \
         and the rest stream away. Hubble's law is local geometry of expansion; \
         redshift is the stretch of the metric, not a Doppler exhaust."
    }
}

#[cfg(test)]
mod tests {
    use super::TheStretch;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = TheStretch::new().status(0.0).unwrap();
        assert!(s.contains("CLICK") || s.contains("GALAXY"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn poke_moves_the_plate() {
        let r = TheStretch::new();
        let mut base = Canvas::new(120, 70);
        let mut poked = Canvas::new(120, 70);
        r.render(&mut base, 0.5);
        r.render_poked(&mut poked, 0.5, &[(0.8, 0.5)]);
        assert_ne!(
            base.to_text(),
            poked.to_text(),
            "click must stretch the field"
        );
    }

    #[test]
    fn click_changes() {
        let r = TheStretch::new();
        let o = r.status(0.0).unwrap();
        let a = r
            .status_input(
                0.0,
                &[RoomInput::PointerDown {
                    x: 0.8,
                    y: 0.2,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
        assert!(a.chars().count() <= 56);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        TheStretch::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 20);
    }

    #[test]
    fn motif_ok() {
        assert!(TheStretch::new().motif().unwrap().line.len() >= 6);
    }
}

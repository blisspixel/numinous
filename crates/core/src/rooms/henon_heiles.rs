//! Henon-Heiles system: galactic dynamics chaos (toy continuous flow).
//!
//! Contour energy orbits in the famous potential. DRAG: TUNE ENERGY.
//! See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const STEPS: usize = 5_000;
const DT: f64 = 0.02;

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

fn energy(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.005
    };
    if let Some((x, _)) = hand {
        0.05 + x * 0.15 + s
    } else {
        0.08 + phase_unit(t) * 0.08 + s
    }
}

fn integrate(e: f64) -> Vec<(f64, f64)> {
    // Start near origin with velocity set by energy budget (toy).
    let mut x: f64 = 0.0;
    let mut y: f64 = 0.1;
    let mut px: f64 = (2.0 * e).sqrt() * 0.5;
    let mut py: f64 = (2.0 * e).sqrt() * 0.5;
    let mut out = Vec::with_capacity(STEPS);
    for _ in 0..STEPS {
        // V = 0.5(x^2+y^2) + x^2 y - y^3/3
        // Fx = -dV/dx = -x - 2 x y
        // Fy = -dV/dy = -y - x^2 + y^2
        let fx = -x - 2.0 * x * y;
        let fy = -y - x * x + y * y;
        px += DT * fx;
        py += DT * fy;
        x += DT * px;
        y += DT * py;
        if !x.is_finite() || !y.is_finite() {
            break;
        }
        if x.abs() > 3.0 || y.abs() > 3.0 {
            break;
        }
        out.push((x, y));
    }
    out
}

fn draw(canvas: &mut dyn Surface, pts: &[(f64, f64)]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || pts.len() < 2 {
        return;
    }
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for &(x, y) in pts {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    min_x = min_x.min(-0.5);
    max_x = max_x.max(0.5);
    min_y = min_y.min(-0.5);
    max_y = max_y.max(0.5);
    let dx = (max_x - min_x).max(1e-6);
    let dy = (max_y - min_y).max(1e-6);
    let mut prev: Option<(i32, i32)> = None;
    for (i, &(x, y)) in pts.iter().enumerate() {
        let u = 0.08 + 0.84 * (x - min_x) / dx;
        let v = 0.08 + 0.84 * (y - min_y) / dy;
        let px = (u * width.saturating_sub(1) as f64).round() as i32;
        let py = ((1.0 - v) * height.saturating_sub(1) as f64).round() as i32;
        if let Some(o) = prev {
            let ch = if i + 200 > pts.len() { '#' } else { '*' };
            canvas.line(o.0, o.1, px, py, ch);
        }
        prev = Some((px, py));
    }
}

/// Henon-Heiles room.
#[derive(Debug, Default)]
pub struct HenonHeiles {
    seed: u64,
}

impl HenonHeiles {
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

impl Room for HenonHeiles {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "henon-heiles",
            title: "Henon-Heiles",
            wing: "Motion & Dynamics",
            blurb: "Galactic potential toy: energy steers order into chaos. t and DRAG: TUNE \
                    ENERGY.",
            accent: [80, 40, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let e = energy(t, None, self.seed);
        draw(canvas, &integrate(e));
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "henon heiles",
            root: 82.41,
            tempo: 78,
            line: &[0, 2, 5, 9, 14, 9, 5, 2],
            encodes: "energy climbing a galactic potential into chaos",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE ENERGY")
    }

    fn status(&self, t: f64) -> Option<String> {
        let e = energy(t, None, self.seed);
        Some(format!("E={e:.3}  galaxy  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let e = energy(t, hands.last().copied(), self.seed);
        draw(canvas, &integrate(e));
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let e = energy(t, hands.last().copied(), self.seed);
        // Lightweight sample (not the full render integrate): burn then span.
        let mut x = 0.0_f64;
        let mut y = 0.1_f64;
        let mut px = (2.0 * e).sqrt() * 0.5;
        let mut py = (2.0 * e).sqrt() * 0.5;
        for _ in 0..80 {
            let fx = -x - 2.0 * x * y;
            let fy = -y - x * x + y * y;
            px += DT * fx;
            py += DT * fy;
            x += DT * px;
            y += DT * py;
            if !x.is_finite() || !y.is_finite() || x.abs() > 3.0 || y.abs() > 3.0 {
                return Some(format!("E={e:.3}  span=0  escape"));
            }
        }
        let mut min_x = x;
        let mut max_x = x;
        let mut min_y = y;
        let mut max_y = y;
        for _ in 0..400 {
            let fx = -x - 2.0 * x * y;
            let fy = -y - x * x + y * y;
            px += DT * fx;
            py += DT * fy;
            x += DT * px;
            y += DT * py;
            if !x.is_finite() || !y.is_finite() || x.abs() > 3.0 || y.abs() > 3.0 {
                break;
            }
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        let span = ((max_x - min_x) * (max_y - min_y)).max(0.0).sqrt();
        // Escape energy for the classic potential is about 1/6.
        let regime = if e < 1.0 / 12.0 {
            "bound"
        } else if e < 1.0 / 6.0 {
            "mixed"
        } else {
            "escape?"
        };
        Some(format!("E={e:.3}  span={span:.2}  {regime}"))
    }

    fn reveal(&self) -> &'static str {
        "Henon and Heiles modeled stellar motion in a galaxy with a cubic \
         potential. At low energy orbits look ordered; above a threshold they \
         fill a chaotic sea. This room is a CPU-honest toy of that portrait."
    }
}

#[cfg(test)]
mod tests {
    use super::HenonHeiles;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = HenonHeiles::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("TUNE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = HenonHeiles::new();
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
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        HenonHeiles::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(HenonHeiles::new().motif().unwrap().line.len() >= 6);
    }
}

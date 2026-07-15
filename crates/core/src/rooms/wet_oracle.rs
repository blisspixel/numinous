//! The Wet Oracle: slime mold races you to the shortest path (and wins).
//!
//! A simple Physarum-inspired agent deposits chemoattractant and follows
//! gradients between food crumbs (Tero 2010 vibe). DRAG: SMEAR THE FOOD.
//! See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const W: usize = 48;
const H: usize = 32;
const AGENTS: usize = 80;
const STEPS: usize = 40;
const SEED: u64 = 0x0051_E01D_5EED_0001;

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

fn idx(x: usize, y: usize) -> usize {
    y * W + x
}

fn simulate(seed: u64, steps: usize, foods: &[(f64, f64)]) -> (Vec<f64>, f64) {
    let mut rng = SplitMix64::new(SEED ^ seed);
    let mut field = vec![0.0f64; W * H];
    // Seed foods.
    for &(fx, fy) in foods {
        let x = ((fx * (W - 1) as f64) as usize).min(W - 1);
        let y = ((fy * (H - 1) as f64) as usize).min(H - 1);
        field[idx(x, y)] += 8.0;
    }
    if foods.is_empty() {
        field[idx(W / 4, H / 2)] += 8.0;
        field[idx(3 * W / 4, H / 2)] += 8.0;
    }
    let mut agents: Vec<(f64, f64, f64)> = (0..AGENTS)
        .map(|_| {
            (
                rng.next_f64() * (W - 1) as f64,
                rng.next_f64() * (H - 1) as f64,
                rng.next_f64() * std::f64::consts::TAU,
            )
        })
        .collect();

    for _ in 0..steps {
        // Diffuse / decay field lightly.
        let snap = field.clone();
        for y in 1..H - 1 {
            for x in 1..W - 1 {
                let s = snap[idx(x, y)]
                    + snap[idx(x - 1, y)]
                    + snap[idx(x + 1, y)]
                    + snap[idx(x, y - 1)]
                    + snap[idx(x, y + 1)];
                field[idx(x, y)] = s * 0.18 * 0.92;
            }
        }
        for &(fx, fy) in foods {
            let x = ((fx * (W - 1) as f64) as usize).min(W - 1);
            let y = ((fy * (H - 1) as f64) as usize).min(H - 1);
            field[idx(x, y)] += 2.0;
        }
        if foods.is_empty() {
            field[idx(W / 4, H / 2)] += 2.0;
            field[idx(3 * W / 4, H / 2)] += 2.0;
        }
        for a in &mut agents {
            let sense = |ang: f64| {
                let sx = (a.0 + ang.cos() * 3.0).clamp(0.0, (W - 1) as f64) as usize;
                let sy = (a.1 + ang.sin() * 3.0).clamp(0.0, (H - 1) as f64) as usize;
                field[idx(sx, sy)]
            };
            let left = sense(a.2 - 0.4);
            let mid = sense(a.2);
            let right = sense(a.2 + 0.4);
            if mid > left && mid > right {
                // keep
            } else if left > right {
                a.2 -= 0.35;
            } else {
                a.2 += 0.35;
            }
            a.0 = (a.0 + a.2.cos() * 0.9).rem_euclid(W as f64);
            a.1 = (a.1 + a.2.sin() * 0.9).rem_euclid(H as f64);
            let ix = a.0 as usize % W;
            let iy = a.1 as usize % H;
            field[idx(ix, iy)] += 0.6;
        }
    }
    let mass: f64 = field.iter().sum();
    (field, mass)
}

fn draw(canvas: &mut dyn Surface, field: &[f64], foods: &[(f64, f64)]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let max = field.iter().cloned().fold(1e-6, f64::max);
    for y in 0..H {
        for x in 0..W {
            let v = field[idx(x, y)] / max;
            if v < 0.08 {
                continue;
            }
            let ch = if v > 0.6 {
                '#'
            } else if v > 0.3 {
                '*'
            } else {
                '.'
            };
            let left = x * width / W;
            let right = (((x + 1) * width / W).max(left + 1)).min(width);
            let top = y * height / H;
            let bottom = (((y + 1) * height / H).max(top + 1)).min(height);
            for py in top..bottom {
                for px in left..right {
                    canvas.plot(px as i32, py as i32, ch);
                }
            }
        }
    }
    for &(fx, fy) in foods {
        let px = (fx * width.saturating_sub(1) as f64).round() as i32;
        let py = (fy * height.saturating_sub(1) as f64).round() as i32;
        canvas.plot(px, py, 'O');
    }
}

/// Wet Oracle room.
#[derive(Debug, Default)]
pub struct WetOracle {
    seed: u64,
}

impl WetOracle {
    /// Create the room with default seed (0).
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed for replayable per-visit novelty.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }
}

impl Room for WetOracle {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "wet-oracle",
            title: "The Wet Oracle",
            wing: "Emergence",
            blurb: "A slime of agents deposits scent and climbs gradients between foods. Race it \
                    to the shortest path and lose (Tero 2010 Physarum). t grows the network; DRAG: \
                    SMEAR THE FOOD.",
            accent: [120, 180, 90],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let steps = 8 + (phase_unit(t) * STEPS as f64) as usize;
        let (field, _) = simulate(self.seed, steps, &[]);
        draw(canvas, &field, &[(0.25, 0.5), (0.75, 0.5)]);
    }

    fn postcard_t(&self) -> f64 {
        0.7
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "physarum net",
            root: 174.61,
            tempo: 90,
            line: &[0, 2, 5, 7, 5, 2, 0, 5],
            encodes: "scent trails condensing into a short wet path",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: SMEAR THE FOOD")
    }

    fn status(&self, t: f64) -> Option<String> {
        let steps = 8 + (phase_unit(t) * STEPS as f64) as usize;
        let (_, mass) = simulate(self.seed, steps, &[]);
        Some(format!("MASS {mass:.0}  FOOD 2  DRAG:SMEAR"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let foods = finite_pokes(pokes);
        if foods.is_empty() {
            self.render(canvas, t);
            return;
        }
        let steps = 8 + (phase_unit(t) * STEPS as f64) as usize;
        let (field, _) = simulate(self.seed, steps, &foods);
        draw(canvas, &field, &foods);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let foods = finite_pokes(&pokes);
        if foods.is_empty() {
            return self.status(t);
        }
        let steps = 8 + (phase_unit(t) * STEPS as f64) as usize;
        let (_, mass) = simulate(self.seed, steps, &foods);
        Some(format!("SMEAR n{}  MASS {mass:.0}  NET", foods.len()))
    }

    fn reveal(&self) -> &'static str {
        "Physarum polycephalum builds efficient networks between food sources \
         without a brain. Tero and colleagues (2010) showed a slime mold can redraw \
         the Tokyo rail map: wet computation as a shortest-path oracle."
    }
}

#[cfg(test)]
mod tests {
    use super::WetOracle;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = WetOracle::new().status(0.0).unwrap();
        assert!(s.contains("DRAG") || s.contains("SMEAR"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn smear_changes() {
        let r = WetOracle::new();
        let o = r.status(0.0).unwrap();
        let a = r
            .status_input(
                0.0,
                &[RoomInput::PointerDown {
                    x: 0.4,
                    y: 0.4,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
        assert!(a.chars().count() <= 56);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(48, 32);
        WetOracle::new().render(&mut c, 0.6);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(WetOracle::new().motif().unwrap().line.len() >= 6);
    }

    #[test]
    fn variation() {
        let mut a = Canvas::new(32, 24);
        let mut b = Canvas::new(32, 24);
        WetOracle::new_with(0).render(&mut a, 0.5);
        WetOracle::new_with(2).render(&mut b, 0.5);
        assert_ne!(a.to_text(), b.to_text());
    }
}

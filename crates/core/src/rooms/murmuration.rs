//! Murmuration: boids with seven neighbors; the shape exists in no bird's head.
//!
//! Each bird steers by separation, alignment, and cohesion among its nearest
//! neighbors. HOLD: BE THE FALCON inserts a predator the flock flees. `t`
//! advances the flight. See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const N: usize = 40;
const NEIGH: usize = 7;
const STEPS: usize = 24;
const SEED: u64 = 0xB01D_5EED_0000_0001;

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

#[derive(Clone, Copy)]
struct Bird {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
}

fn init(seed: u64) -> Vec<Bird> {
    let mut rng = SplitMix64::new(SEED ^ seed);
    (0..N)
        .map(|_| Bird {
            x: rng.next_f64(),
            y: rng.next_f64(),
            vx: (rng.next_f64() - 0.5) * 0.04,
            vy: (rng.next_f64() - 0.5) * 0.04,
        })
        .collect()
}

fn step(birds: &mut [Bird], falcon: Option<(f64, f64)>) {
    let snap = birds.to_vec();
    for (i, b) in birds.iter_mut().enumerate() {
        // Nearest NEIGH by distance.
        let mut idxs: Vec<(f64, usize)> = snap
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(j, o)| {
                let d = (o.x - b.x).hypot(o.y - b.y);
                (d, j)
            })
            .collect();
        idxs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let k = idxs.len().min(NEIGH);
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut ax = 0.0;
        let mut ay = 0.0;
        let mut cx = 0.0;
        let mut cy = 0.0;
        for &(d, j) in idxs.iter().take(k) {
            let o = snap[j];
            if d < 0.06 && d > 1e-9 {
                sx -= (o.x - b.x) / d;
                sy -= (o.y - b.y) / d;
            }
            ax += o.vx;
            ay += o.vy;
            cx += o.x;
            cy += o.y;
        }
        if k > 0 {
            ax /= k as f64;
            ay /= k as f64;
            cx = cx / k as f64 - b.x;
            cy = cy / k as f64 - b.y;
        }
        let mut fx = 0.0;
        let mut fy = 0.0;
        if let Some((px, py)) = falcon {
            let dx = b.x - px;
            let dy = b.y - py;
            let d = dx.hypot(dy).max(1e-3);
            if d < 0.35 {
                fx = dx / d * 0.08;
                fy = dy / d * 0.08;
            }
        }
        b.vx += sx * 0.03 + (ax - b.vx) * 0.05 + cx * 0.02 + fx;
        b.vy += sy * 0.03 + (ay - b.vy) * 0.05 + cy * 0.02 + fy;
        let sp = b.vx.hypot(b.vy).max(1e-6);
        let max_sp = 0.05;
        if sp > max_sp {
            b.vx *= max_sp / sp;
            b.vy *= max_sp / sp;
        }
        b.x = (b.x + b.vx).rem_euclid(1.0);
        b.y = (b.y + b.vy).rem_euclid(1.0);
    }
}

fn flock(seed: u64, steps: usize, falcon: Option<(f64, f64)>) -> (Vec<Bird>, f64) {
    let mut birds = init(seed);
    for _ in 0..steps {
        step(&mut birds, falcon);
    }
    let mx = birds.iter().map(|b| b.x).sum::<f64>() / N as f64;
    let my = birds.iter().map(|b| b.y).sum::<f64>() / N as f64;
    let spread = birds
        .iter()
        .map(|b| (b.x - mx).hypot(b.y - my))
        .sum::<f64>()
        / N as f64;
    (birds, spread)
}

fn draw(canvas: &mut dyn Surface, birds: &[Bird], falcon: Option<(f64, f64)>) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    for b in birds {
        let px = (b.x * width.saturating_sub(1) as f64).round() as i32;
        let py = (b.y * height.saturating_sub(1) as f64).round() as i32;
        canvas.plot(px, py, '*');
    }
    if let Some((x, y)) = falcon {
        let px = (x * width.saturating_sub(1) as f64).round() as i32;
        let py = (y * height.saturating_sub(1) as f64).round() as i32;
        canvas.plot(px, py, '#');
        canvas.plot(px + 1, py, '#');
        canvas.plot(px, py + 1, '#');
    }
}

/// Murmuration room.
#[derive(Debug, Default)]
pub struct Murmuration {
    seed: u64,
}

impl Murmuration {
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

impl Room for Murmuration {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "murmuration",
            title: "Murmuration",
            wing: "Emergence",
            blurb: "Boids with seven neighbors: separate, align, cohere. The flock shape lives in \
                    no single bird. t flies the cloud; HOLD: BE THE FALCON and they part.",
            accent: [80, 100, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let steps = 4 + (phase_unit(t) * STEPS as f64) as usize;
        let (birds, _) = flock(self.seed, steps, None);
        draw(canvas, &birds, None);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "seven neighbors",
            root: 185.0,
            tempo: 132,
            line: &[0, 3, 5, 7, 5, 3, 0, 7],
            encodes: "local rules weaving a global flock",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("HOLD: BE THE FALCON")
    }

    fn status(&self, t: f64) -> Option<String> {
        let steps = 4 + (phase_unit(t) * STEPS as f64) as usize;
        let (_, spread) = flock(self.seed, steps, None);
        Some(format!("N{N}  k={NEIGH}  spread={spread:.2}  HOLD:FALCON"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let falcon = hands.last().copied();
        let steps = 4 + (phase_unit(t) * STEPS as f64) as usize;
        let (birds, _) = flock(self.seed, steps, falcon);
        draw(canvas, &birds, falcon);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let falcon = hands.last().copied();
        let steps = 4 + (phase_unit(t) * STEPS as f64) as usize;
        let (_, spread) = flock(self.seed, steps, falcon);
        let (fx, fy) = falcon.unwrap();
        Some(format!(
            "FALCON@{:.0}%{:.0}%  spread={spread:.2}",
            fx * 100.0,
            fy * 100.0
        ))
    }

    fn reveal(&self) -> &'static str {
        "No bird knows the murmuration. Each only watches a handful of neighbors, \
         yet the whole cloud banks as one. Reynolds boids made that local-to-global \
         leap a rule set; nature got there first."
    }
}

#[cfg(test)]
mod tests {
    use super::Murmuration;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Murmuration::new().status(0.0).unwrap();
        assert!(s.contains("HOLD") || s.contains("FALCON"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn falcon_changes_status() {
        let r = Murmuration::new();
        let o = r.status(0.0).unwrap();
        let a = r
            .status_input(
                0.0,
                &[RoomInput::PointerDown {
                    x: 0.5,
                    y: 0.5,
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
        Murmuration::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(Murmuration::new().motif().unwrap().line.len() >= 6);
    }

    #[test]
    fn variation() {
        let mut a = Canvas::new(32, 24);
        let mut b = Canvas::new(32, 24);
        Murmuration::new_with(0).render(&mut a, 0.4);
        Murmuration::new_with(3).render(&mut b, 0.4);
        assert_ne!(a.to_text(), b.to_text());
    }
}

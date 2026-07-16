//! Gray-Scott reaction-diffusion: spots, stripes, and coral from two chemicals.
//!
//! u and v diffuse and react; feed and kill rates pick the pattern class.
//! DRAG: TUNE FEED/KILL. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const W: usize = 48;
const H: usize = 28;
const STEPS: usize = 80;

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

fn rates(t: f64, hand: Option<(f64, f64)>) -> (f64, f64) {
    // Classic Pearson (f,k) regions.
    if let Some((x, y)) = hand {
        (0.01 + x * 0.08, 0.04 + y * 0.04)
    } else {
        let u = phase_unit(t);
        (0.035 + u * 0.02, 0.06 + (1.0 - u) * 0.01)
    }
}

fn idx(x: usize, y: usize) -> usize {
    y * W + x
}

fn seed_field(seed: u64) -> (Vec<f64>, Vec<f64>) {
    let mut u = vec![1.0f64; W * H];
    let mut v = vec![0.0f64; W * H];
    let cx = W / 2;
    let cy = H / 2;
    let r = 4 + (seed % 3) as usize;
    for y in cy.saturating_sub(r)..=(cy + r).min(H - 1) {
        for x in cx.saturating_sub(r)..=(cx + r).min(W - 1) {
            if (x as i32 - cx as i32).pow(2) + (y as i32 - cy as i32).pow(2) <= (r * r) as i32 {
                u[idx(x, y)] = 0.5;
                v[idx(x, y)] = 0.25;
            }
        }
    }
    (u, v)
}

fn lap(f: &[f64], x: usize, y: usize) -> f64 {
    let xm = if x == 0 { W - 1 } else { x - 1 };
    let xp = if x + 1 == W { 0 } else { x + 1 };
    let ym = if y == 0 { H - 1 } else { y - 1 };
    let yp = if y + 1 == H { 0 } else { y + 1 };
    f[idx(xm, y)] + f[idx(xp, y)] + f[idx(x, ym)] + f[idx(x, yp)] - 4.0 * f[idx(x, y)]
}

fn step(u: &mut [f64], v: &mut [f64], f: f64, k: f64) {
    let du = 0.16;
    let dv = 0.08;
    let mut nu = vec![0.0; W * H];
    let mut nv = vec![0.0; W * H];
    for y in 0..H {
        for x in 0..W {
            let i = idx(x, y);
            let uu = u[i];
            let vv = v[i];
            let uvv = uu * vv * vv;
            nu[i] = (uu + du * lap(u, x, y) - uvv + f * (1.0 - uu)).clamp(0.0, 1.0);
            nv[i] = (vv + dv * lap(v, x, y) + uvv - (f + k) * vv).clamp(0.0, 1.0);
        }
    }
    u.copy_from_slice(&nu);
    v.copy_from_slice(&nv);
}

fn run(f: f64, k: f64, seed: u64, steps: usize, seeds: &[(f64, f64)]) -> Vec<f64> {
    let (mut u, mut v) = seed_field(seed);
    for &(px, py) in seeds {
        let x = ((px * (W - 1) as f64).round() as usize).min(W - 1);
        let y = ((py * (H - 1) as f64).round() as usize).min(H - 1);
        for dy in 0..3 {
            for dx in 0..3 {
                let xx = (x + dx).min(W - 1);
                let yy = (y + dy).min(H - 1);
                u[idx(xx, yy)] = 0.5;
                v[idx(xx, yy)] = 0.25;
            }
        }
    }
    for _ in 0..steps {
        step(&mut u, &mut v, f, k);
    }
    v
}

fn draw(canvas: &mut dyn Surface, v: &[f64]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    for gy in 0..H {
        for gx in 0..W {
            let val = v[idx(gx, gy)];
            if val < 0.08 {
                continue;
            }
            let ch = if val > 0.4 {
                '#'
            } else if val > 0.25 {
                '*'
            } else if val > 0.15 {
                '+'
            } else {
                '.'
            };
            let x0 = (gx as f64 / W as f64 * width as f64).round() as i32;
            let y0 = (gy as f64 / H as f64 * height as f64).round() as i32;
            let x1 = (((gx + 1) as f64 / W as f64) * width as f64).round() as i32;
            let y1 = (((gy + 1) as f64 / H as f64) * height as f64).round() as i32;
            for yy in y0..y1.max(y0 + 1) {
                for xx in x0..x1.max(x0 + 1) {
                    canvas.plot(xx, yy, ch);
                }
            }
        }
    }
}

/// Gray-Scott room.
#[derive(Debug, Default)]
pub struct GrayScott {
    seed: u64,
}

impl GrayScott {
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

impl Room for GrayScott {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "gray-scott",
            title: "The Chemical Garden",
            wing: "Emergence",
            blurb: "Gray-Scott reaction-diffusion: two chemicals paint spots, stripes, and coral. \
                    t drifts feed/kill; DRAG: TUNE FEED/KILL; clicks seed blobs.",
            accent: [80, 200, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (f, k) = rates(t, None);
        let steps = 40 + (phase_unit(t) * STEPS as f64) as usize;
        let v = run(f, k, self.seed, steps, &[]);
        draw(canvas, &v);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "gray scott",
            root: 138.59,
            tempo: 76,
            line: &[0, 5, 7, 10, 12, 10, 7, 5],
            encodes: "feed and kill rates choosing spots, stripes, or coral",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE FEED/KILL")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (f, k) = rates(t, None);
        Some(format!("f={f:.3}  k={k:.3}  DRAG:FK"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (f, k) = rates(t, hands.last().copied());
        let steps = 50 + (phase_unit(t) * 60.0) as usize;
        let v = run(f, k, self.seed ^ hands.len() as u64, steps, &hands);
        draw(canvas, &v);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (f, k) = rates(t, hands.last().copied());
        let pattern = if f < 0.03 {
            "SPOTS"
        } else if k > 0.06 {
            "CORAL"
        } else {
            "MIX"
        };
        Some(format!("f={f:.3} k={k:.3}  {pattern}"))
    }

    fn reveal(&self) -> &'static str {
        "Gray-Scott reaction-diffusion is two chemicals that diffuse and \
         consume each other. A few numbers (feed and kill) choose whether the \
         plate grows spots, stripes, or labyrinthine coral: Turing patterns from algebra."
    }
}

#[cfg(test)]
mod tests {
    use super::GrayScott;
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = GrayScott::new().status(0.4).unwrap();
        assert!(s.contains("DRAG") || s.contains("f="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = GrayScott::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.2,
                    y: 0.8,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(48, 28);
        GrayScott::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 5);
    }

    #[test]
    fn motif_ok() {
        assert!(GrayScott::new().motif().unwrap().line.len() >= 6);
    }
}

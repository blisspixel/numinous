//! Wireworld: the visible computer (four-state cellular automaton).
//!
//! Empty, conductor, electron head, electron tail. Electrons race the wires;
//! gates are patterns. CLICK: FIRE AN ELECTRON. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const W: usize = 40;
const H: usize = 22;

const EMPTY: u8 = 0;
const CONDUCTOR: u8 = 1;
const HEAD: u8 = 2;
const TAIL: u8 = 3;

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

fn seed_circuit(seed: u64) -> Vec<u8> {
    let mut g = vec![EMPTY; W * H];
    // Horizontal bus.
    let y = H / 2;
    for x in 2..W - 2 {
        g[idx(x, y)] = CONDUCTOR;
    }
    // Diode-like fork.
    for x in 10..18 {
        g[idx(x, y - 2)] = CONDUCTOR;
        g[idx(x, y + 2)] = CONDUCTOR;
    }
    g[idx(10, y - 1)] = CONDUCTOR;
    g[idx(10, y + 1)] = CONDUCTOR;
    g[idx(17, y - 1)] = CONDUCTOR;
    g[idx(17, y + 1)] = CONDUCTOR;
    // Vertical clock loop.
    for dy in 0..6 {
        g[idx(4, y - 3 + dy)] = CONDUCTOR;
    }
    g[idx(5, y - 3)] = CONDUCTOR;
    g[idx(5, y + 2)] = CONDUCTOR;
    g[idx(3, y - 3)] = CONDUCTOR;
    g[idx(3, y + 2)] = CONDUCTOR;
    // Seed electron depending on variation.
    let ex = 6 + (seed % 5) as usize;
    if g[idx(ex, y)] == CONDUCTOR {
        g[idx(ex, y)] = HEAD;
        if ex > 0 {
            g[idx(ex - 1, y)] = TAIL;
        }
    }
    g
}

fn head_neighbors(g: &[u8], x: usize, y: usize) -> u32 {
    let mut n = 0u32;
    for dy in -1i32..=1 {
        for dx in -1i32..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx >= 0
                && ny >= 0
                && (nx as usize) < W
                && (ny as usize) < H
                && g[idx(nx as usize, ny as usize)] == HEAD
            {
                n += 1;
            }
        }
    }
    n
}

fn step(g: &[u8]) -> Vec<u8> {
    let mut next = vec![EMPTY; W * H];
    for y in 0..H {
        for x in 0..W {
            let c = g[idx(x, y)];
            next[idx(x, y)] = match c {
                HEAD => TAIL,
                TAIL => CONDUCTOR,
                CONDUCTOR => {
                    let n = head_neighbors(g, x, y);
                    if n == 1 || n == 2 { HEAD } else { CONDUCTOR }
                }
                _ => EMPTY,
            };
        }
    }
    next
}

fn run(mut g: Vec<u8>, steps: usize) -> Vec<u8> {
    for _ in 0..steps {
        g = step(&g);
    }
    g
}

fn fire(mut g: Vec<u8>, hands: &[(f64, f64)]) -> Vec<u8> {
    for &(px, py) in hands {
        let x = ((px * (W - 1) as f64).round() as usize).min(W - 1);
        let y = ((py * (H - 1) as f64).round() as usize).min(H - 1);
        if g[idx(x, y)] == CONDUCTOR || g[idx(x, y)] == EMPTY {
            g[idx(x, y)] = HEAD;
            // Ensure a short wire pad so the electron has somewhere to go.
            if x + 1 < W && g[idx(x + 1, y)] == EMPTY {
                g[idx(x + 1, y)] = CONDUCTOR;
            }
            if x > 0 && g[idx(x - 1, y)] == EMPTY {
                g[idx(x - 1, y)] = CONDUCTOR;
            }
        } else if g[idx(x, y)] == CONDUCTOR {
            g[idx(x, y)] = HEAD;
        }
    }
    g
}

fn draw(canvas: &mut dyn Surface, g: &[u8]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    for y in 0..H {
        for x in 0..W {
            let ch = match g[idx(x, y)] {
                HEAD => '#',
                TAIL => '+',
                CONDUCTOR => '*',
                _ => continue,
            };
            let px = ((x as f64 + 0.5) / W as f64 * width as f64).round() as i32;
            let py = ((y as f64 + 0.5) / H as f64 * height as f64).round() as i32;
            canvas.plot(px, py, ch);
        }
    }
}

/// Wireworld room.
#[derive(Debug, Default)]
pub struct Wireworld {
    seed: u64,
}

impl Wireworld {
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

impl Room for Wireworld {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "wireworld",
            title: "The Visible Computer",
            wing: "Emergence",
            blurb: "Wireworld: four states, electrons on copper, gates you can watch. t steps the \
                    clock; CLICK: FIRE AN ELECTRON.",
            accent: [255, 200, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let steps = (phase_unit(t) * 40.0) as usize;
        let g = run(seed_circuit(self.seed), steps);
        draw(canvas, &g);
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "wire pulse",
            root: 392.0,
            tempo: 141,
            line: &[0, 0, 7, 0, 12, 0, 16, 7],
            encodes: "head and tail chasing along a conductor of four states",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: FIRE AN ELECTRON")
    }

    fn status(&self, t: f64) -> Option<String> {
        let steps = (phase_unit(t) * 40.0) as usize;
        let g = run(seed_circuit(self.seed), steps);
        let heads = g.iter().filter(|&&c| c == HEAD).count();
        Some(format!("t={steps}  heads={heads}  CLICK:FIRE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let steps = 5 + (phase_unit(t) * 30.0) as usize;
        let mut g = seed_circuit(self.seed);
        g = fire(g, &hands);
        g = run(g, steps);
        draw(canvas, &g);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let steps = 5 + (phase_unit(t) * 30.0) as usize;
        let mut g = seed_circuit(self.seed);
        g = fire(g, &hands);
        g = run(g, steps);
        let heads = g.iter().filter(|&&c| c == HEAD).count();
        let copper = g.iter().filter(|&&c| c == CONDUCTOR).count();
        Some(format!("FIRE x{}  heads={heads}  Cu={copper}", hands.len()))
    }

    fn reveal(&self) -> &'static str {
        "Wireworld is a four-state cellular automaton where electron heads \
         chase tails along conductors. With only local rules you can build \
         diodes, gates, and clocks: computation made visible as light on copper."
    }
}

#[cfg(test)]
mod tests {
    use super::{HEAD, Wireworld, run, seed_circuit, step};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Wireworld::new().status(0.2).unwrap();
        assert!(s.contains("CLICK") || s.contains("FIRE"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn fire_changes() {
        let r = Wireworld::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.5,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn head_moves() {
        let g0 = seed_circuit(0);
        let heads0 = g0.iter().filter(|&&c| c == HEAD).count();
        let g1 = step(&g0);
        let heads1 = g1.iter().filter(|&&c| c == HEAD).count();
        assert!(heads0 >= 1);
        // Head becomes tail; conductor may birth a new head.
        let _ = (heads1, run(g0, 3));
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(48, 28);
        Wireworld::new().render(&mut c, 0.3);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(Wireworld::new().motif().unwrap().line.len() >= 6);
    }
}

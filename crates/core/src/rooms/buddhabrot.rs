//! Buddhabrot: the ghost in the Mandelbrot set.
//!
//! Trace orbits of points that escape the Mandelbrot iteration; density of
//! visited cells paints a meditating figure in the fog. DRAG: AIM THE GHOST.
//! See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const SEED: u64 = 0xB0DD_AB00_7000_0001;
const SAMPLES: usize = 2_500;

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

fn escapes(cx: f64, cy: f64, max_iter: u32) -> Option<Vec<(f64, f64)>> {
    let mut zx = 0.0;
    let mut zy = 0.0;
    let mut path = Vec::with_capacity(max_iter as usize);
    for _ in 0..max_iter {
        let zx2 = zx * zx - zy * zy + cx;
        let zy2 = 2.0 * zx * zy + cy;
        zx = zx2;
        zy = zy2;
        path.push((zx, zy));
        if zx * zx + zy * zy > 4.0 {
            return Some(path);
        }
    }
    None
}

fn accumulate(
    width: usize,
    height: usize,
    samples: usize,
    max_iter: u32,
    seed: u64,
    center: (f64, f64),
    zoom: f64,
) -> Vec<u32> {
    let mut hist = vec![0u32; width * height];
    let mut rng = SplitMix64::new(SEED ^ seed);
    // Sample region in c-plane around center with zoom.
    let span = 3.0 / zoom.max(0.5);
    for _ in 0..samples {
        let cx = center.0 + (rng.next_f64() - 0.5) * span * 1.4;
        let cy = center.1 + (rng.next_f64() - 0.5) * span;
        if let Some(path) = escapes(cx, cy, max_iter) {
            for &(zx, zy) in &path {
                // Map z to plate (classic Buddhabrot framing).
                let u = (zx + 2.0) / 3.5;
                let v = (zy + 1.5) / 3.0;
                if (0.0..1.0).contains(&u) && (0.0..1.0).contains(&v) {
                    let x = (u * width.saturating_sub(1) as f64).round() as usize;
                    let y = ((1.0 - v) * height.saturating_sub(1) as f64).round() as usize;
                    if x < width && y < height {
                        hist[y * width + x] = hist[y * width + x].saturating_add(1);
                    }
                }
            }
        }
    }
    hist
}

fn draw(canvas: &mut dyn Surface, hist: &[u32], width: usize, height: usize) {
    let max = hist.iter().copied().max().unwrap_or(1).max(1);
    for y in 0..height {
        for x in 0..width {
            let v = hist[y * width + x];
            if v == 0 {
                continue;
            }
            let n = v as f64 / max as f64;
            let ch = if n > 0.6 {
                '#'
            } else if n > 0.3 {
                '*'
            } else if n > 0.1 {
                '+'
            } else {
                '.'
            };
            canvas.plot(x as i32, y as i32, ch);
        }
    }
}

/// Buddhabrot room.
#[derive(Debug, Default)]
pub struct Buddhabrot {
    seed: u64,
}

impl Buddhabrot {
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

impl Room for Buddhabrot {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "buddhabrot",
            title: "The Ghost in the Set",
            wing: "Fractals",
            blurb: "Buddhabrot: density of escaping Mandelbrot orbits paints a ghostly figure. t \
                    deepens iterations; DRAG: AIM THE GHOST.",
            accent: [200, 180, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (width, height) = canvas.draw_bounds();
        if width == 0 || height == 0 {
            return;
        }
        let max_iter = 40 + (phase_unit(t) * 60.0) as u32;
        let samples = 800 + (phase_unit(t) * SAMPLES as f64) as usize;
        let center = (-0.4, 0.0);
        let hist = accumulate(width, height, samples, max_iter, self.seed, center, 1.0);
        draw(canvas, &hist, width, height);
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "buddha fog",
            root: 98.0,
            tempo: 67,
            line: &[0, 3, 5, 10, 15, 12, 7, 2],
            encodes: "escaping orbits densify into a ghost of the Mandelbrot",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: AIM THE GHOST")
    }

    fn status(&self, t: f64) -> Option<String> {
        let max_iter = 40 + (phase_unit(t) * 60.0) as u32;
        Some(format!("iter={max_iter}  buddha  DRAG:AIM"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let (width, height) = canvas.draw_bounds();
        if width == 0 || height == 0 {
            return;
        }
        let max_iter = 50 + (phase_unit(t) * 50.0) as u32;
        let samples = 1_200;
        // Hand maps to c-plane aim and zoom via y.
        let (cx, cy, zoom) = hands
            .last()
            .map(|&(x, y)| {
                let cx = -2.0 + x * 3.5;
                let cy = -1.5 + y * 3.0;
                let zoom = 0.8 + (1.0 - y) * 2.0;
                (cx, cy, zoom)
            })
            .unwrap_or((-0.4, 0.0, 1.0));
        let hist = accumulate(
            width,
            height,
            samples,
            max_iter,
            self.seed ^ hands.len() as u64,
            (cx, cy),
            zoom,
        );
        draw(canvas, &hist, width, height);
        if let Some(&(x, y)) = hands.last() {
            let px = (x * width.saturating_sub(1) as f64).round() as i32;
            let py = (y * height.saturating_sub(1) as f64).round() as i32;
            canvas.line(px - 2, py, px + 2, py, 'o');
            canvas.line(px, py - 2, px, py + 2, 'o');
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (x, y) = *hands.last().unwrap();
        let cx = -2.0 + x * 3.5;
        let cy = -1.5 + y * 3.0;
        let max_iter = 50 + (phase_unit(t) * 50.0) as u32;
        Some(format!("AIM c=({cx:.2},{cy:.2})  it={max_iter}"))
    }

    fn reveal(&self) -> &'static str {
        "The Buddhabrot is the density plot of points visited by Mandelbrot \
         orbits that eventually escape. Where the usual set shows who stays, \
         this ghost shows the paths of those who leave."
    }
}

#[cfg(test)]
mod tests {
    use super::{Buddhabrot, escapes};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Buddhabrot::new().status(0.4).unwrap();
        assert!(s.contains("DRAG") || s.contains("AIM"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn aim_changes() {
        let r = Buddhabrot::new();
        let o = r.status(0.4).unwrap();
        let a = r
            .status_input(
                0.4,
                &[RoomInput::PointerDown {
                    x: 0.3,
                    y: 0.6,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn escape_detects() {
        assert!(escapes(1.0, 0.0, 50).is_some());
        assert!(escapes(0.0, 0.0, 50).is_none());
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(32, 20);
        Buddhabrot::new().render(&mut c, 0.3);
        assert!(c.ink_count() > 5);
    }

    #[test]
    fn motif_ok() {
        assert!(Buddhabrot::new().motif().unwrap().line.len() >= 6);
    }
}

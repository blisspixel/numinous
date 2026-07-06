//! Zeno's Square: half, then half of what's left, forever, and it adds to one.
//!
//! The proof without words: a unit square filled by rectangles of area 1/2,
//! 1/4, 1/8, ... alternating vertical and horizontal. Zeno said the runner
//! never arrives because infinitely many steps remain; the square says the
//! infinitely many steps fit exactly inside one tile of floor. `t` lays the
//! tiles. See the Full Map in `docs/ROOMS.md`.

use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// How many halvings `t` reaches (past ~14 the tiles are subpixel anyway).
const MAX_TILES: usize = 14;

/// The tiles: each is (x, y, w, h) in unit-square coordinates, tile `i`
/// having area 2^-(i+1), alternating vertical and horizontal cuts.
fn tiles() -> Vec<(f64, f64, f64, f64)> {
    let (mut x, mut y) = (0.0, 0.0);
    let (mut w, mut h) = (1.0, 1.0);
    let mut out = Vec::with_capacity(MAX_TILES);
    for i in 0..MAX_TILES {
        if i % 2 == 0 {
            // Take the left half of what remains.
            out.push((x, y, w / 2.0, h));
            x += w / 2.0;
            w /= 2.0;
        } else {
            // Take the bottom half of what remains.
            out.push((x, y, w, h / 2.0));
            y += h / 2.0;
            h /= 2.0;
        }
    }
    out
}

/// Zeno's Square.
#[derive(Debug, Default)]
pub struct Zeno;

impl Zeno {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Room for Zeno {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "zeno",
            title: "Zeno's Square",
            wing: "Change",
            blurb: "Half the square, then half of what's left, then half of that, forever. \
                    Infinitely many tiles, and they fit exactly. The sum of the halves is one.",
            accent: [200, 160, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let aspect = canvas.char_aspect();
        let side = (width as f64 * 0.8).min(height as f64 * aspect * 0.8);
        let left = (width as f64 - side) / 2.0;
        let top = (height as f64 - side / aspect) / 2.0;
        let to_screen = |x: f64, y: f64| {
            (
                (left + x * side) as i32,
                (top + (1.0 - y) * side / aspect) as i32,
            )
        };
        // The square itself: the destination.
        let (x0, y0) = to_screen(0.0, 1.0);
        let (x1, y1) = to_screen(1.0, 0.0);
        canvas.line(x0, y0, x1, y0, '*');
        canvas.line(x0, y1, x1, y1, '*');
        canvas.line(x0, y0, x0, y1, '*');
        canvas.line(x1, y0, x1, y1, '*');

        // The tiles laid so far: 1/2, then 1/4, then 1/8, ...
        let laid = ((t.clamp(0.0, 1.0) * (MAX_TILES as f64 + 1.0)) as usize).min(MAX_TILES);
        for (i, &(tx, ty, tw, th)) in tiles().iter().take(laid).enumerate() {
            let (px0, py0) = to_screen(tx, ty + th);
            let (px1, py1) = to_screen(tx + tw, ty);
            // Outline bright, fill dithered; later tiles get denser fill.
            canvas.line(px0, py0, px1, py0, '#');
            canvas.line(px0, py1, px1, py1, '#');
            canvas.line(px0, py0, px0, py1, '#');
            canvas.line(px1, py0, px1, py1, '#');
            let step = if i < 4 { 3 } else { 2 };
            let mut py = py0.min(py1);
            while py <= py0.max(py1) {
                let mut px = px0.min(px1);
                while px <= px0.max(px1) {
                    if (px + py) % step == 0 {
                        canvas.plot(px, py, '-');
                    }
                    px += 1;
                }
                py += 1;
            }
        }
    }

    fn reveal(&self) -> &'static str {
        "Zeno argued the runner never arrives: always half the remaining \
         distance to go, infinitely many steps, so motion is impossible. The \
         square is the answer he did not live to see: one half plus one quarter \
         plus one eighth, infinitely many terms, land exactly inside one square \
         and fill it. An infinite sum can be a finite thing. That single idea is \
         the gate to calculus."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "It took humanity two thousand years to answer Zeno properly: the \
             epsilon-delta limit, built by Cauchy and Weierstrass in the 1800s, \
             is the machinery that says precisely when infinitely many steps \
             arrive somewhere. Your phone computes with it constantly.",
            "Not every infinite sum behaves: one half plus one third plus one \
             quarter plus one fifth, the harmonic series, grows without bound, \
             passing any number you name, given time. Which infinities settle \
             and which explode is a genuine craft, and it has a name: analysis.",
        ]
    }

    fn postcard_t(&self) -> f64 {
        0.75
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_TILES, Zeno, tiles};
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn the_tiles_halve_and_sum_toward_one() {
        let all = tiles();
        assert_eq!(all.len(), MAX_TILES);
        let mut sum = 0.0;
        for (i, &(_, _, w, h)) in all.iter().enumerate() {
            let area = w * h;
            assert!(
                (area - 0.5_f64.powi(i as i32 + 1)).abs() < 1e-12,
                "tile {i} has area 2^-(i+1)"
            );
            sum += area;
        }
        assert!((sum - (1.0 - 0.5_f64.powi(MAX_TILES as i32))).abs() < 1e-12);
        assert!(sum > 0.9999, "the square is all but filled");
    }

    #[test]
    fn the_tiles_do_not_overlap_and_stay_inside() {
        let all = tiles();
        for &(x, y, w, h) in &all {
            assert!(x >= -1e-12 && y >= -1e-12 && x + w <= 1.0 + 1e-12 && y + h <= 1.0 + 1e-12);
        }
        for (i, &(ax, ay, aw, ah)) in all.iter().enumerate() {
            for &(bx, by, bw, bh) in all.iter().skip(i + 1) {
                let overlap_w = (ax + aw).min(bx + bw) - ax.max(bx);
                let overlap_h = (ay + ah).min(by + bh) - ay.max(by);
                assert!(
                    overlap_w <= 1e-9 || overlap_h <= 1e-9,
                    "tiles must not overlap"
                );
            }
        }
    }

    #[test]
    fn render_is_deterministic_and_fills_over_time() {
        let room = Zeno::new();
        let mut early = Canvas::new(50, 30);
        let mut late = Canvas::new(50, 30);
        room.render(&mut early, 0.15);
        room.render(&mut late, 0.9);
        assert!(late.ink_count() > early.ink_count(), "more tiles, more ink");
        let mut again = Canvas::new(50, 30);
        room.render(&mut again, 0.9);
        assert_eq!(late.to_text(), again.to_text());
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Zeno::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_opens_the_gate_to_calculus() {
        assert!(Zeno::new().reveal().contains("calculus"));
    }
}

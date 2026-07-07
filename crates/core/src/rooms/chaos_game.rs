//! Chaos Game: a Sierpinski triangle drawn by pure chance.
//!
//! Start somewhere, then repeatedly pick a random corner of a triangle and move
//! a fixed fraction of the way toward it, leaving a dot each time. There is no
//! triangle in the rule, yet a perfect Sierpinski fractal appears. `t` tunes the
//! jump fraction (0.5 is the iconic value). See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// Fixed seed so the render reproduces exactly (determinism, see `docs/QUALITY.md`).
const SEED: u64 = 0x0DDB_1A5E_5EED_1234;
/// Iterations per cell, capped so large canvases stay fast.
const ITERS_PER_CELL: usize = 6;
const MAX_ITERS: usize = 300_000;
/// Discard the first few points so the transient toward the attractor does not show.
const WARMUP: usize = 16;

/// The Chaos Game room.
#[derive(Debug, Default)]
pub struct ChaosGame;

impl ChaosGame {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// The jump fraction selected by phase `t`; 0.5 (the Sierpinski value) at `t = 0`.
    fn ratio_for(t: f64) -> f64 {
        0.5 + 0.12 * t.clamp(0.0, 1.0)
    }
}

impl Room for ChaosGame {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "chaos-game",
            title: "Chaos Game",
            wing: "Emergence",
            blurb: "Jump halfway to a random corner of a triangle, over and over, and pure chance \
                    resolves into a perfect Sierpinski fractal. t tunes the jump fraction.",
            accent: [40, 200, 170],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let ratio = Self::ratio_for(t);
        let (fw, fh) = (width as f64, height as f64);
        let vertices = [
            (fw / 2.0, 0.0),      // apex
            (0.0, fh - 1.0),      // bottom left
            (fw - 1.0, fh - 1.0), // bottom right
        ];

        let mut rng = SplitMix64::new(SEED);
        let (mut px, mut py) = (fw / 2.0, fh / 2.0);
        let iterations = width
            .saturating_mul(height)
            .saturating_mul(ITERS_PER_CELL)
            .min(MAX_ITERS);
        for i in 0..iterations {
            let corner = vertices[rng.below(3) as usize];
            px = px * (1.0 - ratio) + corner.0 * ratio;
            py = py * (1.0 - ratio) + corner.1 * ratio;
            if i >= WARMUP {
                canvas.plot(px.round() as i32, py.round() as i32, '*');
            }
        }
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "three corners",
            root: 164.81,
            tempo: 132,
            line: &[0, 12, 4, 12, 7, 12, 0, 12],
            encodes: "always jumping halfway home: three notes and their echo",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: ADD A CORNER")
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        if pokes.is_empty() {
            self.render(canvas, t);
            return;
        }
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        // The player's corners join the triangle: four corners fold into a
        // square-dust, five into a pentagon-flake, and the rule never changes.
        let ratio = Self::ratio_for(t);
        let (fw, fh) = (width as f64, height as f64);
        let mut vertices = vec![(fw / 2.0, 0.0), (0.0, fh - 1.0), (fw - 1.0, fh - 1.0)];
        for &(x, y) in pokes.iter().take(3) {
            vertices.push((
                x.clamp(0.0, 1.0) * (fw - 1.0),
                y.clamp(0.0, 1.0) * (fh - 1.0),
            ));
        }
        let mut rng = SplitMix64::new(SEED);
        let (mut px, mut py) = (fw / 2.0, fh / 2.0);
        let iterations = width
            .saturating_mul(height)
            .saturating_mul(ITERS_PER_CELL)
            .min(MAX_ITERS);
        let n = vertices.len() as u64;
        for i in 0..iterations {
            let corner = vertices[rng.below(n) as usize];
            px = px * (1.0 - ratio) + corner.0 * ratio;
            py = py * (1.0 - ratio) + corner.1 * ratio;
            if i >= WARMUP {
                canvas.plot(px.round() as i32, py.round() as i32, '*');
            }
        }
        // The corners themselves, bright, so the hand sees what it built.
        for &(vx, vy) in &vertices {
            for dx in -1..=1 {
                for dy in -1..=1 {
                    canvas.plot(vx as i32 + dx, vy as i32 + dy, '#');
                }
            }
        }
    }

    fn reveal(&self) -> &'static str {
        "Every dot landed at random, and there is no triangle anywhere in the \
         rule, only 'jump halfway to a random corner'. Yet a perfect Sierpinski \
         fractal appears every time. Randomness has a shape."
    }
}

#[cfg(test)]
mod tests {
    use super::ChaosGame;
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn ratio_is_one_half_at_zero() {
        assert!((ChaosGame::ratio_for(0.0) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn render_is_deterministic() {
        let room = ChaosGame::new();
        let mut a = Canvas::new(48, 24);
        let mut b = Canvas::new(48, 24);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = ChaosGame::new();
        let mut canvas = Canvas::new(48, 24);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 10);
    }

    #[test]
    fn zero_sized_and_extreme_inputs_do_not_panic() {
        let room = ChaosGame::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-3.0, 0.0, 0.999, 4.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_mentions_the_shape_of_randomness() {
        assert!(ChaosGame::new().reveal().contains("Sierpinski"));
    }
}

//! The tetractys. This room is not in the catalog.
//!
//! Ten points in four rows: one, two, three, four. The Pythagoreans swore their
//! oaths on this figure and called it the fountain of ever-flowing nature. It
//! answers only to those who have learned its name and earned the asking (the
//! faces gate it by rank; see `crate::journey` and `docs/LORE.md`).

use std::f64::consts::TAU;

use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// The hidden room.
#[derive(Debug, Default)]
pub struct Tetractys;

impl Tetractys {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Room for Tetractys {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "tetractys",
            title: "Tetractys",
            wing: "The Order",
            blurb: "One, two, three, four. You were not told about this room, which means you \
                    found it, which means it is yours.",
            accent: [240, 220, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let aspect = canvas.char_aspect();
        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;
        let extent = (width as f64 / 2.0).min(height as f64 / (2.0 * aspect)) * 0.8;
        // The ten points breathe gently with t.
        let breath = 1.0 + 0.06 * (TAU * t.clamp(0.0, 1.0)).sin();
        let spacing = extent / 2.0 * breath;
        // Row r (0..4) has r+1 points, centered; the triangle points upward.
        for row in 0..4u32 {
            let y = cy + (f64::from(row) - 1.5) * spacing * aspect;
            for point in 0..=row {
                let x = cx + (f64::from(point) - f64::from(row) / 2.0) * spacing;
                // Each point is a small filled diamond, so it reads at any size.
                let radius = (spacing / 6.0).max(1.0);
                let steps = 64;
                for i in 0..steps {
                    let theta = TAU * f64::from(i) / f64::from(steps);
                    for reach in 0..=(radius as i32) {
                        let px = x + f64::from(reach) * theta.cos();
                        let py = y + f64::from(reach) * theta.sin() * aspect;
                        canvas.plot(px as i32, py as i32, '#');
                    }
                }
            }
        }
    }

    fn reveal(&self) -> &'static str {
        "One and two and three and four are ten, and ten holds the point, the \
         line, the plane, and the solid: everything. They swore their oaths on \
         this figure and told no one why. Now it is your turn to tell no one."
    }
}

#[cfg(test)]
mod tests {
    use super::Tetractys;
    use crate::canvas::Canvas;
    use crate::registry::room_by_id;
    use crate::room::Room;

    #[test]
    fn the_tetractys_is_not_in_the_catalog() {
        assert!(room_by_id("tetractys").is_none(), "it must stay hidden");
    }

    #[test]
    fn render_draws_and_does_not_panic() {
        let room = Tetractys::new();
        let mut canvas = Canvas::new(40, 24);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 10);
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
    }

    #[test]
    fn the_reveal_keeps_the_oath() {
        assert!(Tetractys::new().reveal().contains("tell no one"));
    }
}

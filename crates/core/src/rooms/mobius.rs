//! The Mobius strip: one side, one edge, and it takes two laps to get home.
//!
//! A band with a half twist. An ant walks the centerline; after one full lap
//! it is on the "other side" without ever crossing an edge, and only after two
//! laps is it home. The boundary, traced, turns out to be a single closed
//! curve. `t` walks the ant. See the Full Map in `docs/ROOMS.md`.

use std::f64::consts::TAU;

use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// The ring radius.
const R: f64 = 1.0;
/// The half-width of the band.
const W: f64 = 0.35;

/// A point on the strip at ring angle `u` and width position `v` in [-W, W],
/// with the half twist that makes it what it is.
fn strip(u: f64, v: f64) -> (f64, f64, f64) {
    let x = (R + v * (u / 2.0).cos()) * u.cos();
    let y = (R + v * (u / 2.0).cos()) * u.sin();
    let z = v * (u / 2.0).sin();
    (x, y, z)
}

/// Project 3D to 2D: a fixed gentle tilt, so the twist reads.
fn project(x: f64, y: f64, z: f64) -> (f64, f64) {
    let tilt = 0.9_f64;
    (x, y * tilt.cos() * 0.55 + z * tilt.sin())
}

/// The Mobius strip room.
#[derive(Debug, Default)]
pub struct Mobius;

impl Mobius {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Room for Mobius {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "mobius",
            title: "Mobius Strip",
            wing: "Shape & Space",
            blurb: "A band with a half twist: one side, one edge. The ant walks a full lap and \
                    arrives on the other side without crossing anything. Two laps to get home.",
            accent: [120, 200, 255],
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
        let scale = (width as f64 / 2.0).min(height as f64 / (2.0 * aspect)) * 0.6;
        let to_screen = |x: f64, y: f64, z: f64| {
            let (px, py) = project(x, y, z);
            ((cx + px * scale) as i32, (cy + py * scale * aspect) as i32)
        };

        // The band: rungs across the strip, dim, so the twist reads as form.
        let rungs = 90;
        for i in 0..rungs {
            let u = TAU * f64::from(i) / f64::from(rungs);
            let (x0, y0, z0) = strip(u, -W);
            let (x1, y1, z1) = strip(u, W);
            let a = to_screen(x0, y0, z0);
            let b = to_screen(x1, y1, z1);
            canvas.line(a.0, a.1, b.0, b.1, '-');
        }
        // The single edge: follow v = +W around TWICE and it closes. One curve.
        let steps = 260;
        let mut previous: Option<(i32, i32)> = None;
        for i in 0..=steps {
            let u = 2.0 * TAU * f64::from(i) / f64::from(steps);
            // After one lap the +W side has become the -W side: same edge.
            let (x, y, z) = strip(u % TAU, if u < TAU { W } else { -W });
            let point = to_screen(x, y, z);
            if let Some((px, py)) = previous {
                canvas.line(px, py, point.0, point.1, '*');
            }
            previous = Some(point);
        }
        // The ant: two laps of the centerline to get home. Its trail shows
        // where it has been; at t = 0.5 it is "underneath", upside down.
        let ant_u = 2.0 * TAU * t.clamp(0.0, 1.0);
        let trail = 40;
        for i in 0..trail {
            let u = ant_u - 0.06 * f64::from(i);
            if u < 0.0 {
                break;
            }
            let (x, y, z) = strip(u % TAU, 0.0);
            let (px, py) = to_screen(x, y, z);
            canvas.plot(px, py, '*');
        }
        let (x, y, z) = strip(ant_u % TAU, 0.0);
        let (px, py) = to_screen(x, y, z);
        for dx in -1..=1 {
            for dy in -1..=1 {
                canvas.plot(px + dx, py + dy, '#');
            }
        }
    }

    fn reveal(&self) -> &'static str {
        "Cut a paper strip, give it a half twist, tape the ends: you have made \
         a shape with one side and one edge. Run a finger along the edge and it \
         visits everything before returning. The ant's lap that ends upside \
         down is not a trick; sidedness itself is a property a surface can \
         simply decline to have."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Cut the strip down its centerline and it does not fall into two \
             pieces: it becomes one longer band with a full twist. Cut a third \
             of the way in and you get two linked bands. Scissors are the best \
             topology teacher there is.",
            "Every conveyor belt and typewriter ribbon built as a Mobius band \
             wears both faces evenly, doubling its life, a patent that exists \
             because sidedness is optional.",
        ]
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }
}

#[cfg(test)]
mod tests {
    use super::{Mobius, W, strip};
    use crate::canvas::Canvas;
    use crate::room::Room;
    use std::f64::consts::TAU;

    #[test]
    fn the_half_twist_swaps_the_sides_after_one_lap() {
        // The point at (u=0, v=+W) and the point one lap on at (u=TAU, v=-W)
        // coincide: the two "sides" are the same side.
        let (x0, y0, z0) = strip(0.0, W);
        let (x1, y1, z1) = strip(TAU, -W);
        // strip() takes u mod TAU in render; compare the raw formula at TAU.
        assert!((x0 - x1).abs() < 1e-9, "{x0} vs {x1}");
        assert!((y0 - y1).abs() < 1e-9, "{y0} vs {y1}");
        assert!((z0 - z1).abs() < 1e-9, "{z0} vs {z1}");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = Mobius::new();
        let mut a = Canvas::new(50, 30);
        let mut b = Canvas::new(50, 30);
        room.render(&mut a, 0.35);
        room.render(&mut b, 0.35);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 40);
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Mobius::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 0.5, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_declines_sidedness() {
        assert!(Mobius::new().reveal().contains("one side"));
    }
}

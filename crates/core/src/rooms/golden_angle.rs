//! Golden Angle: how a sunflower packs its seeds.
//!
//! Place seeds one at a time (Vogel's model): seed `k` sits at angle
//! `k * step` and radius proportional to `sqrt(k)`. At the golden angle
//! (about 137.5 degrees) the seeds pack into a flawless spiral; nudge the angle
//! and the packing shatters into spokes and gaps. `t` detunes the angle. See
//! `docs/ROOMS.md` and `docs/INSIGHTS.md`.

use std::f64::consts::PI;

use crate::room::{Room, RoomMeta};
use crate::surface::Surface;

/// How far `t` can push the angle away from golden, in radians. A small nudge is
/// enough to visibly break the packing.
const MAX_DETUNE: f64 = 0.20;

/// The Golden Angle room.
#[derive(Debug, Default)]
pub struct GoldenAngle;

impl GoldenAngle {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// The golden angle in radians: `pi * (3 - sqrt(5))`, about 2.39996.
    fn golden_angle() -> f64 {
        PI * (3.0 - 5.0_f64.sqrt())
    }

    /// The angle between successive seeds at phase `t`; exactly golden at `t = 0`.
    fn angle_step_for(t: f64) -> f64 {
        Self::golden_angle() + t.clamp(0.0, 1.0) * MAX_DETUNE
    }
}

impl Room for GoldenAngle {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "golden-angle",
            title: "Golden Angle",
            wing: "Number & Pattern",
            blurb: "Place seeds one at a time, each turned a fixed angle from the last; at the \
                    golden angle they pack into a flawless sunflower, and a nudge shatters it. \
                    t detunes the angle.",
            accent: [210, 160, 40],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let step = Self::angle_step_for(t);
        let (fw, fh) = (width as f64, height as f64);
        let (cx, cy) = (fw / 2.0, fh / 2.0);

        let seeds = (width * height / 3).clamp(50, 4000);
        // Scale so the outermost seed just fits both extents (y uses the surface
        // aspect: 0.5 for tall terminal cells, 1.0 for square pixels).
        let aspect = canvas.char_aspect();
        let scale = (fw / 2.0).min(fh / (2.0 * aspect)) / (seeds as f64).sqrt();

        for k in 0..seeds {
            let theta = k as f64 * step;
            let radius = scale * (k as f64).sqrt();
            let x = cx + radius * theta.cos();
            let y = cy + radius * theta.sin() * aspect;
            canvas.plot(x.round() as i32, y.round() as i32, '*');
        }
    }

    fn reveal(&self) -> &'static str {
        "Sunflowers, pinecones, and pineapples all use this exact angle, about \
         137.5 degrees, built from the golden ratio, the most irrational number, \
         so the seeds never line up and never waste space. Evolution found the \
         same number the mathematicians did."
    }
}

#[cfg(test)]
mod tests {
    use super::GoldenAngle;
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn step_is_the_golden_angle_at_zero() {
        assert!((GoldenAngle::angle_step_for(0.0) - 2.399_963_2).abs() < 1e-6);
    }

    #[test]
    fn detuning_increases_the_step() {
        assert!(GoldenAngle::angle_step_for(1.0) > GoldenAngle::angle_step_for(0.0));
    }

    #[test]
    fn render_is_deterministic() {
        let room = GoldenAngle::new();
        let mut a = Canvas::new(40, 30);
        let mut b = Canvas::new(40, 30);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = GoldenAngle::new();
        let mut canvas = Canvas::new(40, 30);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 10);
    }

    #[test]
    fn zero_sized_and_extreme_inputs_do_not_panic() {
        let room = GoldenAngle::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(6, 6);
        for t in [-2.0, 0.0, 0.999, 3.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn reveal_names_the_angle() {
        assert!(GoldenAngle::new().reveal().contains("137.5"));
    }
}

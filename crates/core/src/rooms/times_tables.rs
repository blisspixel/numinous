//! Times Tables: modular multiplication on a circle. The flagship room.
//!
//! Place `N` points evenly on a circle and, from each point `n`, draw a chord to
//! point `round(n * k) mod N`. At `k = 2` a cardioid blooms out of nothing but
//! the two-times table; sweeping `k` upward morphs it through nephroids and
//! nested lobes. See `docs/ROOMS.md` and `docs/INSIGHTS.md`.

use std::f64::consts::{FRAC_PI_2, TAU};

use crate::room::{Room, RoomMeta};
use crate::sound::SoundSpec;
use crate::surface::Surface;

/// Number of points placed around the circle. Higher is smoother and denser.
const POINTS: usize = 240;
/// The multiplier `k` at phase `t = 0` (the two-times table, a cardioid).
const K_MIN: f64 = 2.0;
/// How far `k` sweeps across the phase range `[0, 1)`.
const K_SWEEP: f64 = 8.0;

/// The Times Tables room.
#[derive(Debug, Default)]
pub struct TimesTables {
    seed: u64,
}

impl TimesTables {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed for replayable per-visit novelty.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    fn phase_for(&self, t: f64) -> f64 {
        let phase = t.clamp(0.0, 1.0);
        if self.seed == 0 {
            phase
        } else {
            let offset = 0.08 + (self.seed % 997) as f64 / 997.0 * 0.17;
            (phase + offset).fract()
        }
    }
}

impl Room for TimesTables {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "times-tables",
            title: "Times Tables",
            wing: "Number & Pattern",
            blurb: "From each point n on a circle, draw a chord to point (n times k); \
                    a cardioid blooms out of the two-times table.",
            accent: [40, 150, 190],
        }
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "D minor pentatonic",
            root: 146.83,
            tempo: 96,
            line: &[0, 5, 7, 12, 7, 5, 0, 7],
            encodes: "circling and returning: modular arithmetic closes its loop",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TURN THE DIAL")
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
        let n = POINTS;
        let (fw, fh) = (width as f64, height as f64);
        let cx = fw / 2.0;
        let cy = fh / 2.0;
        let r = (fw.min(fh) / 2.0) * 0.45;
        let phase = self.phase_for(t);
        // base
        self.render(canvas, t);
        // poked: extra "twisted" copies at poke positions, using y for local k shift
        for &(px, py) in pokes {
            let shift = (py - 0.5) * 0.5;
            let k = K_MIN + K_SWEEP * (phase + shift).clamp(0.0, 1.0);
            for i in 0..n {
                let a = (i as f64 / n as f64) * TAU;
                let b = (i as f64 * k / n as f64) * TAU;
                let x1 = cx + r * a.cos();
                let y1 = cy + r * a.sin();
                let x2 = cx + r * b.cos();
                let y2 = cy + r * b.sin();
                // offset by poke
                let ox = (px - 0.5) * 20.0;
                let oy = (py - 0.5) * 20.0;
                canvas.line(
                    (x1 + ox) as i32,
                    (y1 + oy) as i32,
                    (x2 + ox) as i32,
                    (y2 + oy) as i32,
                    '*',
                );
            }
        }
    }

    fn status(&self, t: f64) -> Option<String> {
        let k = K_MIN + K_SWEEP * self.phase_for(t);
        let nearest = k.round();
        let off = (k - nearest).abs();
        let note = if off < 0.02 {
            format!("CLOSED: {} LOBES", (nearest as i64 - 1).max(0))
        } else if off < 0.15 {
            format!("ALMOST: {} LOBES FORMING", (nearest as i64 - 1).max(0))
        } else {
            "OPEN, WANDERING".to_string()
        };
        Some(format!("K = {k:.2}   {note}"))
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let multiplier = K_MIN + K_SWEEP * self.phase_for(t);

        let width = canvas.width() as f64;
        let height = canvas.height() as f64;
        let cx = width / 2.0;
        let cy = height / 2.0;
        // Squash y by the surface's aspect (0.5 for tall terminal cells, 1.0 for
        // square pixels) so the circle stays round on any surface.
        let aspect = canvas.char_aspect();
        // Fit both extents: x uses width/2, y uses radius*aspect <= height/2.
        let radius = (width / 2.0).min(height / (2.0 * aspect)) * 0.9;

        let point = |i: usize| -> (i32, i32) {
            let angle = (i as f64 / POINTS as f64) * TAU - FRAC_PI_2;
            let x = cx + radius * angle.cos();
            let y = cy + radius * angle.sin() * aspect;
            (x.round() as i32, y.round() as i32)
        };

        for n in 0..POINTS {
            let target = ((n as f64) * multiplier).round() as usize % POINTS;
            let (x0, y0) = point(n);
            let (x1, y1) = point(target);
            canvas.line(x0, y0, x1, y1, '*');
        }
    }

    fn reveal(&self) -> &'static str {
        "Set the dial to 2 and this table draws a heart. That cardioid is the \
         exact outline of the Mandelbrot set's main body: a homework grid and the \
         most complex object in mathematics trace the same shape."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Change the multiplier and the curve changes family: times 2 draws a \
             cardioid, times 3 a nephroid, and times n a curve with n minus 1 cusps. \
             The whole zoo is one rule with the dial turned.",
            "The strings you see are modular arithmetic, the same math that lets an \
             accountant check a ledger by casting out nines and lets your bank card \
             verify itself with a check digit. The pretty curve and the checksum are \
             the same idea.",
        ]
    }
    fn sound(&self, t: f64) -> SoundSpec {
        // Pitch rises with the multiplier k; landing on a whole number sounds clean.
        let k = (K_MIN + K_SWEEP * self.phase_for(t)) as f32;
        SoundSpec::tone(55.0 * k, 1.5, 0.2)
    }
}

#[cfg(test)]
mod tests {
    use super::TimesTables;
    use crate::canvas::Canvas;
    use crate::room::Room;

    #[test]
    fn meta_is_stable() {
        let m = TimesTables::new().meta();
        assert_eq!(m.id, "times-tables");
        assert_eq!(m.wing, "Number & Pattern");
    }

    #[test]
    fn reveal_names_the_connection() {
        assert!(TimesTables::new().reveal().contains("Mandelbrot"));
    }

    #[test]
    fn sound_is_a_single_tone() {
        let spec = TimesTables::new().sound(0.0);
        assert_eq!(spec.notes.len(), 1);
        assert!(spec.notes[0].freq > 0.0);
    }

    #[test]
    fn new_with_zero_matches_default() {
        let mut a = Canvas::new(48, 28);
        let mut b = Canvas::new(48, 28);
        TimesTables::new().render(&mut a, 0.35);
        TimesTables::new_with(0).render(&mut b, 0.35);
        assert_eq!(a.to_text(), b.to_text());
        assert_eq!(
            TimesTables::new().status(0.35),
            TimesTables::new_with(0).status(0.35)
        );
    }

    #[test]
    fn new_with_nonzero_produces_variation() {
        let mut a = Canvas::new(48, 28);
        let mut b = Canvas::new(48, 28);
        TimesTables::new_with(0).render(&mut a, 0.2);
        TimesTables::new_with(42).render(&mut b, 0.2);
        assert_ne!(a.to_text(), b.to_text());
        assert_ne!(
            TimesTables::new_with(0).status(0.2),
            TimesTables::new_with(42).status(0.2)
        );
    }

    #[test]
    fn the_dial_reads_back_and_knows_closure() {
        use crate::room::Room;
        let room = TimesTables::new();
        // Some t where k is an integer: K_MIN..K_MIN+K_SWEEP hits integers.
        let t_closed = (2.0 - super::K_MIN) / super::K_SWEEP;
        let closed = room.status(t_closed).expect("the dial speaks");
        assert!(closed.contains("CLOSED"), "{closed}");
        assert!(closed.contains("K = 2.00"), "{closed}");
        let open = room
            .status(t_closed + 0.4 / super::K_SWEEP)
            .expect("still speaks");
        assert!(!open.contains("CLOSED"), "{open}");
    }

    #[test]
    fn render_is_deterministic() {
        let room = TimesTables::new();
        let mut a = Canvas::new(80, 40);
        let mut b = Canvas::new(80, 40);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
    }

    #[test]
    fn render_produces_ink() {
        let room = TimesTables::new();
        let mut canvas = Canvas::new(80, 40);
        room.render(&mut canvas, 0.0);
        assert!(canvas.ink_count() > 0, "the cardioid should draw something");
    }

    #[test]
    fn extreme_phase_values_do_not_panic() {
        let room = TimesTables::new();
        let mut canvas = Canvas::new(40, 20);
        for t in [-5.0, 0.0, 0.5, 0.999, 5.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn tiny_canvas_does_not_panic() {
        let room = TimesTables::new();
        let mut canvas = Canvas::new(1, 1);
        room.render(&mut canvas, 0.3);
    }
}

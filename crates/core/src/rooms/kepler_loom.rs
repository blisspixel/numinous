//! Kepler's Loom: every throw an ellipse, equal areas as metronome.
//!
//! A fixed sun, a flung moon. Newton gives a conic; for bound orbits the path
//! is an ellipse with the sun at one focus. Equal areas in equal times is the
//! metronome: near periapsis the craft races, near apoapsis it crawls. `t`
//! advances the ambient orbit; DRAG: FLING A MOON. See `docs/ROOMS.md`.

use std::f64::consts::TAU;

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

/// Softened gravity constant.
const MU: f64 = 0.45;
const SOFT: f64 = 0.02;
const DT: f64 = 0.0035;
const MAX_STEPS: usize = 2_000;
const ENTRY_STEPS: usize = 500;
const VARIATION_SALT: u64 = 0x4E77_00E1_5EED_0001;

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

#[derive(Clone, Copy, Debug)]
struct State {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
}

fn sun_pos(seed: u64) -> (f64, f64) {
    if seed == 0 {
        (0.5, 0.5)
    } else {
        let mix = seed ^ VARIATION_SALT;
        (
            0.42 + ((mix % 17) as f64) * 0.01,
            0.42 + (((mix / 17) % 17) as f64) * 0.01,
        )
    }
}

fn accel(s: State, sx: f64, sy: f64) -> (f64, f64) {
    let dx = sx - s.x;
    let dy = sy - s.y;
    let r2 = dx * dx + dy * dy + SOFT * SOFT;
    let inv = MU / (r2 * r2.sqrt());
    (dx * inv, dy * inv)
}

fn integrate(mut s: State, sx: f64, sy: f64, steps: usize) -> (Vec<(f64, f64)>, f64, f64) {
    let steps = steps.min(MAX_STEPS);
    let mut path = Vec::with_capacity(steps + 1);
    path.push((s.x, s.y));
    let mut min_r = f64::INFINITY;
    let mut max_r = 0.0_f64;
    for _ in 0..steps {
        let (ax, ay) = accel(s, sx, sy);
        s.vx += ax * DT * 0.5;
        s.vy += ay * DT * 0.5;
        s.x += s.vx * DT;
        s.y += s.vy * DT;
        let (ax2, ay2) = accel(s, sx, sy);
        s.vx += ax2 * DT * 0.5;
        s.vy += ay2 * DT * 0.5;
        let r = (s.x - sx).hypot(s.y - sy);
        min_r = min_r.min(r);
        max_r = max_r.max(r);
        path.push((s.x, s.y));
        if s.x < -0.3 || s.x > 1.3 || s.y < -0.3 || s.y > 1.3 {
            break;
        }
    }
    (path, min_r, max_r)
}

fn ambient_state(t: f64, seed: u64) -> (State, usize) {
    let (sx, sy) = sun_pos(seed);
    let u = phase_unit(t);
    // Circular-ish launch offset.
    let ang = u * TAU;
    let r = 0.22;
    let speed = (MU / r).sqrt() * 0.92;
    let state = State {
        x: sx + r * ang.cos(),
        y: sy + r * ang.sin(),
        vx: -speed * ang.sin(),
        vy: speed * ang.cos(),
    };
    let steps = ENTRY_STEPS + (u * (MAX_STEPS - ENTRY_STEPS) as f64) as usize;
    (state, steps)
}

fn fling_state(from: (f64, f64), to: (f64, f64)) -> State {
    // Release at `to` with velocity from pull (from - to) * gain.
    let gain = 1.8;
    State {
        x: to.0.clamp(0.05, 0.95),
        y: to.1.clamp(0.05, 0.95),
        vx: (from.0 - to.0) * gain,
        vy: (from.1 - to.1) * gain,
    }
}

fn draw_orbit(canvas: &mut dyn Surface, sx: f64, sy: f64, path: &[(f64, f64)]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let to_px = |x: f64, y: f64| -> (i32, i32) {
        (
            (x.clamp(-0.1, 1.1) * (width.saturating_sub(1) as f64)).round() as i32,
            (y.clamp(-0.1, 1.1) * (height.saturating_sub(1) as f64)).round() as i32,
        )
    };
    let (px, py) = to_px(sx, sy);
    for dy in -2..=2 {
        for dx in -2..=2 {
            if dx * dx + dy * dy <= 4 {
                canvas.plot(px + dx, py + dy, '#');
            }
        }
    }
    if path.len() >= 2 {
        let mut prev = to_px(path[0].0, path[0].1);
        for &(x, y) in &path[1..] {
            let cur = to_px(x, y);
            canvas.line(prev.0, prev.1, cur.0, cur.1, '*');
            prev = cur;
        }
        canvas.plot(prev.0, prev.1, 'o');
    }
}

/// Kepler's Loom room.
#[derive(Debug, Default)]
pub struct KeplerLoom {
    seed: u64,
}

impl KeplerLoom {
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

impl Room for KeplerLoom {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "kepler-loom",
            title: "Kepler's Loom",
            wing: "Motion & Dynamics",
            blurb: "Fling a moon around a sun: every bound path is an ellipse with the sun at a \
                    focus. Equal areas in equal times is the metronome. t advances the orbit; \
                    DRAG: FLING A MOON.",
            accent: [220, 200, 100],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (sx, sy) = sun_pos(self.seed);
        let (state, steps) = ambient_state(t, self.seed);
        let (path, _, _) = integrate(state, sx, sy, steps);
        draw_orbit(canvas, sx, sy, &path);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "equal areas",
            root: 185.0,
            tempo: 108,
            line: &[0, 4, 7, 11, 7, 4, 0, 7],
            encodes: "periapsis rush and apoapsis crawl as one metronome",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: FLING A MOON")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (sx, sy) = sun_pos(self.seed);
        let (state, steps) = ambient_state(t, self.seed);
        let (_, rmin, rmax) = integrate(state, sx, sy, steps);
        let ecc = if rmax + rmin > 1e-9 {
            (rmax - rmin) / (rmax + rmin)
        } else {
            0.0
        };
        Some(format!("e={ecc:.2}  r={rmin:.2}/{rmax:.2}  DRAG:FLING"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        if hands.is_empty() {
            self.render(canvas, t);
            return;
        }
        let (sx, sy) = sun_pos(self.seed);
        let newest = *hands.last().expect("nonempty");
        let older = hands
            .len()
            .checked_sub(2)
            .map(|i| hands[i])
            .unwrap_or((newest.0 + 0.12, newest.1));
        let state = fling_state(older, newest);
        let steps = ENTRY_STEPS + (phase_unit(t) * (MAX_STEPS - ENTRY_STEPS) as f64) as usize;
        let (path, _, _) = integrate(state, sx, sy, steps);
        draw_orbit(canvas, sx, sy, &path);
        let (width, height) = canvas.draw_bounds();
        if width > 0 && height > 0 {
            for &(x, y) in &hands {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.plot(px, py, '+');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let (sx, sy) = sun_pos(self.seed);
        let newest = *hands.last().expect("nonempty");
        let older = hands
            .len()
            .checked_sub(2)
            .map(|i| hands[i])
            .unwrap_or((newest.0 + 0.12, newest.1));
        let state = fling_state(older, newest);
        let steps = ENTRY_STEPS + (phase_unit(t) * (MAX_STEPS - ENTRY_STEPS) as f64) as usize;
        let (_, rmin, rmax) = integrate(state, sx, sy, steps);
        let ecc = if rmax + rmin > 1e-9 {
            (rmax - rmin) / (rmax + rmin)
        } else {
            0.0
        };
        let bound = if rmax < 1.2 { "ELLIPSE" } else { "FLYBY" };
        Some(format!("FLING e={ecc:.2}  rp={rmin:.2}  {bound}"))
    }

    fn reveal(&self) -> &'static str {
        "Bound under inverse-square gravity, every closed path is an ellipse with \
         the sun at one focus. The radius vector sweeps equal areas in equal times: \
         that is why periapsis is a sprint and apoapsis a crawl. Kepler found the \
         shapes; Newton found the law."
    }
}

#[cfg(test)]
mod tests {
    use super::{KeplerLoom, State, ambient_state, fling_state, integrate, sun_pos};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn circular_orbit_stays_near_constant_radius() {
        let (sx, sy) = sun_pos(0);
        let (state, _) = ambient_state(0.0, 0);
        let (_, rmin, rmax) = integrate(state, sx, sy, 800);
        assert!(rmax - rmin < 0.15, "rmin={rmin} rmax={rmax}");
    }

    #[test]
    fn fling_moves_the_moon() {
        let s = fling_state((0.7, 0.5), (0.3, 0.5));
        assert!(s.vx > 0.0);
        let (sx, sy) = sun_pos(0);
        let (path, _, _) = integrate(s, sx, sy, 200);
        assert!(path.len() > 10);
    }

    #[test]
    fn gravity_pulls_toward_sun() {
        let (sx, sy) = sun_pos(0);
        let s = State {
            x: 0.2,
            y: 0.5,
            vx: 0.0,
            vy: 0.0,
        };
        let (path, _, _) = integrate(s, sx, sy, 50);
        let last = *path.last().unwrap();
        assert!(last.0 > 0.2, "should fall toward sun at x=0.5");
    }

    #[test]
    fn first_contact_status_invites_fling() {
        let room = KeplerLoom::new();
        let open = room.status(0.0).expect("open");
        assert!(open.contains("DRAG") || open.contains("FLING"), "{open}");
        assert!(open.chars().count() <= 56, "{open}");
    }

    #[test]
    fn fling_changes_status() {
        let room = KeplerLoom::new();
        let open = room.status(0.0).expect("open");
        let input = [RoomInput::PointerDown {
            x: 0.25,
            y: 0.4,
            t: 0.0,
        }];
        let after = room.status_input(0.0, &input).expect("fling");
        assert_ne!(after, open);
        assert!(after.contains("FLING") || after.contains("e="), "{after}");
        assert!(after.chars().count() <= 56, "{after}");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = KeplerLoom::new();
        let mut a = Canvas::new(48, 32);
        let mut b = Canvas::new(48, 32);
        room.render(&mut a, 0.5);
        room.render(&mut b, 0.5);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 15);
    }

    #[test]
    fn hand_fling_changes_orbit() {
        let room = KeplerLoom::new();
        let mut a = Canvas::new(40, 28);
        let mut b = Canvas::new(40, 28);
        room.render(&mut a, 0.3);
        room.render_poked(&mut b, 0.3, &[(0.7, 0.5), (0.25, 0.55)]);
        assert_ne!(a.to_text(), b.to_text());
    }

    #[test]
    fn variation_moves_the_sun() {
        assert_ne!(sun_pos(0), sun_pos(5));
        let mut a = Canvas::new(40, 28);
        let mut b = Canvas::new(40, 28);
        KeplerLoom::new_with(0).render(&mut a, 0.4);
        KeplerLoom::new_with(5).render(&mut b, 0.4);
        assert_ne!(a.to_text(), b.to_text());
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = KeplerLoom::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, f64::NAN, f64::INFINITY] {
            room.render(&mut canvas, t);
            room.render_poked(&mut canvas, t, &[(f64::NAN, f64::INFINITY)]);
        }
    }

    #[test]
    fn reveal_names_ellipse_or_kepler() {
        let text = KeplerLoom::new().reveal().to_ascii_lowercase();
        assert!(text.contains("ellipse") || text.contains("kepler") || text.contains("areas"));
    }

    #[test]
    fn motif_is_playable() {
        let motif = KeplerLoom::new().motif().expect("motif");
        assert!(motif.line.len() >= 6);
        assert!(motif.pattern().seconds() > 0.0);
    }
}

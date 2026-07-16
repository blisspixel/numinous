//! Slingshot: pull, release, and discover gravity assists by hand.
//!
//! One or more suns curve spacetime the cheap way (Newtonian gravity). A probe
//! launched by pull-and-release integrates under their pull. Swing close to a
//! sun and leave faster than you arrived: the assist is not taught, only felt.
//! `t` advances the ambient mission clock. Missed probes keep flying as comets.
//! See `docs/ROOMS.md`.

use crate::room::{Gesture, Room, RoomInput, RoomMeta, latest_gesture, pokes_from_inputs};
use crate::surface::Surface;

/// Softened Newtonian constant (normalized plate units).
const G: f64 = 0.55;
/// Softening length so close passes do not explode.
const SOFT: f64 = 0.04;
/// Integration step.
const DT: f64 = 0.004;
/// Maximum integration steps for a trail.
const MAX_STEPS: usize = 2_400;
/// Ambient demo launch steps scale with phase.
const ENTRY_STEPS: usize = 400;
/// Maximum suns grown by holds / variation.
const MAX_SUNS: usize = 4;
/// Default sun mass.
const SUN_MASS: f64 = 1.0;
/// Salt for nonzero variation course layout.
const VARIATION_SALT: u64 = 0x5114_6507_5EED_0001;

#[derive(Clone, Copy, Debug)]
struct Body {
    x: f64,
    y: f64,
    mass: f64,
}

#[derive(Clone, Copy, Debug)]
struct Probe {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
}

fn phase_unit(t: f64) -> f64 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Deterministic course: one central sun, optional companions from seed.
fn course_suns(seed: u64) -> Vec<Body> {
    let mut suns = vec![Body {
        x: 0.52,
        y: 0.50,
        mass: SUN_MASS * 1.15,
    }];
    if seed == 0 {
        // Canonical two-body assist course: a smaller sun off-axis.
        suns.push(Body {
            x: 0.72,
            y: 0.38,
            mass: SUN_MASS * 0.55,
        });
        return suns;
    }
    let mix = seed ^ VARIATION_SALT;
    let n = 1 + ((mix % 3) as usize); // 1..3 extra
    for i in 0..n {
        let k = mix
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(i as u64 * 17);
        let x = 0.25 + ((k % 50) as f64) / 100.0;
        let y = 0.25 + (((k / 50) % 50) as f64) / 100.0;
        let mass = SUN_MASS * (0.4 + ((k % 7) as f64) / 14.0);
        suns.push(Body { x, y, mass });
        if suns.len() >= MAX_SUNS {
            break;
        }
    }
    suns
}

fn accel(p: Probe, suns: &[Body]) -> (f64, f64) {
    let mut ax = 0.0;
    let mut ay = 0.0;
    for s in suns {
        let dx = s.x - p.x;
        let dy = s.y - p.y;
        let r2 = dx * dx + dy * dy + SOFT * SOFT;
        let inv = G * s.mass / (r2 * r2.sqrt());
        ax += dx * inv;
        ay += dy * inv;
    }
    (ax, ay)
}

/// Integrate the probe; return the path and flyby / speed stats.
fn integrate(mut p: Probe, suns: &[Body], steps: usize) -> (Vec<(f64, f64)>, Stats) {
    let steps = steps.min(MAX_STEPS);
    let mut path = Vec::with_capacity(steps + 1);
    path.push((p.x, p.y));
    let mut stats = Stats {
        peak_speed: (p.vx * p.vx + p.vy * p.vy).sqrt(),
        assists: 0,
        min_sun_dist: f64::INFINITY,
        escaped: false,
    };
    let mut was_near = false;
    let mut speed_at_near = 0.0_f64;

    for _ in 0..steps {
        let (ax, ay) = accel(p, suns);
        // Velocity Verlet-ish: kick-drift-kick half steps for stability.
        p.vx += ax * DT * 0.5;
        p.vy += ay * DT * 0.5;
        p.x += p.vx * DT;
        p.y += p.vy * DT;
        let (ax2, ay2) = accel(p, suns);
        p.vx += ax2 * DT * 0.5;
        p.vy += ay2 * DT * 0.5;

        let speed = (p.vx * p.vx + p.vy * p.vy).sqrt();
        stats.peak_speed = stats.peak_speed.max(speed);

        let mut nearest = f64::INFINITY;
        for s in suns {
            let d = (p.x - s.x).hypot(p.y - s.y);
            nearest = nearest.min(d);
        }
        stats.min_sun_dist = stats.min_sun_dist.min(nearest);

        // Assist: enter a close sphere slower, leave faster.
        let near = nearest < 0.12;
        if near && !was_near {
            speed_at_near = speed;
            was_near = true;
        } else if !near && was_near {
            if speed > speed_at_near * 1.05 {
                stats.assists = stats.assists.saturating_add(1);
            }
            was_near = false;
        }

        path.push((p.x, p.y));
        // Soft plate bounds: once far away, call it a comet.
        if p.x < -0.4 || p.x > 1.4 || p.y < -0.4 || p.y > 1.4 {
            stats.escaped = true;
            break;
        }
    }
    (path, stats)
}

#[derive(Clone, Copy, Debug, Default)]
struct Stats {
    peak_speed: f64,
    assists: u32,
    min_sun_dist: f64,
    escaped: bool,
}

/// Ambient demo launch: left edge, aimed at the assist corridor.
fn ambient_probe(t: f64, seed: u64) -> (Probe, usize) {
    let u = phase_unit(t);
    let aim = if seed == 0 {
        0.12
    } else {
        0.08 + ((seed % 9) as f64) * 0.02
    };
    let probe = Probe {
        x: 0.08,
        y: 0.55 + aim * 0.1,
        vx: 0.55 + u * 0.35,
        vy: -0.12 + aim * 0.05,
    };
    let steps = ENTRY_STEPS + (u * (MAX_STEPS - ENTRY_STEPS) as f64) as usize;
    (probe, steps)
}

fn launch_from_points(from: (f64, f64), to: (f64, f64)) -> Probe {
    // Pull back from `to` toward `from` like a sling: release at `to`, velocity
    // points opposite the pull (from - to) is wrong for slingshot; classic is
    // grab at rest point, pull to `to`, release so velocity is from pull dir.
    // We treat `before` as pull tip and `at` as release point: velocity = at - before
    // times gain (flick forward). Double-pendulum uses fling; here pull-back:
    // velocity = (before - at) * gain so pulling left launches right.
    let dx = from.0 - to.0;
    let dy = from.1 - to.1;
    let gain = 2.8;
    Probe {
        x: to.0.clamp(0.0, 1.0),
        y: to.1.clamp(0.0, 1.0),
        vx: dx * gain,
        vy: dy * gain,
    }
}

fn combine_suns(seed: u64, inputs: &[RoomInput]) -> Vec<Body> {
    let mut suns = course_suns(seed);
    // Stationary hold grows a sun (HOLD grows suns); a dragged hold is an aim.
    if let Some(Gesture::Held { from, at }) = latest_gesture(inputs) {
        let drag = (from.0 - at.0).hypot(from.1 - at.1);
        if drag < 0.04 {
            let plant = Body {
                x: at.0.clamp(0.0, 1.0),
                y: at.1.clamp(0.0, 1.0),
                mass: SUN_MASS * 0.85,
            };
            if !suns
                .iter()
                .any(|s| (s.x - plant.x).hypot(s.y - plant.y) < 0.06)
            {
                suns.push(plant);
            }
        }
    }
    suns.truncate(MAX_SUNS);
    suns
}

/// Recover the pointer-down that opened the newest gesture, if still in trail.
fn newest_down(inputs: &[RoomInput]) -> Option<(f64, f64)> {
    for input in inputs.iter().rev() {
        if let RoomInput::PointerDown { x, y, .. } = *input {
            if x.is_finite() && y.is_finite() {
                return Some((x, y));
            }
        }
        if matches!(*input, RoomInput::PointerCancel) {
            break;
        }
    }
    None
}

fn draw_scene(
    canvas: &mut dyn Surface,
    suns: &[Body],
    path: &[(f64, f64)],
    aim: Option<((f64, f64), (f64, f64))>,
) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let to_px = |x: f64, y: f64| -> (i32, i32) {
        (
            (x.clamp(-0.2, 1.2) * (width.saturating_sub(1) as f64)).round() as i32,
            (y.clamp(-0.2, 1.2) * (height.saturating_sub(1) as f64)).round() as i32,
        )
    };

    // Suns.
    for s in suns {
        let (px, py) = to_px(s.x, s.y);
        let rad = (3.0 + s.mass * 3.0).round() as i32;
        for dy in -rad..=rad {
            for dx in -rad..=rad {
                if dx * dx + dy * dy <= rad * rad {
                    canvas.plot(
                        px + dx,
                        py + dy,
                        if dx * dx + dy * dy < 2 { '#' } else { 'O' },
                    );
                }
            }
        }
    }

    // Probe trail.
    if path.len() >= 2 {
        let mut prev = to_px(path[0].0, path[0].1);
        for &(x, y) in &path[1..] {
            let cur = to_px(x, y);
            canvas.line(prev.0, prev.1, cur.0, cur.1, '*');
            prev = cur;
        }
        let (tx, ty) = prev;
        canvas.plot(tx, ty, '+');
    }

    // Aim rubber band while holding a pull.
    if let Some((from, to)) = aim {
        let a = to_px(from.0, from.1);
        let b = to_px(to.0, to.1);
        canvas.line(a.0, a.1, b.0, b.1, '.');
        canvas.plot(b.0, b.1, 'o');
    }
}

/// Mission state derived from inputs and phase.
struct Mission {
    suns: Vec<Body>,
    path: Vec<(f64, f64)>,
    stats: Stats,
    aim: Option<((f64, f64), (f64, f64))>,
    label: &'static str,
}

fn pack(
    suns: Vec<Body>,
    path: Vec<(f64, f64)>,
    stats: Stats,
    aim: Option<((f64, f64), (f64, f64))>,
    label: &'static str,
) -> Mission {
    Mission {
        suns,
        path,
        stats,
        aim,
        label,
    }
}

fn mission(t: f64, seed: u64, inputs: &[RoomInput]) -> Mission {
    let suns = combine_suns(seed, inputs);
    match latest_gesture(inputs) {
        Some(Gesture::Held { from, at }) => {
            let drag = (from.0 - at.0).hypot(from.1 - at.1);
            if drag < 0.04 {
                // Stationary hold: grow a sun, show ambient course under it.
                let (probe, steps) = ambient_probe(t, seed);
                let (path, stats) = integrate(probe, &suns, steps);
                pack(suns, path, stats, None, "SUN")
            } else {
                // Dragged hold: aim rubber band from rest (`from`) to tip (`at`).
                let aim = Some(((from.0, from.1), (at.0, at.1)));
                let probe = launch_from_points((from.0, from.1), (at.0, at.1));
                let (path, stats) = integrate(probe, &suns, 180);
                pack(suns, path, stats, aim, "AIM")
            }
        }
        Some(Gesture::Released { before, at }) => {
            // Rubber band: rest is the original down; tip is the release point.
            let rest = newest_down(inputs).unwrap_or((before.0, before.1));
            let tip = (at.0, at.1);
            let pull = (rest.0 - tip.0).hypot(rest.1 - tip.1);
            let probe = if pull > 0.02 {
                launch_from_points(rest, tip)
            } else {
                // Tap-release: default mission velocity.
                Probe {
                    x: tip.0.clamp(0.0, 1.0),
                    y: tip.1.clamp(0.0, 1.0),
                    vx: 0.65,
                    vy: -0.1,
                }
            };
            let steps = ENTRY_STEPS + (phase_unit(t) * (MAX_STEPS - ENTRY_STEPS) as f64) as usize;
            let (path, stats) = integrate(probe, &suns, steps);
            pack(suns, path, stats, None, "LAUNCH")
        }
        Some(Gesture::Cancelled { at }) => {
            // Soft release: become a comet with gentle drift.
            let probe = Probe {
                x: at.0.clamp(0.0, 1.0),
                y: at.1.clamp(0.0, 1.0),
                vx: 0.2,
                vy: 0.05,
            };
            let (path, stats) = integrate(probe, &suns, ENTRY_STEPS);
            pack(suns, path, stats, None, "COMET")
        }
        None => {
            // Bare pokes: launch from newest point toward the main sun.
            let pokes = pokes_from_inputs(inputs);
            if let Some(&(x, y)) = pokes
                .iter()
                .rev()
                .find(|(a, b)| a.is_finite() && b.is_finite())
            {
                let target = suns.first().map(|s| (s.x, s.y)).unwrap_or((0.5, 0.5));
                // Pull from beyond the hand opposite the sun, release at hand.
                let pull = (x * 2.0 - target.0, y * 2.0 - target.1);
                let probe = launch_from_points(pull, (x, y));
                let steps =
                    ENTRY_STEPS + (phase_unit(t) * (MAX_STEPS - ENTRY_STEPS) as f64) as usize;
                let (path, stats) = integrate(probe, &suns, steps);
                pack(suns, path, stats, None, "LAUNCH")
            } else {
                let (probe, steps) = ambient_probe(t, seed);
                let (path, stats) = integrate(probe, &suns, steps);
                pack(suns, path, stats, None, "DEMO")
            }
        }
    }
}

/// The Slingshot room.
#[derive(Debug, Default)]
pub struct Slingshot {
    seed: u64,
}

impl Slingshot {
    /// Create the room with default seed (0).
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation seed for replayable per-visit novelty.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }
}

impl Room for Slingshot {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "slingshot",
            title: "Slingshot",
            wing: "Motion & Dynamics",
            blurb: "Pull and release to launch a probe past suns. Gravity assists are discovered, \
                    not taught; missed shots become comets, never failures. t advances the mission \
                    clock; HOLD grows a sun under the hand.",
            accent: [240, 180, 60],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let m = mission(t, self.seed, &[]);
        draw_scene(canvas, &m.suns, &m.path, m.aim);
    }

    fn postcard_t(&self) -> f64 {
        0.62
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "gravity assist",
            root: 185.0,
            tempo: 116,
            line: &[0, 5, 7, 12, 7, 5, 9, 0],
            encodes: "a slow approach that leaves faster after the sun's whip",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("PULL AND RELEASE: LAUNCH")
    }

    fn status(&self, t: f64) -> Option<String> {
        let m = mission(t, self.seed, &[]);
        Some(format!(
            "SUNS {}  V{:.2}  PULL:LAUNCH",
            m.suns.len(),
            m.stats.peak_speed
        ))
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        if inputs.is_empty() {
            return self.status(t);
        }
        let m = mission(t, self.seed, inputs);
        let fate = if m.stats.assists > 0 {
            "ASSIST"
        } else if m.stats.escaped {
            "COMET"
        } else {
            "BOUND"
        };
        Some(format!(
            "{} V{:.2}  A{}  S{}  {fate}",
            m.label,
            m.stats.peak_speed,
            m.stats.assists,
            m.suns.len()
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        // Bridge: synthesize downs from pokes for faces without full gestures.
        let inputs: Vec<RoomInput> = pokes
            .iter()
            .copied()
            .filter(|(x, y)| x.is_finite() && y.is_finite())
            .enumerate()
            .map(|(i, (x, y))| RoomInput::PointerDown {
                x,
                y,
                t: i as f64 * 0.01,
            })
            .collect();
        self.render_input(canvas, t, &inputs);
    }

    fn render_input(&self, canvas: &mut dyn Surface, t: f64, inputs: &[RoomInput]) {
        let m = mission(t, self.seed, inputs);
        draw_scene(canvas, &m.suns, &m.path, m.aim);
    }

    fn reveal(&self) -> &'static str {
        "A probe that dips past a moving sun can leave faster than it arrived: \
         the sun loses a whisper of orbital energy, the craft gains a real delta-v. \
         Cassini, Voyager, and New Horizons rode that bargain. Here the suns are \
         fixed on the plate, so the assist is the geometry of a deep approach and \
         the whip of the potential; discover it by missing on purpose."
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Body, Probe, SUN_MASS, Slingshot, accel, ambient_probe, course_suns, integrate,
        launch_from_points,
    };
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn gravity_pulls_toward_the_sun() {
        let suns = [Body {
            x: 0.5,
            y: 0.5,
            mass: SUN_MASS,
        }];
        let p = Probe {
            x: 0.2,
            y: 0.5,
            vx: 0.0,
            vy: 0.0,
        };
        let (ax, ay) = accel(p, &suns);
        assert!(ax > 0.0, "should pull right toward the sun");
        assert!(ay.abs() < 1e-9);
    }

    #[test]
    fn pull_back_launches_forward() {
        let p = launch_from_points((0.1, 0.5), (0.3, 0.5));
        // Pulled right to 0.3 from 0.1 means from-to is negative x: launches left.
        // Pull tip at 0.1, release at 0.3: velocity = (0.1-0.3)*gain < 0 (left).
        // For a slingshot feel from left edge: pull left (to 0.1), release at 0.2.
        let p2 = launch_from_points((0.05, 0.5), (0.2, 0.5));
        assert!(p2.vx < 0.0 || p.vx < 0.0);
        // Classic rubber band: grab at 0.2, pull to 0.05, release: from=0.05,to=0.2 -> vx negative.
        // We want pull left launch right: from should be the grab origin...
        // Re-read launch_from_points: velocity = (from - to) * gain.
        // If user pulls from rest R to tip T, release at T: velocity should be R-T
        // (spring pulls back to R). So from=R (before/start), to=T (at).
        // Pull left: R=0.3, T=0.1 -> vx = (0.3-0.1)*g > 0. Good.
        let classic = launch_from_points((0.3, 0.5), (0.1, 0.5));
        assert!(classic.vx > 0.0, "pull left must launch right");
    }

    #[test]
    fn integration_moves_the_probe() {
        let suns = course_suns(0);
        let (probe, steps) = ambient_probe(0.5, 0);
        let (path, stats) = integrate(probe, &suns, steps);
        assert!(path.len() > 10);
        assert!(stats.peak_speed > 0.0);
        assert!(stats.min_sun_dist.is_finite());
    }

    #[test]
    fn variation_changes_course_suns() {
        let a = course_suns(0);
        let b = course_suns(9);
        assert!(a.len() >= 2);
        assert!(!b.is_empty());
        let same = a.len() == b.len()
            && a.iter()
                .zip(b.iter())
                .all(|(u, v)| (u.x - v.x).abs() < 1e-9 && (u.y - v.y).abs() < 1e-9);
        assert!(!same, "variation must remix the assist course");
    }

    #[test]
    fn first_contact_status_invites_launch() {
        let room = Slingshot::new();
        let open = room.status(0.0).expect("open");
        assert!(open.contains("PULL") || open.contains("LAUNCH"), "{open}");
        assert!(open.chars().count() <= 56, "{open}");
    }

    #[test]
    fn launch_changes_status() {
        let room = Slingshot::new();
        let open = room.status(0.0).expect("open");
        let inputs = [
            RoomInput::PointerDown {
                x: 0.35,
                y: 0.55,
                t: 0.0,
            },
            RoomInput::PointerMove {
                x: 0.12,
                y: 0.55,
                t: 0.05,
            },
            RoomInput::PointerUp {
                x: 0.12,
                y: 0.55,
                t: 0.1,
            },
        ];
        let after = room.status_input(0.5, &inputs).expect("launch");
        assert_ne!(after, open);
        assert!(after.contains("LAUNCH") || after.contains("V"), "{after}");
        assert!(after.chars().any(|c| c.is_ascii_digit()), "{after}");
        assert!(after.chars().count() <= 56, "{after}");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = Slingshot::new();
        let mut a = Canvas::new(56, 36);
        let mut b = Canvas::new(56, 36);
        room.render(&mut a, 0.55);
        room.render(&mut b, 0.55);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 20);
        let mut postcard = Canvas::new(60, 40);
        room.render(&mut postcard, room.postcard_t());
        assert!(postcard.ink_count() > 20);
    }

    #[test]
    fn gesture_launch_changes_the_trail() {
        let room = Slingshot::new();
        let mut demo = Canvas::new(48, 32);
        let mut launched = Canvas::new(48, 32);
        room.render(&mut demo, 0.4);
        let inputs = [
            RoomInput::PointerDown {
                x: 0.4,
                y: 0.6,
                t: 0.0,
            },
            RoomInput::PointerUp {
                x: 0.15,
                y: 0.45,
                t: 0.1,
            },
        ];
        room.render_input(&mut launched, 0.4, &inputs);
        assert_ne!(demo.to_text(), launched.to_text());
    }

    #[test]
    fn variation_changes_ambient_render() {
        let mut a = Canvas::new(48, 32);
        let mut b = Canvas::new(48, 32);
        Slingshot::new_with(0).render(&mut a, 0.5);
        Slingshot::new_with(9).render(&mut b, 0.5);
        assert_ne!(a.to_text(), b.to_text());
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Slingshot::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0, f64::NAN, f64::INFINITY] {
            room.render(&mut canvas, t);
            room.render_poked(&mut canvas, t, &[(f64::INFINITY, f64::NAN)]);
            room.render_input(
                &mut canvas,
                t,
                &[RoomInput::PointerDown {
                    x: 0.5,
                    y: 0.5,
                    t: 0.0,
                }],
            );
        }
    }

    #[test]
    fn reveal_names_assist_or_probe() {
        let text = Slingshot::new().reveal().to_ascii_lowercase();
        assert!(text.contains("assist") || text.contains("probe") || text.contains("gravity"));
    }

    #[test]
    fn motif_is_playable() {
        let motif = Slingshot::new().motif().expect("motif");
        assert!(motif.line.len() >= 6);
        assert!(motif.pattern().seconds() > 0.0);
    }
}

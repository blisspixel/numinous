//! The Lorenz attractor: the butterfly that made "chaos" a science.
//!
//! Three simple equations for a toy weather model, and the trajectory never
//! settles and never repeats, yet never leaves a butterfly-shaped set. Two starts
//! a millionth apart diverge completely: the butterfly effect. `t` raises the
//! parameter through the onset of chaos. See `docs/ROOMS.md`.

use crate::rng::SplitMix64;
use crate::room::{MAX_ROOM_POKES, Room, RoomMeta};
use crate::surface::Surface;

/// Prandtl number.
const SIGMA: f64 = 10.0;
/// Geometric factor.
const BETA: f64 = 8.0 / 3.0;
/// Integration time step.
const DT: f64 = 0.005;
/// Total integration steps.
const STEPS: usize = 9_000;
/// The classic chaotic parameter used by the divergence instrument.
const CLASSIC_RHO: f64 = 28.0;
/// Distance between the two initial conditions measured by the instrument.
const TWIN_OFFSET: f64 = 1e-4;
/// Draw a subset of the integrated twin states to keep the overlay light.
const TWIN_DRAW_STRIDE: usize = 4;
/// Steps to discard so the path is on the attractor before drawing.
const TRANSIENT: usize = 800;
/// Shadow trajectories are user-seeded, so keep each one short enough for many pokes.
const SHADOW_STEPS: usize = 2_400;
/// Classic x-z projection bounds.
const X_MIN: f64 = -25.0;
const X_MAX: f64 = 25.0;
const Z_MIN: f64 = 0.0;
const Z_MAX: f64 = 55.0;

/// The Lorenz attractor room.
#[derive(Debug, Default)]
pub struct Lorenz {
    seed: u64,
}

impl Lorenz {
    /// Create the room.
    #[must_use]
    pub fn new() -> Self {
        Self { seed: 0 }
    }

    /// Create with variation.
    #[must_use]
    pub fn new_with(seed: u64) -> Self {
        Self { seed }
    }

    fn phase_for(t: f64) -> f64 {
        if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// The Rayleigh parameter at phase `t`, sweeping through the onset of chaos.
    fn rho_for(t: f64) -> f64 {
        24.0 + 6.0 * Self::phase_for(t)
    }
}

/// The Lorenz path from the default start.
fn trajectory(rho: f64, seed: u64) -> Vec<(f64, f64, f64)> {
    let start = varied_start(seed);
    integrate(start.0, start.1, start.2, rho)
}

fn varied_start(seed: u64) -> (f64, f64, f64) {
    if seed == 0 {
        return (0.1, 0.0, 0.0);
    }
    let mut rng = SplitMix64::new(seed ^ 0x10A3_EA57_5EED_1020);
    let x = 0.1 + (rng.next_f64() - 0.5) * 0.2;
    let y = (rng.next_f64() - 0.5) * 0.2;
    let z = (rng.next_f64() - 0.5) * 0.2;
    (x, y, z)
}

/// Integrate the Lorenz system from a start and return the `(x, y, z)` path.
fn integrate(x: f64, y: f64, z: f64, rho: f64) -> Vec<(f64, f64, f64)> {
    integrate_for((x, y, z), rho, STEPS)
}

/// Integrate the Lorenz system from `start` for a bounded number of steps.
fn integrate_for(mut state: (f64, f64, f64), rho: f64, steps: usize) -> Vec<(f64, f64, f64)> {
    let steps = steps.min(STEPS);
    let mut points = Vec::with_capacity(steps);
    for _ in 0..steps {
        state = lorenz_step(state, rho);
        points.push(state);
    }
    points
}

fn lorenz_step((x, y, z): (f64, f64, f64), rho: f64) -> (f64, f64, f64) {
    let dx = SIGMA * (y - x);
    let dy = x * (rho - z) - y;
    let dz = x * y - BETA * z;
    (x + dx * DT, y + dy * DT, z + dz * DT)
}

fn state_distance(a: (f64, f64, f64), b: (f64, f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2) + (a.2 - b.2).powi(2)).sqrt()
}

/// Number of classic twin integration steps visible at phase `t`.
fn twin_steps(t: f64) -> usize {
    (Lorenz::phase_for(t) * STEPS as f64).floor() as usize
}

/// Advance two nearby classic Lorenz runs and return their peak separation.
///
/// Instantaneous separation can shrink when the attractor folds. A running
/// maximum preserves that real motion while giving the player an honest,
/// monotonic record of how much predictability has already been lost.
fn run_twins(
    t: f64,
    seed: u64,
    mut observe: impl FnMut(usize, (f64, f64, f64), (f64, f64, f64)),
) -> f64 {
    let steps = twin_steps(t);
    let mut main = varied_start(seed);
    let mut shadow = (main.0 + TWIN_OFFSET, main.1, main.2);
    let mut peak = state_distance(main, shadow);
    observe(0, main, shadow);

    for step in 1..=steps {
        main = lorenz_step(main, CLASSIC_RHO);
        shadow = lorenz_step(shadow, CLASSIC_RHO);
        peak = peak.max(state_distance(main, shadow));
        observe(step, main, shadow);
    }
    peak
}

fn divergence_peak(t: f64, seed: u64) -> f64 {
    run_twins(t, seed, |_, _, _| {})
}

/// Project a Lorenz `(x, z)` point into the room's classic butterfly view.
fn project(width: usize, height: usize, x: f64, z: f64) -> (i32, i32) {
    let sx = (x - X_MIN) / (X_MAX - X_MIN) * (width as f64 - 1.0);
    let sy = (height as f64 - 1.0) - ((z - Z_MIN) / (Z_MAX - Z_MIN)) * (height as f64 - 1.0);
    (sx as i32, sy as i32)
}

/// Convert a normalized click into an actual Lorenz state in the same projected plane.
fn shadow_start(point: (f64, f64), which: usize) -> Option<(f64, f64, f64)> {
    let (px, py) = point;
    if !px.is_finite() || !py.is_finite() {
        return None;
    }
    let x = X_MIN + px.clamp(0.0, 1.0) * (X_MAX - X_MIN);
    let z = Z_MAX - py.clamp(0.0, 1.0) * (Z_MAX - Z_MIN);
    let y = x + 0.02 * ((which % 7) as f64 - 3.0);
    Some((x, y, z))
}

fn bounded_shadow_starts(pokes: &[(f64, f64)]) -> Vec<(f64, f64, f64)> {
    let mut points: Vec<_> = pokes
        .iter()
        .rev()
        .copied()
        .take(MAX_ROOM_POKES)
        .filter(|&(x, y)| x.is_finite() && y.is_finite())
        .collect();
    points.reverse();
    points
        .into_iter()
        .enumerate()
        .filter_map(|(which, point)| shadow_start(point, which))
        .collect()
}

impl Room for Lorenz {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "lorenz",
            title: "Lorenz Attractor",
            wing: "Chaos & Order",
            blurb: "Three equations for toy weather. The path never repeats and never escapes its \
                    butterfly-shaped set. t raises the parameter through the onset of chaos.",
            accent: [80, 180, 230],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        let points = trajectory(Self::rho_for(t), self.seed);
        let mut previous: Option<(i32, i32)> = None;
        for &(x, _, z) in points.iter().skip(TRANSIENT) {
            let (sx, sy) = project(width, height, x, z);
            if let Some((px, py)) = previous {
                canvas.line(px, py, sx, sy, '#');
            }
            previous = Some((sx, sy));
        }

        // The full attractor remains the stage while the two classic-rho
        // forecasts traced by the status instrument grow across it. Their
        // instantaneous separation may shrink when the attractor folds; the
        // status reports the largest separation this visible run has reached.
        let steps = twin_steps(t);
        let mut main_previous: Option<(i32, i32)> = None;
        let mut shadow_previous: Option<(i32, i32)> = None;
        let mut main_endpoint = (0, 0);
        let mut shadow_endpoint = (0, 0);
        run_twins(t, self.seed, |step, main, shadow| {
            if step % TWIN_DRAW_STRIDE != 0 && step != steps {
                return;
            }
            let main_point = project(width, height, main.0, main.2);
            let shadow_point = project(width, height, shadow.0, shadow.2);
            if let Some(previous) = main_previous {
                canvas.line(previous.0, previous.1, main_point.0, main_point.1, '+');
            }
            if let Some(previous) = shadow_previous {
                canvas.line(previous.0, previous.1, shadow_point.0, shadow_point.1, '-');
            }
            main_previous = Some(main_point);
            shadow_previous = Some(shadow_point);
            main_endpoint = main_point;
            shadow_endpoint = shadow_point;
        });
        canvas.plot(main_endpoint.0, main_endpoint.1, '*');
        canvas.plot(shadow_endpoint.0, shadow_endpoint.1, '*');
    }

    fn status(&self, t: f64) -> Option<String> {
        let peak = divergence_peak(t, self.seed);
        Some(format!("STORM PEAK {peak:.4} AT RHO {CLASSIC_RHO:.0}"))
    }

    fn reveal(&self) -> &'static str {
        "Lorenz found this by rounding 0.506127 to 0.506 in a weather run and \
         watching the forecast diverge completely. That is the butterfly effect: \
         perfectly determined, and still impossible to predict."
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "A minor, unresolved",
            root: 110.0,
            tempo: 84,
            line: &[0, 3, 7, 10, 8, 3, 2, 10, 7, 1],
            encodes: "a line that wanders forever and never lands: the attractor",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: SEED A SHADOW STORM")
    }

    fn status_input(&self, t: f64, inputs: &[crate::room::RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let storms = bounded_shadow_starts(&pokes).len();
        if storms == 0 {
            return self.status(t);
        }
        let peak = divergence_peak(t, self.seed);
        Some(format!(
            "{storms} SHADOW STORM{}   MAIN PEAK {peak:.4}   RHO {CLASSIC_RHO:.0}",
            if storms == 1 { "" } else { "S" }
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let width = canvas.width();
        let height = canvas.height();
        if width == 0 || height == 0 {
            return;
        }
        self.render(canvas, t);
        // Each poke becomes a real Lorenz initial condition in the same x-z
        // plane the player clicked, then the system's sensitive dependence
        // pulls that shadow storm away from the seed.
        let rho = Self::rho_for(t);
        for start in bounded_shadow_starts(pokes) {
            let (seed_x, _, seed_z) = start;
            let (sx, sy) = project(width, height, seed_x, seed_z);

            let mut previous = Some((sx, sy));
            for &(x, _, z) in integrate_for(start, rho, SHADOW_STEPS).iter().step_by(2) {
                let (px, py) = project(width, height, x, z);
                if let Some((last_x, last_y)) = previous {
                    canvas.line(last_x, last_y, px, py, '-');
                }
                previous = Some((px, py));
            }
            canvas.plot(sx, sy, '*');
        }
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "The name butterfly effect comes from the title of Lorenz's 1972 talk: \
             does the flap of a butterfly's wings in Brazil set off a tornado in \
             Texas? A colleague picked the title for him. The butterfly is the most \
             famous thing a session chair ever wrote.",
            "The attractor's fractal dimension is about 2.06: more than a surface, \
             less than a volume. The trajectory needs just a hair more than two \
             dimensions to never cross itself, and that hair is where chaos lives.",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Lorenz, TWIN_OFFSET, X_MAX, X_MIN, Z_MAX, Z_MIN, bounded_shadow_starts, integrate, project,
        run_twins, shadow_start, trajectory,
    };
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};
    use crate::surface::Surface;

    #[test]
    fn action_status_names_seeded_shadow_storms() {
        let room = Lorenz::new();
        let open = room.status(0.5).expect("open");
        assert!(open.contains("STORM PEAK"), "{open}");
        let inputs = [RoomInput::PointerDown {
            x: 0.4,
            y: 0.6,
            t: 0.0,
        }];
        let status = room.status_input(0.5, &inputs).expect("storm");
        assert!(status.contains("1 SHADOW STORM"), "{status}");
        assert_ne!(status, open);
    }

    #[derive(Debug)]
    struct ProbeSurface {
        width: usize,
        height: usize,
        marks: Vec<(i32, i32, char)>,
    }

    impl ProbeSurface {
        fn new(width: usize, height: usize) -> Self {
            Self {
                width,
                height,
                marks: Vec::new(),
            }
        }

        fn marked(&self, x: i32, y: i32, mark: char) -> bool {
            self.marks
                .iter()
                .any(|&(mx, my, m)| mx == x && my == y && m == mark)
        }

        fn has_mark(&self, mark: char) -> bool {
            self.marks.iter().any(|&(_, _, m)| m == mark)
        }
    }

    impl Surface for ProbeSurface {
        fn width(&self) -> usize {
            self.width
        }

        fn height(&self) -> usize {
            self.height
        }

        fn plot(&mut self, x: i32, y: i32, mark: char) {
            if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
                self.marks.push((x, y, mark));
            }
        }
    }

    #[test]
    fn the_path_stays_on_the_attractor() {
        // After the transient the trajectory is bounded inside a known box.
        for &(x, y, z) in trajectory(28.0, 0).iter().skip(800) {
            assert!(x.abs() < 40.0 && y.abs() < 60.0, "escaped: {x}, {y}");
            assert!((-5.0..80.0).contains(&z), "z escaped: {z}");
        }
    }

    #[test]
    fn tiny_start_differences_diverge() {
        // The butterfly effect: two starts a ten-thousandth apart, same system.
        let a = integrate(0.1, 0.0, 0.0, 28.0);
        let b = integrate(0.1001, 0.0, 0.0, 28.0);
        let (ax, _, az) = *a.last().unwrap();
        let (bx, _, bz) = *b.last().unwrap();
        assert!((ax - bx).abs() + (az - bz).abs() > 1.0, "did not diverge");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = Lorenz::new();
        let mut first = Canvas::new(60, 30);
        let mut second = Canvas::new(60, 30);
        room.render(&mut first, 0.7);
        room.render(&mut second, 0.7);
        assert_eq!(first.to_text(), second.to_text());
        assert!(first.ink_count() > 30);
    }

    #[test]
    fn the_divergence_readout_is_born_from_the_perturbation_and_never_falls() {
        for seed in [0, 1, 42, 1_000] {
            let room = Lorenz::new_with(seed);
            let gap = |t: f64| crate::challenge::status_numbers(&room.status(t).unwrap())[0].1;
            let initial = gap(0.0);

            assert!(
                (initial - TWIN_OFFSET).abs() < 1e-12,
                "seed {seed} should begin at the exact perturbation, not after divergence: {initial}"
            );
            let mut previous = initial;
            for sample in 1..=64 {
                let current = gap(sample as f64 / 64.0);
                assert!(
                    current.is_finite() && current >= previous,
                    "seed {seed} fell at sample {sample}: {previous} to {current}"
                );
                previous = current;
            }
            assert!(
                previous > 10.0,
                "seed {seed} should eventually separate across the attractor: {previous}"
            );
        }
        let room = Lorenz::new();
        assert!(
            room.status(0.5).unwrap().contains("PEAK"),
            "the readout must disclose that it reports a running envelope"
        );
        assert_eq!(room.status(f64::NAN), room.status(0.0));
        assert_eq!(room.status(f64::INFINITY), room.status(0.0));
        assert_eq!(room.status(-1.0), room.status(0.0));
        assert_eq!(room.status(9.0), room.status(1.0));
        assert!(
            room.status(0.5).unwrap().len() <= 40,
            "the live instrument must stay one short line"
        );
        // A moving readout means Lorenz now poses predictions on the peak,
        // not on one of the constant explanatory numbers that follow it.
        let prediction = crate::pose_prediction(&room, 5).expect("the storm peak moves");
        assert_eq!(prediction.index, 0);
        assert_eq!(prediction.label, "STORM PEAK");
        assert!((prediction.span.0 - TWIN_OFFSET).abs() < 1e-12);
    }

    #[test]
    fn the_render_draws_both_trajectories_measured_by_the_instrument() {
        let mut surface = ProbeSurface::new(80, 40);
        let room = Lorenz::new();
        let phase = 0.6;
        let mut endpoint = None;
        let peak = run_twins(phase, 0, |_, main, shadow| {
            endpoint = Some((main, shadow));
        });

        room.render(&mut surface, phase);
        assert!(surface.has_mark('+'), "the main forecast is visible");
        assert!(surface.has_mark('-'), "the nearby forecast is visible");
        let (main, shadow) = endpoint.expect("the twin run always has its initial state");
        let main_point = project(surface.width, surface.height, main.0, main.2);
        let shadow_point = project(surface.width, surface.height, shadow.0, shadow.2);
        assert!(surface.marked(main_point.0, main_point.1, '*'));
        assert!(surface.marked(shadow_point.0, shadow_point.1, '*'));
        let status_peak = crate::challenge::status_numbers(&room.status(phase).unwrap())[0].1;
        assert!(
            (status_peak - peak).abs() <= 0.000_05,
            "the visible run and its status must agree: {status_peak} vs {peak}"
        );
    }

    #[test]
    fn new_with_zero_matches_default_and_nonzero_differs() {
        let r0 = Lorenz::new_with(0);
        let r_def = Lorenz::new();
        let mut a = Canvas::new(60, 30);
        let mut b = Canvas::new(60, 30);
        r0.render(&mut a, 0.7);
        r_def.render(&mut b, 0.7);
        assert_eq!(a.to_text(), b.to_text());

        let r42 = Lorenz::new_with(42);
        let r1000 = Lorenz::new_with(1000);
        let mut c = Canvas::new(60, 30);
        let mut d = Canvas::new(60, 30);
        r42.render(&mut c, 0.7);
        r1000.render(&mut d, 0.7);
        assert_ne!(a.to_text(), c.to_text());
        assert_ne!(a.to_text(), d.to_text());
    }

    #[test]
    fn shadow_start_uses_the_clicked_projection() {
        let center = shadow_start((0.25, 0.75), 0).expect("finite click");
        let moved_x = shadow_start((0.75, 0.75), 0).expect("finite click");
        let moved_y = shadow_start((0.25, 0.25), 0).expect("finite click");

        assert!((center.0 + 12.5).abs() < 1e-9, "x maps to Lorenz x");
        assert!((center.2 - 13.75).abs() < 1e-9, "y maps to Lorenz z");
        assert_ne!(
            center.0, moved_x.0,
            "click x moves the seed across the wing"
        );
        assert_ne!(center.2, moved_y.2, "click y moves the seed vertically");
        assert!(shadow_start((f64::NAN, 0.5), 0).is_none());
        assert!(shadow_start((0.5, f64::INFINITY), 0).is_none());
    }

    #[test]
    fn projection_has_fixed_screen_corners() {
        assert_eq!(project(101, 56, X_MIN, Z_MAX), (0, 0));
        assert_eq!(project(101, 56, X_MAX, Z_MIN), (100, 55));
        assert_eq!(project(101, 56, 0.0, 27.5), (50, 27));
    }

    #[test]
    fn public_render_poked_draws_shadow_from_the_hand_point() {
        let room = Lorenz::new();
        let point = (0.35, 0.65);
        let mut base = Canvas::new(80, 40);
        let mut poked = Canvas::new(80, 40);
        let mut probe = ProbeSurface::new(80, 40);
        room.render(&mut base, 0.7);
        room.render_poked(&mut poked, 0.7, &[point]);
        room.render_poked(&mut probe, 0.7, &[point]);

        assert_ne!(base.to_text(), poked.to_text());
        let (x, _, z) = shadow_start(point, 0).expect("finite click");
        let (sx, sy) = project(poked.width(), poked.height(), x, z);
        assert!(
            probe.marked(sx, sy, '*'),
            "the shadow seed should be drawn at the clicked projection"
        );
    }

    #[test]
    fn oversized_and_nonfinite_shadow_pokes_are_bounded() {
        let room = Lorenz::new();
        let newest = vec![(0.9, 0.1); crate::MAX_ROOM_POKES];
        let mut many = vec![(0.2, 0.8); crate::MAX_ROOM_POKES];
        many.extend(std::iter::repeat_n((0.9, 0.1), 100));

        let mut expected = Canvas::new(80, 40);
        let mut actual = Canvas::new(80, 40);
        room.render_poked(&mut expected, 0.6, &newest);
        room.render_poked(&mut actual, 0.6, &many);
        assert_eq!(expected.to_text(), actual.to_text());
        assert_eq!(bounded_shadow_starts(&many), bounded_shadow_starts(&newest));

        let mut finite = Canvas::new(80, 40);
        let mut mixed = Canvas::new(80, 40);
        room.render_poked(&mut finite, 0.6, &[(0.2, 0.8)]);
        room.render_poked(
            &mut mixed,
            0.6,
            &[(f64::NAN, 0.5), (0.5, f64::INFINITY), (0.2, 0.8)],
        );
        assert_eq!(finite.to_text(), mixed.to_text());

        let mut raw_invalid_then_valid = vec![(0.2, 0.8)];
        raw_invalid_then_valid.extend(std::iter::repeat_n(
            (f64::NAN, 0.5),
            crate::MAX_ROOM_POKES + 100,
        ));
        assert!(
            bounded_shadow_starts(&raw_invalid_then_valid).is_empty(),
            "only the raw newest cap is inspected before invalid points are discarded"
        );
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Lorenz::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        room.render_poked(&mut empty, 0.5, &[(0.5, 0.5)]);
        let mut canvas = Canvas::new(8, 8);
        for t in [f64::NAN, -1.0, 0.0, 1.0, 9.0] {
            room.render(&mut canvas, t);
            room.render_poked(&mut canvas, t, &[(f64::NAN, f64::INFINITY)]);
        }
    }

    #[test]
    fn reveal_mentions_the_butterfly_effect() {
        assert!(Lorenz::new().reveal().contains("butterfly effect"));
    }
}

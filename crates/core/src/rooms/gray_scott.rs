//! Gray-Scott reaction-diffusion on a small periodic lattice.
//!
//! The reaction U + 2V -> 3V converts U into V; V decays into an inert product.
//! Feed restores U and removes both species. Each look replays the same initial
//! patch at the selected rates and finite horizon, without classifying a pattern.
//! Equations: Pearson (1993), <https://arxiv.org/pdf/patt-sol/9304003>, Eq. 2.
//! DRAG: X FEED, Y KILL. See `docs/ROOMS.md`.

use super::phase_plane::PhasePlane;
use super::{latest_hand, phase_unit};
use crate::room::{Room, RoomInput};
use crate::surface::Surface;

const W: usize = 48;
const H: usize = 28;
const CELLS: usize = W * H;
const MAX_STEPS: usize = 120;
const DT: f64 = 1.0;
// Cell spacing is one. This is a finite lattice illustration, not Pearson's
// finer mesh or his long-time pattern survey. The diffusion ratio is two.
const DIFFUSION_U: f64 = 0.16;
const DIFFUSION_V: f64 = 0.08;
const VISIBLE_V: f64 = 0.08;
type Field = [f64; 2 * CELLS];

#[derive(Debug, Clone, Copy, PartialEq)]
struct Experiment {
    feed: f64,
    kill: f64,
    seed: u64,
    steps: usize,
}

impl Experiment {
    fn new(t: f64, hand: Option<(f64, f64)>, seed: u64) -> Self {
        let (feed, kill) = hand.map_or((0.04, 0.06), |(x, y)| (0.01 + x * 0.08, 0.04 + y * 0.04));
        Self {
            feed,
            kill,
            seed,
            steps: (phase_unit(t) * MAX_STEPS as f64) as usize,
        }
    }

    fn run(self) -> Field {
        let mut field = seed_field(self.seed);
        for _ in 0..self.steps {
            field = step(field, self.feed, self.kill, DT);
        }
        field
    }

    fn readout(self) -> String {
        let field = self.run();
        let max_v = field[CELLS..].iter().copied().fold(0.0_f64, f64::max);
        format!(
            "DRAG:F/K f~{:.3} k~{:.3} T={} Vmax~{max_v:.3}",
            self.feed,
            self.kill,
            self.steps as f64 * DT,
        )
    }
}

fn idx(x: usize, y: usize) -> usize {
    y * W + x
}

fn seed_field(seed: u64) -> Field {
    let mut field = [0.0; 2 * CELLS];
    field[..CELLS].fill(1.0);
    let cx = W / 2;
    let cy = H / 2;
    let r = 4 + (seed % 3) as usize;
    for y in cy.saturating_sub(r)..=(cy + r).min(H - 1) {
        for x in cx.saturating_sub(r)..=(cx + r).min(W - 1) {
            if (x as i32 - cx as i32).pow(2) + (y as i32 - cy as i32).pow(2) <= (r * r) as i32 {
                field[idx(x, y)] = 0.5;
                field[CELLS + idx(x, y)] = 0.25;
            }
        }
    }
    field
}

fn lap(f: &[f64], x: usize, y: usize) -> f64 {
    let xm = if x == 0 { W - 1 } else { x - 1 };
    let xp = if x + 1 == W { 0 } else { x + 1 };
    let ym = if y == 0 { H - 1 } else { y - 1 };
    let yp = if y + 1 == H { 0 } else { y + 1 };
    f[idx(xm, y)] + f[idx(xp, y)] + f[idx(x, ym)] + f[idx(x, yp)] - 4.0 * f[idx(x, y)]
}

fn derivative(field: &Field, feed: f64, kill: f64) -> Field {
    let (u, v) = field.split_at(CELLS);
    let mut change = [0.0; 2 * CELLS];
    for y in 0..H {
        for x in 0..W {
            let i = idx(x, y);
            let uu = u[i];
            let vv = v[i];
            let uvv = uu * vv * vv;
            change[i] = DIFFUSION_U * lap(u, x, y) - uvv + feed * (1.0 - uu);
            change[CELLS + i] = DIFFUSION_V * lap(v, x, y) + uvv - (feed + kill) * vv;
        }
    }
    change
}

fn step(field: Field, feed: f64, kill: f64, dt: f64) -> Field {
    // The old unit Euler step noticeably shifted finite-time concentrations.
    // RK4 is checked by exact diffusion modes and step refinement below. No
    // concentration clipping silently substitutes a different reaction law.
    crate::numerics::rk4(field, dt, |field| derivative(&field, feed, kill))
}

/// Average the piecewise constant field over a display footprint, restricted
/// to the lattice rectangle. Outside that rectangle is the display margin,
/// not another chemical concentration. Partial boundary pixels therefore use
/// only their covered area. The loops visit only intersecting source cells.
fn footprint_average(v: &[f64], lower: (f64, f64), upper: (f64, f64)) -> Option<f64> {
    if ![lower.0, lower.1, upper.0, upper.1]
        .into_iter()
        .all(f64::is_finite)
    {
        return None;
    }
    let x0 = lower.0.max(0.0);
    let y0 = lower.1.max(0.0);
    let x1 = upper.0.min(W as f64);
    let y1 = upper.1.min(H as f64);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let first_x = x0.floor() as usize;
    let first_y = y0.floor() as usize;
    let baseline = v[idx(first_x, first_y)];
    // Averaging differences preserves a constant field exactly, including
    // values lying on a glyph threshold, instead of rounding it into bands.
    let mut difference = 0.0;
    for gy in first_y..y1.ceil() as usize {
        let dy = y1.min((gy + 1) as f64) - y0.max(gy as f64);
        for gx in first_x..x1.ceil() as usize {
            let dx = x1.min((gx + 1) as f64) - x0.max(gx as f64);
            difference += dx * dy * (v[idx(gx, gy)] - baseline);
        }
    }
    Some(baseline + difference / ((x1 - x0) * (y1 - y0)))
}

fn concentration_mark(v: f64) -> Option<char> {
    if !v.is_finite() || v < VISIBLE_V {
        None
    } else if v > 0.4 {
        Some('#')
    } else if v > 0.25 {
        Some('*')
    } else if v > 0.15 {
        Some('+')
    } else {
        Some('.')
    }
}

fn draw(canvas: &mut dyn Surface, v: &[f64]) {
    // Fit cell edges, so the physical domain remains 48 by 28 unit squares on
    // both pixels and tall terminal characters, with room for App chrome.
    let Some(plane) = PhasePlane::fit(canvas, [(0.0, 0.0), (W as f64, H as f64)].into_iter())
    else {
        return;
    };
    let (width, height) = canvas.draw_bounds();
    for y in 0..height {
        for x in 0..width {
            let Some((left, top)) = plane.world(x as f64 - 0.5, y as f64 - 0.5) else {
                return;
            };
            let Some((right, bottom)) = plane.world(x as f64 + 0.5, y as f64 + 0.5) else {
                return;
            };
            if let Some(mark) =
                footprint_average(v, (left, bottom), (right, top)).and_then(concentration_mark)
            {
                // Raster adds ink. Painting once prevents downsampling from
                // turning overlapping source cells into false bright peaks.
                canvas.plot(x as i32, y as i32, mark);
            }
        }
    }
}

/// Gray-Scott room.
#[derive(Debug, Default)]
pub struct GrayScott {
    seed: u64,
}

impl GrayScott {
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

impl Room for GrayScott {
    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let field = Experiment::new(t, None, self.seed).run();
        draw(canvas, &field[CELLS..]);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "gray scott",
            root: 138.59,
            tempo: 76,
            line: &[0, 5, 7, 10, 12, 10, 7, 5],
            encodes: "a reaction-diffusion motif; concentrations are shown by the field",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: X FEED, Y KILL")
    }

    fn status(&self, t: f64) -> Option<String> {
        Some(Experiment::new(t, None, self.seed).readout())
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let field = Experiment::new(t, latest_hand(pokes), self.seed).run();
        draw(canvas, &field[CELLS..]);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        Some(Experiment::new(t, latest_hand(&pokes), self.seed).readout())
    }

    fn reveal(&self) -> &'static str {
        "In Gray-Scott, U + 2V becomes 3V, and V decays. Feed restores U and \
         removes both species; diffusion spreads them across a periodic lattice. \
         Drag horizontally for feed, vertically for kill. Each look replays the \
         same initial patch at those rates; variation changes its radius. Phase \
         selects elapsed time T from 0 to 120. Vmax measures the largest lattice \
         concentration before display averaging. Display marks use V averaged \
         over each pixel or character cell; averages below 0.08 are hidden. \
         Square lattice geometry is preserved \
         within the display margins. These finite snapshots \
         do not establish a pattern class or a Turing instability."
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CELLS, DIFFUSION_U, DT, Experiment, Field, GrayScott, H, MAX_STEPS, VISIBLE_V, W,
        concentration_mark, derivative, draw, footprint_average, idx, lap, latest_hand, seed_field,
        step,
    };
    use crate::canvas::Canvas;
    use crate::raster::Raster;
    use crate::room::{MAX_ROOM_POKES, Room, RoomInput, inputs_from_pokes};
    use crate::surface::Surface;

    fn uniform(u: f64, v: f64) -> Field {
        let mut field = [v; 2 * CELLS];
        field[..CELLS].fill(u);
        field
    }

    fn integrate(mut field: Field, feed: f64, kill: f64, time: f64, dt: f64) -> Field {
        for _ in 0..(time / dt).round() as usize {
            field = step(field, feed, kill, dt);
            assert!(field.iter().all(|value| value.is_finite() && *value >= 0.0));
            assert!(field[..CELLS].iter().all(|value| *value <= 1.0 + 1e-13));
        }
        field
    }

    fn field_error(left: &Field, right: &Field) -> f64 {
        left.iter()
            .zip(right)
            .map(|(left, right)| (left - right).abs())
            .fold(0.0, f64::max)
    }

    #[test]
    fn equivalent_tunings_keep_the_rates_seed_and_horizon() {
        let room = GrayScott::new_with(2);
        let point = (0.375, 0.5);
        let once = [point];
        let repeated = [point; MAX_ROOM_POKES + 3];
        let changed_then_returned = [(0.9, 0.1), (0.0, 1.0), point];
        let expected = Experiment::new(2.0 / 3.0, Some(point), 2);
        assert_eq!(expected.seed, 2);
        assert_eq!(expected.steps, 80);
        for history in [&once[..], &repeated[..], &changed_then_returned[..]] {
            let experiment = Experiment::new(2.0 / 3.0, latest_hand(history), 2);
            assert_eq!(experiment, expected);
            let mut direct = Canvas::new(W, H);
            room.render_poked(&mut direct, 2.0 / 3.0, history);
            let mut untouched = Canvas::new(W, H);
            room.render(&mut untouched, 2.0 / 3.0);
            assert_eq!(direct.to_text(), untouched.to_text());
            assert_eq!(
                room.status_input(2.0 / 3.0, &inputs_from_pokes(history, 0.0)),
                room.status(2.0 / 3.0),
            );
        }
    }

    #[test]
    fn input_admission_and_elapsed_time_are_shared_by_every_path() {
        let clamped = [(3.0, -2.0), (f64::NAN, 0.5)];
        assert_eq!(latest_hand(&clamped), Some((1.0, 0.0)));
        let selected = Experiment::new(0.5, latest_hand(&clamped), 0);
        assert!((selected.feed - 0.09).abs() < 1e-14);
        assert_eq!(selected.kill, 0.04);
        assert_eq!(selected.steps, 60);

        let mut stale = vec![(1.0, 1.0)];
        stale.extend(vec![(f64::NAN, 0.5); MAX_ROOM_POKES]);
        assert_eq!(latest_hand(&stale), None);
        let room = GrayScott::new();
        assert_eq!(
            room.status_input(0.5, &inputs_from_pokes(&stale, 0.0)),
            room.status(0.5),
        );
        let mut empty = Canvas::new(W, H);
        room.render_poked(&mut empty, 0.5, &[]);
        let mut direct = Canvas::new(W, H);
        room.render(&mut direct, 0.5);
        assert_eq!(empty.to_text(), direct.to_text());

        for phase in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 0.0] {
            let experiment = Experiment::new(phase, None, 1);
            assert_eq!(experiment.steps, 0);
            assert_eq!(experiment.run(), seed_field(1));
        }
        for phase in [1.0, 2.0] {
            assert_eq!(Experiment::new(phase, None, 1).steps, MAX_STEPS);
        }
        assert_eq!(
            Experiment::new(0.0, None, 0).feed,
            Experiment::new(1.0, None, 0).feed
        );
        assert_eq!(
            Experiment::new(0.0, None, 0).kill,
            Experiment::new(1.0, None, 0).kill
        );
        assert_ne!(seed_field(0), seed_field(1));
        assert_eq!(seed_field(0), seed_field(3));
    }

    #[test]
    fn reaction_stoichiometry_and_feed_removal_match_the_equations() {
        let closed = uniform(0.5, 0.25);
        let change = derivative(&closed, 0.0, 0.0);
        for i in 0..CELLS {
            assert_eq!(change[i], -0.03125);
            assert_eq!(change[CELLS + i], 0.03125);
        }
        let next = step(closed, 0.0, 0.0, 0.5);
        assert!(next[0] < 0.5 && next[CELLS] > 0.25);
        assert!((next[0] + next[CELLS] - 0.75).abs() < 1e-14);

        let change = derivative(&uniform(0.8, 0.2), 0.03, 0.05);
        for i in 0..CELLS {
            assert!((change[i] + 0.026).abs() < 1e-15);
            assert!((change[CELLS + i] - 0.016).abs() < 1e-15);
            // U+V changes only by feed*(1-U-V) - kill*V.
            assert!((change[i] + change[CELLS + i] + 0.01).abs() < 1e-15);
        }
    }

    #[test]
    fn uniform_reaction_equilibria_remain_stationary() {
        for (feed, kill) in [(0.01, 0.04), (0.04, 0.06), (0.09, 0.08)] {
            let background = uniform(1.0, 0.0);
            assert_eq!(derivative(&background, feed, kill), [0.0; 2 * CELLS]);
            assert_eq!(integrate(background, feed, kill, 5.0, DT), background);
        }
        // For F=k=0.05, U*(1-U)=0.2 and U*V=0.1 give two nonzero equilibria.
        for sign in [-1.0, 1.0] {
            let u = (1.0 + sign * 0.2_f64.sqrt()) / 2.0;
            let field = uniform(u, 0.1 / u);
            assert!(
                derivative(&field, 0.05, 0.05)
                    .iter()
                    .all(|value| value.abs() < 1e-15)
            );
            assert!(field_error(&integrate(field, 0.05, 0.05, 5.0, DT), &field) < 1e-14);
        }
    }

    #[test]
    fn periodic_diffusion_conserves_mass_and_has_the_expected_fourier_spectrum() {
        let mut impulse = [0.0; CELLS];
        impulse[idx(0, 0)] = 1.0;
        assert_eq!(lap(&impulse, 0, 0), -4.0);
        assert_eq!(lap(&impulse, W - 1, 0), 1.0);
        assert_eq!(lap(&impulse, 0, H - 1), 1.0);
        let mut total = 0.0;
        for y in 0..H {
            for x in 0..W {
                total += lap(&impulse, x, y);
            }
        }
        assert_eq!(total, 0.0);
        for (nx, ny) in [(0, 0), (1, 0), (0, 1), (3, 4), (W / 2, H / 2)] {
            let kx = std::f64::consts::TAU * nx as f64 / W as f64;
            let ky = std::f64::consts::TAU * ny as f64 / H as f64;
            let field: [f64; CELLS] =
                std::array::from_fn(|i| (kx * (i % W) as f64 + ky * (i / W) as f64).cos());
            let eigenvalue = 2.0 * kx.cos() + 2.0 * ky.cos() - 4.0;
            let mut total = 0.0;
            for y in 0..H {
                for x in 0..W {
                    let actual = lap(&field, x, y);
                    assert!((actual - eigenvalue * field[idx(x, y)]).abs() < 3e-14);
                    total += actual;
                }
            }
            assert!(total.abs() < 1e-11, "periodic diffusion cannot create mass");
        }
    }

    #[test]
    fn diffusion_matches_exact_mode_decay_and_fourth_order_time_refinement() {
        let kx = std::f64::consts::TAU * 8.0 / W as f64;
        let ky = std::f64::consts::TAU * 4.0 / H as f64;
        let mode = |i: usize| (kx * (i % W) as f64 + ky * (i / W) as f64).cos();
        let mut field = uniform(0.0, 0.0);
        for (i, u) in field[..CELLS].iter_mut().enumerate() {
            *u = 0.5 + 0.1 * mode(i);
        }
        let eigenvalue = DIFFUSION_U * (2.0 * kx.cos() + 2.0 * ky.cos() - 4.0);
        let mut exact = uniform(0.0, 0.0);
        for (i, u) in exact[..CELLS].iter_mut().enumerate() {
            *u = 0.5 + 0.1 * (5.0 * eigenvalue).exp() * mode(i);
        }
        // V=0 and F=k=0 remove reaction and feed, isolating diffusion through
        // the actual coupled solver rather than a second stencil implementation.
        let coarse = integrate(field, 0.0, 0.0, 5.0, 1.0);
        let fine = integrate(field, 0.0, 0.0, 5.0, 0.5);
        let coarse_error = field_error(&coarse, &exact);
        let fine_error = field_error(&fine, &exact);
        assert!(coarse_error < 3e-6);
        assert!(fine_error < 2e-7);
        assert!((16.0..20.0).contains(&(coarse_error / fine_error)));
        for state in [coarse, fine] {
            assert!((state[..CELLS].iter().sum::<f64>() - CELLS as f64 * 0.5).abs() < 1e-10);
            assert!(state[CELLS..].iter().all(|value| *value == 0.0));
        }
    }

    #[test]
    fn laplacian_has_second_order_spatial_accuracy_for_a_smooth_wave() {
        // The same wave of physical period 12 fits these three periodic domains.
        // Refining its local stencil checks spatial order, not nonlinear pattern
        // convergence for the room's discontinuous seeded patch.
        let k = std::f64::consts::TAU / 12.0;
        let mut errors = Vec::new();
        for spacing in [1.0, 0.5, 0.25] {
            let field: [f64; CELLS] = std::array::from_fn(|i| (k * spacing * (i % W) as f64).cos());
            let approximate = lap(&field, 0, 0) / (spacing * spacing);
            errors.push((approximate + k * k).abs());
        }
        for pair in errors.windows(2) {
            assert!((3.9..4.1).contains(&(pair[0] / pair[1])));
        }
    }

    #[test]
    fn finite_reaction_diffusion_snapshots_meet_the_step_refinement_budget() {
        let fixtures = [
            (0.01, 0.04, 0),
            (0.04, 0.06, 0),
            (0.09, 0.05, 2),
            (0.09, 0.08, 0),
        ];
        for (feed, kill, seed) in fixtures {
            let initial = seed_field(seed);
            let coarse = integrate(initial, feed, kill, 120.0, 1.0);
            let fine = integrate(initial, feed, kill, 120.0, 0.5);
            let reference = integrate(initial, feed, kill, 120.0, 0.25);
            let coarse_error = field_error(&coarse, &reference);
            let fine_error = field_error(&fine, &reference);
            assert!(
                coarse_error < 2e-4,
                "f={feed}, k={kill}, seed={seed}: error={coarse_error}"
            );
            assert!(
                fine_error < 1e-5,
                "f={feed}, k={kill}, seed={seed}: refined error={fine_error}"
            );
            if coarse_error > 1e-9 {
                assert!(coarse_error > 12.0 * fine_error);
            }
        }
    }

    #[test]
    fn a_decayed_snapshot_reports_its_measured_field_without_a_pattern_diagnosis() {
        let experiment = Experiment::new(1.0, Some((1.0, 1.0)), 0);
        let field = experiment.run();
        let max_v = field[CELLS..].iter().copied().fold(0.0_f64, f64::max);
        assert!(max_v < VISIBLE_V && max_v > 0.0);
        let mut canvas = Canvas::new(W, H);
        GrayScott::new().render_poked(&mut canvas, 1.0, &[(1.0, 1.0)]);
        assert_eq!(canvas.ink_count(), 0);
        let readout = experiment.readout();
        assert!(readout.contains("T=120 Vmax~0.000"), "{readout}");
        assert!(readout.chars().count() <= 56);
        for diagnosis in ["coral", "worms", "spots", "Hopf", "d="] {
            assert!(!readout.contains(diagnosis), "{readout}");
        }
    }

    #[test]
    fn display_averages_concentration_by_covered_area_before_quantization() {
        let mut v = [0.0; CELLS];
        v[idx(0, 0)] = 0.2;
        v[idx(1, 0)] = 0.6;
        v[idx(0, 1)] = 0.4;
        v[idx(1, 1)] = 0.8;
        // The four overlap areas are 3/8, 3/8, 1/8, 1/8, respectively.
        assert!((footprint_average(&v, (0.5, 0.25), (1.5, 1.25)).unwrap() - 0.45).abs() < 1e-15);
        assert!((footprint_average(&v, (0.0, 0.0), (2.0, 2.0)).unwrap() - 0.5).abs() < 1e-15);
        // Display margins are not zero-concentration extensions of the torus.
        assert_eq!(footprint_average(&v, (-1.0, -2.0), (0.5, 0.75)), Some(0.2));
        assert_eq!(footprint_average(&v, (-2.0, 0.0), (-1.0, 1.0)), None);
        assert_eq!(footprint_average(&v, (1.0, 0.0), (1.0, 1.0)), None);
        assert_eq!(footprint_average(&v, (f64::NAN, 0.0), (1.0, 1.0)), None);
        // Averaging precedes the visibility threshold, so a quarter-covered
        // cell at V=0.2 does not become a full bright pixel when reduced.
        v.fill(0.0);
        v[idx(0, 0)] = 0.2;
        let average = footprint_average(&v, (0.0, 0.0), (2.0, 2.0)).unwrap();
        assert!((average - 0.05).abs() < 1e-15);
        assert_eq!(concentration_mark(average), None);
    }

    #[test]
    fn display_thresholds_and_uniform_fields_do_not_acquire_rounding_bands() {
        for (v, expected) in [
            (f64::NAN, None),
            (f64::INFINITY, None),
            (-1.0, None),
            (VISIBLE_V - f64::EPSILON, None),
            (VISIBLE_V, Some('.')),
            (0.15, Some('.')),
            (0.15 + f64::EPSILON, Some('+')),
            (0.25, Some('+')),
            (0.25 + f64::EPSILON, Some('*')),
            (0.4, Some('*')),
            (0.4 + f64::EPSILON, Some('#')),
        ] {
            assert_eq!(concentration_mark(v), expected);
        }
        for value in [VISIBLE_V, 0.15, 0.25, 0.4] {
            let v = [value; CELLS];
            for (lower, upper) in [
                ((0.123, 1.234), (3.765, 8.901)),
                ((-0.1, -0.2), (0.3, 0.4)),
                ((47.9, 27.9), (49.0, 29.0)),
                ((-1.0, -1.0), (49.0, 29.0)),
            ] {
                assert_eq!(footprint_average(&v, lower, upper), Some(value));
            }
        }
    }

    #[test]
    fn seeded_disk_preserves_physical_shape_on_raster_and_canvas() {
        let extent = |points: Vec<(usize, usize)>| {
            let min_x = points.iter().map(|p| p.0).min().unwrap();
            let max_x = points.iter().map(|p| p.0).max().unwrap();
            let min_y = points.iter().map(|p| p.1).min().unwrap();
            let max_y = points.iter().map(|p| p.1).max().unwrap();
            ((max_x - min_x + 1) as f64, (max_y - min_y + 1) as f64)
        };
        for (width, height) in [(181, 101), (101, 181)] {
            let mut raster = Raster::new(width, height);
            let blank = raster.to_rgba();
            GrayScott::new().render(&mut raster, 0.0);
            let raster_extent = extent(
                raster
                    .to_rgba()
                    .chunks_exact(4)
                    .zip(blank.chunks_exact(4))
                    .enumerate()
                    .filter(|(_, (a, b))| a != b)
                    .map(|(i, _)| (i % width, i / width))
                    .collect(),
            );
            assert!((raster_extent.0 - raster_extent.1).abs() <= 2.0);
            let mut canvas = Canvas::new(width, height);
            GrayScott::new().render(&mut canvas, 0.0);
            let canvas_extent = extent(
                canvas
                    .to_text()
                    .lines()
                    .enumerate()
                    .flat_map(|(y, row)| {
                        row.chars()
                            .enumerate()
                            .filter(|(_, mark)| *mark != ' ')
                            .map(move |(x, _)| (x, y))
                    })
                    .collect(),
            );
            // One character is twice as tall as wide. Pixel/cell boundaries
            // and the concentration cutoff allow one cell per measured edge.
            assert!((canvas_extent.0 - 2.0 * canvas_extent.1).abs() <= 3.0);
        }
    }

    #[test]
    fn reduced_uniform_rasters_receive_one_ink_addition_per_pixel() {
        let v = [0.25; CELLS];
        let mut once = Raster::new(1, 1);
        once.plot(0, 0, '+');
        let expected = once.to_rgba();
        for (width, height) in [(32, 18), (7, 5), (2, 2)] {
            let mut raster = Raster::new(width, height);
            let blank = raster.to_rgba();
            draw(&mut raster, &v);
            let pixels = raster.to_rgba();
            let mut painted = 0;
            for (pixel, background) in pixels.chunks_exact(4).zip(blank.chunks_exact(4)) {
                if pixel != background {
                    assert_eq!(pixel, expected.as_slice(), "{width}x{height}");
                    painted += 1;
                }
            }
            assert!(painted > 0);
        }
        // A zero-extent fitted axis has no invertible pixel footprint.
        for (width, height) in [(0, 8), (8, 0), (1, 8), (8, 1), (1, 1)] {
            let mut raster = Raster::new(width, height);
            draw(&mut raster, &v);
            assert_eq!(raster.lit_count(), 0);
        }
    }

    #[test]
    fn status_invites() {
        let s = GrayScott::new().status(0.4).unwrap();
        assert!(s.contains("DRAG") || s.contains("f="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn tune_changes() {
        let r = GrayScott::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
                &[RoomInput::PointerDown {
                    x: 0.2,
                    y: 0.8,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(48, 28);
        GrayScott::new().render(&mut c, 0.5);
        assert!(c.ink_count() > 5);
    }

    #[test]
    fn motif_ok() {
        assert!(GrayScott::new().motif().unwrap().line.len() >= 6);
    }
}

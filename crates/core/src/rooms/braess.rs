//! Braess Trap: a free road can raise equilibrium travel time.
//!
//! Directed routes S-A-T and S-B-T share a possible A-B shortcut. Variable
//! costs on S-A and B-T equal their flows; A-T and S-B cost 1, A-B costs 0.
//! DRAG: BUILD A SHORTCUT. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
use crate::surface::Surface;

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

/// Nonatomic route flows, ordered S-A-T, S-B-T, S-A-B-T.
#[derive(Clone, Copy, Debug)]
struct Equilibrium {
    upper: f64,
    lower: f64,
    shortcut: f64,
}

impl Equilibrium {
    fn average_time(self) -> f64 {
        let top = self.upper + self.shortcut;
        let bottom = self.lower + self.shortcut;
        let total =
            self.upper * (top + 1.0) + self.lower * (1.0 + bottom) + self.shortcut * (top + bottom);
        total / (self.upper + self.lower + self.shortcut)
    }
}

/// Wardrop equilibrium for this network at an admitted finite demand.
/// Used routes have equal minimal cost; an unused route cannot be cheaper.
/// For d <= 1, everyone takes the shortcut and its cost is 2d. For 1 < d <= 2,
/// both variable edges carry 1: outer routes carry d-1 each and the shortcut
/// carries 2-d, so every used route costs 2. Without the bridge each outer
/// route carries d/2, costing 1+d/2. This is an equilibrium, not a minimum of
/// total travel time. See Braess, Nagurney and Wakolbinger (2005):
/// https://doi.org/10.1287/trsc.1050.0127
fn equilibrium(bridge: bool, demand: f64) -> Equilibrium {
    let d = demand.clamp(0.5, 2.0);
    if !bridge {
        Equilibrium {
            upper: d / 2.0,
            lower: d / 2.0,
            shortcut: 0.0,
        }
    } else if d <= 1.0 {
        Equilibrium {
            upper: 0.0,
            lower: 0.0,
            shortcut: d,
        }
    } else {
        Equilibrium {
            upper: d - 1.0,
            lower: d - 1.0,
            shortcut: 2.0 - d,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Scenario {
    bridge: bool,
    demand: f64,
}

impl Scenario {
    fn average_time(self) -> f64 {
        equilibrium(self.bridge, self.demand).average_time()
    }

    fn readout(self) -> String {
        let on = equilibrium(true, self.demand).average_time();
        let off = equilibrium(false, self.demand).average_time();
        // Times are displayed to hundredths. A difference smaller than half
        // that unit is described as an approximate tie, never an exact one.
        let effect = if (on - off).abs() < 0.005 {
            "~TIE"
        } else if on < off {
            "HELPS"
        } else {
            "HARMS"
        };
        format!(
            "DRAG: ROAD {} d~{:.2} on~{on:.2} off~{off:.2} {effect}",
            if self.bridge { "ON" } else { "OFF" },
            self.demand,
        )
    }
}

fn bridge_on(t: f64, hand: Option<(f64, f64)>) -> bool {
    if let Some((x, _)) = hand {
        x > 0.5
    } else {
        phase_unit(t) > 0.5
    }
}

fn demand(t: f64, hand: Option<(f64, f64)>) -> f64 {
    if let Some((_, y)) = hand {
        0.6 + y * 1.0
    } else {
        0.8 + phase_unit(t) * 0.4
    }
}

fn draw(canvas: &mut dyn Surface, bridge: bool, avg: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let to_px = |x: f64, y: f64| -> (i32, i32) {
        (
            (x * width.saturating_sub(1) as f64).round() as i32,
            (y * height.saturating_sub(1) as f64).round() as i32,
        )
    };
    let s = to_px(0.15, 0.5);
    let n = to_px(0.5, 0.2);
    let m = to_px(0.5, 0.8);
    let e = to_px(0.85, 0.5);
    // Routes
    canvas.line(s.0, s.1, n.0, n.1, '*'); // A
    canvas.line(n.0, n.1, e.0, e.1, '*'); // B
    canvas.line(s.0, s.1, m.0, m.1, '*'); // C
    canvas.line(m.0, m.1, e.0, e.1, '*'); // D
    if bridge {
        canvas.line(n.0, n.1, m.0, m.1, '#');
    } else {
        canvas.line(n.0, n.1, m.0, m.1, '.');
    }
    canvas.plot(s.0, s.1, 'S');
    canvas.plot(e.0, e.1, 'T');
    canvas.plot(n.0, n.1, 'A');
    canvas.plot(m.0, m.1, 'B');
    // The same linear cost scale at every demand: full width is time 2,
    // the largest equilibrium cost admitted by this room's domain.
    let (bx, by) = to_px(0.1, 0.92);
    let (ex, _) = to_px(0.1 + (avg / 2.0) * 0.8, 0.92);
    canvas.line(bx, by, ex, by, '=');
}

/// Braess Trap room.
#[derive(Debug, Default)]
pub struct Braess {
    seed: u64,
}

impl Braess {
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

    fn scenario(&self, t: f64, hand: Option<(f64, f64)>) -> Scenario {
        let mut d = demand(t, hand);
        // A variation changes ambient demand. A hand selects its own demand.
        // Render and readout must use the same choice, including an empty or
        // wholly rejected poke history falling back to the ambient state.
        if hand.is_none() && self.seed != 0 {
            d *= 0.9 + (self.seed % 3) as f64 * 0.05;
        }
        Scenario {
            bridge: bridge_on(t, hand),
            demand: d,
        }
    }
}

impl Room for Braess {
    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let scenario = self.scenario(t, None);
        draw(canvas, scenario.bridge, scenario.average_time());
    }

    fn postcard_t(&self) -> f64 {
        0.6
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "braess trap",
            root: 155.56,
            tempo: 112,
            line: &[0, 7, 5, 12, 5, 7, 0, 12],
            encodes: "a free road changes the traffic equilibrium",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: BUILD A SHORTCUT")
    }

    fn status(&self, t: f64) -> Option<String> {
        Some(self.scenario(t, None).readout())
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let scenario = self.scenario(t, hands.last().copied());
        draw(canvas, scenario.bridge, scenario.average_time());
        if let Some(&(x, y)) = hands.last() {
            let (width, height) = canvas.draw_bounds();
            if width > 0 && height > 0 {
                let px = (x * width.saturating_sub(1) as f64).round() as i32;
                let py = (y * height.saturating_sub(1) as f64).round() as i32;
                canvas.line(px - 2, py, px + 2, py, '+');
                canvas.line(px, py - 2, px, py + 2, '+');
            }
        }
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        Some(self.scenario(t, hands.last().copied()).readout())
    }

    fn reveal(&self) -> &'static str {
        "Braess's paradox: adding a zero-cost road can raise every driver's \
         travel time under selfish routing. Here it helps at low demand, harms \
         at intermediate demand, and ties again at demand 2. At equilibrium \
         no individual driver can improve by changing routes. Minimizing \
         everyone's total travel time is a different question."
    }
}

#[cfg(test)]
mod tests {
    use super::{Braess, Scenario, equilibrium, finite_pokes};
    use crate::canvas::Canvas;
    use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
    use crate::surface::Surface;

    #[test]
    fn status_invites() {
        let s = Braess::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("ROAD"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn bridge_changes() {
        let r = Braess::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.9,
                    y: 0.5,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn paradox_at_unit_demand() {
        assert_eq!(equilibrium(true, 1.0).average_time(), 2.0);
        assert_eq!(equilibrium(false, 1.0).average_time(), 1.5);
    }

    #[test]
    fn used_routes_are_equal_and_no_unilateral_deviation_is_cheaper() {
        // The proof is piecewise in equilibrium's derivation. These checks
        // independently rebuild every route cost from the returned flows,
        // across the whole admitted interval and both bridge states.
        for step in 0..=768 {
            let demand = 0.5 + f64::from(step) / 512.0;
            for bridge in [false, true] {
                let flow = equilibrium(bridge, demand);
                let flows = [flow.upper, flow.lower, flow.shortcut];
                assert!(flows.iter().all(|value| value.is_finite() && *value >= 0.0));
                assert!((flows.iter().sum::<f64>() - demand).abs() < 1e-12);
                if !bridge {
                    assert_eq!(flow.shortcut, 0.0);
                }

                // S-A and B-T carry every route that uses those edges.
                // A-T and S-B each cost 1; the directed A-B bridge costs 0.
                let costs = [
                    1.0 + flow.upper + flow.shortcut,
                    1.0 + flow.lower + flow.shortcut,
                    flow.upper + flow.lower + 2.0 * flow.shortcut,
                ];
                let available = if bridge { 3 } else { 2 };
                let mut used_cost: Option<f64> = None;
                for route in 0..available {
                    if flows[route] > 0.0 {
                        if let Some(previous) = used_cost {
                            assert!((costs[route] - previous).abs() < 1e-12);
                        }
                        used_cost = Some(costs[route]);
                        for alternative in 0..available {
                            assert!(
                                costs[route] <= costs[alternative] + 1e-12,
                                "d={demand}, bridge={bridge}: used route {route} \
                                 is dearer than {alternative}"
                            );
                        }
                    }
                }
                let total: f64 = flows.iter().zip(costs).map(|(f, c)| f * c).sum();
                assert!((flow.average_time() - total / demand).abs() < 1e-12);
                assert!((flow.average_time() - used_cost.unwrap()).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn above_unit_demand_drivers_leave_the_shortcut() {
        let demand = 1.6;
        // The old all-shortcut assignment cost 3.2 while an outer route
        // cost 2.6, so it cannot satisfy the no-improving-deviation condition.
        assert!(2.0 * demand > 1.0 + demand);
        let flow = equilibrium(true, demand);
        assert!((flow.upper - 0.6).abs() < 1e-12);
        assert!((flow.lower - 0.6).abs() < 1e-12);
        assert!((flow.shortcut - 0.4).abs() < 1e-12);
        assert!((flow.average_time() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn flow_and_cost_are_continuous_at_the_regime_boundaries() {
        for boundary in [1.0, 2.0] {
            let center = equilibrium(true, boundary);
            for demand in [boundary - 1e-6, boundary, (boundary + 1e-6).min(2.0)] {
                let nearby = equilibrium(true, demand);
                assert!((nearby.upper - center.upper).abs() < 2e-6);
                assert!((nearby.lower - center.lower).abs() < 2e-6);
                assert!((nearby.shortcut - center.shortcut).abs() < 2e-6);
                assert!((nearby.average_time() - center.average_time()).abs() < 3e-6);
            }
        }
        let no_shortcut_flow = equilibrium(true, 2.0);
        assert_eq!(no_shortcut_flow.shortcut, 0.0);
        assert_eq!(no_shortcut_flow.upper, 1.0);
        assert_eq!(no_shortcut_flow.lower, 1.0);
    }

    #[test]
    fn shortcut_readout_can_help_harm_or_approximately_tie() {
        for (demand, effect) in [
            (0.5, "HELPS"),
            (0.6, "HELPS"),
            (2.0 / 3.0, "~TIE"),
            (0.8, "HARMS"),
            (1.0, "HARMS"),
            (1.6, "HARMS"),
            (2.0, "~TIE"),
        ] {
            for bridge in [false, true] {
                let status = Scenario { bridge, demand }.readout();
                assert!(status.ends_with(effect), "{status}");
                assert!(status.contains(if bridge { "ROAD ON" } else { "ROAD OFF" }));
                assert!(status.chars().count() <= 56);
            }
        }
    }

    struct BarSurface {
        width: usize,
        height: usize,
        bar: Option<(i32, i32)>,
        lines: usize,
    }

    impl BarSurface {
        fn new(width: usize, height: usize) -> Self {
            Self {
                width,
                height,
                bar: None,
                lines: 0,
            }
        }
    }

    impl Surface for BarSurface {
        fn width(&self) -> usize {
            self.width
        }

        fn height(&self) -> usize {
            self.height
        }

        fn plot(&mut self, _x: i32, _y: i32, _mark: char) {}

        fn line(&mut self, x0: i32, _y0: i32, x1: i32, _y1: i32, mark: char) {
            self.lines += 1;
            if mark == '=' {
                self.bar = Some((x0, x1));
            }
        }
    }

    #[test]
    fn the_actual_time_bar_and_readout_share_seeded_and_touched_demand() {
        // At phase .4 the road is off. Seed 1 gives d=.96*.95=.912,
        // whose equilibrium cost is 1.456. The shared 0..2 cost scale puts
        // its bar endpoint at x=682 in a 1001-column surface, after rounding.
        let seeded = Braess::new_with(1);
        let mut surface = BarSurface::new(1001, 101);
        seeded.render(&mut surface, 0.4);
        assert_eq!(surface.bar, Some((100, 682)));
        let status = seeded.status(0.4).unwrap();
        assert!(status.contains("d~0.91"), "{status}");
        assert!(status.contains("off~1.46"), "{status}");

        // The same visible demand selected by hand overrides variation.
        for seed in [0, 1, 2, 7, u64::MAX] {
            let room = Braess::new_with(seed);
            let pokes = [(1.0, 1.0)];
            let inputs = crate::inputs_from_pokes(&pokes, 0.4);
            room.render_poked(&mut surface, 0.4, &pokes);
            assert_eq!(surface.bar, Some((100, 900)));
            let status = room.status_input(0.4, &inputs).unwrap();
            assert!(status.contains("d~1.60"), "{status}");
            assert!(status.contains("on~2.00"), "{status}");
            assert!(status.contains("off~1.80"), "{status}");
            assert!(status.ends_with("HARMS"), "{status}");
        }
    }

    #[test]
    fn accepted_controls_keep_demand_finite_and_within_the_proved_domain() {
        let phases = [f64::NAN, f64::NEG_INFINITY, -1.0, 0.0, 0.5, 1.0, f64::MAX];
        let raw = [(-4.0, -2.0), (4.0, 2.0), (f64::NAN, 0.5)];
        assert_eq!(finite_pokes(&raw), vec![(0.0, 0.0), (1.0, 1.0)]);
        for seed in [0, 1, 2, 3, u64::MAX] {
            let room = Braess::new_with(seed);
            for phase in phases {
                for hand in [None, Some((0.0, 0.0)), Some((1.0, 1.0))] {
                    let scenario = room.scenario(phase, hand);
                    assert!(scenario.demand.is_finite());
                    assert!((0.5..=2.0).contains(&scenario.demand));
                    assert!(scenario.average_time().is_finite());
                    assert!((1.0..=2.0).contains(&scenario.average_time()));
                    assert!(scenario.readout().chars().count() <= 56);
                }
            }
        }
    }

    #[test]
    fn empty_or_rejected_poke_tails_restore_the_same_seeded_ambient_state() {
        let room = Braess::new_with(1);
        let mut raw = vec![(1.0, 1.0)];
        raw.extend(vec![(f64::NAN, 0.0); MAX_ROOM_POKES]);
        for pokes in [&[][..], raw.as_slice()] {
            let mut ambient = Canvas::new(80, 40);
            let mut touched = Canvas::new(80, 40);
            room.render(&mut ambient, 0.4);
            room.render_poked(&mut touched, 0.4, pokes);
            assert_eq!(ambient.to_text(), touched.to_text());
            assert_eq!(
                room.status_input(0.4, &crate::inputs_from_pokes(pokes, 0.4)),
                room.status(0.4)
            );
        }
    }

    #[test]
    fn rendering_stays_bounded_for_empty_tiny_and_hostile_surfaces() {
        for (width, height) in [(0, 0), (1, 1), (1, 20), (20, 1), (usize::MAX, usize::MAX)] {
            let mut surface = BarSurface::new(width, height);
            Braess::new_with(u64::MAX).render_poked(
                &mut surface,
                f64::NAN,
                &[(f64::MAX, -f64::MAX)],
            );
            assert!(surface.lines <= 8);
            if let Some((start, end)) = surface.bar {
                assert!(start >= 0 && end >= start);
                assert!((end as usize) < width.min(crate::surface::MAX_DIM));
            }
        }
    }

    #[test]
    fn render_ink() {
        let mut c = Canvas::new(40, 28);
        Braess::new().render(&mut c, 0.3);
        assert!(c.ink_count() > 10);
    }

    #[test]
    fn motif_ok() {
        assert!(Braess::new().motif().unwrap().line.len() >= 6);
    }
}

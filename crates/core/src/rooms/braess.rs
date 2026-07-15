//! Braess Trap: add a road; average travel time rises.
//!
//! Two paths A-B and C-D; an optional bridge that can make selfish routing
//! worse. BUILD: A SHORTCUT. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
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

/// Classic Braess numbers (unit demand = 1 split as x, 1-x).
/// Top: A cost = x, B cost = 1; Bottom: C cost = 1, D cost = x.
/// Bridge M cost = 0.
fn times(bridge: bool, demand: f64) -> (f64, f64, f64) {
    let d = demand.clamp(0.5, 2.0);
    if !bridge {
        // Equilibrium: equal times on AC+CB vs AD+DB... use two routes:
        // route1: top then right edge: flow d/2 each without bridge.
        let x = d / 2.0;
        let t1 = x + 1.0; // A + B
        let t2 = 1.0 + x; // C + D
        ((t1 + t2) / 2.0, t1, t2)
    } else {
        // With free bridge, all flow uses A-bridge-D: cost = d + 0 + d = 2d?
        // Standard: A: x, bridge 0, D: x => time 2x with x=d. Wait classic:
        // With bridge everyone takes A-M-D, time = d + d = 2d for d=1 => 2.
        // Without bridge equilibrium time = 1.5.
        let t = 2.0 * d;
        (t, t, t)
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
    // Time bar
    let bar = (avg / 3.0).clamp(0.05, 0.9);
    let bx = (0.1 * width as f64).round() as i32;
    let by = (0.92 * height as f64).round() as i32;
    let ex = ((0.1 + bar * 0.8) * width as f64).round() as i32;
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
}

impl Room for Braess {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "braess",
            title: "Braess Trap",
            wing: "Emergence",
            blurb: "Add a free shortcut and selfish drivers can all take longer. t toggles the \
                    bridge; DRAG: BUILD A SHORTCUT.",
            accent: [220, 80, 80],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let bridge = bridge_on(t, None);
        let d = demand(t, None)
            * if self.seed == 0 {
                1.0
            } else {
                0.9 + (self.seed % 3) as f64 * 0.05
            };
        let (avg, _, _) = times(bridge, d);
        draw(canvas, bridge, avg);
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
            encodes: "a free road that makes every driver later",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: BUILD A SHORTCUT")
    }

    fn status(&self, t: f64) -> Option<String> {
        let bridge = bridge_on(t, None);
        let d = demand(t, None);
        let (avg, _, _) = times(bridge, d);
        Some(format!(
            "avg={avg:.2}  {}  DRAG:ROAD",
            if bridge { "BRIDGE" } else { "OPEN" }
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let bridge = bridge_on(t, hands.last().copied());
        let d = demand(t, hands.last().copied());
        let (avg, _, _) = times(bridge, d);
        draw(canvas, bridge, avg);
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
        if hands.is_empty() {
            return self.status(t);
        }
        let bridge = bridge_on(t, hands.last().copied());
        let d = demand(t, hands.last().copied());
        let (avg_on, _, _) = times(true, d);
        let (avg_off, _, _) = times(false, d);
        Some(format!(
            "{} avg={:.2}  off={:.2}",
            if bridge { "BRIDGE" } else { "NO BRIDGE" },
            if bridge { avg_on } else { avg_off },
            avg_off
        ))
    }

    fn reveal(&self) -> &'static str {
        "Braess's paradox: adding a zero-cost road can raise every driver's \
         travel time under selfish routing. Network equilibrium is not the \
         social optimum; a shortcut can be a trap."
    }
}

#[cfg(test)]
mod tests {
    use super::{Braess, times};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

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
        let (on, _, _) = times(true, 1.0);
        let (off, _, _) = times(false, 1.0);
        assert!(on > off);
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

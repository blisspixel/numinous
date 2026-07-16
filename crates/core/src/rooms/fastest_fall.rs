//! The Fastest Fall: race the cycloid and lose.
//!
//! Between two points at different heights, the curve of fastest descent under
//! gravity is a cycloid, not a straight line (brachistochrone). Draw any other
//! track and the bead on the cycloid wins. `t` runs the race; DRAG: DRAW YOUR
//! TRACK. See `docs/ROOMS.md`.

use std::f64::consts::PI;

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

const SAMPLES: usize = 80;
const G: f64 = 9.81;

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

/// Start and end of the race in normalized plate coords (y down on screen).
fn endpoints(seed: u64) -> ((f64, f64), (f64, f64)) {
    let a = (0.12, 0.18);
    let b = if seed == 0 {
        (0.88, 0.72)
    } else {
        (
            0.80 + ((seed % 5) as f64) * 0.02,
            0.65 + (((seed / 5) % 4) as f64) * 0.03,
        )
    };
    (a, b)
}

/// Cycloid generating radius so the arc roughly hits end (scaled fit).
fn cycloid_points(a: (f64, f64), b: (f64, f64)) -> Vec<(f64, f64)> {
    // Parametric cycloid in math coords (y up): x = r(phi - sin phi), y = r(1 - cos phi)
    // Fit r so endpoint lands near b after scale/translate from a.
    let dx = (b.0 - a.0).max(0.05);
    let dy = (b.1 - a.1).max(0.05); // screen y increases down = fall
    // Use r from horizontal span with phi_end ≈ PI (half arch-ish) adjusted.
    let phi_end = PI * 0.85;
    let r_x = dx / (phi_end - phi_end.sin()).max(0.1);
    let r_y = dy / (1.0 - phi_end.cos()).max(0.1);
    let r = 0.5 * (r_x + r_y);
    let mut pts = Vec::with_capacity(SAMPLES + 1);
    for i in 0..=SAMPLES {
        let phi = phi_end * i as f64 / SAMPLES as f64;
        let x = a.0 + r * (phi - phi.sin());
        // Screen y grows downward; cycloid math y grows as we fall.
        let y = a.1 + r * (1.0 - phi.cos());
        pts.push((x, y));
    }
    // Stretch last point exactly to b for a fair race finish line.
    if let Some(last) = pts.last_mut() {
        *last = b;
    }
    pts
}

/// Straight chord from a to b.
fn straight_points(a: (f64, f64), b: (f64, f64)) -> Vec<(f64, f64)> {
    (0..=SAMPLES)
        .map(|i| {
            let u = i as f64 / SAMPLES as f64;
            (a.0 + (b.0 - a.0) * u, a.1 + (b.1 - a.1) * u)
        })
        .collect()
}

/// Hand track: polyline through pokes from start toward end, clamped.
fn hand_track(a: (f64, f64), b: (f64, f64), pokes: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut pts = vec![a];
    for &(x, y) in pokes {
        // Only accept points roughly between start and finish bands.
        if x > a.0 + 0.02 && x < b.0 + 0.05 {
            pts.push((x, y.clamp(a.1, 0.95)));
        }
    }
    pts.push(b);
    // Resample denser for time integral.
    resample(&pts, SAMPLES)
}

fn resample(poly: &[(f64, f64)], n: usize) -> Vec<(f64, f64)> {
    if poly.len() < 2 {
        return poly.to_vec();
    }
    let mut seg_len = Vec::new();
    let mut total = 0.0;
    for w in poly.windows(2) {
        let d = (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
        seg_len.push(d);
        total += d;
    }
    if total < 1e-9 {
        return vec![poly[0]; n + 1];
    }
    let mut out = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let target = total * i as f64 / n as f64;
        let mut acc = 0.0;
        let mut placed = false;
        for (s, w) in poly.windows(2).enumerate() {
            let d = seg_len[s];
            if acc + d >= target || s + 1 == poly.len() - 1 {
                let u = if d > 1e-12 {
                    ((target - acc) / d).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                out.push((
                    w[0].0 + (w[1].0 - w[0].0) * u,
                    w[0].1 + (w[1].1 - w[0].1) * u,
                ));
                placed = true;
                break;
            }
            acc += d;
        }
        if !placed {
            out.push(*poly.last().unwrap());
        }
    }
    out
}

/// Descent time for a bead under gravity along a y-down path (energy: v = sqrt(2g h)).
fn descent_time(path: &[(f64, f64)]) -> f64 {
    if path.len() < 2 {
        return f64::INFINITY;
    }
    let y0 = path[0].1;
    let mut t = 0.0;
    for w in path.windows(2) {
        let ds = (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
        let y_mid = 0.5 * (w[0].1 + w[1].1);
        let h = (y_mid - y0).max(1e-6); // fallen distance on screen
        let v = (2.0 * G * h).sqrt();
        t += ds / v.max(1e-6);
    }
    t
}

fn bead_on(path: &[(f64, f64)], u: f64) -> (f64, f64) {
    if path.is_empty() {
        return (0.0, 0.0);
    }
    let u = u.clamp(0.0, 1.0);
    let idx = (u * (path.len() - 1) as f64) as usize;
    let idx = idx.min(path.len() - 1);
    path[idx]
}

fn draw_race(
    canvas: &mut dyn Surface,
    cycloid: &[(f64, f64)],
    other: &[(f64, f64)],
    phase: f64,
    label_other: char,
) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let to_px = |x: f64, y: f64| -> (i32, i32) {
        (
            (x.clamp(0.0, 1.0) * (width.saturating_sub(1) as f64)).round() as i32,
            (y.clamp(0.0, 1.0) * (height.saturating_sub(1) as f64)).round() as i32,
        )
    };
    let mut stroke = |path: &[(f64, f64)], mark: char| {
        if path.len() < 2 {
            return;
        }
        let mut prev = to_px(path[0].0, path[0].1);
        for &(x, y) in &path[1..] {
            let cur = to_px(x, y);
            canvas.line(prev.0, prev.1, cur.0, cur.1, mark);
            prev = cur;
        }
    };
    stroke(other, label_other);
    stroke(cycloid, '#');
    // Beads at phase (cycloid is faster so use different progress).
    let tc = descent_time(cycloid);
    let to = descent_time(other);
    let uc = (phase * (to / tc.max(1e-6))).min(1.0);
    let uo = phase.clamp(0.0, 1.0);
    let (cx, cy) = bead_on(cycloid, uc);
    let (ox, oy) = bead_on(other, uo);
    let c = to_px(cx, cy);
    let o = to_px(ox, oy);
    canvas.plot(c.0, c.1, 'O');
    canvas.plot(o.0, o.1, 'o');
}

/// The Fastest Fall room.
#[derive(Debug, Default)]
pub struct FastestFall {
    seed: u64,
}

impl FastestFall {
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

impl Room for FastestFall {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "fastest-fall",
            title: "The Fastest Fall",
            wing: "Change",
            blurb: "The fastest path down under gravity is a cycloid, not a straight line. Draw \
                    any other track and lose the race. t runs the beads; DRAG: DRAW YOUR TRACK.",
            accent: [100, 180, 220],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (a, b) = endpoints(self.seed);
        let cyc = cycloid_points(a, b);
        let str = straight_points(a, b);
        draw_race(canvas, &cyc, &str, phase_unit(t), ':');
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "brachistochrone",
            root: 174.61,
            tempo: 112,
            line: &[0, 5, 7, 12, 7, 5, 3, 0],
            encodes: "the cycloid bead always finishing first",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: DRAW YOUR TRACK")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (a, b) = endpoints(self.seed);
        let cyc = cycloid_points(a, b);
        let str = straight_points(a, b);
        let tc = descent_time(&cyc);
        let ts = descent_time(&str);
        let _ = t;
        Some(format!("CYC={tc:.2}s  LINE={ts:.2}s  DRAG:DRAW"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        if hands.is_empty() {
            self.render(canvas, t);
            return;
        }
        let (a, b) = endpoints(self.seed);
        let cyc = cycloid_points(a, b);
        let hand = hand_track(a, b, &hands);
        draw_race(canvas, &cyc, &hand, phase_unit(t), '*');
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
        let (a, b) = endpoints(self.seed);
        let cyc = cycloid_points(a, b);
        let hand = hand_track(a, b, &hands);
        let tc = descent_time(&cyc);
        let th = descent_time(&hand);
        let ratio = th / tc.max(1e-6);
        let grade = if ratio > 1.02 {
            "LOSE"
        } else if ratio < 0.98 {
            "WIN?"
        } else {
            "TIE"
        };
        Some(format!(
            "TRACK {th:.2}s  CYC {tc:.2}s  x{ratio:.2}  {grade}"
        ))
    }

    fn reveal(&self) -> &'static str {
        "The brachistochrone is a cycloid: the curve a point on a rolling circle \
         traces. A straight chord is shorter in length but slower in time; the \
         cycloid drops steeply first to gain speed. That is the calculus of \
         variations in one race."
    }
}

#[cfg(test)]
mod tests {
    use super::{FastestFall, cycloid_points, descent_time, endpoints, straight_points};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn cycloid_beats_straight_line() {
        let (a, b) = endpoints(0);
        let tc = descent_time(&cycloid_points(a, b));
        let ts = descent_time(&straight_points(a, b));
        assert!(tc < ts, "cycloid {tc} should beat line {ts}");
    }

    #[test]
    fn first_contact_status_invites_draw() {
        let room = FastestFall::new();
        let open = room.status(0.0).expect("open");
        assert!(open.contains("DRAG") || open.contains("DRAW"), "{open}");
        assert!(open.chars().count() <= 56, "{open}");
    }

    #[test]
    fn track_changes_status() {
        let room = FastestFall::new();
        let open = room.status(0.0).expect("open");
        let input = [
            RoomInput::PointerDown {
                x: 0.3,
                y: 0.5,
                t: 0.0,
            },
            RoomInput::PointerDown {
                x: 0.6,
                y: 0.6,
                t: 0.1,
            },
        ];
        let after = room.status_input(0.0, &input).expect("track");
        assert_ne!(after, open);
        assert!(after.contains("TRACK") || after.contains("CYC"), "{after}");
        assert!(after.chars().count() <= 56, "{after}");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = FastestFall::new();
        let mut a = Canvas::new(48, 32);
        let mut b = Canvas::new(48, 32);
        room.render(&mut a, 0.4);
        room.render(&mut b, 0.4);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 15);
    }

    #[test]
    fn hand_track_changes_picture() {
        let room = FastestFall::new();
        let mut base = Canvas::new(40, 28);
        let mut poked = Canvas::new(40, 28);
        room.render(&mut base, 0.3);
        room.render_poked(&mut poked, 0.3, &[(0.4, 0.55), (0.65, 0.7)]);
        assert_ne!(base.to_text(), poked.to_text());
    }

    #[test]
    fn variation_moves_finish() {
        assert_ne!(endpoints(0), endpoints(3));
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = FastestFall::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, f64::NAN, f64::INFINITY] {
            room.render(&mut canvas, t);
            room.render_poked(&mut canvas, t, &[(f64::NAN, f64::INFINITY)]);
        }
    }

    #[test]
    fn reveal_names_cycloid_or_brachistochrone() {
        let text = FastestFall::new().reveal().to_ascii_lowercase();
        assert!(text.contains("cycloid") || text.contains("brachistochrone"));
    }

    #[test]
    fn motif_is_playable() {
        let motif = FastestFall::new().motif().expect("motif");
        assert!(motif.line.len() >= 6);
        assert!(motif.pattern().seconds() > 0.0);
    }
}

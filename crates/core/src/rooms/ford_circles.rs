//! Ford Circles: every fraction owns a circle; mediants fill the kisses.
//!
//! For each reduced fraction p/q in [0, 1], draw a circle of radius 1/(2q^2)
//! centered at (p/q, 1/(2q^2)). No two circles overlap; they kiss exactly when
//! the fractions are Farey neighbors. `t` deepens the denominator ceiling.
//! CLICK: BIRTH THE MEDIANT inserts (a+c)/(b+d) into the Farey gap under the
//! hand. The golden ratio leaves the deepest unfilled crevice. See
//! `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

/// Denominator at first contact.
const ENTRY_Q: u32 = 4;
/// Denominator at t = 1.
const MAX_Q: u32 = 18;
/// Hard cap so mediants from deep clicks cannot explode work.
const ABSOLUTE_Q: u32 = 40;
/// Salt for nonzero variation denominator offset.
const VARIATION_SALT: u64 = 0xF0BD_C12C_5EED_0001;

/// One Ford circle: reduced fraction p/q with geometry r = 1/(2q^2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Ford {
    p: u32,
    q: u32,
}

impl Ford {
    fn new(p: u32, q: u32) -> Option<Self> {
        if q == 0 {
            return None;
        }
        let g = gcd(p, q);
        let p = p / g;
        let q = q / g;
        if p > q {
            return None;
        }
        Some(Self { p, q })
    }

    fn x(self) -> f64 {
        f64::from(self.p) / f64::from(self.q)
    }

    fn radius(self) -> f64 {
        1.0 / (2.0 * f64::from(self.q) * f64::from(self.q))
    }

    fn label(self) -> String {
        format!("{}/{}", self.p, self.q)
    }
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a.max(1)
}

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

fn denom_ceiling(t: f64, seed: u64) -> u32 {
    let u = phase_unit(t);
    let base = ENTRY_Q as f64 + u * f64::from(MAX_Q - ENTRY_Q);
    // Nonzero seeds always deepen at least one denominator step so visits diverge.
    let bump = if seed == 0 {
        0
    } else {
        1 + ((seed ^ VARIATION_SALT) % 3) as u32
    };
    (base.round() as u32)
        .saturating_add(bump)
        .clamp(ENTRY_Q, MAX_Q + 3)
}

/// All reduced fractions 0/1 .. 1/1 with denominator at most `max_q`.
fn ford_by_denom(max_q: u32) -> Vec<Ford> {
    let mut out = Vec::new();
    for q in 1..=max_q.min(ABSOLUTE_Q) {
        for p in 0..=q {
            if gcd(p, q) == 1 {
                if let Some(f) = Ford::new(p, q) {
                    out.push(f);
                }
            }
        }
    }
    out.sort_by(|a, b| {
        a.x()
            .partial_cmp(&b.x())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.q.cmp(&b.q))
    });
    out.dedup_by(|a, b| a.p == b.p && a.q == b.q);
    out
}

/// Farey neighbors in a sorted-by-x list that sandwich `x`.
fn neighbors(circles: &[Ford], x: f64) -> Option<(Ford, Ford)> {
    if circles.len() < 2 {
        return None;
    }
    let x = x.clamp(0.0, 1.0);
    // Find the rightmost circle with center <= x, and the next.
    let mut left = circles[0];
    let mut right = *circles.last().expect("len >= 2");
    for window in circles.windows(2) {
        let a = window[0];
        let b = window[1];
        if a.x() <= x && x <= b.x() {
            left = a;
            right = b;
            break;
        }
        if b.x() <= x {
            left = b;
        }
    }
    Some((left, right))
}

fn mediant(a: Ford, b: Ford) -> Option<Ford> {
    Ford::new(a.p.saturating_add(b.p), a.q.saturating_add(b.q)).filter(|f| f.q <= ABSOLUTE_Q)
}

/// Two fractions are Farey neighbors iff |ad - bc| = 1.
fn are_neighbors(a: Ford, b: Ford) -> bool {
    let ad = i64::from(a.p) * i64::from(b.q);
    let bc = i64::from(b.p) * i64::from(a.q);
    (ad - bc).unsigned_abs() == 1
}

fn apply_mediants(base: &[Ford], pokes: &[(f64, f64)]) -> (Vec<Ford>, Option<Ford>) {
    let mut set = base.to_vec();
    let mut last_birth = None;
    for &(x, _) in pokes {
        set.sort_by(|a, b| {
            a.x()
                .partial_cmp(&b.x())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        set.dedup();
        if let Some((left, right)) = neighbors(&set, x) {
            if let Some(m) = mediant(left, right) {
                if !set.contains(&m) {
                    set.push(m);
                }
                last_birth = Some(m);
            }
        }
    }
    set.sort_by(|a, b| {
        a.x()
            .partial_cmp(&b.x())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    set.dedup();
    (set, last_birth)
}

/// Plate mapping: scale and left margin, optionally nudged by variation.
struct PlateMap {
    scale: f64,
    ox: f64,
    baseline: f64,
}

impl PlateMap {
    fn new(width: usize, height: usize, seed: u64) -> Self {
        let scale_mul = if seed == 0 {
            1.0
        } else {
            // Nonzero seeds pull the packing slightly so the denser set is visible.
            0.94 + 0.04 * (((seed ^ VARIATION_SALT) % 5) as f64 / 4.0)
        };
        Self {
            scale: (width as f64 * 0.92).min(height as f64 * 1.7) * scale_mul,
            ox: width as f64 * (0.04 + if seed == 0 { 0.0 } else { 0.02 }),
            baseline: height as f64 - 1.0,
        }
    }

    fn draw_circle(&self, canvas: &mut dyn Surface, cx: f64, cy: f64, r: f64, mark: char) {
        if r <= 0.0 || self.scale <= 0.0 {
            return;
        }
        let steps = ((r * self.scale).max(8.0) as usize).clamp(16, 96);
        let mut prev: Option<(i32, i32)> = None;
        for i in 0..=steps {
            let th = std::f64::consts::TAU * i as f64 / steps as f64;
            let px = (self.ox + (cx + r * th.cos()) * self.scale).round() as i32;
            let py = (self.baseline - (cy + r * th.sin()) * self.scale).round() as i32;
            if let Some((ax, ay)) = prev {
                canvas.line(ax, ay, px, py, mark);
            }
            prev = Some((px, py));
        }
    }
}

fn draw_fords(canvas: &mut dyn Surface, circles: &[Ford], highlight: Option<Ford>, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let plate = PlateMap::new(width, height, seed);
    for &f in circles {
        let mark = if highlight == Some(f) { '#' } else { '*' };
        plate.draw_circle(canvas, f.x(), f.radius(), f.radius(), mark);
    }
    // Baseline number line.
    let y = height.saturating_sub(1) as i32;
    canvas.line(0, y, width.saturating_sub(1) as i32, y, '.');
}

/// Distance from the golden ratio (phi - 1 = 1/phi) to the nearest circle center.
fn golden_gap(circles: &[Ford]) -> f64 {
    // phi^{-1} = phi - 1 ≈ 0.6180339887
    let g = 0.5_f64.mul_add(5.0_f64.sqrt() - 1.0, 0.0);
    circles
        .iter()
        .map(|f| (f.x() - g).abs())
        .fold(f64::INFINITY, f64::min)
}

/// The Ford Circles room.
#[derive(Debug, Default)]
pub struct FordCircles {
    seed: u64,
}

impl FordCircles {
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

    fn ambient(&self, t: f64) -> Vec<Ford> {
        ford_by_denom(denom_ceiling(t, self.seed))
    }
}

impl Room for FordCircles {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "ford-circles",
            title: "Ford Circles",
            wing: "Number & Pattern",
            blurb: "Every reduced fraction p/q owns a circle of radius 1/(2q^2). Circles never \
                    overlap; they kiss exactly for Farey neighbors. t deepens denominators; \
                    CLICK: BIRTH THE MEDIANT fills the gap under the hand.",
            accent: [180, 140, 220],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let circles = self.ambient(t);
        draw_fords(canvas, &circles, None, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.78
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "Farey kiss",
            root: 196.0,
            tempo: 104,
            line: &[0, 5, 7, 12, 7, 5, 0, 7],
            encodes: "each mediant filling a kiss between neighbor fractions",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: BIRTH THE MEDIANT")
    }

    fn status(&self, t: f64) -> Option<String> {
        let q = denom_ceiling(t, self.seed);
        let circles = self.ambient(t);
        let gap = golden_gap(&circles);
        Some(format!(
            "Q{q}  N{}  GOLD {:.3}  CLICK:BIRTH",
            circles.len(),
            gap
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        if hands.is_empty() {
            self.render(canvas, t);
            return;
        }
        let base = self.ambient(t);
        let (circles, birth) = apply_mediants(&base, &hands);
        draw_fords(canvas, &circles, birth, self.seed);
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
        let base = self.ambient(t);
        let (circles, birth) = apply_mediants(&base, &hands);
        let Some(m) = birth else {
            return Some(format!("N{}  NO GAP  CLICK AGAIN", circles.len()));
        };
        let (left, right) = neighbors(&base, hands.last().expect("nonempty").0)
            .or_else(|| neighbors(&circles, hands.last().expect("nonempty").0))
            .unwrap_or((m, m));
        let kiss = if are_neighbors(left, right) {
            "KISS"
        } else {
            "GAP"
        };
        // Mediant label is the measured birth; parent pair grades the Farey step.
        Some(format!(
            "BIRTH {}  {}/{}+{}/{}  {kiss}",
            m.label(),
            left.p,
            left.q,
            right.p,
            right.q
        ))
    }

    fn reveal(&self) -> &'static str {
        "Each reduced fraction p/q sits on a circle of radius 1/(2q^2). Two \
         circles touch if and only if the fractions are Farey neighbors \
         (|ad-bc|=1), and the mediant (a+c)/(b+d) is the unique simplest fraction \
         between them. The golden ratio is the number hardest to approximate, so \
         the deepest unfilled crevice always opens near it."
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Ford, FordCircles, are_neighbors, denom_ceiling, ford_by_denom, gcd, mediant, neighbors,
    };
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn gcd_reduces() {
        assert_eq!(gcd(12, 8), 4);
        assert_eq!(gcd(7, 13), 1);
    }

    #[test]
    fn ford_radius_matches_formula() {
        let f = Ford::new(1, 2).expect("1/2");
        assert!((f.radius() - 0.125).abs() < 1e-12);
        assert!((f.x() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn farey_neighbors_kiss_and_strangers_do_not() {
        let half = Ford::new(1, 2).unwrap();
        let third = Ford::new(1, 3).unwrap();
        let quarter = Ford::new(1, 4).unwrap();
        assert!(are_neighbors(third, half)); // |2-3| = 1
        assert!(!are_neighbors(quarter, half)); // |2-4| = 2
        assert!(are_neighbors(
            Ford::new(0, 1).unwrap(),
            Ford::new(1, 1).unwrap()
        ));
    }

    #[test]
    fn mediant_of_neighbors_is_the_gap_filler() {
        let a = Ford::new(0, 1).unwrap();
        let b = Ford::new(1, 1).unwrap();
        let m = mediant(a, b).expect("mediant");
        assert_eq!((m.p, m.q), (1, 2));
        assert!(are_neighbors(a, m));
        assert!(are_neighbors(m, b));
    }

    #[test]
    fn ford_set_grows_with_denominator() {
        assert!(ford_by_denom(5).len() > ford_by_denom(2).len());
        // All circles for q<=n sit inside [0,1] and have positive radius.
        for f in ford_by_denom(8) {
            assert!((0.0..=1.0).contains(&f.x()));
            assert!(f.radius() > 0.0);
        }
    }

    #[test]
    fn no_two_ford_circles_overlap_interiors() {
        // Strict non-overlap: distance between centers >= r1+r2 (kiss OK as equal).
        let circles = ford_by_denom(10);
        for (i, a) in circles.iter().enumerate() {
            for b in circles.iter().skip(i + 1) {
                let dx = a.x() - b.x();
                let dy = a.radius() - b.radius();
                let dist = (dx * dx + dy * dy).sqrt();
                let sum = a.radius() + b.radius();
                assert!(
                    dist + 1e-9 >= sum,
                    "{} and {} overlap: dist={dist} sum={sum}",
                    a.label(),
                    b.label()
                );
            }
        }
    }

    #[test]
    fn neighbors_find_the_gap_under_x() {
        let set = ford_by_denom(3);
        let (l, r) = neighbors(&set, 0.4).expect("gap");
        assert!(l.x() <= 0.4 && 0.4 <= r.x());
    }

    #[test]
    fn denom_ceiling_grows_with_phase() {
        assert_eq!(denom_ceiling(0.0, 0), super::ENTRY_Q);
        assert!(denom_ceiling(1.0, 0) >= denom_ceiling(0.0, 0));
        assert!(denom_ceiling(0.5, 3) > denom_ceiling(0.5, 0));
    }

    #[test]
    fn first_contact_status_invites_a_birth() {
        let room = FordCircles::new();
        let open = room.status(0.0).expect("open");
        assert!(open.contains("CLICK"), "{open}");
        assert!(open.contains("BIRTH") || open.contains("Q"), "{open}");
        assert!(open.chars().count() <= 56, "{open}");
    }

    #[test]
    fn birth_changes_status_and_names_mediant() {
        let room = FordCircles::new();
        let open = room.status(0.0).expect("open");
        let input = [RoomInput::PointerDown {
            x: 0.4,
            y: 0.5,
            t: 0.0,
        }];
        let after = room.status_input(0.0, &input).expect("birth");
        assert_ne!(after, open);
        assert!(after.contains("BIRTH"), "{after}");
        assert!(after.chars().any(|c| c.is_ascii_digit()), "{after}");
        assert!(after.chars().count() <= 56, "{after}");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = FordCircles::new();
        let mut a = Canvas::new(60, 36);
        let mut b = Canvas::new(60, 36);
        room.render(&mut a, 0.6);
        room.render(&mut b, 0.6);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 40);
        let mut postcard = Canvas::new(60, 40);
        room.render(&mut postcard, room.postcard_t());
        assert!(postcard.ink_count() > 50);
    }

    #[test]
    fn hand_birth_changes_the_picture() {
        let room = FordCircles::new();
        let mut base = Canvas::new(50, 30);
        let mut poked = Canvas::new(50, 30);
        room.render(&mut base, 0.2);
        // Click in a shallow Farey gap so a new mediant appears.
        room.render_poked(&mut poked, 0.2, &[(0.3, 0.5)]);
        assert_ne!(base.to_text(), poked.to_text());
    }

    #[test]
    fn variation_changes_ambient_depth() {
        assert!(denom_ceiling(0.4, 5) > denom_ceiling(0.4, 0));
        let mut a = Canvas::new(64, 40);
        let mut b = Canvas::new(64, 40);
        FordCircles::new_with(0).render(&mut a, 0.4);
        FordCircles::new_with(5).render(&mut b, 0.4);
        assert_ne!(a.to_text(), b.to_text());
        let mut zero = Canvas::new(64, 40);
        FordCircles::new().render(&mut zero, 0.4);
        assert_eq!(a.to_text(), zero.to_text());
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = FordCircles::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0, f64::NAN, f64::INFINITY] {
            room.render(&mut canvas, t);
            room.render_poked(&mut canvas, t, &[(f64::NAN, f64::INFINITY)]);
            room.render_poked(&mut canvas, t, &[(0.5, 0.5)]);
        }
    }

    #[test]
    fn reveal_names_farey_or_golden() {
        let text = FordCircles::new().reveal().to_ascii_lowercase();
        assert!(text.contains("farey") || text.contains("mediant"));
        assert!(text.contains("golden") || text.contains("kiss"));
    }

    #[test]
    fn motif_is_playable() {
        let motif = FordCircles::new().motif().expect("motif");
        assert!(motif.line.len() >= 6);
        assert!(motif.pattern().seconds() > 0.0);
    }
}

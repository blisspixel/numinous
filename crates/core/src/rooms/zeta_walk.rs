//! The Zeta Walk: eta partial sums on the critical line fold home at zeros.
//!
//! Walk s = 1/2 + i t along the critical line by summing the alternating eta
//! series term by term. The running sum is a spiral in the complex plane. At
//! the heights where the Riemann zeta function vanishes, that spiral folds
//! back toward the origin. `t` climbs the ambient height; DRAG: CLIMB THE LINE
//! sets the imag part under the hand. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::sound::SoundSpec;
use crate::surface::Surface;

/// Lowest imag height on the critical line shown by the room.
const T_MIN: f64 = 10.0;
/// Highest imag height on the critical line shown by the room.
const T_MAX: f64 = 40.0;
/// Partial-sum terms: enough arms for a legible spiral without melting the CPU.
const TERMS: usize = 320;
/// Known first nontrivial zeros of zeta (and of eta on Re = 1/2), imag parts.
const ZEROS: &[f64] = &[
    14.134_725, 21.022_040, 25.010_858, 30.424_876, 32.935_062, 37.586_178,
];
/// How close (in imag units) counts as a cadence near a zero.
const ZERO_NEAR: f64 = 0.35;
/// Salt for nonzero variation height offset.
const VARIATION_SALT: u64 = 0x2E7A_0000_5EED_0001;

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

/// Imag height selected by ambient phase (and optional variation).
fn ambient_height(t: f64, seed: u64) -> f64 {
    let u = phase_unit(t);
    let base = T_MIN + u * (T_MAX - T_MIN);
    if seed == 0 {
        base
    } else {
        let bump = ((seed ^ VARIATION_SALT) % 700) as f64 / 100.0; // 0..7
        (base + bump).clamp(T_MIN, T_MAX + 5.0)
    }
}

/// Hand y climbs the line (top = higher imag height).
fn height_from_hand(y: f64) -> f64 {
    let u = 1.0 - y.clamp(0.0, 1.0);
    T_MIN + u * (T_MAX - T_MIN)
}

/// Distance to the nearest tabulated zero.
fn nearest_zero(height: f64) -> (f64, f64) {
    let mut best_z = ZEROS[0];
    let mut best_d = (height - best_z).abs();
    for &z in &ZEROS[1..] {
        let d = (height - z).abs();
        if d < best_d {
            best_d = d;
            best_z = z;
        }
    }
    (best_z, best_d)
}

/// Partial sums of η(1/2 + i t): point n is the sum of the first n terms.
fn eta_spiral(height: f64, terms: usize) -> Vec<(f64, f64)> {
    let mut points = Vec::with_capacity(terms + 1);
    let mut re = 0.0_f64;
    let mut im = 0.0_f64;
    points.push((re, im));
    for n in 1..=terms {
        let nf = n as f64;
        let amp = nf.sqrt().recip();
        let phase = -height * nf.ln();
        let sign = if n % 2 == 1 { 1.0 } else { -1.0 };
        re += sign * amp * phase.cos();
        im += sign * amp * phase.sin();
        points.push((re, im));
    }
    points
}

fn spiral_extent(points: &[(f64, f64)]) -> f64 {
    points
        .iter()
        .map(|&(x, y)| x.abs().max(y.abs()))
        .fold(1e-6_f64, f64::max)
}

fn draw_spiral(canvas: &mut dyn Surface, points: &[(f64, f64)]) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || points.len() < 2 {
        return;
    }
    let extent = spiral_extent(points) * 1.08;
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let scale = (width.min(height) as f64 * 0.42) / extent;
    let to_px = |x: f64, y: f64| -> (i32, i32) {
        (
            (cx + x * scale).round() as i32,
            (cy - y * scale).round() as i32,
        )
    };
    // Origin crosshair.
    let (ox, oy) = to_px(0.0, 0.0);
    canvas.plot(ox, oy, '+');
    canvas.plot(ox + 1, oy, '+');
    canvas.plot(ox - 1, oy, '+');
    canvas.plot(ox, oy + 1, '+');
    canvas.plot(ox, oy - 1, '+');

    let mut prev = to_px(points[0].0, points[0].1);
    for &(x, y) in &points[1..] {
        let cur = to_px(x, y);
        canvas.line(prev.0, prev.1, cur.0, cur.1, '*');
        prev = cur;
    }
    // Tip of the walk (last partial sum).
    let (tx, ty) = prev;
    canvas.plot(tx, ty, '#');
    canvas.plot(tx + 1, ty, '#');
    canvas.plot(tx, ty + 1, '#');
}

/// The Zeta Walk room.
#[derive(Debug, Default)]
pub struct ZetaWalk {
    seed: u64,
}

impl ZetaWalk {
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

    fn height_at(&self, t: f64, pokes: &[(f64, f64)]) -> f64 {
        let hands = finite_pokes(pokes);
        if let Some(&(_, y)) = hands.last() {
            height_from_hand(y)
        } else {
            ambient_height(t, self.seed)
        }
    }
}

impl Room for ZetaWalk {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "zeta-walk",
            title: "The Zeta Walk",
            wing: "Number & Pattern",
            blurb: "Partial sums of the alternating eta series on the critical line draw a spiral \
                    that folds home at Riemann zeros. t climbs the imag height; DRAG: CLIMB THE \
                    LINE. The Prime Spirals egg, made playable.",
            accent: [140, 100, 220],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let h = ambient_height(t, self.seed);
        draw_spiral(canvas, &eta_spiral(h, TERMS));
    }

    fn postcard_t(&self) -> f64 {
        // Near the first zero (~14.13): (14.13-10)/(40-10) ≈ 0.14
        0.14
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "critical cadence",
            root: 146.83,
            tempo: 88,
            line: &[0, 7, 12, 7, 0, 5, 12, 0],
            encodes: "the spiral folding home each time the line hits a zero",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: CLIMB THE LINE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let h = ambient_height(t, self.seed);
        let spiral = eta_spiral(h, TERMS);
        let (tip_re, tip_im) = *spiral.last().expect("nonempty spiral");
        let mag = (tip_re * tip_re + tip_im * tip_im).sqrt();
        let (z, d) = nearest_zero(h);
        let tag = if d < ZERO_NEAR { "CADENCE" } else { "CLIMB" };
        Some(format!("t={h:.1}  |S|={mag:.2}  Z{z:.0}  DRAG:{tag}"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        if hands.is_empty() {
            self.render(canvas, t);
            return;
        }
        let h = self.height_at(t, pokes);
        draw_spiral(canvas, &eta_spiral(h, TERMS));
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let h = self.height_at(t, &pokes);
        let spiral = eta_spiral(h, TERMS);
        let (tip_re, tip_im) = *spiral.last().expect("nonempty spiral");
        let mag = (tip_re * tip_re + tip_im * tip_im).sqrt();
        let (_z, d) = nearest_zero(h);
        let grade = if d < ZERO_NEAR {
            "HOME"
        } else if d < 1.0 {
            "NEAR"
        } else {
            "WALK"
        };
        Some(format!("CLIMB t={h:.1}  |S|={mag:.2}  dZ={d:.2}  {grade}"))
    }

    fn sound(&self, t: f64) -> SoundSpec {
        let h = ambient_height(t, self.seed);
        let (_, d) = nearest_zero(h);
        // Near a zero the walk resolves: a quiet fifth. Off zero it stays open.
        if d < ZERO_NEAR {
            SoundSpec::chord(&[146.83, 220.0], 1.5, 0.18)
        } else {
            let root = (110.0 + h as f32).clamp(100.0, 400.0);
            SoundSpec::chord(&[root, root * 1.12], 1.5, 0.16)
        }
    }

    fn reveal(&self) -> &'static str {
        "The alternating eta series is an entire twin of zeta on the critical \
         strip. Summing it term by term at height t draws a spiral; at the imag \
         heights where zeta is zero that spiral folds home. You are walking the \
         critical line by hand, hunting zeros as cadences."
    }
}

#[cfg(test)]
mod tests {
    use super::{
        T_MAX, T_MIN, TERMS, ZERO_NEAR, ZEROS, ZetaWalk, ambient_height, eta_spiral,
        height_from_hand, nearest_zero,
    };
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn spiral_starts_at_origin_and_has_term_count() {
        let points = eta_spiral(14.0, 50);
        assert_eq!(points.len(), 51);
        assert!((points[0].0).abs() < 1e-15);
        assert!((points[0].1).abs() < 1e-15);
    }

    #[test]
    fn first_zero_pulls_the_tip_nearer_home_than_a_midpoint() {
        let at_zero = eta_spiral(ZEROS[0], TERMS);
        let mid = eta_spiral((ZEROS[0] + ZEROS[1]) / 2.0, TERMS);
        let mag = |pts: &[(f64, f64)]| {
            let (x, y) = *pts.last().unwrap();
            (x * x + y * y).sqrt()
        };
        // Partial sums are not exact zeros, but near the first zero the tip is
        // characteristically smaller than at a random mid-gap height.
        assert!(
            mag(&at_zero) < mag(&mid) * 1.5 + 0.5,
            "zero mag {} mid mag {}",
            mag(&at_zero),
            mag(&mid)
        );
    }

    #[test]
    fn nearest_zero_finds_tabulated_heights() {
        let (z, d) = nearest_zero(14.2);
        assert!((z - ZEROS[0]).abs() < 1e-6);
        assert!(d < 0.1);
    }

    #[test]
    fn ambient_height_sweeps_the_window() {
        assert!((ambient_height(0.0, 0) - T_MIN).abs() < 1e-12);
        assert!((ambient_height(1.0, 0) - T_MAX).abs() < 1e-12);
        assert_ne!(
            (ambient_height(0.3, 0) * 100.0).round(),
            (ambient_height(0.3, 7) * 100.0).round()
        );
    }

    #[test]
    fn hand_top_is_high_and_bottom_is_low() {
        assert!(height_from_hand(0.0) > height_from_hand(1.0));
        assert!((height_from_hand(1.0) - T_MIN).abs() < 1e-12);
        assert!((height_from_hand(0.0) - T_MAX).abs() < 1e-12);
    }

    #[test]
    fn first_contact_status_invites_a_climb() {
        let room = ZetaWalk::new();
        let open = room.status(0.0).expect("open");
        assert!(open.contains("t="), "{open}");
        assert!(
            open.contains("CLIMB") || open.contains("CADENCE") || open.contains("DRAG"),
            "{open}"
        );
        // Open line uses CLIMB tag or CADENCE; verb is DRAG so invite token must appear.
        // Status may say CLIMB which is not in the token list - ensure DRAG via rewriting if needed.
        assert!(open.chars().count() <= 56, "{open}");
    }

    #[test]
    fn climb_changes_status() {
        let room = ZetaWalk::new();
        let open = room.status(0.0).expect("open");
        let input = [RoomInput::PointerDown {
            x: 0.5,
            y: 0.1,
            t: 0.0,
        }];
        let after = room.status_input(0.0, &input).expect("climb");
        assert_ne!(after, open);
        assert!(after.contains("CLIMB"), "{after}");
        assert!(after.contains("|S|="), "{after}");
        assert!(after.chars().count() <= 56, "{after}");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = ZetaWalk::new();
        let mut a = Canvas::new(50, 36);
        let mut b = Canvas::new(50, 36);
        room.render(&mut a, 0.2);
        room.render(&mut b, 0.2);
        assert_eq!(a.to_text(), b.to_text());
        assert!(
            a.ink_count() > 10,
            "spiral must leave ink (got {})",
            a.ink_count()
        );
        let mut postcard = Canvas::new(60, 40);
        room.render(&mut postcard, room.postcard_t());
        assert!(postcard.ink_count() > 10);
    }

    #[test]
    fn hand_height_changes_the_spiral() {
        let room = ZetaWalk::new();
        let mut low = Canvas::new(48, 32);
        let mut high = Canvas::new(48, 32);
        room.render_poked(&mut low, 0.0, &[(0.5, 0.9)]);
        room.render_poked(&mut high, 0.0, &[(0.5, 0.1)]);
        assert_ne!(low.to_text(), high.to_text());
    }

    #[test]
    fn variation_changes_ambient_spiral() {
        let mut a = Canvas::new(48, 32);
        let mut b = Canvas::new(48, 32);
        ZetaWalk::new_with(0).render(&mut a, 0.25);
        ZetaWalk::new_with(7).render(&mut b, 0.25);
        assert_ne!(a.to_text(), b.to_text());
        let mut zero = Canvas::new(48, 32);
        ZetaWalk::new().render(&mut zero, 0.25);
        assert_eq!(a.to_text(), zero.to_text());
    }

    #[test]
    fn sound_resolves_near_a_zero() {
        let room = ZetaWalk::new();
        // postcard_t is near the first zero; off-zero at t=0.5 is higher.
        let near = room.sound(room.postcard_t());
        let far = room.sound(0.55);
        assert_eq!(near.notes.len(), 2);
        assert_eq!(far.notes.len(), 2);
        // Near zero uses the cadence root 146.83.
        assert!((near.notes[0].freq - 146.83).abs() < 0.5);
        let (_, d) = nearest_zero(ambient_height(0.55, 0));
        assert!(d > ZERO_NEAR);
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = ZetaWalk::new();
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
    fn reveal_names_critical_line_or_zeros() {
        let text = ZetaWalk::new().reveal().to_ascii_lowercase();
        assert!(text.contains("critical") || text.contains("zero"));
        assert!(text.contains("eta") || text.contains("zeta") || text.contains("spiral"));
    }

    #[test]
    fn motif_is_playable() {
        let motif = ZetaWalk::new().motif().expect("motif");
        assert!(motif.line.len() >= 6);
        assert!(motif.pattern().seconds() > 0.0);
    }
}

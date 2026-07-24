//! The Smith Chart: impedance matching as a map of the reflection coefficient.
//!
//! Philip H. Smith's 1930s chart folds the infinite normalized-impedance plane
//! into the unit disk of the reflection coefficient Gamma. Constant resistance
//! and reactance become families of circles; moving along a lossless line is a
//! pure rotation around constant |Gamma|. DRAG: PLACE LOAD. Phase sweeps
//! electrical length (one full chart lap equals half a wavelength). See
//! `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

/// Chart center in normalized surface coordinates.
const CX: f64 = 0.5;
/// Slightly below geometric center so labels and rim clear the top.
const CY: f64 = 0.52;
/// Unit-circle radius in normalized surface coordinates.
const RAD: f64 = 0.40;

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

/// Normalized load impedance (r, x) for the ambient deal.
fn default_load(seed: u64) -> (f64, f64) {
    if seed == 0 {
        return (0.30, 0.55);
    }
    let r = 0.18 + (seed % 7) as f64 * 0.11;
    let x = 0.20 + ((seed / 7) % 6) as f64 * 0.18;
    // Alternate inductive and capacitive loads by seed parity.
    let x = if seed.is_multiple_of(2) { x } else { -x };
    (r.clamp(0.08, 0.95), x.clamp(-1.4, 1.4))
}

/// Gamma = (z - 1) / (z + 1) for normalized z = r + j x.
fn z_to_gamma(r: f64, x: f64) -> (f64, f64) {
    let zr = r - 1.0;
    let zi = x;
    let dr = r + 1.0;
    let di = x;
    let den = dr * dr + di * di;
    if den < 1e-14 {
        return (1.0, 0.0);
    }
    let re = (zr * dr + zi * di) / den;
    let im = (zi * dr - zr * di) / den;
    (re, im)
}

/// z = (1 + Gamma) / (1 - Gamma).
fn gamma_to_z(gr: f64, gi: f64) -> (f64, f64) {
    let nr = 1.0 + gr;
    let ni = gi;
    let dr = 1.0 - gr;
    let di = -gi;
    let den = dr * dr + di * di;
    if den < 1e-14 {
        return (1.0e6, 0.0);
    }
    let r = (nr * dr + ni * di) / den;
    let x = (ni * dr - nr * di) / den;
    (r, x)
}

/// Rotate Gamma clockwise in the chart (toward the generator on a lossless line).
/// One full phase lap is half a wavelength on the line.
fn rotate_gamma(gr: f64, gi: f64, t: f64) -> (f64, f64) {
    let angle = -std::f64::consts::TAU * phase_unit(t);
    let c = angle.cos();
    let s = angle.sin();
    (gr * c - gi * s, gr * s + gi * c)
}

fn gamma_mag(gr: f64, gi: f64) -> f64 {
    (gr * gr + gi * gi).sqrt()
}

/// Map Gamma (real, imag) into normalized surface coordinates.
fn gamma_to_uv(gr: f64, gi: f64) -> (f64, f64) {
    (CX + gr * RAD, CY - gi * RAD)
}

/// Inverse of [`gamma_to_uv`], clamped to the closed unit disk.
fn uv_to_gamma(u: f64, v: f64) -> (f64, f64) {
    let mut gr = (u - CX) / RAD;
    let mut gi = (CY - v) / RAD;
    let m = gamma_mag(gr, gi);
    if m > 1.0 {
        gr /= m;
        gi /= m;
    }
    (gr, gi)
}

/// Ambient or hand-placed load Gamma (load plane, before line rotation).
fn load_gamma(hand: Option<(f64, f64)>, seed: u64) -> (f64, f64) {
    if let Some((u, v)) = hand {
        return uv_to_gamma(u, v);
    }
    let (r, x) = default_load(seed);
    z_to_gamma(r, x)
}

fn working_point(hand: Option<(f64, f64)>, t: f64, seed: u64) -> (f64, f64, f64, f64) {
    let (gr0, gi0) = load_gamma(hand, seed);
    let (gr, gi) = rotate_gamma(gr0, gi0, t);
    let (r, x) = gamma_to_z(gr, gi);
    (gr, gi, r, x)
}

/// Compact status line: |Gamma|, normalized z, electrical angle, and a tag.
fn format_status(hand: Option<(f64, f64)>, t: f64, seed: u64, invite: bool) -> String {
    let (gr, gi, r, x) = working_point(hand, t, seed);
    let g = gamma_mag(gr, gi);
    let deg = (phase_unit(t) * 360.0).round() as i32;
    let tag = if g < 0.08 {
        "MATCHED"
    } else if (r - 1.0).abs() < 0.08 {
        "CANCEL-X"
    } else if invite {
        "DRAG:LOAD"
    } else {
        "MISMATCH"
    };
    let r = r.clamp(-9.9, 9.9);
    let x = x.clamp(-9.9, 9.9);
    format!("|G|={g:.2}  z={r:.2}{x:+.2}j  l={deg}  {tag}")
}

fn plot_uv(canvas: &mut dyn Surface, u: f64, v: f64, ch: char) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let px = (u.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32;
    let py = (v.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as i32;
    canvas.plot(px, py, ch);
}

fn line_uv(canvas: &mut dyn Surface, u0: f64, v0: f64, u1: f64, v1: f64, ch: char) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let x0 = (u0.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32;
    let y0 = (v0.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as i32;
    let x1 = (u1.clamp(0.0, 1.0) * width.saturating_sub(1) as f64).round() as i32;
    let y1 = (v1.clamp(0.0, 1.0) * height.saturating_sub(1) as f64).round() as i32;
    canvas.line(x0, y0, x1, y1, ch);
}

/// Draw a circle specified in the Gamma plane (optional clip to the unit disk).
fn draw_gamma_circle(
    canvas: &mut dyn Surface,
    c_re: f64,
    c_im: f64,
    rad_g: f64,
    steps: usize,
    ch: char,
    clip_unit: bool,
) {
    if rad_g <= 0.0 || !rad_g.is_finite() || steps < 8 {
        return;
    }
    let mut prev: Option<(f64, f64)> = None;
    for s in 0..=steps {
        let a = std::f64::consts::TAU * s as f64 / steps as f64;
        let gr = c_re + rad_g * a.cos();
        let gi = c_im + rad_g * a.sin();
        if clip_unit && gamma_mag(gr, gi) > 1.002 {
            prev = None;
            continue;
        }
        let (u, v) = gamma_to_uv(gr, gi);
        if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
            prev = None;
            continue;
        }
        if let Some((pu, pv)) = prev {
            line_uv(canvas, pu, pv, u, v, ch);
        }
        prev = Some((u, v));
    }
}

/// Draw a constant-resistance circle (center and radius in the Gamma plane).
fn draw_r_circle(canvas: &mut dyn Surface, r: f64, ch: char) {
    if r < 0.0 || !r.is_finite() {
        return;
    }
    let c_re = r / (r + 1.0);
    let rad_g = 1.0 / (r + 1.0);
    draw_gamma_circle(canvas, c_re, 0.0, rad_g, 72, ch, true);
}

/// Draw a constant-reactance arc (Gamma-plane circle, clipped to the unit disk).
fn draw_x_arc(canvas: &mut dyn Surface, x: f64, ch: char) {
    if x.abs() < 1e-9 || !x.is_finite() {
        return;
    }
    let c_im = 1.0 / x;
    let rad_g = c_im.abs();
    draw_gamma_circle(canvas, 1.0, c_im, rad_g, 96, ch, true);
}

fn draw(canvas: &mut dyn Surface, hand: Option<(f64, f64)>, t: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }

    // Outer |Gamma| = 1 rim and real-axis diameter.
    draw_gamma_circle(canvas, 0.0, 0.0, 1.0, 96, '#', false);
    let (u_left, v_axis) = gamma_to_uv(-1.0, 0.0);
    let (u_right, _) = gamma_to_uv(1.0, 0.0);
    line_uv(canvas, u_left, v_axis, u_right, v_axis, '-');
    // Short / open markers: left short (Z=0), right open (Z=inf).
    plot_uv(canvas, u_left, v_axis, 'S');
    plot_uv(canvas, u_right, v_axis, 'O');
    // Perfect match at the center.
    plot_uv(canvas, CX, CY, '+');

    // Constant-resistance family.
    for &r in &[0.2_f64, 0.5, 1.0, 2.0, 5.0] {
        let ch = if (r - 1.0).abs() < 1e-9 { '=' } else { '.' };
        draw_r_circle(canvas, r, ch);
    }
    // Constant-reactance family (inductive above, capacitive below).
    for &x in &[0.2_f64, 0.5, 1.0, 2.0, 5.0] {
        draw_x_arc(canvas, x, ':');
        draw_x_arc(canvas, -x, ':');
    }

    let (gr0, gi0) = load_gamma(hand, seed);
    let mag0 = gamma_mag(gr0, gi0).min(1.0);
    // Constant-|Gamma| orbit: pure rotation along a lossless line.
    if mag0 > 0.02 {
        draw_gamma_circle(canvas, 0.0, 0.0, mag0, 64, '*', false);
    }

    // Load marker (before line) and working point (after electrical length).
    let (u0, v0) = gamma_to_uv(gr0, gi0);
    plot_uv(canvas, u0, v0, 'L');
    let (gr, gi, _, _) = working_point(hand, t, seed);
    let (uw, vw) = gamma_to_uv(gr, gi);
    plot_uv(canvas, uw, vw, 'o');
    // Short radial tick so the moving point reads as a bead on the orbit.
    if mag0 > 0.05 {
        let (ux, vx) = gamma_to_uv(gr * 0.92, gi * 0.92);
        line_uv(canvas, ux, vx, uw, vw, '+');
    }
}

/// The Scariest Chart: Smith chart impedance matching.
#[derive(Debug, Default)]
pub struct SmithChart {
    seed: u64,
}

impl SmithChart {
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

impl Room for SmithChart {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "smith-chart",
            title: "The Scariest Chart",
            wing: "Waves & Sound",
            blurb: "Smith chart: the infinite impedance plane folded into a unit \
                    circle of reflection. Phase walks the line; DRAG: PLACE LOAD.",
            accent: [60, 200, 180],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, None, t, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.18
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "smith chart",
            root: 392.00,
            tempo: 88,
            line: &[0, 7, 12, 7, 5, 12, 7, 0],
            encodes: "constant |Gamma| rotates; match is the chart center",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: PLACE LOAD")
    }

    fn goal(&self) -> Option<&'static str> {
        Some("Land the moving bead on the unit-resistance ring (r=1).")
    }

    fn goal_met(&self, t: f64, inputs: &[RoomInput]) -> bool {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        let hand = hands.last().copied();
        let (_, _, r, _) = working_point(hand, t, self.seed);
        (r - 1.0).abs() < 0.08
    }

    fn status(&self, t: f64) -> Option<String> {
        // Ambient: phase is the line length. Tag the unit-R landing so the
        // invite is honest when the bead already sits on the goal ring.
        Some(format_status(None, t, self.seed, true))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        draw(canvas, hands.last().copied(), t, self.seed);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        Some(format_status(hands.last().copied(), t, self.seed, false))
    }

    fn reveal(&self) -> &'static str {
        "The scary chart is a conformal map of the reflection coefficient. \
         Normalize impedance by Z0, then Gamma = (z-1)/(z+1) packs every \
         passive load into the unit disk. Constant R and X become circles; a \
         lossless line is pure rotation. Match is the center: Gamma = 0."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        // At most CUT_LEVELS.len() (3): each cut unlocks at LV 5 / 12 / 24.
        &[
            "Philip H. Smith at Bell Labs (1937 chart; 1939 Electronics paper) \
             built shortwave arrays when line lengths met the wavelength and \
             standing waves wasted power. Mizuhashi (Japan, 1937) and Volpert \
             (USSR, 1939) found the same graphical idea; WWII radar made \
             Smith's form the standard.",
            "A full trip around a constant-|Gamma| circle is half a wavelength \
             on the line: the standing-wave pattern repeats every lambda/2. \
             Rotation is pure electrical length on a lossless feed.",
            "A stub is a dangling open or short of precise length: pure \
             reactance made of the same cable you are matching. Parallel stubs \
             use the dual admittance chart. VNAs still draw this map because \
             you can see why a tweak moves the bead.",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SmithChart, default_load, gamma_mag, gamma_to_z, load_gamma, rotate_gamma, working_point,
        z_to_gamma,
    };
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = SmithChart::new().status(0.2).unwrap();
        assert!(s.contains("DRAG") || s.contains("LOAD") || s.contains("|G|"));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn status_tags_unit_r_landing() {
        let room = SmithChart::new();
        let mut tagged = false;
        for k in 0..64 {
            let t = k as f64 / 64.0;
            let s = room.status(t).unwrap();
            assert!(s.chars().count() <= 56, "status too long: {s}");
            if s.contains("CANCEL-X") || s.contains("MATCHED") {
                tagged = true;
                break;
            }
        }
        assert!(tagged, "status must name CANCEL-X when the bead hits r=1");
    }

    #[test]
    fn drag_changes_status() {
        let r = SmithChart::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.72,
                    y: 0.40,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
        assert!(a.contains("|G|") || a.contains("z="));
    }

    #[test]
    fn z_gamma_round_trip() {
        let (r, x) = (0.4, 0.7);
        let (gr, gi) = z_to_gamma(r, x);
        let (r2, x2) = gamma_to_z(gr, gi);
        assert!((r2 - r).abs() < 1e-9);
        assert!((x2 - x).abs() < 1e-9);
    }

    #[test]
    fn match_is_chart_center() {
        let (gr, gi) = z_to_gamma(1.0, 0.0);
        assert!(gamma_mag(gr, gi) < 1e-9);
    }

    #[test]
    fn short_and_open_sit_on_the_rim() {
        let (gs, _) = z_to_gamma(0.0, 0.0);
        assert!((gs + 1.0).abs() < 1e-9);
        let (go, gi) = z_to_gamma(1.0e6, 0.0);
        assert!((go - 1.0).abs() < 1e-4);
        assert!(gi.abs() < 1e-4);
    }

    #[test]
    fn rotation_preserves_magnitude() {
        let (gr, gi) = z_to_gamma(0.3, 0.5);
        let m0 = gamma_mag(gr, gi);
        let (a, b) = rotate_gamma(gr, gi, 0.37);
        assert!((gamma_mag(a, b) - m0).abs() < 1e-9);
    }

    #[test]
    fn full_phase_lap_is_identity() {
        let (gr, gi) = load_gamma(None, 0);
        let (a, b) = rotate_gamma(gr, gi, 0.0);
        let (c, d) = rotate_gamma(gr, gi, 1.0);
        // phase_unit clamps 1.0 to 1.0, angle = -2pi, identity.
        assert!((a - c).abs() < 1e-9);
        assert!((b - d).abs() < 1e-9);
    }

    #[test]
    fn ambient_bead_moves() {
        let r = SmithChart::new();
        let mut a = Canvas::new(64, 48);
        let mut b = Canvas::new(64, 48);
        r.render(&mut a, 0.0);
        r.render(&mut b, 0.25);
        assert_ne!(a.to_text(), b.to_text(), "working point must orbit");
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(64, 48);
        SmithChart::new().render(&mut c, 0.18);
        assert!(c.ink_count() > 40);
    }

    #[test]
    fn goal_can_land_on_unit_r() {
        // Search a few hand placements and phases for a unit-resistance hit.
        let room = SmithChart::new();
        let mut found = false;
        for xi in 0..12 {
            for yi in 0..12 {
                let x = 0.15 + xi as f64 * 0.06;
                let y = 0.15 + yi as f64 * 0.06;
                for k in 0..16 {
                    let t = k as f64 / 16.0;
                    let inputs = [RoomInput::PointerDown { x, y, t: 0.0 }];
                    if room.goal_met(t, &inputs) {
                        found = true;
                        break;
                    }
                }
                if found {
                    break;
                }
            }
            if found {
                break;
            }
        }
        assert!(found, "some load and line length should hit r=1");
    }

    #[test]
    fn working_point_reports_finite_z() {
        let (_, _, r, x) = working_point(None, 0.3, 0);
        assert!(r.is_finite() && x.is_finite());
        let (dr, dx) = default_load(0);
        assert!(dr > 0.0);
        assert!(dx.abs() > 0.0);
    }

    #[test]
    fn motif_ok() {
        assert!(SmithChart::new().motif().unwrap().line.len() >= 6);
    }

    #[test]
    fn variation_changes_load() {
        let a = SmithChart::new_with(0).status(0.1).unwrap();
        let b = SmithChart::new_with(11).status(0.1).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn deep_cuts_fit_the_unlock_road() {
        assert!(
            SmithChart::new().deep_cuts().len() <= crate::journey::CUT_LEVELS.len(),
            "deep cuts must unlock on the shared LV 5/12/24 road"
        );
    }

    #[test]
    fn plate_is_not_phase_thin() {
        let room = SmithChart::new();
        let mut worst = usize::MAX;
        for t in [0.0_f64, 0.25, 0.5, 0.75, 1.0] {
            let mut c = Canvas::new(120, 70);
            room.render(&mut c, t);
            worst = worst.min(c.ink_count());
        }
        assert!(
            worst >= 80,
            "worst phase ink {worst} is under the 80-ink phase-thin bar"
        );
    }

    #[test]
    fn hand_moves_the_domain() {
        let room = SmithChart::new();
        let mut base = Canvas::new(120, 70);
        let mut poked = Canvas::new(120, 70);
        room.render(&mut base, 0.3);
        room.render_poked(&mut poked, 0.3, &[(0.28, 0.38)]);
        assert_ne!(
            base.to_text(),
            poked.to_text(),
            "placing a load must change the plate"
        );
    }

    #[test]
    fn dial_is_not_dead() {
        let room = SmithChart::new();
        let mut left = Canvas::new(120, 70);
        let mut right = Canvas::new(120, 70);
        room.render_poked(&mut left, 0.3, &[(0.22, 0.52)]);
        room.render_poked(&mut right, 0.3, &[(0.78, 0.52)]);
        assert_ne!(
            left.to_text(),
            right.to_text(),
            "left and right loads must differ"
        );
    }

    #[test]
    fn passive_disk_keeps_nonnegative_resistance() {
        // Every hand point maps into |Gamma| <= 1, so Re(z) >= 0.
        for xi in 0..11 {
            for yi in 0..11 {
                let u = xi as f64 / 10.0;
                let v = yi as f64 / 10.0;
                let (_, _, r, _) = working_point(Some((u, v)), 0.1, 0);
                assert!(
                    r >= -1e-9,
                    "active-looking r={r} at hand ({u},{v}) should stay passive"
                );
            }
        }
    }

    #[test]
    fn ambient_line_walk_can_hit_unit_r() {
        let room = SmithChart::new();
        let mut hit = false;
        for k in 0..64 {
            let t = k as f64 / 64.0;
            if room.goal_met(t, &[]) {
                hit = true;
                break;
            }
        }
        assert!(
            hit,
            "default load must cross r=1 while phase walks the orbit"
        );
    }
}

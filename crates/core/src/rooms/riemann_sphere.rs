//! The Riemann sphere: the complex plane compactified by one point at infinity.
//!
//! Stereographic projection from the north pole maps the unit sphere (minus that
//! pole) onto the complex plane. The south pole is z = 0, the equator is the
//! unit circle, and the north pole is infinity. DRAG: PLACE z on the plane; the
//! bead lifts onto the sphere. Phase walks ambient |z| toward infinity. See
//! `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

/// Half-width of the complex plane window in surface coordinates (around center).
const PLANE_HALF: f64 = 0.42;
/// Complex half-span mapped into that window: hand at the rim is about |z| = SPAN.
/// Chosen so a drag to the drawn plane edge can earn INF (north pole).
const SPAN: f64 = 4.0;
/// Sphere drawing radius in normalized surface coordinates.
const SPHERE_RAD: f64 = 0.28;
/// Sphere center on the plate (above the plane strip).
const SCX: f64 = 0.50;
const SCY: f64 = 0.38;
/// Complex plane center (below the sphere).
const PCX: f64 = 0.50;
const PCY: f64 = 0.78;
/// View tilt for the sphere projection (radians).
const TILT: f64 = 0.55;
/// |z| at which the lift counts as the north pole (goal).
/// Must sit inside the drawn plane window so DRAG to the rim can earn INF.
const INF_R: f64 = 3.2;
/// Z coordinate threshold matching INF_R on the unit sphere (~0.82 at |z|=3.2).
const INF_Z: f64 = 0.80;

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

/// Ambient deal on the complex plane (before phase walks |z| out).
fn default_z(seed: u64) -> (f64, f64) {
    if seed == 0 {
        return (0.55, 0.40);
    }
    let re = 0.20 + (seed % 9) as f64 * 0.14;
    let im = 0.15 + ((seed / 9) % 7) as f64 * 0.12;
    let im = if seed.is_multiple_of(2) { im } else { -im };
    (re.clamp(0.12, 1.4), im.clamp(-1.2, 1.2))
}

/// Map plate UV into the complex plane window.
fn uv_to_z(u: f64, v: f64) -> (f64, f64) {
    let re = ((u - PCX) / PLANE_HALF) * SPAN;
    let im = ((PCY - v) / PLANE_HALF) * SPAN;
    (re, im)
}

/// Map complex z into the plane strip (clamped to the drawn window).
fn z_to_plane_uv(re: f64, im: f64) -> (f64, f64) {
    let u = PCX + (re / SPAN) * PLANE_HALF;
    let v = PCY - (im / SPAN) * PLANE_HALF;
    (u.clamp(0.02, 0.98), v.clamp(0.58, 0.96))
}

/// Inverse stereographic: complex z lifts to the unit sphere (X, Y, Z).
/// North pole is the point at infinity; south pole is z = 0.
fn lift(re: f64, im: f64) -> (f64, f64, f64) {
    let r2 = re * re + im * im;
    let den = 1.0 + r2;
    if !den.is_finite() || den < 1e-18 {
        return (0.0, 0.0, 1.0);
    }
    let x = 2.0 * re / den;
    let y = 2.0 * im / den;
    let z = (r2 - 1.0) / den;
    (x, y, z)
}

fn mag(re: f64, im: f64) -> f64 {
    (re * re + im * im).sqrt()
}

/// Working complex point: hand places z; ambient walks |z| outward with phase.
fn working_z(hand: Option<(f64, f64)>, t: f64, seed: u64) -> (f64, f64) {
    if let Some((u, v)) = hand {
        return uv_to_z(u, v);
    }
    let (re0, im0) = default_z(seed);
    let m0 = mag(re0, im0).max(1e-9);
    // Phase multiplies radius so the ambient bead climbs toward infinity.
    let grow = 1.0 + phase_unit(t) * (INF_R * 1.35 / m0);
    let arg = im0.atan2(re0) + phase_unit(t) * std::f64::consts::TAU * 0.35;
    let m = m0 * grow;
    (m * arg.cos(), m * arg.sin())
}

fn is_inf(re: f64, im: f64) -> bool {
    let (_, _, z) = lift(re, im);
    mag(re, im) >= INF_R || z >= INF_Z
}

fn is_zero(re: f64, im: f64) -> bool {
    mag(re, im) < 0.12
}

fn is_unit(re: f64, im: f64) -> bool {
    (mag(re, im) - 1.0).abs() < 0.10
}

fn format_status(hand: Option<(f64, f64)>, t: f64, seed: u64, invite: bool) -> String {
    let (re, im) = working_z(hand, t, seed);
    let m = mag(re, im);
    let tag = if is_inf(re, im) {
        "INF"
    } else if is_zero(re, im) {
        "ZERO"
    } else if is_unit(re, im) {
        "UNIT"
    } else if invite {
        "DRAG:z"
    } else {
        "C"
    };
    // Clamp display so the line stays readable and <= 56 chars.
    let re_d = re.clamp(-99.0, 99.0);
    let im_d = im.clamp(-99.0, 99.0);
    let m_d = m.min(99.0);
    if m_d >= 10.0 {
        format!("|z|={m_d:.1}  z={re_d:.0}{im_d:+.0}i  {tag}")
    } else {
        format!("|z|={m_d:.2}  z={re_d:.1}{im_d:+.1}i  {tag}")
    }
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

/// Project sphere coordinates (unit ball) to plate UV with azimuth and tilt.
fn sphere_to_uv(x: f64, y: f64, z: f64, az: f64) -> (f64, f64) {
    let c = az.cos();
    let s = az.sin();
    let xr = x * c - y * s;
    let yr = x * s + y * c;
    // Tilt so the north pole reads above the equator.
    let yt = yr * TILT.cos() - z * TILT.sin();
    let zt = yr * TILT.sin() + z * TILT.cos();
    // Orthographic-ish: ignore depth for outline clarity; scale by sphere radius.
    let u = SCX + xr * SPHERE_RAD;
    let v = SCY - zt * SPHERE_RAD;
    // Mild foreshortening from yt so the back stays readable as a dim cue.
    let _ = yt;
    (u, v)
}

fn draw_sphere_wire(canvas: &mut dyn Surface, az: f64) {
    // Equator and a few parallels.
    for &lat in &[-0.6_f64, -0.3, 0.0, 0.3, 0.6] {
        let z = lat;
        let r = (1.0 - z * z).sqrt();
        if !r.is_finite() || r < 0.05 {
            continue;
        }
        let ch = if lat.abs() < 1e-9 { '=' } else { '.' };
        let mut prev: Option<(f64, f64)> = None;
        for s in 0..=48 {
            let a = std::f64::consts::TAU * s as f64 / 48.0;
            let (u, v) = sphere_to_uv(r * a.cos(), r * a.sin(), z, az);
            if let Some((pu, pv)) = prev {
                line_uv(canvas, pu, pv, u, v, ch);
            }
            prev = Some((u, v));
        }
    }
    // Meridians.
    for k in 0..8 {
        let long = std::f64::consts::TAU * k as f64 / 8.0;
        let mut prev: Option<(f64, f64)> = None;
        for s in 0..=32 {
            let lat = -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * s as f64 / 32.0;
            let cl = lat.cos();
            let x = cl * long.cos();
            let y = cl * long.sin();
            let z = lat.sin();
            let (u, v) = sphere_to_uv(x, y, z, az);
            if let Some((pu, pv)) = prev {
                line_uv(canvas, pu, pv, u, v, ':');
            }
            prev = Some((u, v));
        }
    }
    // Poles: south = 0, north = infinity.
    let (us, vs) = sphere_to_uv(0.0, 0.0, -1.0, az);
    let (un, vn) = sphere_to_uv(0.0, 0.0, 1.0, az);
    plot_uv(canvas, us, vs, '0');
    plot_uv(canvas, un, vn, 'N');
}

fn draw_complex_plane(canvas: &mut dyn Surface) {
    // Axes through the origin of the plane window.
    let (u0, v_axis) = z_to_plane_uv(-SPAN, 0.0);
    let (u1, _) = z_to_plane_uv(SPAN, 0.0);
    line_uv(canvas, u0, v_axis, u1, v_axis, '-');
    let (u_im, v0) = z_to_plane_uv(0.0, -SPAN);
    let (_, v1) = z_to_plane_uv(0.0, SPAN);
    line_uv(canvas, u_im, v0, u_im, v1, '|');
    // Unit circle |z| = 1.
    let mut prev: Option<(f64, f64)> = None;
    for s in 0..=48 {
        let a = std::f64::consts::TAU * s as f64 / 48.0;
        let (u, v) = z_to_plane_uv(a.cos(), a.sin());
        if let Some((pu, pv)) = prev {
            line_uv(canvas, pu, pv, u, v, '*');
        }
        prev = Some((u, v));
    }
    plot_uv(canvas, PCX, PCY, '+');
}

fn draw(canvas: &mut dyn Surface, hand: Option<(f64, f64)>, t: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }

    let az = phase_unit(t) * std::f64::consts::TAU * 0.5
        + if seed == 0 {
            0.0
        } else {
            (seed % 17) as f64 * 0.11
        };

    draw_sphere_wire(canvas, az);
    draw_complex_plane(canvas);

    let (re, im) = working_z(hand, t, seed);
    let (x, y, z) = lift(re, im);

    // Lifted bead on the sphere.
    let (us, vs) = sphere_to_uv(x, y, z, az);
    plot_uv(canvas, us, vs, 'o');

    // Plane image of the same point (clamped into the window when far out).
    let (up, vp) = z_to_plane_uv(re, im);
    plot_uv(canvas, up, vp, 'z');

    // Chord from south pole toward the lift, so the climb reads as motion.
    let (u0, v0) = sphere_to_uv(0.0, 0.0, -1.0, az);
    line_uv(canvas, u0, v0, us, vs, '+');

    // When near infinity, mark the north pole brightly.
    if is_inf(re, im) {
        let (un, vn) = sphere_to_uv(0.0, 0.0, 1.0, az);
        plot_uv(canvas, un, vn, '@');
    }
}

/// The Riemann sphere: complex plane plus infinity as a compact surface.
#[derive(Debug, Default)]
pub struct RiemannSphere {
    seed: u64,
}

impl RiemannSphere {
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

impl Room for RiemannSphere {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "riemann-sphere",
            title: "Riemann Sphere",
            wing: "Shape & Space",
            blurb: "One sphere holds every complex number and infinity. \
                    Stereographic projection; DRAG: PLACE z; phase walks |z| out.",
            accent: [180, 140, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, None, t, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.22
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "riemann sphere",
            root: 329.63,
            tempo: 76,
            line: &[0, 5, 12, 17, 12, 7, 5, 0],
            encodes: "plane plus infinity closes as one sphere",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: PLACE z")
    }

    fn goal(&self) -> Option<&'static str> {
        Some("Send the lifted bead to the north pole (infinity).")
    }

    fn goal_met(&self, t: f64, inputs: &[RoomInput]) -> bool {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        let (re, im) = working_z(hands.last().copied(), t, self.seed);
        is_inf(re, im)
    }

    fn status(&self, t: f64) -> Option<String> {
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
        "The Riemann sphere is the complex plane with one point at infinity. \
         Stereographic projection from the north pole maps the sphere minus \
         that pole onto C. Circles through the pole become straight lines; \
         every other circle stays a circle. One compact surface holds all of C."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Bernhard Riemann made the plane-plus-infinity into a genuine \
             surface so meromorphic functions become maps of a compact world. \
             Stereographic projection was already used in maps and astronomy; \
             he gave complex analysis its natural domain.",
            "Under stereographic projection, circles through the north pole \
             become straight lines on the plane, and every other circle stays \
             a circle. Mobius maps of C union infinity are exactly the \
             conformal automorphisms of the sphere.",
            "The Bloch sphere of a qubit is the same picture with different \
             labels: pure states live on the surface, |0> and |1> are poles. \
             The celestial sphere and crystal-ball projections are cousins.",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{
        INF_R, INF_Z, RiemannSphere, default_z, is_inf, is_unit, is_zero, lift, mag, working_z,
    };
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    /// Stereographic projection from the north pole (inverse of [`super::lift`]).
    fn project_sphere(x: f64, y: f64, z: f64) -> (f64, f64) {
        let den = 1.0 - z;
        if den.abs() < 1e-12 {
            return (0.0, 0.0);
        }
        (x / den, y / den)
    }

    #[test]
    fn status_invites() {
        let s = RiemannSphere::new().status(0.2).unwrap();
        assert!(s.contains("DRAG") || s.contains("|z|") || s.contains("z="));
        assert!(s.chars().count() <= 56, "status too long: {s}");
    }

    #[test]
    fn status_tags_inf_on_long_walk() {
        let room = RiemannSphere::new();
        let mut tagged = false;
        for k in 0..64 {
            let t = k as f64 / 64.0;
            let s = room.status(t).unwrap();
            assert!(s.chars().count() <= 56, "status too long: {s}");
            if s.contains("INF") {
                tagged = true;
                break;
            }
        }
        assert!(tagged, "ambient phase walk must reach INF");
    }

    #[test]
    fn drag_changes_status() {
        let r = RiemannSphere::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.85,
                    y: 0.70,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
        assert!(a.contains("|z|") || a.contains("z="));
    }

    #[test]
    fn lift_south_pole_is_zero() {
        let (x, y, z) = lift(0.0, 0.0);
        assert!(x.abs() < 1e-12);
        assert!(y.abs() < 1e-12);
        assert!((z + 1.0).abs() < 1e-12);
    }

    #[test]
    fn lift_unit_circle_is_equator() {
        let (x, y, z) = lift(1.0, 0.0);
        assert!((x - 1.0).abs() < 1e-12);
        assert!(y.abs() < 1e-12);
        assert!(z.abs() < 1e-12);
        let (x2, y2, z2) = lift(0.0, 1.0);
        assert!(x2.abs() < 1e-12);
        assert!((y2 - 1.0).abs() < 1e-12);
        assert!(z2.abs() < 1e-12);
    }

    #[test]
    fn large_z_approaches_north_pole() {
        let (_, _, z) = lift(INF_R, 0.0);
        assert!(z >= INF_Z - 1e-9, "lift Z={z} should meet INF_Z={INF_Z}");
        assert!(is_inf(INF_R, 0.0));
        let (_, _, z_far) = lift(20.0, 0.0);
        assert!(z_far > 0.99);
    }

    #[test]
    fn stereographic_round_trip() {
        let (re, im) = (1.3, -0.7);
        let (x, y, z) = lift(re, im);
        let (re2, im2) = project_sphere(x, y, z);
        assert!((re2 - re).abs() < 1e-9);
        assert!((im2 - im).abs() < 1e-9);
    }

    #[test]
    fn unit_sphere_norm() {
        for &(re, im) in &[(0.0, 0.0), (1.0, 0.0), (0.5, -0.5), (3.0, 2.0)] {
            let (x, y, z) = lift(re, im);
            let n = (x * x + y * y + z * z).sqrt();
            assert!((n - 1.0).abs() < 1e-9, "norm {n} at z={re}+{im}i");
        }
    }

    #[test]
    fn ambient_bead_moves() {
        let r = RiemannSphere::new();
        let mut a = Canvas::new(64, 48);
        let mut b = Canvas::new(64, 48);
        r.render(&mut a, 0.0);
        r.render(&mut b, 0.4);
        assert_ne!(a.to_text(), b.to_text(), "phase must move the bead or view");
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(64, 48);
        RiemannSphere::new().render(&mut c, 0.22);
        assert!(c.ink_count() > 40);
    }

    #[test]
    fn ambient_walk_hits_infinity() {
        let room = RiemannSphere::new();
        let mut hit = false;
        for k in 0..64 {
            let t = k as f64 / 64.0;
            if room.goal_met(t, &[]) {
                hit = true;
                break;
            }
        }
        assert!(hit, "default ambient walk must reach the north pole");
    }

    #[test]
    fn hand_on_plane_rim_hits_infinity() {
        let room = RiemannSphere::new();
        // Right rim of the drawn complex plane (u = PCX + PLANE_HALF).
        let inputs = [RoomInput::PointerDown {
            x: 0.92,
            y: 0.78,
            t: 0.0,
        }];
        assert!(
            room.goal_met(0.0, &inputs),
            "drag to the plane rim must earn the north-pole goal"
        );
        assert!(is_inf(INF_R, 0.0));
    }

    #[test]
    fn zero_and_unit_tags() {
        assert!(is_zero(0.0, 0.0));
        assert!(is_unit(1.0, 0.0));
        assert!(!is_inf(1.0, 0.0));
    }

    #[test]
    fn motif_ok() {
        assert!(RiemannSphere::new().motif().unwrap().line.len() >= 6);
    }

    #[test]
    fn variation_changes_deal() {
        let a = RiemannSphere::new_with(0).status(0.05).unwrap();
        let b = RiemannSphere::new_with(11).status(0.05).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn deep_cuts_fit_the_unlock_road() {
        assert!(
            RiemannSphere::new().deep_cuts().len() <= crate::journey::CUT_LEVELS.len(),
            "deep cuts must unlock on the shared LV 5/12/24 road"
        );
    }

    #[test]
    fn plate_is_not_phase_thin() {
        let room = RiemannSphere::new();
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
        let room = RiemannSphere::new();
        let mut base = Canvas::new(120, 70);
        let mut poked = Canvas::new(120, 70);
        room.render(&mut base, 0.3);
        room.render_poked(&mut poked, 0.3, &[(0.70, 0.72)]);
        assert_ne!(
            base.to_text(),
            poked.to_text(),
            "placing z must change the plate"
        );
    }

    #[test]
    fn dial_is_not_dead() {
        let room = RiemannSphere::new();
        let mut left = Canvas::new(120, 70);
        let mut right = Canvas::new(120, 70);
        room.render_poked(&mut left, 0.3, &[(0.30, 0.78)]);
        room.render_poked(&mut right, 0.3, &[(0.80, 0.78)]);
        assert_ne!(
            left.to_text(),
            right.to_text(),
            "left and right z must differ"
        );
    }

    #[test]
    fn working_point_finite() {
        let (re, im) = working_z(None, 0.3, 0);
        assert!(re.is_finite() && im.is_finite());
        let (dr, di) = default_z(0);
        assert!(mag(dr, di) > 0.0);
    }

    #[test]
    fn status_length_all_phases() {
        let room = RiemannSphere::new();
        for k in 0..20 {
            let t = k as f64 / 20.0;
            let s = room.status(t).unwrap();
            assert!(s.chars().count() <= 56, "ambient status too long: {s}");
            let s2 = room
                .status_input(
                    t,
                    &[RoomInput::PointerDown {
                        x: 0.2 + t * 0.5,
                        y: 0.65,
                        t: 0.0,
                    }],
                )
                .unwrap();
            assert!(s2.chars().count() <= 56, "hand status too long: {s2}");
        }
    }
}

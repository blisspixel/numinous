//! The Bloch sphere: pure quantum states as points on a unit sphere.
//!
//! Every pure state of a qubit is a direction on S^2. The north pole is |0>,
//! the south pole is |1>, and the equator is equal superpositions with a free
//! relative phase. Stereographic labels from the north pole recover a complex
//! coordinate for the pure-state ray. DRAG: PLACE STATE. Phase precesses around
//! the Z axis (Larmor-style). See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

/// Sphere center and radius in normalized surface coordinates.
const CX: f64 = 0.50;
const CY: f64 = 0.50;
const RAD: f64 = 0.38;
const TILT: f64 = 0.62;
/// Goal band: |cos theta| small means near the equator (equal |0>/|1| weight).
const EQUATOR_COS: f64 = 0.12;

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

/// Default pure state (theta, phi) in radians before phase precession.
fn default_angles(seed: u64) -> (f64, f64) {
    if seed == 0 {
        return (0.85, 0.40);
    }
    let theta = 0.35 + (seed % 11) as f64 * 0.22;
    let phi = (seed % 17) as f64 * 0.37;
    (theta.clamp(0.15, std::f64::consts::PI - 0.15), phi)
}

/// Map plate UV to a unit direction on the sphere (orthographic pick).
fn uv_to_state(u: f64, v: f64) -> (f64, f64, f64) {
    let mut x = (u - CX) / RAD;
    let mut z = (CY - v) / RAD;
    // Undo a mild tilt so vertical on plate is not pure Z.
    let yt = 0.0;
    let zr = z * TILT.cos() + yt * TILT.sin();
    let yr = -z * TILT.sin() + yt * TILT.cos();
    let mut y = yr;
    z = zr;
    let m = (x * x + y * y + z * z).sqrt();
    if m < 1e-9 || !m.is_finite() {
        return (0.0, 0.0, 1.0);
    }
    if m > 1.0 {
        x /= m;
        y /= m;
        z /= m;
    } else {
        // Project disk interior onto the front hemisphere (positive Y-ish).
        let h = (1.0 - x * x - z * z).max(0.0).sqrt();
        y = if y >= 0.0 { h } else { -h };
        let m2 = (x * x + y * y + z * z).sqrt();
        if m2 > 1e-12 {
            x /= m2;
            y /= m2;
            z /= m2;
        }
    }
    (x, y, z)
}

/// Spherical angles: theta from +Z (0 = |0>), phi around Z.
fn cart_to_angles(x: f64, y: f64, z: f64) -> (f64, f64) {
    let theta = z.clamp(-1.0, 1.0).acos();
    let phi = y.atan2(x);
    (theta, phi)
}

fn angles_to_cart(theta: f64, phi: f64) -> (f64, f64, f64) {
    let st = theta.sin();
    (st * phi.cos(), st * phi.sin(), theta.cos())
}

/// Working pure state after hand or ambient deal + phase precession about Z.
fn working_state(hand: Option<(f64, f64)>, t: f64, seed: u64) -> (f64, f64, f64, f64, f64) {
    let (theta0, phi0) = if let Some((u, v)) = hand {
        let (x, y, z) = uv_to_state(u, v);
        cart_to_angles(x, y, z)
    } else {
        default_angles(seed)
    };
    // Ambient: precess phi; hand: still precess so The Show moves.
    let phi = phi0 + phase_unit(t) * std::f64::consts::TAU;
    let (x, y, z) = angles_to_cart(theta0, phi);
    (x, y, z, theta0, phi)
}

fn p0(theta: f64) -> f64 {
    let c = (theta * 0.5).cos();
    c * c
}

fn is_equator(theta: f64) -> bool {
    theta.cos().abs() < EQUATOR_COS
}

fn is_north(theta: f64) -> bool {
    theta < 0.22
}

fn is_south(theta: f64) -> bool {
    theta > std::f64::consts::PI - 0.22
}

fn format_status(hand: Option<(f64, f64)>, t: f64, seed: u64, invite: bool) -> String {
    let (_, _, _, theta, phi) = working_state(hand, t, seed);
    let deg_t = (theta * 180.0 / std::f64::consts::PI).round() as i32;
    let deg_p = {
        let mut d = (phi * 180.0 / std::f64::consts::PI).round() as i32;
        d = ((d % 360) + 360) % 360;
        d
    };
    let p = p0(theta);
    let tag = if is_equator(theta) {
        "EQ"
    } else if is_north(theta) {
        "|0>"
    } else if is_south(theta) {
        "|1>"
    } else if invite {
        "DRAG:STATE"
    } else {
        "PSI"
    };
    // P0 is Born probability of |0>.
    format!("th={deg_t} ph={deg_p}  P0={p:.2}  {tag}")
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

fn sphere_to_uv(x: f64, y: f64, z: f64, az: f64) -> (f64, f64) {
    let c = az.cos();
    let s = az.sin();
    let xr = x * c - y * s;
    let yr = x * s + y * c;
    let yt = yr * TILT.cos() - z * TILT.sin();
    let zt = yr * TILT.sin() + z * TILT.cos();
    let _ = yt;
    (CX + xr * RAD, CY - zt * RAD)
}

fn draw_wire(canvas: &mut dyn Surface, az: f64) {
    // Equator and latitude rings.
    for &lat_z in &[-0.5_f64, 0.0, 0.5] {
        let r = (1.0 - lat_z * lat_z).sqrt();
        if !r.is_finite() || r < 0.05 {
            continue;
        }
        let ch = if lat_z.abs() < 1e-9 { '=' } else { '.' };
        let mut prev: Option<(f64, f64)> = None;
        for s in 0..=48 {
            let a = std::f64::consts::TAU * s as f64 / 48.0;
            let (u, v) = sphere_to_uv(r * a.cos(), r * a.sin(), lat_z, az);
            if let Some((pu, pv)) = prev {
                line_uv(canvas, pu, pv, u, v, ch);
            }
            prev = Some((u, v));
        }
    }
    // Meridians (longitude).
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
    // Poles |0> and |1>.
    let (un, vn) = sphere_to_uv(0.0, 0.0, 1.0, az);
    let (us, vs) = sphere_to_uv(0.0, 0.0, -1.0, az);
    plot_uv(canvas, un, vn, '0');
    plot_uv(canvas, us, vs, '1');
    // |+> and |-> marks on the equator for orientation.
    let (up, vp) = sphere_to_uv(1.0, 0.0, 0.0, az);
    let (um, vm) = sphere_to_uv(-1.0, 0.0, 0.0, az);
    plot_uv(canvas, up, vp, '+');
    plot_uv(canvas, um, vm, '-');
}

fn draw(canvas: &mut dyn Surface, hand: Option<(f64, f64)>, t: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let az = if seed == 0 {
        0.15
    } else {
        (seed % 19) as f64 * 0.09
    };

    draw_wire(canvas, az);

    let (x, y, z, theta, _) = working_state(hand, t, seed);
    let (u, v) = sphere_to_uv(x, y, z, az);
    // State vector from origin to the pure state.
    line_uv(canvas, CX, CY, u, v, '+');
    plot_uv(canvas, u, v, 'o');

    if is_equator(theta) {
        // Highlight the equator landing.
        let mut prev: Option<(f64, f64)> = None;
        for s in 0..=48 {
            let a = std::f64::consts::TAU * s as f64 / 48.0;
            let (ue, ve) = sphere_to_uv(a.cos(), a.sin(), 0.0, az);
            if let Some((pu, pv)) = prev {
                line_uv(canvas, pu, pv, ue, ve, '*');
            }
            prev = Some((ue, ve));
        }
    }
}

/// The Bloch sphere room: pure qubit states as a compact map.
#[derive(Debug, Default)]
pub struct BlochSphere {
    seed: u64,
}

impl BlochSphere {
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

impl Room for BlochSphere {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "bloch-sphere",
            title: "Bloch Sphere",
            wing: "Shape & Space",
            blurb: "Every pure qubit state is a point on a sphere. |0> and |1> \
                    are poles; the equator is equal superpositions. DRAG: PLACE STATE.",
            accent: [100, 200, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, None, t, self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.28
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "bloch sphere",
            root: 311.13,
            tempo: 80,
            line: &[0, 7, 12, 19, 12, 7, 5, 0],
            encodes: "pure states on a sphere; equator is equal weight",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: PLACE STATE")
    }

    fn goal(&self) -> Option<&'static str> {
        Some("Land the state on the equator (equal superposition).")
    }

    fn goal_met(&self, t: f64, inputs: &[RoomInput]) -> bool {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        let (_, _, _, theta, _) = working_state(hands.last().copied(), t, self.seed);
        is_equator(theta)
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
        "The Bloch sphere is the pure-state space of a single qubit. North is \
         |0>, south is |1>, and every other point is a coherent superposition. \
         Born probability for |0> is cos^2(theta/2). Relative phase is the \
         longitude. Unitary gates are rotations of this sphere."
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Felix Bloch's nuclear-spin picture and the geometric pure-state \
             sphere are cousins of the Riemann sphere: pure states are CP^1, \
             which is one complex coordinate plus infinity.",
            "Measurement along Z collapses to a pole with probabilities \
             cos^2(theta/2) and sin^2(theta/2). A state on the equator is \
             maximally uncertain in the Z basis and pure in a rotated basis.",
            "Gates are rotations: X, Y, Z are 180-degree turns; Hadamard sends \
             |0> to |+> on the equator. Mixed states fill the ball interior; \
             this room shows only the pure surface.",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BlochSphere, EQUATOR_COS, angles_to_cart, cart_to_angles, is_equator, is_north, is_south,
        p0, working_state,
    };
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = BlochSphere::new().status(0.2).unwrap();
        assert!(s.contains("DRAG") || s.contains("th=") || s.contains("P0"));
        assert!(s.chars().count() <= 56, "status too long: {s}");
    }

    #[test]
    fn status_length_all_phases() {
        let room = BlochSphere::new();
        for k in 0..20 {
            let t = k as f64 / 20.0;
            let s = room.status(t).unwrap();
            assert!(s.chars().count() <= 56, "{s}");
        }
    }

    #[test]
    fn drag_changes_status() {
        let r = BlochSphere::new();
        let o = r.status(0.2).unwrap();
        let a = r
            .status_input(
                0.2,
                &[RoomInput::PointerDown {
                    x: 0.70,
                    y: 0.50,
                    t: 0.0,
                }],
            )
            .unwrap();
        assert_ne!(o, a);
    }

    #[test]
    fn poles_and_equator() {
        assert!(is_north(0.0));
        assert!(is_south(std::f64::consts::PI));
        assert!(is_equator(std::f64::consts::FRAC_PI_2));
        assert!((p0(0.0) - 1.0).abs() < 1e-12);
        assert!((p0(std::f64::consts::PI) - 0.0).abs() < 1e-12);
        assert!((p0(std::f64::consts::FRAC_PI_2) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn angle_round_trip() {
        let (x, y, z) = angles_to_cart(1.1, 0.7);
        let (th, ph) = cart_to_angles(x, y, z);
        assert!((th - 1.1).abs() < 1e-9);
        assert!((ph - 0.7).abs() < 1e-9);
        let n = (x * x + y * y + z * z).sqrt();
        assert!((n - 1.0).abs() < 1e-9);
    }

    #[test]
    fn ambient_precesses() {
        let (_, _, _, th0, ph0) = working_state(None, 0.0, 0);
        let (_, _, _, th1, ph1) = working_state(None, 0.25, 0);
        assert!((th0 - th1).abs() < 1e-12, "precession keeps theta");
        assert!((ph0 - ph1).abs() > 0.1, "phi must move with phase");
    }

    #[test]
    fn hand_can_hit_equator() {
        let room = BlochSphere::new();
        // Mid-height of the plate is near the equator for orthographic pick.
        let mut found = false;
        for yi in 0..21 {
            let y = 0.30 + yi as f64 * 0.02;
            for xi in 0..21 {
                let x = 0.30 + xi as f64 * 0.02;
                let inputs = [RoomInput::PointerDown { x, y, t: 0.0 }];
                if room.goal_met(0.0, &inputs) {
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }
        assert!(found, "some hand placement should land near the equator");
        let _ = EQUATOR_COS;
    }

    #[test]
    fn ambient_bead_moves() {
        let r = BlochSphere::new();
        let mut a = Canvas::new(64, 48);
        let mut b = Canvas::new(64, 48);
        r.render(&mut a, 0.0);
        r.render(&mut b, 0.35);
        assert_ne!(a.to_text(), b.to_text());
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(64, 48);
        BlochSphere::new().render(&mut c, 0.28);
        assert!(c.ink_count() > 40);
    }

    #[test]
    fn plate_is_not_phase_thin() {
        let room = BlochSphere::new();
        let mut worst = usize::MAX;
        for t in [0.0_f64, 0.25, 0.5, 0.75, 1.0] {
            let mut c = Canvas::new(120, 70);
            room.render(&mut c, t);
            worst = worst.min(c.ink_count());
        }
        assert!(worst >= 80, "worst phase ink {worst}");
    }

    #[test]
    fn hand_moves_the_domain() {
        let room = BlochSphere::new();
        let mut base = Canvas::new(120, 70);
        let mut poked = Canvas::new(120, 70);
        room.render(&mut base, 0.3);
        room.render_poked(&mut poked, 0.3, &[(0.65, 0.45)]);
        assert_ne!(base.to_text(), poked.to_text());
    }

    #[test]
    fn dial_is_not_dead() {
        let room = BlochSphere::new();
        let mut left = Canvas::new(120, 70);
        let mut right = Canvas::new(120, 70);
        room.render_poked(&mut left, 0.3, &[(0.35, 0.50)]);
        room.render_poked(&mut right, 0.3, &[(0.70, 0.50)]);
        assert_ne!(left.to_text(), right.to_text());
    }

    #[test]
    fn motif_ok() {
        assert!(BlochSphere::new().motif().unwrap().line.len() >= 6);
    }

    #[test]
    fn variation_changes_deal() {
        let a = BlochSphere::new_with(0).status(0.05).unwrap();
        let b = BlochSphere::new_with(11).status(0.05).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn deep_cuts_fit_the_unlock_road() {
        assert!(BlochSphere::new().deep_cuts().len() <= crate::journey::CUT_LEVELS.len());
    }
}

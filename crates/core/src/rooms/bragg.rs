//! Bragg's law: constructive interference from crystal planes.
//!
//! DRAG: TUNE ANGLE. See `docs/ROOMS.md`.

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

fn angle(t: f64, hand: Option<(f64, f64)>, seed: u64) -> f64 {
    let s = if seed == 0 {
        0.0
    } else {
        (seed % 5) as f64 * 0.02
    };
    if let Some((x, _)) = hand {
        0.15 + x * 1.2 + s
    } else {
        0.25 + phase_unit(t) * 1.0 + s
    }
}

fn plane_spacing(seed: u64) -> f64 {
    1.0 + if seed == 0 {
        0.0
    } else {
        (seed % 3) as f64 * 0.1
    }
}

fn bragg_state(theta: f64, seed: u64) -> (f64, u32, f64) {
    let theta = if theta.is_finite() {
        theta.clamp(0.0, std::f64::consts::FRAC_PI_2)
    } else {
        0.0
    };
    let path = 2.0 * plane_spacing(seed) * theta.sin();
    let order = path.round().max(1.0) as u32;
    let detune = (path - order as f64).abs();
    (path, order, detune)
}

fn draw(canvas: &mut dyn Surface, theta: f64, seed: u64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    let theta = theta.clamp(0.1, 1.45);
    // Crystal planes as horizontal lines.
    let n_planes = 5;
    let gap = height as f64 / (n_planes as f64 + 1.0);
    for i in 1..=n_planes {
        let y = (i as f64 * gap).round() as i32;
        canvas.line(2, y, width.saturating_sub(3) as i32, y, '-');
    }
    // Incident and reflected rays on middle plane.
    let mid = ((n_planes as f64 / 2.0).ceil() * gap).round() as i32;
    let cx = (width / 2) as i32;
    let len = (width.min(height) as f64 * 0.35) as i32;
    let dx = (len as f64 * theta.cos()).round() as i32;
    let dy = (len as f64 * theta.sin()).round() as i32;
    // Incident from upper left.
    canvas.line(cx - dx, mid - dy, cx, mid, '#');
    // Reflected to upper right.
    canvas.line(cx, mid, cx + dx, mid - dy, '#');
    // Path difference mark 2 d sin theta on a side scale.
    let (path, _, detune) = bragg_state(theta, seed);
    let bar_h = ((path / 3.0).clamp(0.0, 1.0) * (height as f64 * 0.5)).round() as i32;
    let bx = width.saturating_sub(4) as i32;
    let by0 = height as i32 / 2;
    canvas.line(bx, by0 - bar_h, bx, by0 + bar_h, '|');
    // Lambda marks: bright when path ~ n lambda (lambda toy = 1).
    let bright = detune < 0.12;
    let ch = if bright { '*' } else { '.' };
    // Detector arc.
    let steps = 24;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=steps {
        let u = i as f64 / steps as f64;
        let ang = -theta + u * 2.0 * theta;
        let px = cx + (len as f64 * 0.9 * ang.cos()).round() as i32;
        let py = mid - (len as f64 * 0.9 * ang.sin().abs()).round() as i32;
        if let Some((ox, oy)) = prev {
            canvas.line(ox, oy, px, py, ch);
        }
        prev = Some((px, py));
    }
}

/// Bragg room.
#[derive(Debug, Default)]
pub struct Bragg {
    seed: u64,
}

impl Bragg {
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

impl Room for Bragg {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "bragg",
            title: "Bragg Diffraction",
            wing: "Waves & Sound",
            blurb: "n lambda = 2 d sin theta on crystal planes. t and DRAG: TUNE ANGLE.",
            accent: [40, 100, 160],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        draw(canvas, angle(t, None, self.seed), self.seed);
    }

    fn postcard_t(&self) -> f64 {
        0.55
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "bragg",
            root: 554.37,
            tempo: 88,
            line: &[0, 5, 8, 12, 8, 5, 0, 12],
            encodes: "path difference 2 d sin theta equals integer wavelengths",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE ANGLE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let th = angle(t, None, self.seed);
        let (path, _, _) = bragg_state(th, self.seed);
        Some(format!("th={th:.2}  2dsin={path:.2}  DRAG:ANG"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        let th = angle(t, hands.last().copied(), self.seed);
        draw(canvas, th, self.seed);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let th = angle(t, hands.last().copied(), self.seed);
        let (path, order, detune) = bragg_state(th, self.seed);
        let grade = if detune < 0.12 { "PEAK" } else { "OFF" };
        Some(format!("{grade} n={order}  pd={path:.2}  err={detune:.2}"))
    }

    fn reveal(&self) -> &'static str {
        "Bragg's law says crystals diffract when n lambda = 2 d sin theta: the \
         extra path between reflections from successive planes is a whole number \
         of wavelengths. X-ray crystallography is this geometry made practical."
    }
}

#[cfg(test)]
mod tests {
    use super::{Bragg, bragg_state, plane_spacing};
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn status_invites() {
        let s = Bragg::new().status(0.3).unwrap();
        assert!(s.contains("DRAG") || s.contains("th="));
        assert!(s.chars().count() <= 56);
    }

    #[test]
    fn angle_changes() {
        let r = Bragg::new();
        let o = r.status(0.3).unwrap();
        let a = r
            .status_input(
                0.3,
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
    fn bragg_state_uses_the_rendered_plane_spacing() {
        let theta = std::f64::consts::FRAC_PI_6;
        let seed = 2;
        let (path, order, detune) = bragg_state(theta, seed);
        assert!((plane_spacing(seed) - 1.2).abs() < 1e-12);
        assert!((path - 1.2).abs() < 1e-12);
        assert_eq!(order, 1);
        assert!((detune - 0.2).abs() < 1e-12);
    }

    #[test]
    fn first_order_peak_has_zero_detune() {
        let theta = (0.5_f64).asin();
        let (path, order, detune) = bragg_state(theta, 0);
        assert!((path - 1.0).abs() < 1e-12);
        assert_eq!(order, 1);
        assert!(detune < 1e-12);
    }

    #[test]
    fn newest_angle_does_not_change_with_duplicate_history() {
        let room = Bragg::new_with(2);
        let hand = (0.5, 0.5);
        let mut single = Canvas::new(48, 24);
        let mut duplicate = Canvas::new(48, 24);
        room.render_poked(&mut single, 0.3, &[hand]);
        room.render_poked(&mut duplicate, 0.3, &[hand, hand]);
        assert_eq!(single.to_text(), duplicate.to_text());
    }

    #[test]
    fn postcard_has_ink() {
        let mut c = Canvas::new(48, 24);
        Bragg::new().render(&mut c, 0.55);
        assert!(c.ink_count() > 0);
    }
}

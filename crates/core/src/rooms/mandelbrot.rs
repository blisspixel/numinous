//! The Mandelbrot set: infinite complexity from one line of arithmetic.
//!
//! For each point `c` in the complex plane, iterate `z -> z*z + c` from zero and
//! ask whether it stays bounded. The points that do form the set; the points that
//! escape, shaded by how fast, form its infinitely detailed halo. `t` zooms from
//! the whole set toward the seahorse valley. See `docs/ROOMS.md`.

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::{MAX_DIM, Surface};

use super::FRACTAL_MAX_ITER;
const DIVE_FACTOR: f64 = 0.5;
/// Exponential live-camera zoom per elapsed second.
const LIVE_ZOOM_RATE: f64 = 0.18;
/// Exponential approach to the automatic seahorse-valley destination.
const LIVE_CENTER_RATE: f64 = 0.35;
/// Precision floor for the live camera. Smaller spans cease to move reliably
/// once their pixel offsets approach `f64` precision at the current center.
const MIN_LIVE_HALF_SPAN: f64 = 1.0e-12;

#[derive(Debug, Clone, Copy, PartialEq)]
struct DivePoint {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct DiveEvent {
    point: DivePoint,
    t: f64,
}

fn finite_phase(t: f64) -> f64 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn drawing_dims(canvas: &dyn Surface) -> Option<(usize, usize)> {
    let width = canvas.width();
    let height = canvas.height();
    if width == 0 || height == 0 {
        None
    } else {
        Some((width.min(MAX_DIM), height.min(MAX_DIM)))
    }
}

fn screen_coord(norm: f64, extent: usize) -> i32 {
    debug_assert!(extent > 0);
    (norm.clamp(0.0, 1.0) * extent.saturating_sub(1) as f64).round() as i32
}

fn bounded_dive_points(pokes: &[(f64, f64)], width: usize, height: usize) -> Vec<DivePoint> {
    let start = pokes.len().saturating_sub(MAX_ROOM_POKES);
    pokes[start..]
        .iter()
        .filter_map(|&(x, y)| {
            if !x.is_finite() || !y.is_finite() {
                return None;
            }
            let nx = x.clamp(0.0, 1.0);
            let ny = y.clamp(0.0, 1.0);
            Some(DivePoint {
                x: screen_coord(nx, width),
                y: screen_coord(ny, height),
            })
        })
        .collect()
}

/// The Mandelbrot room.
#[derive(Debug, Default)]
pub struct Mandelbrot {
    seed: u64,
}

/// Persistent camera for a live Mandelbrot encounter.
///
/// The deterministic room renderer continues to map normalized phase to a
/// reproducible postcard. An interactive face can own this separate state so
/// elapsed time keeps moving inward across phase boundaries and bounded input
/// history cannot discard the selected view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MandelbrotCamera {
    center_x: f64,
    center_y: f64,
    horizontal_half_span: f64,
    target_x: f64,
    target_y: f64,
}

impl MandelbrotCamera {
    /// Start at the deterministic opening view for `seed`.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self::from_phase(0.0, seed)
    }

    /// Start at one deterministic room phase before continuing persistently.
    #[must_use]
    pub fn from_phase(t: f64, seed: u64) -> Self {
        let (center_x, center_y, horizontal_half_span) = automatic_view(t, seed);
        let (target_x, target_y, _) = automatic_view(1.0, seed);
        Self {
            center_x,
            center_y,
            horizontal_half_span,
            target_x,
            target_y,
        }
    }

    /// Return `(center_x, center_y, horizontal_half_span)` for CPU or GPU use.
    #[must_use]
    pub fn view(self) -> (f64, f64, f64) {
        (self.center_x, self.center_y, self.horizontal_half_span)
    }

    /// Continue the inward zoom by an elapsed duration in seconds.
    ///
    /// Non-finite and non-positive durations are ignored. The precision floor
    /// prevents underflow and an apparent frozen or invalid camera after a
    /// long unattended session.
    pub fn advance(&mut self, elapsed_seconds: f64) {
        if !elapsed_seconds.is_finite() || elapsed_seconds <= 0.0 {
            return;
        }
        let center_blend = 1.0 - (-LIVE_CENTER_RATE * elapsed_seconds).exp();
        self.center_x += (self.target_x - self.center_x) * center_blend;
        self.center_y += (self.target_y - self.center_y) * center_blend;
        self.horizontal_half_span = (self.horizontal_half_span
            * (-LIVE_ZOOM_RATE * elapsed_seconds).exp())
        .max(MIN_LIVE_HALF_SPAN);
    }

    /// Center the camera on a normalized screen point and dive one level.
    ///
    /// Returns `false` for a zero-size viewport or non-finite point. Valid
    /// coordinates are clamped to the visible viewport before conversion to
    /// the complex plane.
    pub fn dive(&mut self, x: f64, y: f64, width: usize, height: usize) -> bool {
        if width == 0 || height == 0 || !x.is_finite() || !y.is_finite() {
            return false;
        }
        let x = x.clamp(0.0, 1.0);
        let y = y.clamp(0.0, 1.0);
        let vertical_half_span = self.horizontal_half_span * height as f64 / width as f64;
        self.center_x += (2.0 * x - 1.0) * self.horizontal_half_span;
        self.center_y += (2.0 * y - 1.0) * vertical_half_span;
        self.target_x = self.center_x;
        self.target_y = self.center_y;
        self.horizontal_half_span =
            (self.horizontal_half_span * DIVE_FACTOR).max(MIN_LIVE_HALF_SPAN);
        true
    }

    /// Restore the deterministic opening view for `seed`.
    pub fn reset(&mut self, seed: u64) {
        *self = Self::new(seed);
    }

    /// Render this exact camera through the shared deterministic CPU path.
    pub fn render(&self, canvas: &mut dyn Surface) {
        let Some((width, height)) = drawing_dims(canvas) else {
            return;
        };
        render_view(
            canvas,
            width,
            height,
            self.center_x,
            self.center_y,
            self.horizontal_half_span,
        );
    }
}

impl Mandelbrot {
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

/// How many iterations `z -> z*z + c` survives before escaping `|z| > 2`.
fn escape_iters(cx: f64, cy: f64, max: u32) -> u32 {
    let (mut zx, mut zy) = (0.0, 0.0);
    let mut i = 0;
    while i < max && zx * zx + zy * zy <= 4.0 {
        let next_x = zx * zx - zy * zy + cx;
        zy = 2.0 * zx * zy + cy;
        zx = next_x;
        i += 1;
    }
    i
}

/// Linear interpolation from `a` to `b` by `t`.
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn render_view(
    canvas: &mut dyn Surface,
    width: usize,
    height: usize,
    center_x: f64,
    center_y: f64,
    zoom: f64,
) {
    let scale = 2.0 * zoom / width as f64;
    let half_w = width as f64 / 2.0;
    let half_h = height as f64 / 2.0;
    for py in 0..height {
        for px in 0..width {
            let cx = center_x + (px as f64 - half_w) * scale;
            let cy = center_y + (py as f64 - half_h) * scale;
            let iters = escape_iters(cx, cy, FRACTAL_MAX_ITER);
            let mark = if iters == FRACTAL_MAX_ITER {
                '#'
            } else if iters > 24 {
                '*'
            } else if iters > 6 {
                '-'
            } else {
                continue;
            };
            canvas.plot(px as i32, py as i32, mark);
        }
    }
}

/// Return the automatic camera as `(center_x, center_y, horizontal_half_span)`.
///
/// The CPU room and accelerated app path share this calculation so touching
/// the room cannot jump between two unrelated cameras.
#[must_use]
pub fn automatic_view(t: f64, seed: u64) -> (f64, f64, f64) {
    let t = finite_phase(t);
    let seed_offset = (seed % 1000) as f64 * 0.00001;
    (
        lerp(-0.5, -0.745, t) + seed_offset,
        lerp(0.0, 0.113, t) + seed_offset,
        1.5 * 0.15_f64.powf(t),
    )
}

fn selected_view_from_points(
    dives: &[DivePoint],
    width: usize,
    height: usize,
    seed: u64,
    initial_t: f64,
) -> (f64, f64, f64) {
    let (mut center_x, mut center_y, mut zoom) = automatic_view(initial_t, seed);
    for dive in dives {
        let scale = 2.0 * zoom / width as f64;
        center_x += (f64::from(dive.x) - width as f64 / 2.0) * scale;
        center_y += (f64::from(dive.y) - height as f64 / 2.0) * scale;
        zoom *= DIVE_FACTOR;
    }
    (center_x, center_y, zoom)
}

/// Return the persistent camera selected by normalized click points.
///
/// The app GPU path and the core render path share this calculation so a click
/// keeps the same palette, iteration budget, and camera on accelerated systems.
#[must_use]
pub fn selected_view(
    pokes: &[(f64, f64)],
    width: usize,
    height: usize,
    seed: u64,
    initial_t: f64,
) -> (f64, f64, f64) {
    if width == 0 || height == 0 {
        return automatic_view(initial_t, seed);
    }
    selected_view_from_points(
        &bounded_dive_points(pokes, width, height),
        width,
        height,
        seed,
        initial_t,
    )
}

/// Return the persistent camera selected by phase-stamped input events.
///
/// The first valid click fixes the automatic camera at the instant of the
/// click. Later animation frames retain that camera while additional clicks
/// dive farther into it.
#[must_use]
pub fn selected_view_input(
    inputs: &[RoomInput],
    width: usize,
    height: usize,
    seed: u64,
    current_t: f64,
) -> (f64, f64, f64) {
    if width == 0 || height == 0 {
        return automatic_view(current_t, seed);
    }
    let events = dive_events(inputs, width, height);
    let Some(first) = events.first() else {
        return automatic_view(current_t, seed);
    };
    let dives: Vec<_> = events.iter().map(|event| event.point).collect();
    selected_view_from_points(&dives, width, height, seed, first.t)
}

fn valid_dive_inputs(inputs: &[RoomInput]) -> Vec<(f64, f64, f64)> {
    let events: Vec<_> = inputs
        .iter()
        .filter_map(|input| match *input {
            RoomInput::PointerDown { x, y, t } => Some((x, y, t)),
            _ => None,
        })
        .collect();
    let start = events.len().saturating_sub(MAX_ROOM_POKES);
    events[start..]
        .iter()
        .filter_map(|&(x, y, t)| {
            if !x.is_finite() || !y.is_finite() || !t.is_finite() {
                return None;
            }
            Some((x.clamp(0.0, 1.0), y.clamp(0.0, 1.0), finite_phase(t)))
        })
        .collect()
}

fn dive_events(inputs: &[RoomInput], width: usize, height: usize) -> Vec<DiveEvent> {
    valid_dive_inputs(inputs)
        .into_iter()
        .map(|(x, y, t)| DiveEvent {
            point: DivePoint {
                x: screen_coord(x, width),
                y: screen_coord(y, height),
            },
            t,
        })
        .collect()
}

impl Room for Mandelbrot {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "mandelbrot",
            title: "Mandelbrot Set",
            wing: "Fractals & the Infinite",
            blurb: "Iterate z into z squared plus c and ask if it stays bounded. The points that \
                    do form the most complex object in mathematics. t zooms toward the seahorses.",
            accent: [70, 130, 255],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let Some((width, height)) = drawing_dims(canvas) else {
            return;
        };
        let (center_x, center_y, zoom) = automatic_view(t, self.seed);
        render_view(canvas, width, height, center_x, center_y, zoom);
    }

    fn reveal(&self) -> &'static str {
        "You can zoom into this shape forever and keep finding new detail, all from \
         squaring a number and adding a constant. Its main body has the cardioid \
         shape wrapped by Times Tables at 2; along its real slice, the quadratic \
         family is the Logistic Map in a stretched and shifted orbit coordinate."
    }

    #[allow(dead_code)]
    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "C deep boundary",
            root: 65.41,
            tempo: 80,
            line: &[0, 12, 7, 3, 8, 5, 1, 0],
            encodes: "escape-time falling back toward an infinite edge",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: DIVE AT POINT")
    }

    fn status(&self, _t: f64) -> Option<String> {
        Some("AUTO DIVE   CLICK TO CHOOSE A TARGET".into())
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let dives = valid_dive_inputs(inputs).len();
        if dives == 0 {
            return self.status(t);
        }
        let magnification = 1_u64 << dives.min(63);
        Some(format!(
            "DIVE {dives}   ZOOM {magnification}X   TARGET SELECTED"
        ))
    }

    #[allow(dead_code)]
    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        if pokes.is_empty() {
            self.render(canvas, t);
            return;
        }
        let Some((width, height)) = drawing_dims(canvas) else {
            return;
        };
        let dives = bounded_dive_points(pokes, width, height);
        if dives.is_empty() {
            self.render(canvas, t);
            return;
        }
        let (center_x, center_y, zoom) =
            selected_view_from_points(&dives, width, height, self.seed, t);
        render_view(canvas, width, height, center_x, center_y, zoom);
    }

    fn render_input(&self, canvas: &mut dyn Surface, t: f64, inputs: &[RoomInput]) {
        let Some((width, height)) = drawing_dims(canvas) else {
            return;
        };
        let (center_x, center_y, zoom) = selected_view_input(inputs, width, height, self.seed, t);
        render_view(canvas, width, height, center_x, center_y, zoom);
    }

    fn deep_cuts(&self) -> &'static [&'static str] {
        &[
            "Nobody knows the exact area of this set. It is about 1.5065918849, \
             measured by throwing billions of points at it, and there is no known \
             closed form. One of the most famous objects in mathematics, and we \
             cannot tell you how big it is.",
            "Shishikura proved in 1991 that the boundary you are zooming along has \
             Hausdorff dimension exactly 2: a curve so wrinkled it is, in the fractal \
             sense, as thick as the plane it lives in.",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DivePoint, MIN_LIVE_HALF_SPAN, Mandelbrot, MandelbrotCamera, automatic_view,
        bounded_dive_points, escape_iters, finite_phase, selected_view, selected_view_from_points,
        selected_view_input,
    };
    use crate::canvas::Canvas;
    use crate::room::{MAX_ROOM_POKES, Room, RoomInput};
    use crate::surface::{MAX_DIM, Surface};

    #[test]
    fn origin_is_in_the_set() {
        assert_eq!(escape_iters(0.0, 0.0, 160), 160);
    }

    #[test]
    fn far_points_escape_quickly() {
        assert!(escape_iters(2.0, 2.0, 160) < 6);
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = Mandelbrot::new();
        let mut a = Canvas::new(50, 30);
        let mut b = Canvas::new(50, 30);
        room.render(&mut a, 0.0);
        room.render(&mut b, 0.0);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 20);
    }

    #[test]
    fn selected_input_camera_stays_fixed_after_the_click() {
        let inputs = [RoomInput::PointerDown {
            x: 0.65,
            y: 0.35,
            t: 0.2,
        }];
        let early = selected_view_input(&inputs, 900, 700, 17, 0.3);
        let late = selected_view_input(&inputs, 900, 700, 17, 0.8);

        assert_eq!(early, late);
    }

    #[test]
    fn persistent_camera_keeps_zooming_across_normalized_phase_boundaries() {
        let mut camera = MandelbrotCamera::from_phase(0.99, 17);
        let before = camera.view();
        let target = automatic_view(1.0, 17);
        camera.advance(0.5);
        let after_boundary = camera.view();
        camera.advance(12.0);
        let later = camera.view();

        let distance = |view: (f64, f64, f64)| (view.0 - target.0).hypot(view.1 - target.1);
        assert!(distance(after_boundary) < distance(before));
        assert!(distance(later) < distance(after_boundary));
        assert!(after_boundary.2 < before.2);
        assert!(later.2 < after_boundary.2);
        assert!(later.2 >= MIN_LIVE_HALF_SPAN);
    }

    #[test]
    fn persistent_camera_dives_at_the_screen_point_then_continues() {
        let mut camera = MandelbrotCamera::new(0);
        let opening = camera.view();
        assert!(camera.dive(0.75, 0.25, 900, 600));
        let selected = camera.view();

        assert!((selected.0 - (opening.0 + opening.2 * 0.5)).abs() < 1e-12);
        assert!((selected.1 - (opening.1 - opening.2 / 3.0)).abs() < 1e-12);
        assert!((selected.2 - opening.2 * 0.5).abs() < 1e-12);

        camera.advance(1.0);
        let continued = camera.view();
        assert_eq!((continued.0, continued.1), (selected.0, selected.1));
        assert!(continued.2 < selected.2);
    }

    #[test]
    fn persistent_camera_rejects_hostile_updates_and_resets_exactly() {
        let opening = MandelbrotCamera::new(23);
        let mut camera = opening;
        for elapsed in [f64::NAN, f64::INFINITY, -1.0, 0.0] {
            camera.advance(elapsed);
        }
        assert_eq!(camera, opening);
        assert!(!camera.dive(f64::NAN, 0.5, 900, 700));
        assert!(!camera.dive(0.5, f64::INFINITY, 900, 700));
        assert!(!camera.dive(0.5, 0.5, 0, 700));
        assert_eq!(camera, opening);

        assert!(camera.dive(2.0, -1.0, 900, 700));
        camera.advance(1.0e9);
        assert_eq!(camera.view().2, MIN_LIVE_HALF_SPAN);
        camera.reset(23);
        assert_eq!(camera, opening);
    }

    #[test]
    fn persistent_camera_cpu_render_is_deterministic() {
        let mut camera = MandelbrotCamera::new(5);
        assert!(camera.dive(0.63, 0.41, 64, 40));
        camera.advance(0.75);
        let mut a = Canvas::new(64, 40);
        let mut b = Canvas::new(64, 40);
        camera.render(&mut a);
        camera.render(&mut b);

        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 20);
    }

    fn render_text(room: &Mandelbrot, t: f64, pokes: &[(f64, f64)]) -> String {
        let mut canvas = Canvas::new(48, 32);
        room.render_poked(&mut canvas, t, pokes);
        canvas.to_text()
    }

    fn render_input_text(
        room: &Mandelbrot,
        current_t: f64,
        event_t: f64,
        points: &[(f64, f64)],
    ) -> String {
        let inputs: Vec<_> = points
            .iter()
            .map(|&(x, y)| RoomInput::PointerDown { x, y, t: event_t })
            .collect();
        let mut canvas = Canvas::new(48, 32);
        room.render_input(&mut canvas, current_t, &inputs);
        canvas.to_text()
    }

    #[test]
    fn nonfinite_phase_falls_back_to_first_frame() {
        let room = Mandelbrot::new();
        assert_eq!(finite_phase(f64::NAN), 0.0);
        assert_eq!(finite_phase(f64::INFINITY), 0.0);

        let mut first = Canvas::new(36, 24);
        let mut nan = Canvas::new(36, 24);
        room.render(&mut first, 0.0);
        room.render(&mut nan, f64::NAN);
        assert_eq!(nan.to_text(), first.to_text());
    }

    #[test]
    fn dive_points_use_the_newest_bounded_raw_tail() {
        let newest: Vec<_> = (0..MAX_ROOM_POKES)
            .map(|i| {
                (
                    i as f64 / (MAX_ROOM_POKES - 1) as f64,
                    if i % 2 == 0 { 0.15 } else { 0.85 },
                )
            })
            .collect();
        let mut all = vec![(0.2, 0.8); MAX_ROOM_POKES + 7];
        all.extend(newest.iter().copied());
        let discarded_prefix = all[..all.len() - MAX_ROOM_POKES].to_vec();

        assert_eq!(
            bounded_dive_points(&all, 48, 32),
            bounded_dive_points(&newest, 48, 32)
        );
        assert_ne!(
            bounded_dive_points(&all, 48, 32),
            bounded_dive_points(&discarded_prefix, 48, 32)
        );
    }

    #[test]
    fn render_poked_uses_newest_raw_tail() {
        let room = Mandelbrot::new();
        let newest = vec![(0.85, 0.2); MAX_ROOM_POKES];
        let mut all = vec![(0.15, 0.8); MAX_ROOM_POKES + 5];
        all.extend(newest.iter().copied());
        let prefix = all[..all.len() - MAX_ROOM_POKES].to_vec();

        assert_eq!(
            render_text(&room, 0.4, &all),
            render_text(&room, 0.4, &newest)
        );
        assert_ne!(
            selected_view_from_points(&bounded_dive_points(&all, 48, 32), 48, 32, 0, 0.4),
            selected_view_from_points(&bounded_dive_points(&prefix, 48, 32), 48, 32, 0, 0.4)
        );
    }

    #[test]
    fn raw_newest_tail_is_capped_before_nonfinite_filtering() {
        let room = Mandelbrot::new();
        let mut with_invalid_tail = vec![(0.4, 0.6); MAX_ROOM_POKES];
        with_invalid_tail.extend(vec![(f64::NAN, f64::INFINITY); MAX_ROOM_POKES + 5]);

        assert!(bounded_dive_points(&with_invalid_tail, 48, 32).is_empty());
        assert_eq!(
            render_text(&room, 0.4, &with_invalid_tail),
            render_text(&room, 0.4, &[])
        );
    }

    #[test]
    fn nonfinite_pokes_do_not_consume_dive_identity() {
        let room = Mandelbrot::new();
        let finite = vec![(0.25, 0.75)];
        let with_bad_points = vec![(f64::NAN, 0.4), (0.25, 0.75), (0.2, f64::INFINITY)];

        assert_eq!(
            bounded_dive_points(&with_bad_points, 48, 32),
            bounded_dive_points(&finite, 48, 32)
        );
        assert_eq!(
            render_text(&room, 0.35, &with_bad_points),
            render_text(&room, 0.35, &finite)
        );
    }

    #[test]
    fn finite_dive_points_clamp_to_visible_edges() {
        assert_eq!(
            bounded_dive_points(&[(1.5, -1.0)], 10, 8),
            vec![DivePoint { x: 9, y: 0 }]
        );
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Mandelbrot::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 0.5, 1.0, 9.0] {
            room.render(&mut canvas, t);
        }
    }

    #[test]
    fn huge_custom_surface_does_not_render_unbounded_regions() {
        #[derive(Default)]
        struct HugeSurface {
            width: usize,
            height: usize,
            plots: usize,
            max_abs_coord: i32,
        }

        impl Surface for HugeSurface {
            fn width(&self) -> usize {
                self.width
            }

            fn height(&self) -> usize {
                self.height
            }

            fn plot(&mut self, x: i32, y: i32, _mark: char) {
                self.plots += 1;
                self.max_abs_coord = self.max_abs_coord.max(x.abs()).max(y.abs());
            }
        }

        let room = Mandelbrot::new();
        for (width, height) in [(usize::MAX, 8), (8, usize::MAX)] {
            let mut surface = HugeSurface {
                width,
                height,
                ..HugeSurface::default()
            };
            room.render_poked(&mut surface, 0.8, &[(0.5, 0.5)]);

            assert!(surface.plots <= MAX_DIM * 16);
            assert!(surface.max_abs_coord <= MAX_DIM.saturating_sub(1) as i32);
        }
    }

    #[test]
    fn reveal_names_both_cross_room_connections() {
        let reveal = Mandelbrot::new().reveal();
        assert!(reveal.contains("forever"));
        assert!(reveal.contains("Times Tables"));
        assert!(reveal.contains("Logistic Map"));
        assert!(reveal.contains("orbit coordinate"));
    }

    #[test]
    fn verb_and_poked_no_panic() {
        let room = Mandelbrot::new();
        assert!(room.verb().is_some());
        let mut c = Canvas::new(20, 15);
        room.render_poked(&mut c, 0.5, &[(0.5, 0.5)]);
        // just no panic, ink may vary
    }

    #[test]
    fn selected_dive_ignores_global_phase_until_reset() {
        let room = Mandelbrot::new();
        let selected_early = render_input_text(&room, 0.1, 0.6, &[(0.5, 0.5)]);
        let selected_late = render_input_text(&room, 0.9, 0.6, &[(0.5, 0.5)]);
        let deeper = render_input_text(&room, 0.9, 0.6, &[(0.5, 0.5), (0.5, 0.5)]);

        assert_eq!(selected_early, selected_late);
        assert_ne!(selected_late, deeper);
        assert_ne!(selected_late, render_text(&room, 0.9, &[]));
    }

    #[test]
    fn first_dive_uses_the_automatic_view_that_was_clicked() {
        let automatic = automatic_view(0.6, 0);
        let selected = selected_view_from_points(&[DivePoint { x: 24, y: 16 }], 48, 32, 0, 0.6);

        assert!((selected.0 - automatic.0).abs() < 1e-12);
        assert!((selected.1 - automatic.1).abs() < 1e-12);
        assert!((selected.2 - automatic.2 * super::DIVE_FACTOR).abs() < 1e-12);
        assert_ne!(
            selected,
            selected_view_from_points(&[DivePoint { x: 24, y: 16 }], 48, 32, 0, 0.0)
        );
    }

    #[test]
    fn normalized_selected_view_matches_the_internal_camera() {
        let pokes = [(0.5, 0.5), (0.75, 0.25)];
        let points = bounded_dive_points(&pokes, 900, 700);
        assert_eq!(
            selected_view(&pokes, 900, 700, 17, 0.63),
            selected_view_from_points(&points, 900, 700, 17, 0.63)
        );
        assert_eq!(
            selected_view(&pokes, 0, 700, 17, 0.63),
            automatic_view(0.63, 17)
        );
    }
}

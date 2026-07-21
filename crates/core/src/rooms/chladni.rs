//! Chladni Figures: sand flees a singing plate and draws the silence.
//!
//! A square plate is driven in two mode numbers (n, m). Displacement follows
//! Chladni's free-plate formula, cos(n pi x) cos(m pi y) - cos(m pi x) cos(n pi y).
//! Sand settles where the plate does not move (the nodal curves). `t` walks a
//! gallery of mode pairs; DRAG tunes n and m under the hand, and the drive tone
//! is the same number as the figure. See `docs/ROOMS.md`.

use std::f64::consts::PI;

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::sound::SoundSpec;
use crate::surface::Surface;

/// Mode pairs the phase sweep visits (n != m so the free-plate formula is alive).
const MODE_GALLERY: &[(u32, u32)] = &[
    (1, 2),
    (1, 3),
    (2, 3),
    (1, 4),
    (2, 5),
    (3, 4),
    (1, 5),
    (3, 5),
    (2, 4),
    (4, 5),
    (3, 6),
    (2, 6),
];
/// Hand can dial each mode index from 1 through this.
const MAX_MODE: u32 = 6;
/// Relative amplitude below which a cell counts as a node (sand settles).
const NODE_FRAC: f64 = 0.07;
/// Fundamental for the drive-tone mapping (Hz).
const DRIVE_ROOT_HZ: f32 = 55.0;
/// Salt for nonzero variation mode-gallery offset.
const VARIATION_SALT: u64 = 0x0C14_AD41_5EED_0001;

fn phase_unit(t: f64) -> f64 {
    if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Chladni free-plate displacement at normalized plate coordinates in [0, 1].
fn displacement(x: f64, y: f64, n: u32, m: u32) -> f64 {
    let nx = f64::from(n) * PI * x;
    let my = f64::from(m) * PI * y;
    let mx = f64::from(m) * PI * x;
    let ny = f64::from(n) * PI * y;
    nx.cos() * my.cos() - mx.cos() * ny.cos()
}

/// Ambient gallery pair selected by phase and variation.
fn gallery_modes(t: f64, seed: u64) -> (u32, u32) {
    let len = MODE_GALLERY.len();
    let offset = if seed == 0 {
        0
    } else {
        ((seed ^ VARIATION_SALT) as usize) % len
    };
    let u = phase_unit(t);
    let idx = ((u * len as f64) as usize + offset) % len;
    MODE_GALLERY[idx]
}

/// Hand-tuned integer modes. Equal indices step m up so the figure never goes blank.
fn tuned_modes(x: f64, y: f64) -> (u32, u32) {
    let n = 1 + (x.clamp(0.0, 1.0) * f64::from(MAX_MODE - 1)).round() as u32;
    let mut m = 1 + (y.clamp(0.0, 1.0) * f64::from(MAX_MODE - 1)).round() as u32;
    if m == n {
        m = if n < MAX_MODE { n + 1 } else { n - 1 };
    }
    (n.clamp(1, MAX_MODE), m.clamp(1, MAX_MODE))
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

/// Drive frequency (Hz) for mode pair (n, m): proportional to sqrt(n^2 + m^2).
fn drive_hz(n: u32, m: u32) -> f32 {
    let nn = f64::from(n * n + m * m).sqrt() as f32;
    (DRIVE_ROOT_HZ * nn).clamp(80.0, 880.0)
}

fn node_count_sample(n: u32, m: u32, samples: usize) -> usize {
    let mut nodes = 0;
    for iy in 0..samples {
        for ix in 0..samples {
            let x = (ix as f64 + 0.5) / samples as f64;
            let y = (iy as f64 + 0.5) / samples as f64;
            if displacement(x, y, n, m).abs() < NODE_FRAC {
                nodes += 1;
            }
        }
    }
    nodes
}

fn draw_plate(canvas: &mut dyn Surface, n: u32, m: u32) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 {
        return;
    }
    // Sample the plate on the pixel grid; sand lives on nodes.
    for py in 0..height {
        for px in 0..width {
            let x = (px as f64 + 0.5) / width as f64;
            let y = (py as f64 + 0.5) / height as f64;
            let z = displacement(x, y, n, m).abs();
            if z < NODE_FRAC {
                // Dense sand on deep nodes; lighter dust on the node fringe.
                let ch = if z < NODE_FRAC * 0.35 { '#' } else { ':' };
                canvas.plot(px as i32, py as i32, ch);
            }
        }
    }
    // Plate rim so the singing square is always framed.
    let last_x = width.saturating_sub(1) as i32;
    let last_y = height.saturating_sub(1) as i32;
    canvas.line(0, 0, last_x, 0, '.');
    canvas.line(0, last_y, last_x, last_y, '.');
    canvas.line(0, 0, 0, last_y, '.');
    canvas.line(last_x, 0, last_x, last_y, '.');
}

/// Chladni Figures room.
#[derive(Debug, Default)]
pub struct Chladni {
    seed: u64,
}

impl Chladni {
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

    fn modes_at(&self, t: f64) -> (u32, u32) {
        gallery_modes(t, self.seed)
    }
}

impl Room for Chladni {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "chladni",
            title: "Chladni Figures",
            wing: "Waves & Sound",
            blurb: "Sand flees a singing plate and draws the silence: nodal curves of a free square \
                    plate under two mode numbers. t walks the mode gallery; DRAG tunes n and m, and \
                    the drive tone is the figure.",
            accent: [200, 190, 120],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let (n, m) = self.modes_at(t);
        draw_plate(canvas, n, m);
    }

    fn postcard_t(&self) -> f64 {
        0.42
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "singing plate",
            root: 220.0,
            tempo: 96,
            line: &[0, 7, 0, 12, 7, 0, 5, 12],
            encodes: "drive tone locking while sand finds the still lines",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("DRAG: TUNE THE PLATE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let (n, m) = self.modes_at(t);
        let hz = drive_hz(n, m).round() as i32;
        Some(format!("MODE {n}:{m}  DRIVE {hz}Hz  DRAG:TUNE"))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let hands = finite_pokes(pokes);
        if hands.is_empty() {
            self.render(canvas, t);
            return;
        }
        let (nx, ny) = *hands.last().expect("nonempty hands");
        let (n, m) = tuned_modes(nx, ny);
        draw_plate(canvas, n, m);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        let Some(&(x, y)) = hands.last() else {
            return self.status(t);
        };
        let (n, m) = tuned_modes(x, y);
        let hz = drive_hz(n, m).round() as i32;
        let nodes = node_count_sample(n, m, 24);
        Some(format!("TUNED {n}:{m}  DRIVE {hz}Hz  NODES {nodes}"))
    }

    fn sound(&self, t: f64) -> SoundSpec {
        let (n, m) = self.modes_at(t);
        // The drive tone is the room: one partial for each mode index, same numbers.
        SoundSpec::chord(
            &[DRIVE_ROOT_HZ * n as f32, DRIVE_ROOT_HZ * m as f32],
            1.5,
            0.22,
        )
    }

    fn reveal(&self) -> &'static str {
        "Sand draws the places the plate does not move. You can hear the modes \
         as the drive tone, and you can see them as still curves, the same two \
         numbers twice. Whether every shape has a unique sound is subtler: \
         Gordon, Webb, and Wolpert (1992) built distinct drums that share every \
         frequency, so you cannot always hear the shape of a drum."
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Chladni, DRIVE_ROOT_HZ, MAX_MODE, MODE_GALLERY, displacement, drive_hz, gallery_modes,
        tuned_modes,
    };
    use crate::canvas::Canvas;
    use crate::room::{Room, RoomInput};

    #[test]
    fn equal_modes_make_a_blank_free_plate() {
        // The free-plate formula is identically zero when n equals m.
        for i in 0..11 {
            let x = i as f64 / 10.0;
            for j in 0..11 {
                let y = j as f64 / 10.0;
                assert!(displacement(x, y, 3, 3).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn unequal_modes_are_not_blank() {
        let mut peak: f64 = 0.0;
        for i in 0..20 {
            for j in 0..20 {
                let z = displacement(i as f64 / 19.0, j as f64 / 19.0, 1, 2).abs();
                peak = peak.max(z);
            }
        }
        assert!(peak > 0.5, "mode 1:2 must move the plate");
    }

    #[test]
    fn tuned_modes_never_collapse_to_equal_pair() {
        for i in 0..=10 {
            for j in 0..=10 {
                let (n, m) = tuned_modes(i as f64 / 10.0, j as f64 / 10.0);
                assert_ne!(n, m);
                assert!((1..=MAX_MODE).contains(&n));
                assert!((1..=MAX_MODE).contains(&m));
            }
        }
    }

    #[test]
    fn gallery_walks_and_variation_offsets() {
        let a = gallery_modes(0.0, 0);
        let b = gallery_modes(0.9, 0);
        assert!(MODE_GALLERY.contains(&a));
        assert_ne!(a, b);
        assert_ne!(gallery_modes(0.0, 0), gallery_modes(0.0, 3));
    }

    #[test]
    fn first_contact_status_invites_a_tune() {
        let room = Chladni::new();
        let open = room.status(0.0).expect("open");
        assert!(open.contains("MODE"), "{open}");
        assert!(open.contains("DRAG"), "{open}");
        assert!(open.contains("DRIVE"), "{open}");
        assert!(open.chars().count() <= 56, "{open}");
    }

    #[test]
    fn tune_changes_status_and_reports_nodes() {
        let room = Chladni::new();
        let open = room.status(0.0).expect("open");
        let input = [RoomInput::PointerDown {
            x: 0.8,
            y: 0.2,
            t: 0.0,
        }];
        let after = room.status_input(0.0, &input).expect("tuned");
        assert_ne!(after, open);
        assert!(after.contains("TUNED"), "{after}");
        assert!(after.contains("NODES"), "{after}");
        assert!(after.chars().any(|c| c.is_ascii_digit()), "{after}");
        assert!(after.chars().count() <= 56, "{after}");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = Chladni::new();
        let mut a = Canvas::new(48, 32);
        let mut b = Canvas::new(48, 32);
        room.render(&mut a, 0.3);
        room.render(&mut b, 0.3);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 30);

        let mut postcard = Canvas::new(60, 40);
        room.render(&mut postcard, room.postcard_t());
        assert!(postcard.ink_count() > 40);
    }

    #[test]
    fn hand_tune_changes_the_figure() {
        let room = Chladni::new();
        let mut base = Canvas::new(48, 32);
        let mut poked = Canvas::new(48, 32);
        room.render(&mut base, 0.1);
        room.render_poked(&mut poked, 0.1, &[(0.9, 0.1)]);
        assert_ne!(base.to_text(), poked.to_text());
    }

    #[test]
    fn variation_changes_ambient_gallery() {
        let mut a = Canvas::new(40, 28);
        let mut b = Canvas::new(40, 28);
        Chladni::new_with(0).render(&mut a, 0.2);
        Chladni::new_with(5).render(&mut b, 0.2);
        assert_ne!(a.to_text(), b.to_text());
        let mut zero = Canvas::new(40, 28);
        Chladni::new().render(&mut zero, 0.2);
        assert_eq!(a.to_text(), zero.to_text());
    }

    #[test]
    fn drive_hz_grows_with_mode_energy() {
        assert!(drive_hz(5, 6) > drive_hz(1, 2));
        assert!((80.0..=880.0).contains(&drive_hz(1, 2)));
    }

    #[test]
    fn sound_carries_both_mode_partials() {
        let room = Chladni::new();
        let (n, m) = room.modes_at(0.0);
        let sound = room.sound(0.0);
        assert_eq!(sound.notes.len(), 2);
        let freqs: Vec<i32> = sound
            .notes
            .iter()
            .map(|note| note.freq.round() as i32)
            .collect();
        assert!(freqs.contains(&((DRIVE_ROOT_HZ * n as f32).round() as i32)));
        assert!(freqs.contains(&((DRIVE_ROOT_HZ * m as f32).round() as i32)));
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Chladni::new();
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
    fn reveal_names_hearing_the_shape_of_a_drum() {
        let text = Chladni::new().reveal().to_ascii_lowercase();
        assert!(text.contains("drum") || text.contains("shape"));
        assert!(text.contains("gordon") || text.contains("1992") || text.contains("frequency"));
    }

    #[test]
    fn motif_is_playable() {
        let motif = Chladni::new().motif().expect("motif");
        assert!(motif.line.len() >= 6);
        assert!(motif.pattern().seconds() > 0.0);
    }
}

//! The Ripple Tank: drop pebbles and watch interference write itself.
//!
//! Each pebble is a monochromatic point source. At every plate point the
//! amplitudes add; bright lanes are constructive interference, dead-calm lanes
//! are destructive. Two sources make the double slit by hand. `t` ages the
//! wave phase; CLICK: DROP A PEBBLE. See `docs/ROOMS.md`.

use std::f64::consts::TAU;

use crate::room::{MAX_ROOM_POKES, Room, RoomInput, RoomMeta};
use crate::surface::Surface;

/// Ambient sources when the hand has not yet dropped a pebble.
const AMBIENT: &[(f64, f64)] = &[(0.32, 0.50), (0.68, 0.50)];
/// Wavelength in plate units (normalized width = 1).
const WAVELENGTH: f64 = 0.11;
/// Salt for nonzero variation source drift.
const VARIATION_SALT: u64 = 0x2197_007E_5EED_0001;

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

/// Ambient sources, optionally drifted by variation so visits do not clone.
fn ambient_sources(seed: u64) -> Vec<(f64, f64)> {
    if seed == 0 {
        return AMBIENT.to_vec();
    }
    let mix = seed ^ VARIATION_SALT;
    let dx = (((mix % 9) as f64) / 8.0 - 0.5) * 0.08;
    let dy = ((((mix / 9) % 7) as f64) / 6.0 - 0.5) * 0.10;
    AMBIENT
        .iter()
        .map(|&(x, y)| ((x + dx).clamp(0.05, 0.95), (y + dy).clamp(0.05, 0.95)))
        .collect()
}

fn sources_for(seed: u64, pokes: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let hands = finite_pokes(pokes);
    if hands.is_empty() {
        ambient_sources(seed)
    } else {
        hands
    }
}

/// Superposed circular waves from each source at normalized plate (x, y).
fn field(x: f64, y: f64, sources: &[(f64, f64)], age: f64) -> f64 {
    let k = TAU / WAVELENGTH;
    let mut sum = 0.0;
    for &(sx, sy) in sources {
        let dx = x - sx;
        let dy = y - sy;
        let r = (dx * dx + dy * dy).sqrt();
        // Soft 1/sqrt(r) falloff keeps far crests visible without blowing the center.
        let amp = 1.0 / (1.0 + 4.0 * r).sqrt();
        sum += amp * (k * r - age * TAU).sin();
    }
    sum
}

fn peak_field(sources: &[(f64, f64)], age: f64, samples: usize) -> f64 {
    let mut peak = 0.0_f64;
    for iy in 0..samples {
        for ix in 0..samples {
            let x = (ix as f64 + 0.5) / samples as f64;
            let y = (iy as f64 + 0.5) / samples as f64;
            peak = peak.max(field(x, y, sources, age).abs());
        }
    }
    peak.max(1e-6)
}

fn draw_tank(canvas: &mut dyn Surface, sources: &[(f64, f64)], age: f64) {
    let (width, height) = canvas.draw_bounds();
    if width == 0 || height == 0 || sources.is_empty() {
        return;
    }
    let peak = peak_field(sources, age, 16);
    for py in 0..height {
        for px in 0..width {
            let x = (px as f64 + 0.5) / width as f64;
            let y = (py as f64 + 0.5) / height as f64;
            let z = field(x, y, sources, age) / peak;
            let ch = if z > 0.55 {
                '#'
            } else if z > 0.25 {
                '*'
            } else if z > 0.08 {
                ':'
            } else if z < -0.55 {
                '='
            } else if z < -0.25 {
                '-'
            } else if z.abs() < 0.05 {
                // Dead-calm lanes: destructive interference.
                '.'
            } else {
                ' '
            };
            if ch != ' ' {
                canvas.plot(px as i32, py as i32, ch);
            }
        }
    }
    // Mark pebble centers so drops stay legible.
    for &(sx, sy) in sources {
        let px = (sx * width.saturating_sub(1) as f64).round() as i32;
        let py = (sy * height.saturating_sub(1) as f64).round() as i32;
        canvas.plot(px, py, '+');
        canvas.plot(px + 1, py, '+');
        canvas.plot(px, py + 1, '+');
    }
}

/// Count approximate calm samples (destructive cores) on a coarse grid.
fn calm_count(sources: &[(f64, f64)], age: f64, samples: usize) -> usize {
    let peak = peak_field(sources, age, samples);
    let mut calm = 0;
    for iy in 0..samples {
        for ix in 0..samples {
            let x = (ix as f64 + 0.5) / samples as f64;
            let y = (iy as f64 + 0.5) / samples as f64;
            if (field(x, y, sources, age) / peak).abs() < 0.05 {
                calm += 1;
            }
        }
    }
    calm
}

/// The Ripple Tank room.
#[derive(Debug, Default)]
pub struct Ripple {
    seed: u64,
}

impl Ripple {
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
}

impl Room for Ripple {
    fn meta(&self) -> RoomMeta {
        RoomMeta {
            id: "ripple",
            title: "The Ripple Tank",
            wing: "Waves & Sound",
            blurb: "Drop pebbles; circular waves interfere into bright fans and dead-calm lanes. \
                    Two sources build the double slit by hand. t ages the phase; CLICK: DROP A \
                    PEBBLE.",
            accent: [70, 160, 220],
        }
    }

    fn render(&self, canvas: &mut dyn Surface, t: f64) {
        let age = phase_unit(t);
        let sources = ambient_sources(self.seed);
        draw_tank(canvas, &sources, age);
    }

    fn postcard_t(&self) -> f64 {
        0.35
    }

    fn motif(&self) -> Option<crate::motifs::Motif> {
        Some(crate::motifs::Motif {
            key: "double slit calm",
            root: 164.81,
            tempo: 112,
            line: &[0, 5, 7, 5, 0, 7, 12, 0],
            encodes: "two sources locking into bright and silent lanes",
        })
    }

    fn verb(&self) -> Option<&'static str> {
        Some("CLICK: DROP A PEBBLE")
    }

    fn status(&self, t: f64) -> Option<String> {
        let age = phase_unit(t);
        let sources = ambient_sources(self.seed);
        let calm = calm_count(&sources, age, 20);
        Some(format!(
            "SRC {}  CALM {}  CLICK: DROP PEBBLE",
            sources.len(),
            calm
        ))
    }

    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let age = phase_unit(t);
        let sources = sources_for(self.seed, pokes);
        draw_tank(canvas, &sources, age);
    }

    fn status_input(&self, t: f64, inputs: &[RoomInput]) -> Option<String> {
        let pokes = crate::pokes_from_inputs(inputs);
        let hands = finite_pokes(&pokes);
        if hands.is_empty() {
            return self.status(t);
        }
        let age = phase_unit(t);
        let sources = sources_for(self.seed, &pokes);
        let calm = calm_count(&sources, age, 20);
        let (nx, ny) = *hands.last().expect("nonempty hands");
        Some(format!(
            "DROP {:.0}%{:.0}%  SRC {}  CALM {}",
            nx * 100.0,
            ny * 100.0,
            sources.len(),
            calm
        ))
    }

    fn reveal(&self) -> &'static str {
        "Where crests meet crests the water heaves; where crest meets trough it \
         goes still. Those dead-calm lanes are not missing waves: they are waves \
         canceling. Two pebbles draw the double-slit pattern by the same arithmetic. \
         Feynman called interference the only mystery of quantum mechanics; here it \
         is just water, and your hand."
    }
}

#[cfg(test)]
mod tests {
    use super::{AMBIENT, Ripple, ambient_sources, calm_count, field, finite_pokes, sources_for};
    use crate::canvas::Canvas;
    use crate::room::{MAX_ROOM_POKES, Room, RoomInput};

    #[test]
    fn single_source_is_not_blank() {
        let sources = [(0.5, 0.5)];
        let mut peak = 0.0_f64;
        for i in 0..20 {
            let x = i as f64 / 19.0;
            peak = peak.max(field(x, 0.5, &sources, 0.0).abs());
        }
        assert!(peak > 0.2);
    }

    #[test]
    fn two_coherent_sources_make_calm_lanes() {
        let sources = AMBIENT.to_vec();
        let calm = calm_count(&sources, 0.0, 24);
        // Destructive lanes should appear somewhere on a coarse grid.
        assert!(calm > 5, "expected interference nulls, got {calm}");
    }

    #[test]
    fn field_is_antisymmetric_under_half_period_phase() {
        let sources = [(0.4, 0.5)];
        let a = field(0.7, 0.5, &sources, 0.0);
        let b = field(0.7, 0.5, &sources, 0.5);
        // age * TAU advances by pi at age 0.5: sin(theta - pi) = -sin(theta).
        assert!((a + b).abs() < 1e-9, "a={a} b={b}");
    }

    #[test]
    fn first_contact_status_invites_a_drop() {
        let room = Ripple::new();
        let open = room.status(0.0).expect("open");
        assert!(open.contains("SRC"), "{open}");
        assert!(open.contains("CLICK"), "{open}");
        assert!(open.contains("DROP") || open.contains("PEBBLE"), "{open}");
        assert!(open.chars().count() <= 56, "{open}");
    }

    #[test]
    fn drop_changes_status() {
        let room = Ripple::new();
        let open = room.status(0.0).expect("open");
        let input = [RoomInput::PointerDown {
            x: 0.25,
            y: 0.75,
            t: 0.0,
        }];
        let after = room.status_input(0.0, &input).expect("drop");
        assert_ne!(after, open);
        assert!(after.contains("DROP"), "{after}");
        assert!(after.contains("SRC 1"), "{after}");
        assert!(after.chars().count() <= 56, "{after}");
    }

    #[test]
    fn render_is_deterministic_and_has_ink() {
        let room = Ripple::new();
        let mut a = Canvas::new(48, 32);
        let mut b = Canvas::new(48, 32);
        room.render(&mut a, 0.2);
        room.render(&mut b, 0.2);
        assert_eq!(a.to_text(), b.to_text());
        assert!(a.ink_count() > 40);
        let mut postcard = Canvas::new(60, 40);
        room.render(&mut postcard, room.postcard_t());
        assert!(postcard.ink_count() > 50);
    }

    #[test]
    fn hand_pebbles_replace_ambient_sources() {
        let ambient = sources_for(0, &[]);
        assert_eq!(ambient.len(), 2);
        let one = sources_for(0, &[(0.1, 0.2)]);
        assert_eq!(one, vec![(0.1, 0.2)]);
    }

    #[test]
    fn variation_moves_ambient_sources() {
        assert_ne!(ambient_sources(0), ambient_sources(11));
        let mut a = Canvas::new(40, 28);
        let mut b = Canvas::new(40, 28);
        Ripple::new_with(0).render(&mut a, 0.3);
        Ripple::new_with(11).render(&mut b, 0.3);
        assert_ne!(a.to_text(), b.to_text());
    }

    #[test]
    fn finite_pokes_use_newest_tail() {
        let newest: Vec<_> = (0..MAX_ROOM_POKES)
            .map(|i| ((i as f64 + 0.5) / MAX_ROOM_POKES as f64, 0.4))
            .collect();
        let mut old = vec![(0.9, 0.9); MAX_ROOM_POKES + 5];
        old.extend(newest.clone());
        assert_eq!(finite_pokes(&old), finite_pokes(&newest));
    }

    #[test]
    fn extreme_inputs_do_not_panic() {
        let room = Ripple::new();
        let mut empty = Canvas::new(0, 0);
        room.render(&mut empty, 0.5);
        let mut canvas = Canvas::new(8, 8);
        for t in [-1.0, 0.0, 1.0, 9.0, f64::NAN, f64::INFINITY] {
            room.render(&mut canvas, t);
            room.render_poked(&mut canvas, t, &[(f64::NAN, f64::INFINITY)]);
            room.render_poked(&mut canvas, t, &[(0.5, 0.5), (0.2, 0.8)]);
        }
    }

    #[test]
    fn reveal_names_interference_or_double_slit() {
        let text = Ripple::new().reveal().to_ascii_lowercase();
        assert!(text.contains("interfer") || text.contains("double"));
        assert!(text.contains("cancel") || text.contains("calm") || text.contains("still"));
    }

    #[test]
    fn motif_is_playable() {
        let motif = Ripple::new().motif().expect("motif");
        assert!(motif.line.len() >= 6);
        assert!(motif.pattern().seconds() > 0.0);
    }
}

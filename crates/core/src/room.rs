//! The [`Room`] contract: the plugin every mathematical phenomenon implements.
//!
//! This is the seam described in `docs/ARCHITECTURE.md`. It is intentionally
//! small in this first increment (metadata plus a deterministic ASCII render);
//! audio, parameters, challenges, and reveals join the trait as those systems
//! come online.

use crate::sound::SoundSpec;
use crate::surface::Surface;

/// The face-neutral default action for rooms without a dedicated touch verb.
pub const DEFAULT_ROOM_ACTION: &str = "SCRUB TIME";

/// The touch-first default action for App arrival cards and HUD hints.
pub const DEFAULT_TOUCH_ROOM_ACTION: &str = "DRAG: SCRUB TIME";

/// Maximum static hand points a face should pass to [`Room::render_poked`].
pub const MAX_ROOM_POKES: usize = 24;

/// Static, human- and agent-readable description of a room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomMeta {
    /// Stable identifier used on the command line and in the registry, e.g. `"times-tables"`.
    pub id: &'static str,
    /// Display title, e.g. `"Times Tables"`.
    pub title: &'static str,
    /// The Wing this room belongs to, e.g. `"Number & Pattern"`.
    pub wing: &'static str,
    /// One-line description of what the room does.
    pub blurb: &'static str,
    /// The room's signature accent color as `[r, g, b]`, added per stroke on the
    /// near-black stage so overlapping strokes glow (see `docs/VISUALS.md`).
    pub accent: [u8; 3],
}

/// A playable mathematical phenomenon.
///
/// Implementations are deterministic: the same inputs always produce the same
/// output, so renders reproduce exactly across runs, faces, and machines.
pub trait Room {
    /// This room's static metadata.
    fn meta(&self) -> RoomMeta;

    /// Render a single deterministic frame into `surface`.
    ///
    /// The surface may be an ASCII `Canvas`, a pixel `Raster`, or any other
    /// target. `t` is a normalized phase in `[0.0, 1.0)`; implementations should
    /// clamp defensively and never panic on any value or surface size.
    fn render(&self, surface: &mut dyn Surface, t: f64);

    /// The revelation: the short, true insight that reframes what the player
    /// just did (see `docs/INSIGHTS.md`). Surfaced only when asked, never pushed.
    fn reveal(&self) -> &'static str;

    /// The phase this room is proudest of: what the gallery, the contact
    /// sheet, and any other postcard should show. Found by looking (the beauty
    /// QA loop in `docs/QUALITY.md`); defaults to the start of the sweep.
    fn postcard_t(&self) -> f64 {
        0.0
    }

    /// The room's musical identity: a phrase, not a tone (Engine A2).
    /// None falls back to the generic seeded chiptune bed.
    fn motif(&self) -> Option<crate::motifs::Motif> {
        None
    }

    /// The live readout: what the dial says right now ("K = 2.98, 2 LOBES,
    /// ALMOST CLOSING"). The math answering back as you scrub. None stays
    /// silent; keep it one short line.
    fn status(&self, t: f64) -> Option<String> {
        let _ = t;
        None
    }

    /// The touch-surface verb: what a hand can do here, named for the arrival
    /// card ("CLICK: DROP A STORM", "DRAG: TURN THE DIAL"). None means the room
    /// has no dedicated poke or drag behavior; faces may still offer generic
    /// phase scrubbing through [`room_action`] or [`room_touch_action`].
    fn verb(&self) -> Option<&'static str> {
        None
    }

    /// Render with hands in the scene: `pokes` are normalized (x, y) points
    /// the player has placed, newest last. Faces bound this history to
    /// [`MAX_ROOM_POKES`]. Rooms that answer override this; the default
    /// performs exactly as `render`, so nothing changes until a room chooses
    /// to listen.
    fn render_poked(&self, canvas: &mut dyn Surface, t: f64, pokes: &[(f64, f64)]) {
        let _ = pokes;
        self.render(canvas, t);
    }

    /// Deeper cuts, in order of depth: true, retellable, math-teacher-grade
    /// gems that unlock as the journey deepens (the faces choose thresholds;
    /// see `docs/PLAYFUL.md`). The knowledge is the loot. Empty by default.
    fn deep_cuts(&self) -> &'static [&'static str] {
        &[]
    }

    /// This room's sound at phase `t` (the "everything is an instrument" pillar,
    /// see `docs/SOUND.md`). The default is a single tone that rises with `t`;
    /// rooms override this to give their own voice.
    fn sound(&self, t: f64) -> SoundSpec {
        let phase = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let octaves = phase as f32;
        SoundSpec::tone(110.0 * 2.0_f32.powf(octaves), 1.5, 0.2)
    }
}

/// The face-neutral action line for text and protocol faces.
pub fn room_action(room: &dyn Room) -> &'static str {
    room.verb().unwrap_or(DEFAULT_ROOM_ACTION)
}

/// The touch-first action line for app arrival cards and HUD hints.
pub fn room_touch_action(room: &dyn Room) -> &'static str {
    room.verb().unwrap_or(DEFAULT_TOUCH_ROOM_ACTION)
}

#[cfg(test)]
mod tests {
    use super::{Room, RoomMeta};
    use crate::surface::Surface;

    struct DefaultSoundRoom;

    impl Room for DefaultSoundRoom {
        fn meta(&self) -> RoomMeta {
            RoomMeta {
                id: "default-sound",
                title: "Default Sound",
                wing: "Tests",
                blurb: "A test room for the trait default.",
                accent: [0, 0, 0],
            }
        }

        fn render(&self, _surface: &mut dyn Surface, _t: f64) {}

        fn reveal(&self) -> &'static str {
            "The default sound remains finite."
        }
    }

    #[test]
    fn default_sound_handles_nonfinite_phase() {
        let room = DefaultSoundRoom;
        let base = room.sound(0.0);

        for t in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let spec = room.sound(t);
            assert_eq!(spec, base);
            assert!(spec.notes.iter().all(|note| note.freq.is_finite()));
        }
    }
}

//! The [`Room`] contract: the plugin every mathematical phenomenon implements.
//!
//! This is the seam described in `docs/ARCHITECTURE.md`. It is intentionally
//! small in this first increment (metadata plus a deterministic ASCII render);
//! audio, parameters, challenges, and reveals join the trait as those systems
//! come online.

use crate::sound::SoundSpec;
use crate::surface::Surface;

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

    /// This room's sound at phase `t` (the "everything is an instrument" pillar,
    /// see `docs/SOUND.md`). The default is a single tone that rises with `t`;
    /// rooms override this to give their own voice.
    fn sound(&self, t: f64) -> SoundSpec {
        let octaves = t.clamp(0.0, 1.0) as f32;
        SoundSpec::tone(220.0 * 2.0_f32.powf(octaves), 1.5, 0.3)
    }
}

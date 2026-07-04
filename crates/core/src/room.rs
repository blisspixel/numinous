//! The [`Room`] contract: the plugin every mathematical phenomenon implements.
//!
//! This is the seam described in `docs/ARCHITECTURE.md`. It is intentionally
//! small in this first increment (metadata plus a deterministic ASCII render);
//! audio, parameters, challenges, and reveals join the trait as those systems
//! come online.

use crate::canvas::Canvas;

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
}

/// A playable mathematical phenomenon.
///
/// Implementations are deterministic: the same inputs always produce the same
/// output, so renders reproduce exactly across runs, faces, and machines.
pub trait Room {
    /// This room's static metadata.
    fn meta(&self) -> RoomMeta;

    /// Render a single deterministic frame into `canvas`.
    ///
    /// `t` is a normalized phase in `[0.0, 1.0)`; implementations should clamp
    /// defensively and never panic on any value.
    fn render_ascii(&self, canvas: &mut Canvas, t: f64);

    /// The revelation: the short, true insight that reframes what the player
    /// just did (see `docs/INSIGHTS.md`). Surfaced only when asked, never pushed.
    fn reveal(&self) -> &'static str;
}

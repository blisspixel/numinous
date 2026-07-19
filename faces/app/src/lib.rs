//! Reusable boundaries owned by the windowed Numinous App.
//!
//! Window creation and device routing remain in the binary. This library
//! exposes the read-only local session viewer plus deterministic Studio and Nim
//! renderers so integration tests, the live App, and future App shells exercise
//! the same pairing, retention, and visual replay implementations.

pub mod nim_render;
pub mod session_viewer;
pub mod studio_render;

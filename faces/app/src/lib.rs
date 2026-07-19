//! Reusable boundaries owned by the windowed Numinous App.
//!
//! Window creation and device routing remain in the binary. This library
//! exposes the read-only local session viewer so integration tests and future
//! App shells exercise the same pairing, retention, and replay implementation.

pub mod session_viewer;

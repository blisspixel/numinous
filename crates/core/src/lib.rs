//! Numinous headless core.
//!
//! This is the windowless engine that all three faces (App, CLI, MCP) build on
//! (see `docs/INTERFACES.md`). It owns the [`Room`] contract, the room
//! [`registry`], and a deterministic ASCII [`Canvas`].
//!
//! In this first increment the core is intentionally std-only and renders rooms
//! as deterministic ASCII, which the CLI shows in the terminal and which agents
//! can read as text. GPU rendering (`wgpu`), real-time audio (`cpal`), and the
//! Studio runtime are layered on top of this contract in later increments; see
//! `docs/ARCHITECTURE.md` and `docs/ROADMAP.md`.

// The core is the library that everything depends on; hold it to the strictest
// documentation bar (see docs/ENGINEERING.md).
#![deny(missing_docs)]

pub mod aliens;
pub mod canvas;
pub mod codebreaker;
pub mod font;
pub mod quiz;
pub mod raster;
pub mod registry;
pub mod rng;
pub mod room;
pub mod rooms;
pub mod secret;
pub mod sim;
pub mod sims;
pub mod sound;
pub mod surface;

pub use aliens::{AlienMessage, alien_message};
pub use canvas::Canvas;
pub use codebreaker::{Feedback, grade, hint, secret_code};
pub use font::{draw_text, text_width, wrap_text};
pub use quiz::{QuizChoice, QuizRound, build_round};
pub use raster::Raster;
pub use registry::{all_rooms, room_by_id};
pub use rng::SplitMix64;
pub use room::{Room, RoomMeta};
pub use secret::akousma;
pub use sim::{Lever, Sim, SimMeta, default_params, lever_value};
pub use sims::{all_sims, sim_by_id};
pub use sound::{Note, SoundSpec};
pub use surface::Surface;

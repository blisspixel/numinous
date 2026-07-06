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
pub mod ansi;
pub mod canvas;
pub mod chiptune;
pub mod codebreaker;
pub mod era;
pub mod font;
pub mod humor;
pub mod journey;
pub mod munchers;
pub mod nim;
pub mod quiz;
pub mod raster;
pub mod registry;
pub mod resonance;
pub mod rng;
pub mod room;
pub mod rooms;
pub mod scores;
pub mod secret;
pub mod seti;
pub mod sim;
pub mod sims;
pub mod sound;
pub mod studio;
pub mod surface;
pub mod trophies;

pub use aliens::{AlienMessage, alien_message, to_base};
pub use ansi::to_ansi;
pub use canvas::Canvas;
pub use chiptune::{Pattern, Voice, compose, pitch};
pub use codebreaker::{Feedback, grade, hint, secret_code};
pub use era::Era;
pub use font::{draw_text, text_width, wrap_text};
pub use humor::{Joke, explain_joke, jokes};
pub use journey::{
    Boon, CUT_LEVELS, Journey, MAX_LEVEL, Rank, UNLOCKS, boon_options, constellation, level_lore,
};
pub use munchers::{Board, Munched, board_text, build_board, grade as grade_munch};
pub use nim::{
    apply as nim_apply, finished as nim_finished, new_game as nim_new, order_move as nim_order,
    the_secret as nim_secret,
};
pub use quiz::{QuizChoice, QuizRound, build_round, build_round_sized};
pub use raster::Raster;
pub use registry::{all_rooms, hidden_room_by_id, room_by_id};
pub use resonance::{Resonance, resonances};
pub use rng::SplitMix64;
pub use room::{Room, RoomMeta};
pub use scores::Scoreboard;
pub use secret::{akousma, deep_akousma};
pub use seti::{SetiChannel, SetiScan, build_scan};
pub use sim::{Lever, Sim, SimMeta, default_params, lever_value};
pub use sims::{all_sims, sim_by_id};
pub use sound::{Note, SoundSpec};
pub use studio::{Expr, eval, parse, plot_text, to_melody};
pub use surface::Surface;
pub use trophies::{Trophy, trophies};

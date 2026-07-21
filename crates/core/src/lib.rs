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
pub mod cairn;
pub mod canvas;
pub mod challenge;
pub mod chiptune;
pub mod citations;
pub mod codebreaker;
pub mod concepts;
pub mod era;
pub mod fifteen;
pub mod font;
pub mod hackenbush;
pub mod humor;
pub mod insights;
pub use insights::{INSIGHTS, insight};
pub mod journal;
pub mod journey;
pub mod life_sound;
pub mod motifs;
pub mod munch_arcade;
pub mod munchers;
pub mod nim;
pub mod party;
pub mod persistence;
pub mod predict;
pub mod quiz;
pub mod radio;
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
pub mod spectrum;
pub mod studio;
pub mod surface;
pub mod trophies;

pub use aliens::{AlienMessage, alien_message, to_base};
pub use ansi::to_ansi;
pub use cairn::{
    Bequest, CairnRead, CairnStone, count as cairn_count, deposit, draw_stone, encode,
    founding_bequests, picture, read_at, submission_line,
};
pub use canvas::{Canvas, RenderDelta};
pub use challenge::{
    Challenge, ChallengeGrade, ParameterGoal, ParameterGrade, grade_challenge, grade_parameter,
    pose_challenge, pose_parameter_goal,
};
pub use chiptune::{
    Arrangement, ChipNote, Pattern, StereoSignalMetrics, Voice, compose, game_buzz, game_tick,
    munch_crunch, pitch, quantize_pcm16, stereo_signal_metrics,
};
pub use citations::{for_room as room_citation, for_room_unlocked as room_citation_unlocked};
pub use codebreaker::{Feedback, grade, hint, secret_code};
pub use concepts::concept;
pub use era::Era;
pub use font::{draw_text, text_width, wrap_text};
pub use humor::{Joke, explain_joke, jokes};
pub use journal::{Journal, JournalEntry};
pub use journey::{
    Boon, CUT_LEVELS, Journey, MAX_LEVEL, Rank, UNLOCKS, boon_options, constellation, level_lore,
};
pub use motifs::{MAX_ROOM_BED_EVENTS, Motif, ROOM_BED_SOURCE_RATE};
pub use munchers::{
    Board, FULL_DECK_ROUND, Munched, board_text, build_board, grade as grade_munch,
};
pub use nim::{
    apply as nim_apply, finished as nim_finished, new_game as nim_new, order_move as nim_order,
    the_secret as nim_secret,
};
pub use persistence::{
    LocalCacheInventory, LocalCairnInventory, LocalFileInventory, LocalJourneyInventory,
    LocalScoresInventory, LocalStateEraseError, LocalStateEraseSelection, LocalStateInventory,
    LocalStateLock, LocalStatePaths, erase_journal_file, erase_local_state, inspect_local_state,
    load_journal_file, load_journey_file, load_scoreboard_file, lock_local_state,
    persist_journey_delta, record_journal_file, record_score_file, remove_persisted_file,
};
pub use predict::{
    Band, Prediction, PredictionCurveError, PredictionCurveGrade, PredictionCurveSample,
    PredictionGrade, grade_prediction, grade_prediction_curve, pose_prediction,
    prediction_rate_window,
};
pub use quiz::{ICONIC, QuizChoice, QuizRound, build_round, build_round_pool, build_round_sized};
pub use radio::{STATIONS, Station, brief_for, length_for, station};
pub use raster::Raster;
pub use registry::{all_rooms, all_rooms_with, hidden_room_by_id, room_by_id};
pub use resonance::{Resonance, resonances};
pub use rng::SplitMix64;
pub use room::{
    DEFAULT_ROOM_ACTION, DEFAULT_TOUCH_ROOM_ACTION, Gesture, MAX_ROOM_INPUTS, MAX_ROOM_POKES, Room,
    RoomInput, RoomMeta, held_pokes_from_inputs, inputs_from_pokes, latest_gesture,
    pokes_from_inputs, renderable_poke_count, room_action, room_touch_action,
};
pub use scores::Scoreboard;
pub use secret::{akousma, deep_akousma};
pub use seti::{SetiChannel, SetiScan, build_scan};
pub use sim::{Lever, Sim, SimMeta, default_params, lever_value};
pub use sims::{all_sims, sim_by_id};
pub use sound::{Note, ParametricSound, SoundSpec};
pub use spectrum::{
    BAND_COUNT, BAND_NAMES, SpectrumBarLayout, arrangement_spectrum, band_energies,
    bass_mid_treble, draw_spectrum_bars, low_band_onset, normalize_bands,
};
pub use studio::{
    Expr, MAX_STUDIO_SOURCE_CHARS, StudioCreation, eval, parse, plot_text, to_melody,
};
pub use surface::Surface;
pub use trophies::{Trophy, trophies};

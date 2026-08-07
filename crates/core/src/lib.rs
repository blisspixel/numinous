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

/// The tracked roadmap, for the tests that require a measured limitation to be
/// named where somebody reads it.
///
/// Anchored on the crate root rather than on the position of the file that
/// includes it, so moving a source file cannot quietly break the path. Held in
/// one place rather than in each test for the same reason a shrink-only list is
/// held in one place: two copies of a path are two things that can drift, and a
/// test that fails to find the roadmap would be reporting on nothing.
#[cfg(test)]
pub(crate) const ROADMAP: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/ROADMAP.md"
));

/// The part of the roadmap that lists what the am-track cannot decide for
/// itself, as the locks that read it need it.
///
/// Panics if the section is gone, which is the point: a lock whose section has
/// been renamed away must fail loudly rather than silently check an empty
/// string.
#[cfg(test)]
pub(crate) fn roadmap_decisions() -> &'static str {
    ROADMAP
        .split_once("### Decisions the am-track is waiting on")
        .map(|(_, rest)| rest)
        .and_then(|rest| rest.split_once("\n### "))
        .map(|(section, _)| section)
        .expect("the roadmap has a decisions section for the am-track")
}

pub mod aliens;
pub mod ansi;
pub mod cairn;
pub mod canvas;
pub mod challenge;
pub mod chiptune;
pub mod citations;
pub mod codebreaker;
pub mod concepts;
pub mod dichromacy;
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
pub mod motion;
pub mod munch_arcade;
pub mod munchers;
pub mod nim;
pub mod party;
pub mod persistence;
pub mod photosensitivity;
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
pub mod share;
pub mod sim;
pub mod sims;
pub mod sound;
pub mod spectrum;
pub mod studio;
pub mod surface;
pub mod trophies;

pub use aliens::{AlienMessage, alien_message, to_base};
pub use ansi::{to_ansi, to_mono, to_terminal};
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
pub use codebreaker::{
    Feedback, MAX_CODE_DIGITS, MIN_CODE_DIGITS, grade, hint, secret_code, supports_code_length,
};
pub use concepts::{concept, explain_text};
pub use era::Era;
pub use font::{draw_text, text_width, wrap_text};
pub use humor::{Joke, explain_joke, jokes};
pub use journal::{
    JOURNAL_SCHEMA_VERSION, JOURNAL_SOURCE_LEGACY_IMPORT, JOURNAL_SOURCE_NUMINOUS_RESULT,
    JOURNAL_SOURCE_PLAYER_PROVIDED, JOURNAL_SOURCE_SELF_AUTHORED, Journal, JournalEntry,
    JournalError, JournalRecord, MAX_JOURNAL_AFFECT_CHARS, MAX_JOURNAL_ENTRIES,
    MAX_JOURNAL_KIND_CHARS, MAX_JOURNAL_SUBJECT_CHARS, MAX_JOURNAL_TEXT_CHARS,
};
pub use journey::{
    Boon, CUT_LEVELS, Journey, MAX_LEVEL, Rank, UNLOCKS, boon_options, constellation, level_lore,
};
pub use motifs::{MAX_ROOM_BED_EVENTS, Motif, ROOM_BED_SOURCE_RATE};
pub use motion::{Motion, REDUCED_MOTION_VAR, setting_is_on};
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
    LocalStateLock, LocalStatePaths, correct_journal_file, erase_journal_file, erase_local_state,
    inspect_journal_file, inspect_local_state, load_journal_file, load_journey_file,
    load_scoreboard_file, lock_local_state, persist_journey_delta, record_journal_file,
    record_score_file, remove_persisted_file, try_load_journal_file,
};
pub use photosensitivity::{
    DARK_CEILING, GENERAL_FLASH_DELTA, MAX_FLASHES_PER_SECOND, count_flashes, flashes_per_second,
    frame_luminance, peak_flashes_per_second, relative_luminance, within_budget,
};
pub use predict::{
    Band, Prediction, PredictionCurveError, PredictionCurveGrade, PredictionCurveSample,
    PredictionGrade, grade_prediction, grade_prediction_curve, pose_prediction,
    prediction_rate_window,
};
pub use quiz::{ICONIC, QuizChoice, QuizRound, build_round, build_round_pool, build_round_sized};
pub use radio::{STATIONS, Station, brief_for, length_for, station};
pub use raster::Raster;
pub use registry::{
    MAX_ECHOED_ID, MAX_ROOM_SUGGESTIONS, all_rooms, all_rooms_with, display_safe, echoable_id,
    hidden_room_by_id, must_escape_for_display, nearest_names, nearest_room_ids, room_by_id,
};
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
pub use share::{
    ShareBundleMeta, ShareKind, ShareMeta, StudioShareMeta, create_share_bundle_dir, sidecar_path,
    write_share_bundle_readme, write_share_sidecar, write_studio_share_readme,
};
pub use sim::{Lever, Sim, SimMeta, default_params, lever_value};
pub use sims::{all_sims, sim_by_id};
pub use sound::{Note, ParametricSound, SoundSpec};
pub use spectrum::{
    BAND_COUNT, BAND_NAMES, ONSET_HIT, SpectrumBarLayout, SpectrumLevers, arrangement_spectrum,
    band_energies, bass_mid_treble, draw_spectrum_bars, levers_from_bands, low_band_onset,
    normalize_bands, spectrum_hand_point, spectrum_phase_nudge, spectrum_should_poke,
    spectrum_time_scale,
};
pub use studio::{
    Expr, MAX_META_TEXT_CHARS, MAX_SHARE_INPUT_BYTES, MAX_STUDIO_SOURCE_CHARS, NumFileError,
    STUDIO_RECIPES, StudioCreation, eval, parse, plot_text, studio_auto_recipe, studio_recipe,
    studio_recipe_count, to_melody,
};
pub use surface::Surface;
pub use trophies::{Trophy, trophies};

//! Versioned Numinous Encounter Receipts: a replay proof, not a memory.

use crate::rooms::canonical_room_id;
use crate::studio_request::{
    DEFAULT_MELODY_NOTES, DEFAULT_STUDIO_PARAMETER, DEFAULT_STUDIO_XMAX, DEFAULT_STUDIO_XMIN,
};
use std::fmt;

/// Schema name for a Numinous Encounter Receipt, matching the
/// `numinous.temporal-evidence` style.
pub const ENCOUNTER_RECEIPT_SCHEMA: &str = "numinous.encounter-receipt";

/// Current receipt schema version.
pub const ENCOUNTER_RECEIPT_SCHEMA_VERSION: u64 = 1;

/// Public `play_room` destination phase when the caller omits `t`.
pub const PLAY_ROOM_DEFAULT_T: f64 = 0.0;

/// Public `play_room` canvas width when the caller omits `width`.
pub const PLAY_ROOM_DEFAULT_WIDTH: u64 = 72;

/// Public `play_room` canvas height when the caller omits `height`.
pub const PLAY_ROOM_DEFAULT_HEIGHT: u64 = 32;

/// Public `play_room` variation when the caller omits `variation`.
pub const PLAY_ROOM_DEFAULT_VARIATION: u64 = 0;

/// The `play_room` tool name on a receipt.
pub const ENCOUNTER_TOOL_PLAY_ROOM: &str = "play_room";

/// The `listen_room` tool name on a receipt.
pub const ENCOUNTER_TOOL_LISTEN_ROOM: &str = "listen_room";

/// The `sing_expression` tool name on a receipt.
pub const ENCOUNTER_TOOL_SING_EXPRESSION: &str = "sing_expression";

const ACTION_LABEL: &[u8] = b"numinous.encounter-action\0";
const RESULT_LABEL: &[u8] = b"numinous.encounter-result\0";
const RECEIPT_LABEL: &[u8] = b"numinous.encounter-receipt\0";

/// A closed, ordered play_room action after defaults and room aliases resolve.
///
/// `receipt` and `response_mode` are never members: they are presentation
/// switches, not part of what was played.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayRoomAction {
    room: String,
    t: f64,
    width: u64,
    height: u64,
    variation: u64,
    from_t: Option<f64>,
    dwell: Option<Vec<f64>>,
    pokes: Vec<(f64, f64)>,
    gesture: Vec<CanonicalGesture>,
    place_wager: Option<String>,
    number_wager: Option<f64>,
    bin_wager: Option<u64>,
    ending_wager: Option<String>,
    speed_wager: Option<String>,
    policy_wager: Option<String>,
    die_choice: Option<String>,
    counter_wager: Option<String>,
    aha_summon: bool,
}

/// One replayable pointer event in the action tuple.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CanonicalGesture {
    /// Pointer down.
    Down {
        /// Normalized column.
        x: f64,
        /// Normalized row.
        y: f64,
        /// Phase of the event.
        t: f64,
    },
    /// Pointer move.
    Move {
        /// Normalized column.
        x: f64,
        /// Normalized row.
        y: f64,
        /// Phase of the event.
        t: f64,
    },
    /// Pointer up.
    Up {
        /// Normalized column.
        x: f64,
        /// Normalized row.
        y: f64,
        /// Phase of the event.
        t: f64,
    },
    /// Gesture cancelled.
    Cancel,
}

/// Domain counts from a cell-level delta. No bounding box, no pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncounterDeltaCounts {
    /// Cells whose ink changed.
    pub cells_changed: u64,
    /// Cells that gained ink.
    pub ink_added: u64,
    /// Cells that lost ink.
    pub ink_removed: u64,
    /// Cells whose ink moved without a net add or remove.
    pub ink_reshaped: u64,
    /// Cells in the compared frames.
    pub total_cells: u64,
}

/// Domain counts from a multi-look stay. No frames, no explanation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncounterDwellCounts {
    /// How many looks the stay held.
    pub looks: u64,
    /// Cells that never changed.
    pub unchanged_cells: u64,
    /// Cells that were never ink.
    pub never_ink: u64,
    /// Cells that were always ink.
    pub always_ink: u64,
    /// Dark cells inside the region that moved.
    pub never_ink_in_changed_region: u64,
    /// Dark cells fully ringed by cells that were never dark.
    pub never_ink_enclosed: u64,
    /// Cells in each look.
    pub total_cells: u64,
}

/// Domain fields of one play_room result that a receipt may bind.
///
/// Prose, compact text, the ASCII render, and audio are intentionally
/// absent. Two plays that print differently but measure the same stay equal.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayRoomResult {
    room: String,
    t: f64,
    width: u64,
    height: u64,
    variation: u64,
    status: Option<String>,
    goal: Option<String>,
    goal_met: bool,
    delta: Option<EncounterDeltaCounts>,
    aha_beat: Option<String>,
    aha_grade: Option<String>,
    aha_allow_reveal: Option<bool>,
    temporal: Option<EncounterDeltaCounts>,
    dwell: Option<EncounterDwellCounts>,
}

/// A versioned replay-and-provenance artifact for one exact play.
///
/// There is no issued time: two identical plays produce the same receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncounterReceipt {
    replay_abi_version: u16,
    fingerprint: [u8; 32],
    tool: EncounterTool,
    action_digest: [u8; 32],
    result_digest: [u8; 32],
    package_version: String,
    build_semantic_id: [u8; 32],
}

/// Tools that may appear on a receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterTool {
    /// The `play_room` tool.
    PlayRoom,
    /// The `listen_room` tool.
    ListenRoom,
    /// The `sing_expression` tool.
    SingExpression,
}

impl EncounterTool {
    /// The MCP tool name carried on the receipt.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::PlayRoom => ENCOUNTER_TOOL_PLAY_ROOM,
            Self::ListenRoom => ENCOUNTER_TOOL_LISTEN_ROOM,
            Self::SingExpression => ENCOUNTER_TOOL_SING_EXPRESSION,
        }
    }

    /// Parse a closed tool name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            ENCOUNTER_TOOL_PLAY_ROOM => Some(Self::PlayRoom),
            ENCOUNTER_TOOL_LISTEN_ROOM => Some(Self::ListenRoom),
            ENCOUNTER_TOOL_SING_EXPRESSION => Some(Self::SingExpression),
            _ => None,
        }
    }
}

impl fmt::Display for EncounterTool {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

impl PlayRoomAction {
    /// Builds an action with public defaults and a resolved room id.
    #[must_use]
    pub fn new(room: &str) -> Self {
        Self {
            room: canonical_room_id(room).to_string(),
            t: PLAY_ROOM_DEFAULT_T,
            width: PLAY_ROOM_DEFAULT_WIDTH,
            height: PLAY_ROOM_DEFAULT_HEIGHT,
            variation: PLAY_ROOM_DEFAULT_VARIATION,
            from_t: None,
            dwell: None,
            pokes: Vec::new(),
            gesture: Vec::new(),
            place_wager: None,
            number_wager: None,
            bin_wager: None,
            ending_wager: None,
            speed_wager: None,
            policy_wager: None,
            die_choice: None,
            counter_wager: None,
            aha_summon: false,
        }
    }

    /// Destination phase.
    #[must_use]
    pub fn with_t(mut self, t: f64) -> Self {
        self.t = t;
        self
    }

    /// Canvas size in columns and rows.
    #[must_use]
    pub fn with_size(mut self, width: u64, height: u64) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Variation seed.
    #[must_use]
    pub fn with_variation(mut self, variation: u64) -> Self {
        self.variation = variation;
        self
    }

    /// Optional origin phase for two-observation evidence.
    #[must_use]
    pub fn with_from_t(mut self, from_t: Option<f64>) -> Self {
        self.from_t = from_t;
        self
    }

    /// Optional stay phases.
    #[must_use]
    pub fn with_dwell(mut self, dwell: Option<Vec<f64>>) -> Self {
        self.dwell = dwell;
        self
    }

    /// Compact poke coordinates.
    #[must_use]
    pub fn with_pokes(mut self, pokes: Vec<(f64, f64)>) -> Self {
        self.pokes = pokes;
        self
    }

    /// Replayable gesture events.
    #[must_use]
    pub fn with_gesture(mut self, gesture: Vec<CanonicalGesture>) -> Self {
        self.gesture = gesture;
        self
    }

    /// Times Tables place wager.
    #[must_use]
    pub fn with_place_wager(mut self, wager: Option<String>) -> Self {
        self.place_wager = wager;
        self
    }

    /// Buffon number wager.
    #[must_use]
    pub fn with_number_wager(mut self, wager: Option<f64>) -> Self {
        self.number_wager = wager;
        self
    }

    /// Galton bin wager.
    #[must_use]
    pub fn with_bin_wager(mut self, wager: Option<u64>) -> Self {
        self.bin_wager = wager;
        self
    }

    /// Double Pendulum ending wager.
    #[must_use]
    pub fn with_ending_wager(mut self, wager: Option<String>) -> Self {
        self.ending_wager = wager;
        self
    }

    /// Kepler speed wager.
    #[must_use]
    pub fn with_speed_wager(mut self, wager: Option<String>) -> Self {
        self.speed_wager = wager;
        self
    }

    /// Parrondo policy wager.
    #[must_use]
    pub fn with_policy_wager(mut self, wager: Option<String>) -> Self {
        self.policy_wager = wager;
        self
    }

    /// Nontransitive first die.
    #[must_use]
    pub fn with_die_choice(mut self, choice: Option<String>) -> Self {
        self.die_choice = choice;
        self
    }

    /// Nontransitive counter wager.
    #[must_use]
    pub fn with_counter_wager(mut self, wager: Option<String>) -> Self {
        self.counter_wager = wager;
        self
    }

    /// Engineered-aha summon flag. Omitted and false are the same.
    #[must_use]
    pub fn with_aha_summon(mut self, summon: bool) -> Self {
        self.aha_summon = summon;
        self
    }

    /// Canonical room id.
    #[must_use]
    pub fn room(&self) -> &str {
        &self.room
    }

    /// Destination phase.
    #[must_use]
    pub fn t(&self) -> f64 {
        self.t
    }

    /// Canvas width in columns.
    #[must_use]
    pub fn width(&self) -> u64 {
        self.width
    }

    /// Canvas height in rows.
    #[must_use]
    pub fn height(&self) -> u64 {
        self.height
    }

    /// Variation seed.
    #[must_use]
    pub fn variation(&self) -> u64 {
        self.variation
    }

    /// Optional origin phase.
    #[must_use]
    pub fn from_t(&self) -> Option<f64> {
        self.from_t
    }

    /// Optional stay phases.
    #[must_use]
    pub fn dwell(&self) -> Option<&[f64]> {
        self.dwell.as_deref()
    }

    /// Compact poke coordinates.
    #[must_use]
    pub fn pokes(&self) -> &[(f64, f64)] {
        &self.pokes
    }

    /// Replayable gesture events.
    #[must_use]
    pub fn gesture(&self) -> &[CanonicalGesture] {
        &self.gesture
    }

    /// Times Tables place wager.
    #[must_use]
    pub fn place_wager(&self) -> Option<&str> {
        self.place_wager.as_deref()
    }

    /// Buffon number wager.
    #[must_use]
    pub fn number_wager(&self) -> Option<f64> {
        self.number_wager
    }

    /// Galton bin wager.
    #[must_use]
    pub fn bin_wager(&self) -> Option<u64> {
        self.bin_wager
    }

    /// Double Pendulum ending wager.
    #[must_use]
    pub fn ending_wager(&self) -> Option<&str> {
        self.ending_wager.as_deref()
    }

    /// Kepler speed wager.
    #[must_use]
    pub fn speed_wager(&self) -> Option<&str> {
        self.speed_wager.as_deref()
    }

    /// Parrondo policy wager.
    #[must_use]
    pub fn policy_wager(&self) -> Option<&str> {
        self.policy_wager.as_deref()
    }

    /// Nontransitive first die.
    #[must_use]
    pub fn die_choice(&self) -> Option<&str> {
        self.die_choice.as_deref()
    }

    /// Nontransitive counter wager.
    #[must_use]
    pub fn counter_wager(&self) -> Option<&str> {
        self.counter_wager.as_deref()
    }

    /// Engineered-aha summon flag.
    #[must_use]
    pub fn aha_summon(&self) -> bool {
        self.aha_summon
    }

    /// Canonical length-prefixed bytes of this action.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        push_bytes(&mut bytes, ACTION_LABEL);
        push_u64(&mut bytes, ENCOUNTER_RECEIPT_SCHEMA_VERSION);
        push_str(&mut bytes, ENCOUNTER_TOOL_PLAY_ROOM);
        push_str(&mut bytes, &self.room);
        push_f64(&mut bytes, self.t);
        push_u64(&mut bytes, self.width);
        push_u64(&mut bytes, self.height);
        push_u64(&mut bytes, self.variation);
        push_option_f64(&mut bytes, self.from_t);
        match &self.dwell {
            None => bytes.push(0),
            Some(phases) => {
                bytes.push(1);
                push_u64(&mut bytes, phases.len() as u64);
                for phase in phases {
                    push_f64(&mut bytes, *phase);
                }
            }
        }
        push_u64(&mut bytes, self.pokes.len() as u64);
        for (x, y) in &self.pokes {
            push_f64(&mut bytes, *x);
            push_f64(&mut bytes, *y);
        }
        push_u64(&mut bytes, self.gesture.len() as u64);
        for event in &self.gesture {
            event.write(&mut bytes);
        }
        push_option_str(&mut bytes, self.place_wager.as_deref());
        push_option_f64(&mut bytes, self.number_wager);
        push_option_u64(&mut bytes, self.bin_wager);
        push_option_str(&mut bytes, self.ending_wager.as_deref());
        push_option_str(&mut bytes, self.speed_wager.as_deref());
        push_option_str(&mut bytes, self.policy_wager.as_deref());
        push_option_str(&mut bytes, self.die_choice.as_deref());
        push_option_str(&mut bytes, self.counter_wager.as_deref());
        bytes.push(u8::from(self.aha_summon));
        bytes
    }
}

impl CanonicalGesture {
    fn write(self, bytes: &mut Vec<u8>) {
        match self {
            Self::Down { x, y, t } => {
                bytes.push(0);
                push_f64(bytes, x);
                push_f64(bytes, y);
                push_f64(bytes, t);
            }
            Self::Move { x, y, t } => {
                bytes.push(1);
                push_f64(bytes, x);
                push_f64(bytes, y);
                push_f64(bytes, t);
            }
            Self::Up { x, y, t } => {
                bytes.push(2);
                push_f64(bytes, x);
                push_f64(bytes, y);
                push_f64(bytes, t);
            }
            Self::Cancel => bytes.push(3),
        }
    }
}

impl PlayRoomResult {
    /// Builds a result with public defaults and a resolved room id.
    #[must_use]
    pub fn new(room: &str) -> Self {
        Self {
            room: canonical_room_id(room).to_string(),
            t: PLAY_ROOM_DEFAULT_T,
            width: PLAY_ROOM_DEFAULT_WIDTH,
            height: PLAY_ROOM_DEFAULT_HEIGHT,
            variation: PLAY_ROOM_DEFAULT_VARIATION,
            status: None,
            goal: None,
            goal_met: false,
            delta: None,
            aha_beat: None,
            aha_grade: None,
            aha_allow_reveal: None,
            temporal: None,
            dwell: None,
        }
    }

    /// Destination phase.
    #[must_use]
    pub fn with_t(mut self, t: f64) -> Self {
        self.t = t;
        self
    }

    /// Canvas size in columns and rows.
    #[must_use]
    pub fn with_size(mut self, width: u64, height: u64) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Variation seed.
    #[must_use]
    pub fn with_variation(mut self, variation: u64) -> Self {
        self.variation = variation;
        self
    }

    /// Room status readout, when the room has one.
    #[must_use]
    pub fn with_status(mut self, status: Option<String>) -> Self {
        self.status = status;
        self
    }

    /// Room goal text, when the room has one.
    #[must_use]
    pub fn with_goal(mut self, goal: Option<String>) -> Self {
        self.goal = goal;
        self
    }

    /// Whether the room's own goal was met.
    #[must_use]
    pub fn with_goal_met(mut self, goal_met: bool) -> Self {
        self.goal_met = goal_met;
        self
    }

    /// Touch-delta counts, when a hand was supplied.
    #[must_use]
    pub fn with_delta(mut self, delta: Option<EncounterDeltaCounts>) -> Self {
        self.delta = delta;
        self
    }

    /// Engineered-aha beat label.
    #[must_use]
    pub fn with_aha_beat(mut self, beat: Option<String>) -> Self {
        self.aha_beat = beat;
        self
    }

    /// Engineered-aha graded sentence, when consolidation produced one.
    #[must_use]
    pub fn with_aha_grade(mut self, grade: Option<String>) -> Self {
        self.aha_grade = grade;
        self
    }

    /// Whether the staged aha has opened reveal. Absent when no aha ran.
    #[must_use]
    pub fn with_aha_allow_reveal(mut self, allow_reveal: Option<bool>) -> Self {
        self.aha_allow_reveal = allow_reveal;
        self
    }

    /// Two-observation temporal counts.
    #[must_use]
    pub fn with_temporal(mut self, temporal: Option<EncounterDeltaCounts>) -> Self {
        self.temporal = temporal;
        self
    }

    /// Multi-look dwell counts.
    #[must_use]
    pub fn with_dwell(mut self, dwell: Option<EncounterDwellCounts>) -> Self {
        self.dwell = dwell;
        self
    }

    /// Canonical length-prefixed bytes of this result.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        push_bytes(&mut bytes, RESULT_LABEL);
        push_u64(&mut bytes, ENCOUNTER_RECEIPT_SCHEMA_VERSION);
        push_str(&mut bytes, ENCOUNTER_TOOL_PLAY_ROOM);
        push_str(&mut bytes, &self.room);
        push_f64(&mut bytes, self.t);
        push_u64(&mut bytes, self.width);
        push_u64(&mut bytes, self.height);
        push_u64(&mut bytes, self.variation);
        push_option_str(&mut bytes, self.status.as_deref());
        push_option_str(&mut bytes, self.goal.as_deref());
        bytes.push(u8::from(self.goal_met));
        push_option_delta(&mut bytes, self.delta);
        push_option_str(&mut bytes, self.aha_beat.as_deref());
        push_option_str(&mut bytes, self.aha_grade.as_deref());
        match self.aha_allow_reveal {
            None => bytes.push(0),
            Some(allow) => {
                bytes.push(1);
                bytes.push(u8::from(allow));
            }
        }
        push_option_delta(&mut bytes, self.temporal);
        match self.dwell {
            None => bytes.push(0),
            Some(held) => {
                bytes.push(1);
                push_u64(&mut bytes, held.looks);
                push_u64(&mut bytes, held.unchanged_cells);
                push_u64(&mut bytes, held.never_ink);
                push_u64(&mut bytes, held.always_ink);
                push_u64(&mut bytes, held.never_ink_in_changed_region);
                push_u64(&mut bytes, held.never_ink_enclosed);
                push_u64(&mut bytes, held.total_cells);
            }
        }
        bytes
    }
}

/// A closed listen_room action after defaults and room aliases resolve.
#[derive(Debug, Clone, PartialEq)]
pub struct ListenRoomAction {
    room: String,
    t: f64,
    variation: u64,
    ambient_events: bool,
    audio: bool,
    pokes: Vec<(f64, f64)>,
    gesture: Vec<CanonicalGesture>,
}

impl ListenRoomAction {
    /// Builds a listen with public defaults and a resolved room id.
    #[must_use]
    pub fn new(room: &str) -> Self {
        Self {
            room: canonical_room_id(room).to_string(),
            t: PLAY_ROOM_DEFAULT_T,
            variation: PLAY_ROOM_DEFAULT_VARIATION,
            ambient_events: false,
            audio: false,
            pokes: Vec::new(),
            gesture: Vec::new(),
        }
    }

    /// Destination phase.
    #[must_use]
    pub fn with_t(mut self, t: f64) -> Self {
        self.t = t;
        self
    }

    /// Variation seed.
    #[must_use]
    pub fn with_variation(mut self, variation: u64) -> Self {
        self.variation = variation;
        self
    }

    /// Whether the caller asked for complete bed events.
    #[must_use]
    pub fn with_ambient_events(mut self, ambient_events: bool) -> Self {
        self.ambient_events = ambient_events;
        self
    }

    /// Whether the caller asked for a WAV.
    #[must_use]
    pub fn with_audio(mut self, audio: bool) -> Self {
        self.audio = audio;
        self
    }

    /// Compact poke coordinates.
    #[must_use]
    pub fn with_pokes(mut self, pokes: Vec<(f64, f64)>) -> Self {
        self.pokes = pokes;
        self
    }

    /// Replayable gesture events.
    #[must_use]
    pub fn with_gesture(mut self, gesture: Vec<CanonicalGesture>) -> Self {
        self.gesture = gesture;
        self
    }

    /// Canonical room id.
    #[must_use]
    pub fn room(&self) -> &str {
        &self.room
    }

    /// Destination phase.
    #[must_use]
    pub fn t(&self) -> f64 {
        self.t
    }

    /// Variation seed.
    #[must_use]
    pub fn variation(&self) -> u64 {
        self.variation
    }

    /// Whether complete bed events were requested.
    #[must_use]
    pub fn ambient_events(&self) -> bool {
        self.ambient_events
    }

    /// Whether a WAV was requested.
    #[must_use]
    pub fn audio(&self) -> bool {
        self.audio
    }

    /// Compact poke coordinates.
    #[must_use]
    pub fn pokes(&self) -> &[(f64, f64)] {
        &self.pokes
    }

    /// Replayable gesture events.
    #[must_use]
    pub fn gesture(&self) -> &[CanonicalGesture] {
        &self.gesture
    }

    /// Canonical length-prefixed bytes of this action.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        push_bytes(&mut bytes, ACTION_LABEL);
        push_u64(&mut bytes, ENCOUNTER_RECEIPT_SCHEMA_VERSION);
        push_str(&mut bytes, ENCOUNTER_TOOL_LISTEN_ROOM);
        push_str(&mut bytes, &self.room);
        push_f64(&mut bytes, self.t);
        push_u64(&mut bytes, self.variation);
        bytes.push(u8::from(self.ambient_events));
        bytes.push(u8::from(self.audio));
        push_u64(&mut bytes, self.pokes.len() as u64);
        for (x, y) in &self.pokes {
            push_f64(&mut bytes, *x);
            push_f64(&mut bytes, *y);
        }
        push_u64(&mut bytes, self.gesture.len() as u64);
        for event in &self.gesture {
            event.write(&mut bytes);
        }
        bytes
    }
}

/// Domain fields of one listen_room result that a receipt may bind.
///
/// Notation counts, motif identity, bed counts, and audio size stay. WAV
/// bytes and the prose table do not.
#[derive(Debug, Clone, PartialEq)]
pub struct ListenRoomResult {
    room: String,
    t: f64,
    variation: u64,
    duration_seconds: f64,
    note_count: u64,
    returned_note_count: u64,
    truncated: bool,
    motif_key: Option<String>,
    motif_tempo: Option<u64>,
    motif_encodes: Option<String>,
    bed_duration_seconds: Option<f64>,
    bed_event_count: Option<u64>,
    audio_encoded_bytes: Option<u64>,
}

impl ListenRoomResult {
    /// Builds a listen result with public defaults and a resolved room id.
    #[must_use]
    pub fn new(room: &str) -> Self {
        Self {
            room: canonical_room_id(room).to_string(),
            t: PLAY_ROOM_DEFAULT_T,
            variation: PLAY_ROOM_DEFAULT_VARIATION,
            duration_seconds: 0.0,
            note_count: 0,
            returned_note_count: 0,
            truncated: false,
            motif_key: None,
            motif_tempo: None,
            motif_encodes: None,
            bed_duration_seconds: None,
            bed_event_count: None,
            audio_encoded_bytes: None,
        }
    }

    /// Destination phase.
    #[must_use]
    pub fn with_t(mut self, t: f64) -> Self {
        self.t = t;
        self
    }

    /// Variation seed.
    #[must_use]
    pub fn with_variation(mut self, variation: u64) -> Self {
        self.variation = variation;
        self
    }

    /// Sonification duration in seconds.
    #[must_use]
    pub fn with_duration_seconds(mut self, duration_seconds: f64) -> Self {
        self.duration_seconds = duration_seconds;
        self
    }

    /// How many mathematical notes the room produced.
    #[must_use]
    pub fn with_note_count(mut self, note_count: u64) -> Self {
        self.note_count = note_count;
        self
    }

    /// How many notes the reply actually listed.
    #[must_use]
    pub fn with_returned_note_count(mut self, returned_note_count: u64) -> Self {
        self.returned_note_count = returned_note_count;
        self
    }

    /// Whether the note list was truncated.
    #[must_use]
    pub fn with_truncated(mut self, truncated: bool) -> Self {
        self.truncated = truncated;
        self
    }

    /// Motif identity, when the room has one.
    #[must_use]
    pub fn with_motif(
        mut self,
        key: Option<String>,
        tempo: Option<u64>,
        encodes: Option<String>,
    ) -> Self {
        self.motif_key = key;
        self.motif_tempo = tempo;
        self.motif_encodes = encodes;
        self
    }

    /// Stable bed duration and event count, when a motif produced one.
    #[must_use]
    pub fn with_bed(mut self, duration_seconds: Option<f64>, event_count: Option<u64>) -> Self {
        self.bed_duration_seconds = duration_seconds;
        self.bed_event_count = event_count;
        self
    }

    /// Encoded WAV size, when audio was sent. Never the samples.
    #[must_use]
    pub fn with_audio_encoded_bytes(mut self, encoded_bytes: Option<u64>) -> Self {
        self.audio_encoded_bytes = encoded_bytes;
        self
    }

    /// Canonical length-prefixed bytes of this result.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        push_bytes(&mut bytes, RESULT_LABEL);
        push_u64(&mut bytes, ENCOUNTER_RECEIPT_SCHEMA_VERSION);
        push_str(&mut bytes, ENCOUNTER_TOOL_LISTEN_ROOM);
        push_str(&mut bytes, &self.room);
        push_f64(&mut bytes, self.t);
        push_u64(&mut bytes, self.variation);
        push_f64(&mut bytes, self.duration_seconds);
        push_u64(&mut bytes, self.note_count);
        push_u64(&mut bytes, self.returned_note_count);
        bytes.push(u8::from(self.truncated));
        push_option_str(&mut bytes, self.motif_key.as_deref());
        push_option_u64(&mut bytes, self.motif_tempo);
        push_option_str(&mut bytes, self.motif_encodes.as_deref());
        push_option_f64(&mut bytes, self.bed_duration_seconds);
        push_option_u64(&mut bytes, self.bed_event_count);
        push_option_u64(&mut bytes, self.audio_encoded_bytes);
        bytes
    }
}

/// A closed sing_expression action after Studio defaults resolve.
#[derive(Debug, Clone, PartialEq)]
pub struct SingExpressionAction {
    expr: String,
    xmin: f64,
    xmax: f64,
    a: f64,
    notes: u64,
    audio: bool,
}

impl SingExpressionAction {
    /// Builds a sing with public Studio defaults.
    #[must_use]
    pub fn new(expr: &str) -> Self {
        Self {
            expr: expr.to_string(),
            xmin: DEFAULT_STUDIO_XMIN,
            xmax: DEFAULT_STUDIO_XMAX,
            a: DEFAULT_STUDIO_PARAMETER,
            notes: DEFAULT_MELODY_NOTES as u64,
            audio: false,
        }
    }

    /// Expression window and parameter.
    #[must_use]
    pub fn with_window(mut self, xmin: f64, xmax: f64, a: f64) -> Self {
        self.xmin = xmin;
        self.xmax = xmax;
        self.a = a;
        self
    }

    /// Note count.
    #[must_use]
    pub fn with_notes(mut self, notes: u64) -> Self {
        self.notes = notes;
        self
    }

    /// Whether the caller asked for a WAV.
    #[must_use]
    pub fn with_audio(mut self, audio: bool) -> Self {
        self.audio = audio;
        self
    }

    /// Source expression.
    #[must_use]
    pub fn expr(&self) -> &str {
        &self.expr
    }

    /// Left window edge.
    #[must_use]
    pub fn xmin(&self) -> f64 {
        self.xmin
    }

    /// Right window edge.
    #[must_use]
    pub fn xmax(&self) -> f64 {
        self.xmax
    }

    /// Studio parameter `a`.
    #[must_use]
    pub fn a(&self) -> f64 {
        self.a
    }

    /// Note count.
    #[must_use]
    pub fn notes(&self) -> u64 {
        self.notes
    }

    /// Whether a WAV was requested.
    #[must_use]
    pub fn audio(&self) -> bool {
        self.audio
    }

    /// Canonical length-prefixed bytes of this action.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        push_bytes(&mut bytes, ACTION_LABEL);
        push_u64(&mut bytes, ENCOUNTER_RECEIPT_SCHEMA_VERSION);
        push_str(&mut bytes, ENCOUNTER_TOOL_SING_EXPRESSION);
        push_str(&mut bytes, &self.expr);
        push_f64(&mut bytes, self.xmin);
        push_f64(&mut bytes, self.xmax);
        push_f64(&mut bytes, self.a);
        push_u64(&mut bytes, self.notes);
        bytes.push(u8::from(self.audio));
        bytes
    }
}

/// Domain fields of one sing_expression result that a receipt may bind.
#[derive(Debug, Clone, PartialEq)]
pub struct SingExpressionResult {
    expr: String,
    duration_seconds: f64,
    note_count: u64,
    audio_encoded_bytes: Option<u64>,
}

impl SingExpressionResult {
    /// Builds a sing result for this expression.
    #[must_use]
    pub fn new(expr: &str) -> Self {
        Self {
            expr: expr.to_string(),
            duration_seconds: 0.0,
            note_count: 0,
            audio_encoded_bytes: None,
        }
    }

    /// Melody duration in seconds.
    #[must_use]
    pub fn with_duration_seconds(mut self, duration_seconds: f64) -> Self {
        self.duration_seconds = duration_seconds;
        self
    }

    /// How many notes were sung.
    #[must_use]
    pub fn with_note_count(mut self, note_count: u64) -> Self {
        self.note_count = note_count;
        self
    }

    /// Encoded WAV size, when audio was sent. Never the samples.
    #[must_use]
    pub fn with_audio_encoded_bytes(mut self, encoded_bytes: Option<u64>) -> Self {
        self.audio_encoded_bytes = encoded_bytes;
        self
    }

    /// Canonical length-prefixed bytes of this result.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        push_bytes(&mut bytes, RESULT_LABEL);
        push_u64(&mut bytes, ENCOUNTER_RECEIPT_SCHEMA_VERSION);
        push_str(&mut bytes, ENCOUNTER_TOOL_SING_EXPRESSION);
        push_str(&mut bytes, &self.expr);
        push_f64(&mut bytes, self.duration_seconds);
        push_u64(&mut bytes, self.note_count);
        push_option_u64(&mut bytes, self.audio_encoded_bytes);
        bytes
    }
}

impl EncounterReceipt {
    /// Builds a receipt. The package version must be nonempty ASCII.
    #[must_use]
    pub fn new(
        replay_abi_version: u16,
        fingerprint: [u8; 32],
        tool: EncounterTool,
        action_digest: [u8; 32],
        result_digest: [u8; 32],
        package_version: &str,
        build_semantic_id: [u8; 32],
    ) -> Option<Self> {
        (!package_version.is_empty() && package_version.is_ascii()).then(|| Self {
            replay_abi_version,
            fingerprint,
            tool,
            action_digest,
            result_digest,
            package_version: package_version.to_string(),
            build_semantic_id,
        })
    }

    /// Schema name.
    #[must_use]
    pub const fn schema(&self) -> &'static str {
        ENCOUNTER_RECEIPT_SCHEMA
    }

    /// Schema version.
    #[must_use]
    pub const fn schema_version(&self) -> u64 {
        ENCOUNTER_RECEIPT_SCHEMA_VERSION
    }

    /// Replay ABI copied from the live broadcast identity.
    #[must_use]
    pub const fn replay_abi_version(&self) -> u16 {
        self.replay_abi_version
    }

    /// Compatibility fingerprint bytes.
    #[must_use]
    pub const fn fingerprint(&self) -> &[u8; 32] {
        &self.fingerprint
    }

    /// Tool that produced the play.
    #[must_use]
    pub const fn tool(&self) -> EncounterTool {
        self.tool
    }

    /// Digest of the canonical action tuple.
    #[must_use]
    pub const fn action_digest(&self) -> &[u8; 32] {
        &self.action_digest
    }

    /// Digest of the canonical result tuple.
    #[must_use]
    pub const fn result_digest(&self) -> &[u8; 32] {
        &self.result_digest
    }

    /// Package version string, for example `0.4.0-alpha.9`.
    #[must_use]
    pub fn package_version(&self) -> &str {
        &self.package_version
    }

    /// Build-semantic identity bytes.
    #[must_use]
    pub const fn build_semantic_id(&self) -> &[u8; 32] {
        &self.build_semantic_id
    }

    /// Canonical length-prefixed bytes of this receipt.
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        push_bytes(&mut bytes, RECEIPT_LABEL);
        push_u64(&mut bytes, ENCOUNTER_RECEIPT_SCHEMA_VERSION);
        bytes.extend_from_slice(&self.replay_abi_version.to_le_bytes());
        bytes.extend_from_slice(&self.fingerprint);
        push_str(&mut bytes, self.tool.name());
        bytes.extend_from_slice(&self.action_digest);
        bytes.extend_from_slice(&self.result_digest);
        push_str(&mut bytes, &self.package_version);
        bytes.extend_from_slice(&self.build_semantic_id);
        bytes
    }
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_f64(bytes: &mut Vec<u8>, value: f64) {
    push_u64(bytes, value.to_bits());
}

fn push_bytes(bytes: &mut Vec<u8>, value: &[u8]) {
    push_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value);
}

fn push_str(bytes: &mut Vec<u8>, value: &str) {
    push_bytes(bytes, value.as_bytes());
}

fn push_option_str(bytes: &mut Vec<u8>, value: Option<&str>) {
    match value {
        None => bytes.push(0),
        Some(text) => {
            bytes.push(1);
            push_str(bytes, text);
        }
    }
}

fn push_option_f64(bytes: &mut Vec<u8>, value: Option<f64>) {
    match value {
        None => bytes.push(0),
        Some(number) => {
            bytes.push(1);
            push_f64(bytes, number);
        }
    }
}

fn push_option_u64(bytes: &mut Vec<u8>, value: Option<u64>) {
    match value {
        None => bytes.push(0),
        Some(number) => {
            bytes.push(1);
            push_u64(bytes, number);
        }
    }
}

fn push_option_delta(bytes: &mut Vec<u8>, value: Option<EncounterDeltaCounts>) {
    match value {
        None => bytes.push(0),
        Some(delta) => {
            bytes.push(1);
            push_u64(bytes, delta.cells_changed);
            push_u64(bytes, delta.ink_added);
            push_u64(bytes, delta.ink_removed);
            push_u64(bytes, delta.ink_reshaped);
            push_u64(bytes, delta.total_cells);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ENCOUNTER_RECEIPT_SCHEMA, ENCOUNTER_RECEIPT_SCHEMA_VERSION, EncounterDeltaCounts,
        EncounterReceipt, EncounterTool, ListenRoomAction, PLAY_ROOM_DEFAULT_HEIGHT,
        PLAY_ROOM_DEFAULT_T, PLAY_ROOM_DEFAULT_VARIATION, PLAY_ROOM_DEFAULT_WIDTH, PlayRoomAction,
        PlayRoomResult, SingExpressionAction,
    };
    use crate::roadmap_decisions;

    #[test]
    fn schema_name_matches_the_temporal_evidence_style() {
        assert_eq!(ENCOUNTER_RECEIPT_SCHEMA, "numinous.encounter-receipt");
        assert_eq!(ENCOUNTER_RECEIPT_SCHEMA_VERSION, 1);
        assert!(
            !roadmap_decisions().contains("numinous.encounter-receipt"),
            "a built schema does not belong in the waiting-on list"
        );
    }

    #[test]
    fn omitted_defaults_match_explicit_public_defaults() {
        let omitted = PlayRoomAction::new("times-tables");
        let explicit = PlayRoomAction::new("times-tables")
            .with_t(PLAY_ROOM_DEFAULT_T)
            .with_size(PLAY_ROOM_DEFAULT_WIDTH, PLAY_ROOM_DEFAULT_HEIGHT)
            .with_variation(PLAY_ROOM_DEFAULT_VARIATION)
            .with_from_t(None)
            .with_dwell(None)
            .with_pokes(Vec::new())
            .with_gesture(Vec::new())
            .with_aha_summon(false);
        assert_eq!(omitted.canonical_bytes(), explicit.canonical_bytes());
    }

    #[test]
    fn room_aliases_resolve_before_the_tuple_is_written() {
        let areas = PlayRoomAction::new("kepler-areas");
        let laws = PlayRoomAction::new("kepler-laws");
        assert_eq!(areas.canonical_bytes(), laws.canonical_bytes());
    }

    #[test]
    fn changing_phase_poke_or_wager_changes_the_action_bytes() {
        let base = PlayRoomAction::new("times-tables");
        let phase = PlayRoomAction::new("times-tables").with_t(0.35);
        let poke = PlayRoomAction::new("times-tables").with_pokes(vec![(0.2, 0.8)]);
        let wager =
            PlayRoomAction::new("times-tables").with_place_wager(Some("mandelbrot".to_string()));
        assert_ne!(base.canonical_bytes(), phase.canonical_bytes());
        assert_ne!(base.canonical_bytes(), poke.canonical_bytes());
        assert_ne!(base.canonical_bytes(), wager.canonical_bytes());
        assert_ne!(phase.canonical_bytes(), poke.canonical_bytes());
    }

    #[test]
    fn result_bytes_bind_counts_and_ignore_absent_prose_slots() {
        let quiet = PlayRoomResult::new("times-tables");
        let counted = PlayRoomResult::new("times-tables").with_delta(Some(EncounterDeltaCounts {
            cells_changed: 4,
            ink_added: 2,
            ink_removed: 1,
            ink_reshaped: 1,
            total_cells: 2304,
        }));
        assert_ne!(quiet.canonical_bytes(), counted.canonical_bytes());
        assert!(
            !String::from_utf8_lossy(&counted.canonical_bytes()).contains("Status:"),
            "result bytes must not carry content prose"
        );
        let closed = PlayRoomResult::new("times-tables").with_aha_allow_reveal(Some(false));
        let opened = PlayRoomResult::new("times-tables").with_aha_allow_reveal(Some(true));
        assert_ne!(quiet.canonical_bytes(), closed.canonical_bytes());
        assert_ne!(closed.canonical_bytes(), opened.canonical_bytes());
    }

    #[test]
    fn listen_and_sing_omitted_defaults_match_explicit_public_defaults() {
        use crate::studio_request::{
            DEFAULT_MELODY_NOTES, DEFAULT_STUDIO_PARAMETER, DEFAULT_STUDIO_XMAX,
            DEFAULT_STUDIO_XMIN,
        };
        let listen = ListenRoomAction::new("times-tables");
        let listen_explicit = ListenRoomAction::new("times-tables")
            .with_t(PLAY_ROOM_DEFAULT_T)
            .with_variation(PLAY_ROOM_DEFAULT_VARIATION)
            .with_ambient_events(false)
            .with_audio(false);
        assert_eq!(listen.canonical_bytes(), listen_explicit.canonical_bytes());
        let sing = SingExpressionAction::new("sin(x)");
        let sing_explicit = SingExpressionAction::new("sin(x)")
            .with_window(
                DEFAULT_STUDIO_XMIN,
                DEFAULT_STUDIO_XMAX,
                DEFAULT_STUDIO_PARAMETER,
            )
            .with_notes(DEFAULT_MELODY_NOTES as u64)
            .with_audio(false);
        assert_eq!(sing.canonical_bytes(), sing_explicit.canonical_bytes());
        assert_ne!(
            ListenRoomAction::new("times-tables")
                .with_audio(true)
                .canonical_bytes(),
            listen.canonical_bytes()
        );
    }

    #[test]
    fn two_identical_receipts_write_the_same_bytes() {
        let first = EncounterReceipt::new(
            1,
            [7; 32],
            EncounterTool::PlayRoom,
            [3; 32],
            [9; 32],
            "0.4.0-alpha.9",
            [11; 32],
        )
        .expect("valid receipt");
        let second = EncounterReceipt::new(
            1,
            [7; 32],
            EncounterTool::PlayRoom,
            [3; 32],
            [9; 32],
            "0.4.0-alpha.9",
            [11; 32],
        )
        .expect("valid receipt");
        assert_eq!(first, second);
        assert_eq!(first.canonical_bytes(), second.canonical_bytes());
        assert_eq!(first.schema(), ENCOUNTER_RECEIPT_SCHEMA);
        assert_eq!(first.tool().name(), "play_room");
        assert_eq!(
            EncounterTool::from_name("listen_room"),
            Some(EncounterTool::ListenRoom)
        );
        assert_eq!(
            EncounterTool::from_name("sing_expression"),
            Some(EncounterTool::SingExpression)
        );
        assert!(
            EncounterReceipt::new(
                1,
                [0; 32],
                EncounterTool::PlayRoom,
                [0; 32],
                [0; 32],
                "",
                [0; 32],
            )
            .is_none()
        );
        assert!(
            EncounterReceipt::new(
                1,
                [0; 32],
                EncounterTool::PlayRoom,
                [0; 32],
                [0; 32],
                "0.4.0-α",
                [0; 32],
            )
            .is_none()
        );
    }
}

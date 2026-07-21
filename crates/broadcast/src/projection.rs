use crate::{Compatibility, CompatibilityError};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::error::Error;
use std::fmt;

/// Maximum requested ASCII width retained in a public `play_room` action.
pub const PLAY_ROOM_MAX_WIDTH: u64 = 512;

/// Maximum requested ASCII height retained in a public `play_room` action.
pub const PLAY_ROOM_MAX_HEIGHT: u64 = 256;

/// Stable game identities whose replays may cross the local broadcast seam.
pub const NUMINOUS_GAME_IDS: [&str; 11] = [
    "aliens",
    "crack",
    "fifteen",
    "gauntlet",
    "hackenbush",
    "munch",
    "munch_arcade",
    "nim",
    "party",
    "quiz",
    "seti",
];

/// Every MCP tool whose action and public result may enter a viewer session.
pub const ALL_PUBLIC_TOOLS: [PublicTool; 23] = [
    PublicTool::ListRooms,
    PublicTool::DescribeRoom,
    PublicTool::RevealRoom,
    PublicTool::PlayRoom,
    PublicTool::Challenge,
    PublicTool::Predict,
    PublicTool::ListenRoom,
    PublicTool::ListSims,
    PublicTool::RunSim,
    PublicTool::PlotExpression,
    PublicTool::SingExpression,
    PublicTool::ExplainJoke,
    PublicTool::Munch,
    PublicTool::MunchArcade,
    PublicTool::Nim,
    PublicTool::Crack,
    PublicTool::Seti,
    PublicTool::Aliens,
    PublicTool::Gauntlet,
    PublicTool::Hackenbush,
    PublicTool::Party,
    PublicTool::Fifteen,
    PublicTool::Quiz,
];

/// An explicitly allowlisted replayable MCP action family.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicTool {
    /// Read the room catalog.
    ListRooms,
    /// Read one room description.
    DescribeRoom,
    /// Read one room insight.
    RevealRoom,
    /// Render and interact with one room.
    PlayRoom,
    /// Pose or grade one room challenge.
    Challenge,
    /// Pose or grade one prediction.
    Predict,
    /// Inspect one room's mathematical sound.
    ListenRoom,
    /// Read the simulation catalog.
    ListSims,
    /// Run one bounded simulation.
    RunSim,
    /// Render one bounded Studio expression.
    PlotExpression,
    /// Project one bounded Studio expression as notation.
    SingExpression,
    /// Inspect one built-in joke.
    ExplainJoke,
    /// Play Munch.
    Munch,
    /// Replay Munch Arcade.
    MunchArcade,
    /// Play Nim.
    Nim,
    /// Play Crack the Code.
    Crack,
    /// Play SETI.
    Seti,
    /// Play Aliens.
    Aliens,
    /// Play the Gauntlet.
    Gauntlet,
    /// Play Hackenbush.
    Hackenbush,
    /// Play Party.
    Party,
    /// Play Fifteen's Bet.
    Fifteen,
    /// Play Guess the Shape.
    Quiz,
}

impl PublicTool {
    /// Resolves an exact allowlisted MCP tool name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        ALL_PUBLIC_TOOLS
            .iter()
            .copied()
            .find(|tool| tool.name() == name)
    }

    /// Returns the stable MCP tool name carried by replay evidence.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ListRooms => "list_rooms",
            Self::DescribeRoom => "describe_room",
            Self::RevealRoom => "reveal_room",
            Self::PlayRoom => "play_room",
            Self::Challenge => "challenge",
            Self::Predict => "predict",
            Self::ListenRoom => "listen_room",
            Self::ListSims => "list_sims",
            Self::RunSim => "run_sim",
            Self::PlotExpression => "plot_expression",
            Self::SingExpression => "sing_expression",
            Self::ExplainJoke => "explain_joke",
            Self::Munch => "munch",
            Self::MunchArcade => "munch_arcade",
            Self::Nim => "nim",
            Self::Crack => "crack",
            Self::Seti => "seti",
            Self::Aliens => "aliens",
            Self::Gauntlet => "gauntlet",
            Self::Hackenbush => "hackenbush",
            Self::Party => "party",
            Self::Fifteen => "fifteen",
            Self::Quiz => "quiz",
        }
    }
}

/// One self-contained public action and the result returned to its guest.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicToolEvent {
    /// Explicitly allowlisted tool identity.
    pub tool: PublicTool,
    /// Validated replay arguments without JSON-RPC metadata.
    pub arguments: Map<String, Value>,
    /// Public MCP tool result without its JSON-RPC request identity.
    pub result: Map<String, Value>,
}

impl PublicToolEvent {
    /// Copies one validated public projection into its strict wire shape.
    pub fn new(
        tool: PublicTool,
        arguments: &Value,
        result: &Value,
    ) -> Result<Self, ProjectionError> {
        let arguments = arguments
            .as_object()
            .cloned()
            .ok_or(ProjectionError::ArgumentsNotObject)?;
        let result = result
            .as_object()
            .cloned()
            .ok_or(ProjectionError::ResultNotObject)?;
        Ok(Self {
            tool,
            arguments,
            result,
        })
    }
}

/// Builds the shared App and MCP replay identity from current public catalogs.
pub fn numinous_compatibility() -> Result<Compatibility, CompatibilityError> {
    let rooms = numinous_core::all_rooms();
    let simulations = numinous_core::all_sims();
    Compatibility::from_catalogs(
        rooms.iter().map(|room| room.meta().id),
        simulations.iter().map(|simulation| simulation.meta().id),
        NUMINOUS_GAME_IDS,
    )
}

/// A malformed internal public projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionError {
    /// Validated tool arguments were not represented as an object.
    ArgumentsNotObject,
    /// The MCP tool result was not represented as an object.
    ResultNotObject,
}

impl fmt::Display for ProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArgumentsNotObject => formatter.write_str("public arguments must be an object"),
            Self::ResultNotObject => formatter.write_str("public result must be an object"),
        }
    }
}

impl Error for ProjectionError {}

#[cfg(test)]
mod tests {
    use super::{ALL_PUBLIC_TOOLS, PublicTool, PublicToolEvent, numinous_compatibility};
    use serde_json::json;
    use std::collections::HashSet;

    #[test]
    fn public_tool_names_are_unique_and_resolve_exactly() {
        let names: HashSet<_> = ALL_PUBLIC_TOOLS.iter().map(|tool| tool.name()).collect();
        assert_eq!(names.len(), ALL_PUBLIC_TOOLS.len());
        for tool in ALL_PUBLIC_TOOLS {
            assert_eq!(PublicTool::from_name(tool.name()), Some(tool));
        }
        assert_eq!(PublicTool::from_name("journey"), None);
        assert_eq!(PublicTool::from_name("PLAY_ROOM"), None);
    }

    #[test]
    fn public_events_require_object_boundaries_and_reject_unknown_wire_fields() {
        let event = PublicToolEvent::new(
            PublicTool::PlayRoom,
            &json!({"id": "times-tables", "t": 0.25}),
            &json!({"content": [], "structuredContent": {"room": "times-tables"}}),
        )
        .expect("object event");
        let encoded = serde_json::to_value(&event).expect("serialize event");
        assert_eq!(encoded["tool"], "play_room");
        assert!(PublicToolEvent::new(PublicTool::PlayRoom, &json!([]), &json!({})).is_err());
        assert!(PublicToolEvent::new(PublicTool::PlayRoom, &json!({}), &json!(null)).is_err());

        let mut unknown = encoded;
        unknown["privateTrace"] = json!("hidden");
        assert!(serde_json::from_value::<PublicToolEvent>(unknown).is_err());
    }

    #[test]
    fn current_numinous_catalogs_form_one_nonempty_compatibility_identity() {
        let first = numinous_compatibility().expect("catalog identity");
        let second = numinous_compatibility().expect("same catalog identity");
        assert_eq!(first, second);
        assert!(!first.fingerprint.as_bytes().iter().all(|byte| *byte == 0));
    }
}

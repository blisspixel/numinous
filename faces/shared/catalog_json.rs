//! Optional room-list fields shared by the CLI and MCP catalogs.

use numinous_core::RoomSignposts;
use serde_json::Value;

/// Unmarked rooms retain their original row shape.
pub(crate) fn mark_room(row: &mut Value, id: &str) {
    let signs = RoomSignposts::for_room(id);
    if signs.front_hall {
        row["frontHall"] = Value::Bool(true);
    }
    if signs.authored_mathematics {
        row["authoredMathematics"] = Value::Bool(true);
    }
}

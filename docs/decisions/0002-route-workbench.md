# Route state and caller-paced calculation

Date: 2026-09-30.

## Context

The opening Route Lab example compares routes through the ordinary room replay
interface. Custom street networks need editable roads, deliveries, undo, and
recorded solver decisions. A closed street can disconnect a required delivery;
the editor must retain that map so a player can inspect and repair it.

`RouteProblem` promises a feasible, bounded, immutable problem. Studio capsules
describe expressions and sampling windows, not street graphs. Neither contract
should silently change to accommodate an editor.

## Decision

Core owns a separate bounded workbench. Its network snapshots preserve road
identity and cost when a road closes. Structural validation rejects malformed
IDs, duplicate roads and stops, invalid orders, and resource bounds. Feasibility
is a separate calculation: a structurally valid edited network can be infeasible
and still have a usable snapshot and undo history.

Successful state changes advance a checked revision, preserve the player's
candidate where valid, and invalidate prior traces. Undo restores a previous
network under a new revision. A no-op does not create a new historical edit.
The history is bounded; an old entry leaving that window is not a persisted
project or a promise of unlimited undo.

Shortest-path playback records actual settling and strict relaxation decisions.
A cursor selects a prefix of that sequence. Moving the cursor does not rerun
the search as though it performed less work, change the route, or auto-advance.
Imported trace metadata binds both the current revision and a canonical network
digest; the events are regenerated against those roads. An inconsistent cursor
is refused. A caller can deliberately recompute the digest, so this binding
detects inconsistent state and does not attest authorship or history.

The CLI and MCP translate one shared JSON shape to these core types. Requests
carry an optional snapshot and an explicit action. Responses return the updated
snapshot, typed feasibility, route comparisons, and any requested trace. There
is no hidden server session, filesystem lookup, map service, or account behind
this interface. Unknown fields and unsupported actions are errors rather than
silent defaults.

The App retains its fixed delivery example and uses the same workbench for
supported edits and caller-paced trace. Its bounded room input history can include private
state checkpoints so long sessions preserve the exact state and bounded undo.
Those checkpoints are reconstructed state, not evidence of physical gestures.

## Consequences

All faces calculate costs and accepted changes in the same owner. A network with
unreachable required stops remains inspectable. An isolated unused junction does
not prevent a round-trip comparison. Playback and mathematical work have
distinct meanings, and malformed or stale trace metadata cannot supply invented events.

A caller can carry the returned JSON into another request or process. This is
snapshot continuation. [Portable route creations](0003-route-creations.md)
build the separate keep, reopen, remix, and question contract on the existing
project chain. Native authoring owns the same workbench directly rather than
extending the fixed room's private replay checkpoint into an editor format.

Implementation and behavioral evidence live in `route_workbench.rs`, the shared
face adapter, and the App, CLI, and MCP regressions. [Route Lab](../ROUTE_LAB.md)
owns the player-facing status and the remaining scope.
